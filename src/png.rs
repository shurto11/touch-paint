//! PNG の保存・読込。

use crate::canvas::Canvas;
use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// キャンバスを dir に touch-paint-<epoch>.png として保存し、パスを返す。
/// dir が無ければ作成する。
pub fn save(c: &Canvas, dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("保存フォルダ作成失敗: {dir:?}"))?;
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let path = dir.join(format!("touch-paint-{secs}.png"));
    let img = RgbaImage::from_raw(c.w, c.h, c.to_rgba()).context("バッファ→画像変換失敗")?;
    img.save(&path).with_context(|| format!("PNG保存失敗: {path:?}"))?;
    Ok(path)
}

/// PNG を読み込み、キャンバスにアスペクト維持でフィットさせる。
pub fn load_into(c: &mut Canvas, path: &Path, bg: (u8, u8, u8)) -> Result<()> {
    let img = image::open(path)
        .with_context(|| format!("PNG読込失敗: {path:?}"))?
        .to_rgba8();
    c.load_rgba_fit(&img, bg);
    Ok(())
}
