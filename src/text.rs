//! BGRAバッファへの描画プリミティブ。
//! draw_text / blend は dopagaki/src/overlay.rs から一般化して移植した。

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use anyhow::{Context, Result};
use std::process::Command;
use std::sync::Arc;

/// fc-matchでシステムのCJKフォントを探して読み込む(起動時に1回)。
pub fn load_font() -> Result<Arc<FontVec>> {
    let out = Command::new("fc-match")
        .args(["-f", "%{file}\n%{index}", "Noto Sans CJK JP"])
        .output()
        .context("fc-match実行失敗")?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut lines = s.lines();
    let file = lines.next().context("fc-match: フォントが見つかりません")?;
    let index: u32 = lines.next().and_then(|i| i.parse().ok()).unwrap_or(0);
    let data = std::fs::read(file).with_context(|| format!("フォント読み込み失敗: {file}"))?;
    let font = FontVec::try_from_vec_and_index(data, index)
        .with_context(|| format!("フォントパース失敗: {file}"))?;
    Ok(Arc::new(font))
}

/// BGRAバッファ(stride=w)の矩形を (b,g,r) で塗りつぶす。範囲外はクリップ。
pub fn fill_rect(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, rw: i32, rh: i32, c: (u8, u8, u8)) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + rw).min(w as i32);
    let y1 = (y + rh).min(h as i32);
    for py in y0..y1 {
        for px in x0..x1 {
            let i = ((py as u32 * w + px as u32) * 4) as usize;
            buf[i] = c.0;
            buf[i + 1] = c.1;
            buf[i + 2] = c.2;
            buf[i + 3] = 0;
        }
    }
}

/// 中心 (cx,cy) 半径 r の塗りつぶし円。
pub fn fill_circle(buf: &mut [u8], w: u32, h: u32, cx: i32, cy: i32, r: i32, c: (u8, u8, u8)) {
    let r2 = r * r;
    for py in (cy - r).max(0)..=(cy + r).min(h as i32 - 1) {
        for px in (cx - r).max(0)..=(cx + r).min(w as i32 - 1) {
            let dx = px - cx;
            let dy = py - cy;
            if dx * dx + dy * dy <= r2 {
                let i = ((py as u32 * w + px as u32) * 4) as usize;
                buf[i] = c.0;
                buf[i + 1] = c.1;
                buf[i + 2] = c.2;
                buf[i + 3] = 0;
            }
        }
    }
}

/// 太さ t の矩形枠を描く。
pub fn draw_border(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, rw: i32, rh: i32, t: i32, c: (u8, u8, u8)) {
    fill_rect(buf, w, h, x, y, rw, t, c); // 上
    fill_rect(buf, w, h, x, y + rh - t, rw, t, c); // 下
    fill_rect(buf, w, h, x, y, t, rh, c); // 左
    fill_rect(buf, w, h, x + rw - t, y, t, rh, c); // 右
}

fn text_width<F: Font, SF: ScaleFont<F>>(scaled: &SF, text: &str) -> f32 {
    text.chars().map(|c| scaled.h_advance(scaled.glyph_id(c))).sum()
}

/// テキストを (x,y) を左上として px サイズ・色 c で1行描画する。
pub fn draw_text(
    buf: &mut [u8],
    w: u32,
    h: u32,
    x: f32,
    y: f32,
    text: &str,
    font: &FontVec,
    px: f32,
    c: (u8, u8, u8),
) {
    let scaled = font.as_scaled(PxScale::from(px));
    let ascent = scaled.ascent();
    let mut cursor = x;
    for ch in text.chars() {
        let gid = scaled.glyph_id(ch);
        let advance = scaled.h_advance(gid);
        if cursor + advance >= 0.0 && cursor < w as f32 {
            let glyph = gid.with_scale_and_position(scaled.scale(), ab_glyph::point(cursor, y + ascent));
            if let Some(og) = scaled.font().outline_glyph(glyph) {
                let bounds = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    if cov < 0.05 {
                        return;
                    }
                    let bx = bounds.min.x as i64 + gx as i64;
                    let by = bounds.min.y as i64 + gy as i64;
                    blend(buf, w, h, bx, by, c.0, c.1, c.2, cov);
                });
            }
        }
        cursor += advance;
    }
}

/// テキストの描画幅(px)を測る。中央寄せに使う。
pub fn measure_text(font: &FontVec, text: &str, px: f32) -> f32 {
    let scaled = font.as_scaled(PxScale::from(px));
    text_width(&scaled, text)
}

#[inline]
fn blend(buf: &mut [u8], w: u32, h: u32, x: i64, y: i64, b: u8, g: u8, r: u8, cov: f32) {
    if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
        return;
    }
    let i = ((y as u32 * w + x as u32) * 4) as usize;
    let a = cov.clamp(0.0, 1.0);
    buf[i] = (buf[i] as f32 * (1.0 - a) + b as f32 * a) as u8;
    buf[i + 1] = (buf[i + 1] as f32 * (1.0 - a) + g as f32 * a) as u8;
    buf[i + 2] = (buf[i + 2] as f32 * (1.0 - a) + r as f32 * a) as u8;
}
