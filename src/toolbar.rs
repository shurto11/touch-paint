//! 画面上部のツールバー。ボタンのレイアウト・ヒットテスト・描画を担う。

use crate::geom::Rect;
use crate::text;
use crate::tools::{ToolState, BRUSH_RADII, PALETTE};
use ab_glyph::FontVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Color(usize),
    Size(usize),
    Eraser,
    Undo,
    Redo,
    Save,
    Open,
    Quit,
}

struct Button {
    rect: Rect,
    kind: Action,
}

pub struct Toolbar {
    pub h: u32,
    w: u32,
    btns: Vec<Button>,
}

/// 機能ボタン(色・太さ以外)のラベル。
const FUNC_BUTTONS: [(Action, &str); 6] = [
    (Action::Eraser, "消"),
    (Action::Undo, "<"),
    (Action::Redo, ">"),
    (Action::Save, "保存"),
    (Action::Open, "開く"),
    (Action::Quit, "x"),
];

impl Toolbar {
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        let h = (screen_h / 12).max(56);
        let n: u32 = (PALETTE.len() + BRUSH_RADII.len() + FUNC_BUTTONS.len()) as u32;
        let bw = h.min(screen_w / n) as i32;
        let bh = h as i32;
        let mut btns = Vec::new();
        let mut x = 0i32;
        for i in 0..PALETTE.len() {
            btns.push(Button { rect: Rect { x, y: 0, w: bw, h: bh }, kind: Action::Color(i) });
            x += bw;
        }
        for i in 0..BRUSH_RADII.len() {
            btns.push(Button { rect: Rect { x, y: 0, w: bw, h: bh }, kind: Action::Size(i) });
            x += bw;
        }
        for (kind, _) in FUNC_BUTTONS {
            btns.push(Button { rect: Rect { x, y: 0, w: bw, h: bh }, kind });
            x += bw;
        }
        Self { h, w: screen_w, btns }
    }

    /// ピクセル座標にあるボタンの Action を返す。
    pub fn hit(&self, px: i32, py: i32) -> Option<Action> {
        self.btns.iter().find(|b| b.rect.contains(px, py)).map(|b| b.kind)
    }

    /// ツールバーを buf(screen_w × h, BGRA) へ描画する。状態変化時のみ呼ぶ。
    pub fn render(&self, buf: &mut [u8], tools: &ToolState, font: &FontVec) {
        let w = self.w;
        let h = self.h;
        // 背景
        text::fill_rect(buf, w, h, 0, 0, w as i32, h as i32, (40, 40, 40));
        let pad = 4;
        for b in &self.btns {
            let r = b.rect;
            match b.kind {
                Action::Color(i) => {
                    let c = PALETTE[i];
                    text::fill_rect(buf, w, h, r.x + pad, r.y + pad, r.w - 2 * pad, r.h - 2 * pad, c);
                    if !tools.eraser && tools.color_idx == i {
                        text::draw_border(buf, w, h, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 3, (255, 255, 255));
                    }
                }
                Action::Size(i) => {
                    let cx = r.x + r.w / 2;
                    let cy = r.y + r.h / 2;
                    text::fill_circle(buf, w, h, cx, cy, BRUSH_RADII[i].min(r.h / 2 - pad), (220, 220, 220));
                    if tools.size_idx == i {
                        text::draw_border(buf, w, h, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 3, (255, 255, 255));
                    }
                }
                kind => {
                    let label = FUNC_BUTTONS.iter().find(|(k, _)| *k == kind).map(|(_, l)| *l).unwrap_or("");
                    let active = kind == Action::Eraser && tools.eraser;
                    if active {
                        text::fill_rect(buf, w, h, r.x + pad, r.y + pad, r.w - 2 * pad, r.h - 2 * pad, (90, 90, 90));
                    }
                    let px = (h as f32 * 0.5).max(18.0);
                    let tw = text::measure_text(font, label, px);
                    let tx = r.x as f32 + (r.w as f32 - tw) / 2.0;
                    let ty = r.y as f32 + (r.h as f32 - px) / 2.0;
                    text::draw_text(buf, w, h, tx, ty, label, font, px, (235, 235, 235));
                }
            }
        }
    }
}
