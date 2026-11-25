use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;

/// /dev/fb0 への直接描画。fbterm環境ではstride(1376px)≠width(1366px)なので
/// 行オフセットは必ずstride基準で計算する。ピクセルはBGRA(リトルエンディアンXRGB8888)。
pub struct Framebuffer {
    file: File,
    pub width: u32,
    pub height: u32,
    /// 1行あたりのピクセル数(strideバイト/4)
    pub stride: u32,
}

impl Framebuffer {
    pub fn open() -> Result<Self> {
        let vsize = std::fs::read_to_string("/sys/class/graphics/fb0/virtual_size")
            .context("fb0 virtual_size読み取り失敗")?;
        let (w, h) = vsize.trim().split_once(',').context("virtual_sizeの形式が不正")?;
        let stride_bytes: u32 = std::fs::read_to_string("/sys/class/graphics/fb0/stride")
            .context("fb0 stride読み取り失敗")?
            .trim()
            .parse()?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/fb0")
            .context("/dev/fb0を開けません(videoグループ権限が必要)")?;
        Ok(Self {
            file,
            width: w.parse()?,
            height: h.trim().parse()?,
            stride: stride_bytes / 4,
        })
    }

    /// bgra(w*h*4バイト)を(x,y)に行単位で書き込む
    pub fn blit(&self, x: u32, y: u32, w: u32, h: u32, bgra: &[u8]) -> Result<()> {
        let (x, y, w, h) = self.clamp(x, y, w, h);
        let row_bytes = (w * 4) as usize;
        for j in 0..h {
            let src = &bgra[j as usize * row_bytes..][..row_bytes];
            let off = (((y + j) * self.stride + x) * 4) as u64;
            self.file.write_at(src, off)?;
        }
        Ok(())
    }

    /// src(BGRA, 1行=src_stride_px*4バイト)の部分矩形(sx,sy,w,h)を fb の(dx,dy)へ転送。
    /// キャンバスのダーティ矩形をコピー無しで書き込むために使う。
    pub fn blit_region(
        &self,
        dx: u32,
        dy: u32,
        w: u32,
        h: u32,
        src: &[u8],
        src_stride_px: u32,
        sx: u32,
        sy: u32,
    ) -> Result<()> {
        let (dx, dy, w, h) = self.clamp(dx, dy, w, h);
        let row = (w * 4) as usize;
        for j in 0..h {
            let s = (((sy + j) * src_stride_px + sx) * 4) as usize;
            let off = (((dy + j) * self.stride + dx) * 4) as u64;
            self.file.write_at(&src[s..s + row], off)?;
        }
        Ok(())
    }

    pub fn fill(&self, x: u32, y: u32, w: u32, h: u32, b: u8, g: u8, r: u8) -> Result<()> {
        let (x, y, w, h) = self.clamp(x, y, w, h);
        let mut row = Vec::with_capacity((w * 4) as usize);
        for _ in 0..w {
            row.extend_from_slice(&[b, g, r, 0]);
        }
        for j in 0..h {
            let off = (((y + j) * self.stride + x) * 4) as u64;
            self.file.write_at(&row, off)?;
        }
        Ok(())
    }

    pub fn clear(&self, x: u32, y: u32, w: u32, h: u32) -> Result<()> {
        self.fill(x, y, w, h, 0, 0, 0)
    }

    fn clamp(&self, x: u32, y: u32, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let x = x.min(self.width);
        let y = y.min(self.height);
        (x, y, w.min(self.width - x), h.min(self.height - y))
    }
}
