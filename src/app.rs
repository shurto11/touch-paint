//! アプリ状態の集約。タッチイベントを受けてキャンバス描画と fb 転送を行う。

use crate::canvas::Canvas;
use crate::fb::Framebuffer;
use crate::geom::Rect;
use crate::history::History;
use crate::png;
use crate::protocol::Event;
use crate::toolbar::{Action, Toolbar};
use crate::tools::ToolState;
use ab_glyph::FontVec;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const HISTORY_CAP: usize = 20;
const BG: (u8, u8, u8) = (0, 0, 0);

pub struct App {
    fb: Framebuffer,
    font: Arc<FontVec>,
    canvas: Canvas,
    tools: ToolState,
    toolbar: Toolbar,
    toolbar_buf: Vec<u8>,
    history: History,
    open_path: Option<PathBuf>,
    save_dir: PathBuf,
    painting: bool,
    in_toolbar: bool,
    last: (i32, i32),
    term: Arc<AtomicBool>,
    /// fb-server から通知された表示可否("touch-paint"シーンが非アクティブなら false)。
    visible: bool,
}

impl App {
    pub fn new(
        fb: Framebuffer,
        font: Arc<FontVec>,
        open_path: Option<PathBuf>,
        term: Arc<AtomicBool>,
    ) -> Result<Self> {
        let toolbar = Toolbar::new(fb.width, fb.height);
        let canvas_h = fb.height.saturating_sub(toolbar.h);
        let mut canvas = Canvas::new(fb.width, canvas_h, BG);
        let toolbar_buf = vec![0u8; (fb.width * toolbar.h * 4) as usize];

        // 起動時に CLI 引数の PNG をロード
        if let Some(p) = &open_path {
            if let Err(e) = png::load_into(&mut canvas, p, BG) {
                eprintln!("touch-paint: {e:#}");
            }
        }

        // 保存先は専用フォルダにまとめる。TOUCH_PAINT_DIR があればそこ自体、
        // 無ければ cwd 下に touch-paint-images/ を作る。
        let save_dir = match std::env::var("TOUCH_PAINT_DIR") {
            Ok(d) if !d.is_empty() => PathBuf::from(d),
            _ => std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("touch-paint-images"),
        };

        Ok(Self {
            fb,
            font,
            canvas,
            tools: ToolState::new(),
            toolbar,
            toolbar_buf,
            history: History::new(HISTORY_CAP),
            open_path,
            save_dir,
            painting: false,
            in_toolbar: false,
            last: (0, 0),
            term,
            visible: true,
        })
    }

    /// fb-server からの可視性通知を反映する。非表示化した瞬間は画面全体をクリアし、
    /// 再表示時は全面描画し直す。
    pub fn set_visible(&mut self, v: bool) -> Result<()> {
        if v == self.visible {
            return Ok(());
        }
        self.visible = v;
        if v {
            self.full_render()?;
        } else {
            self.fb.clear(0, 0, self.fb.width, self.fb.height)?;
        }
        Ok(())
    }

    /// ツールバーとキャンバスを全面描画する（起動時・undo/redo・open後）。
    pub fn full_render(&mut self) -> Result<()> {
        self.toolbar.render(&mut self.toolbar_buf, &self.tools, &self.font);
        self.fb.blit(0, 0, self.fb.width, self.toolbar.h, &self.toolbar_buf)?;
        self.blit_canvas_all()?;
        Ok(())
    }

    fn blit_canvas_all(&self) -> Result<()> {
        if !self.visible {
            return Ok(());
        }
        self.fb.blit(0, self.toolbar.h, self.canvas.w, self.canvas.h, &self.canvas.buf)?;
        Ok(())
    }

    fn render_toolbar(&mut self) -> Result<()> {
        if !self.visible {
            return Ok(());
        }
        self.toolbar.render(&mut self.toolbar_buf, &self.tools, &self.font);
        self.fb.blit(0, 0, self.fb.width, self.toolbar.h, &self.toolbar_buf)?;
        Ok(())
    }

    /// キャンバス座標系のダーティ矩形を fb へ転送する。
    fn blit_dirty(&self, r: Rect) -> Result<()> {
        if !self.visible || r.is_empty() {
            return Ok(());
        }
        let x = r.x.max(0) as u32;
        let y = r.y.max(0) as u32;
        let x1 = ((r.x + r.w) as u32).min(self.canvas.w);
        let y1 = ((r.y + r.h) as u32).min(self.canvas.h);
        if x1 <= x || y1 <= y {
            return Ok(());
        }
        self.fb.blit_region(
            x,
            y + self.toolbar.h,
            x1 - x,
            y1 - y,
            &self.canvas.buf,
            self.canvas.w,
            x,
            y,
        )
    }

    fn px(&self, fx: f64, fy: f64) -> (i32, i32) {
        let x = (fx * self.fb.width as f64).round() as i32;
        let y = (fy * self.fb.height as f64).round() as i32;
        (x, y)
    }

    pub fn handle(&mut self, ev: Event) -> Result<()> {
        match ev {
            Event::Down { fx, fy, .. } => {
                let (px, py) = self.px(fx, fy);
                if py < self.toolbar.h as i32 {
                    self.in_toolbar = true;
                    if let Some(action) = self.toolbar.hit(px, py) {
                        self.do_action(action)?;
                    }
                } else {
                    self.painting = true;
                    self.in_toolbar = false;
                    self.history.begin(&self.canvas);
                    let cy = py - self.toolbar.h as i32;
                    let color = self.tools.color(BG);
                    let dirty = self.canvas.stamp(px, cy, self.tools.radius(), color);
                    self.last = (px, cy);
                    self.blit_dirty(dirty)?;
                }
            }
            Event::Move { fx, fy, .. } => {
                if self.painting && !self.in_toolbar {
                    let (px, py) = self.px(fx, fy);
                    let cy = py - self.toolbar.h as i32;
                    let color = self.tools.color(BG);
                    let (lx, ly) = self.last;
                    let dirty = self.canvas.stroke_segment(lx, ly, px, cy, self.tools.radius(), color);
                    self.last = (px, cy);
                    self.blit_dirty(dirty)?;
                }
            }
            Event::Up { .. } => {
                self.painting = false;
                self.in_toolbar = false;
            }
        }
        Ok(())
    }

    fn do_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Color(i) => {
                self.tools.color_idx = i;
                self.tools.eraser = false;
                self.render_toolbar()?;
            }
            Action::Size(i) => {
                self.tools.size_idx = i;
                self.render_toolbar()?;
            }
            Action::Eraser => {
                self.tools.eraser = !self.tools.eraser;
                self.render_toolbar()?;
            }
            Action::Undo => {
                if self.history.undo(&mut self.canvas) {
                    self.blit_canvas_all()?;
                }
            }
            Action::Redo => {
                if self.history.redo(&mut self.canvas) {
                    self.blit_canvas_all()?;
                }
            }
            Action::Save => match png::save(&self.canvas, &self.save_dir) {
                Ok(path) => eprintln!("touch-paint: 保存しました {path:?}"),
                Err(e) => eprintln!("touch-paint: 保存失敗 {e:#}"),
            },
            Action::Open => {
                if let Some(p) = self.open_path.clone() {
                    match png::load_into(&mut self.canvas, &p, BG) {
                        Ok(()) => {
                            self.history.clear();
                            self.blit_canvas_all()?;
                        }
                        Err(e) => eprintln!("touch-paint: {e:#}"),
                    }
                } else {
                    eprintln!("touch-paint: 開くファイルが指定されていません(起動時に引数で指定)");
                }
            }
            Action::Quit => {
                self.term.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    /// 終了時にフレームバッファを全クリアする。
    pub fn shutdown(&self) {
        let _ = self.fb.clear(0, 0, self.fb.width, self.fb.height);
    }
}
