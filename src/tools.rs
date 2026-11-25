//! ペンの色と太さの定義、および現在のツール状態。

/// パレット8色。fb が BGRA なので (B, G, R) で保持する。
pub const PALETTE: [(u8, u8, u8); 8] = [
    (255, 255, 255), // 白
    (60, 60, 255),   // 赤
    (0, 150, 255),   // 橙
    (0, 230, 255),   // 黄
    (60, 200, 60),   // 緑
    (230, 160, 0),   // 青
    (220, 60, 200),  // 紫
    (255, 80, 255),  // マゼンタ
];

/// ブラシ半径の4段階（ピクセル）。
pub const BRUSH_RADII: [i32; 4] = [2, 5, 10, 18];

/// 現在のツール状態。
pub struct ToolState {
    pub color_idx: usize,
    pub size_idx: usize,
    pub eraser: bool,
}

impl ToolState {
    pub fn new() -> Self {
        Self { color_idx: 0, size_idx: 1, eraser: false }
    }

    pub fn radius(&self) -> i32 {
        BRUSH_RADII[self.size_idx]
    }

    /// 実際に描く色。消しゴム時は背景色を返すので描画経路は同一でよい。
    pub fn color(&self, bg: (u8, u8, u8)) -> (u8, u8, u8) {
        if self.eraser {
            bg
        } else {
            PALETTE[self.color_idx]
        }
    }
}
