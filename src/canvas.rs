//! 描画キャンバス。描画領域のみを BGRA で保持し、fb と同フォーマットなので
//! 部分矩形をコピー無しで fb へ転送できる。

use crate::geom::Rect;
use image::RgbaImage;

pub struct Canvas {
    pub buf: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

impl Canvas {
    /// 背景色 bg=(B,G,R) で塗りつぶした空キャンバスを作る。
    pub fn new(w: u32, h: u32, bg: (u8, u8, u8)) -> Self {
        let mut c = Self { buf: vec![0u8; (w * h * 4) as usize], w, h };
        c.fill(bg);
        c
    }

    pub fn fill(&mut self, c: (u8, u8, u8)) {
        for px in self.buf.chunks_exact_mut(4) {
            px[0] = c.0;
            px[1] = c.1;
            px[2] = c.2;
            px[3] = 0;
        }
    }

    /// 半径 r の塗りつぶし円を (cx,cy) に1個押す。触れた bbox を返す。
    /// 範囲外は clamp するのでツールバー帯への侵入(cy<0)も安全。
    pub fn stamp(&mut self, cx: i32, cy: i32, r: i32, c: (u8, u8, u8)) -> Rect {
        let x0 = (cx - r).max(0);
        let y0 = (cy - r).max(0);
        let x1 = (cx + r).min(self.w as i32 - 1);
        let y1 = (cy + r).min(self.h as i32 - 1);
        if x1 < x0 || y1 < y0 {
            return Rect::EMPTY;
        }
        let r2 = r * r;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= r2 {
                    let i = ((y as u32 * self.w + x as u32) * 4) as usize;
                    self.buf[i] = c.0;
                    self.buf[i + 1] = c.1;
                    self.buf[i + 2] = c.2;
                    self.buf[i + 3] = 0;
                }
            }
        }
        Rect::from_bounds(x0, y0, x1, y1)
    }

    /// p0→p1 を spacing 間隔で補間スタンプし、bbox の union を返す。
    pub fn stroke_segment(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        r: i32,
        c: (u8, u8, u8),
    ) -> Rect {
        let dist = (((x1 - x0).pow(2) + (y1 - y0).pow(2)) as f32).sqrt();
        let spacing = (r as f32 * 0.5).max(1.0);
        let n = (dist / spacing).ceil().max(1.0) as i32;
        let mut acc = Rect::EMPTY;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let x = (x0 as f32 + (x1 - x0) as f32 * t).round() as i32;
            let y = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
            acc = acc.union(self.stamp(x, y, r, c));
        }
        acc
    }

    /// BGRA → RGBA(不透明) 変換。PNG 保存用。
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut o = vec![0u8; self.buf.len()];
        for (s, d) in self.buf.chunks_exact(4).zip(o.chunks_exact_mut(4)) {
            d[0] = s[2]; // R
            d[1] = s[1]; // G
            d[2] = s[0]; // B
            d[3] = 255;
        }
        o
    }

    /// RGBA画像をアスペクト維持でキャンバスにフィットさせ、背景 bg で
    /// レターボックスして書き込む。
    pub fn load_rgba_fit(&mut self, img: &RgbaImage, bg: (u8, u8, u8)) {
        self.fill(bg);
        let (sw, sh) = img.dimensions();
        if sw == 0 || sh == 0 {
            return;
        }
        let scale = f32::min(self.w as f32 / sw as f32, self.h as f32 / sh as f32);
        let fw = ((sw as f32 * scale) as u32).clamp(1, self.w);
        let fh = ((sh as f32 * scale) as u32).clamp(1, self.h);
        let resized = image::imageops::resize(img, fw, fh, image::imageops::FilterType::Triangle);
        let ox = (self.w - fw) / 2;
        let oy = (self.h - fh) / 2;
        for y in 0..fh {
            for x in 0..fw {
                let p = resized.get_pixel(x, y).0; // RGBA
                let i = (((oy + y) * self.w + (ox + x)) * 4) as usize;
                self.buf[i] = p[2]; // B
                self.buf[i + 1] = p[1]; // G
                self.buf[i + 2] = p[0]; // R
                self.buf[i + 3] = 0;
            }
        }
    }
}
