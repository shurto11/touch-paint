//! ストローク単位のフルスナップショット undo/redo。
//! 1段 = キャンバスバッファ全体。実装が単純で確実。

use crate::canvas::Canvas;
use std::collections::VecDeque;

pub struct History {
    undo: VecDeque<Vec<u8>>,
    redo: Vec<Vec<u8>>,
    cap: usize,
}

impl History {
    pub fn new(cap: usize) -> Self {
        Self { undo: VecDeque::new(), redo: Vec::new(), cap }
    }

    /// ストローク開始(down)の瞬間に現在のキャンバスを記録する。
    pub fn begin(&mut self, c: &Canvas) {
        self.undo.push_back(c.buf.clone());
        while self.undo.len() > self.cap {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    /// 直前の状態へ戻す。戻せたら true。
    pub fn undo(&mut self, c: &mut Canvas) -> bool {
        if let Some(prev) = self.undo.pop_back() {
            self.redo.push(std::mem::replace(&mut c.buf, prev));
            true
        } else {
            false
        }
    }

    /// undo を取り消す。進めたら true。
    pub fn redo(&mut self, c: &mut Canvas) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push_back(std::mem::replace(&mut c.buf, next));
            true
        } else {
            false
        }
    }

    /// PNG 読込時など、履歴を全消去する。
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
