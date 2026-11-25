/// ピクセル単位の矩形。描画のダーティ範囲計算に使う。
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// 面積ゼロの空矩形。union の初期値に使う。
    pub const EMPTY: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

    /// 境界 (x0,y0)-(x1,y1) を含む矩形（両端含む）。
    pub fn from_bounds(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
        Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 }
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// 2つの矩形を包含する最小矩形。
    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.w).max(other.x + other.w);
        let y1 = (self.y + self.h).max(other.y + other.h);
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
}
