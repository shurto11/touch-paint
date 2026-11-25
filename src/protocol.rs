//! touch-server ⇄ クライアント間のメッセージ型（JSON Lines over Unix socket）。
//! touch-server/src/protocol.rs から流用。サーバーは down / move / up の区切りだけを送る。

use serde::{Deserialize, Serialize};

/// アクティブなターゲットの tmux ペイン配置。touch-paint では使わない（無視する）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaneCtx {
    pub win_w: u32,
    pub win_h: u32,
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

/// サーバー → クライアントへ送るタッチイベント。座標は画面全体に対する 0..1 の割合 (frac)。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
    /// 接地。
    Down {
        fx: f64,
        fy: f64,
        #[serde(default = "one")]
        n: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctx: Option<PaneCtx>,
    },
    /// ドラッグ中の移動。dx/dy は直前レポートからの移動量(frac, 下/右が正)。
    Move {
        fx: f64,
        fy: f64,
        dx: f64,
        dy: f64,
        #[serde(default = "one")]
        n: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctx: Option<PaneCtx>,
    },
    /// 離した瞬間。開始(fx0,fy0)〜終了(fx1,fy1)の frac と押下時間 dur(秒)。
    Up {
        fx0: f64,
        fy0: f64,
        fx1: f64,
        fy1: f64,
        dur: f64,
        #[serde(default = "one")]
        n: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ctx: Option<PaneCtx>,
    },
}

/// `n`(指の本数)のデシリアライズ既定値。
fn one() -> usize {
    1
}

/// クライアント → サーバー（接続直後に 1 行だけ送る）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello {
    pub hello: String,
    #[serde(default)]
    pub pane: Option<String>,
    /// フレームバッファ上の占有矩形(画面に対する 0..1 の割合)。
    /// この矩形内のタッチはフォーカスに関係なく最優先でこのクライアントへ届く。
    #[serde(default)]
    pub region: Option<FracRect>,
}

/// 画面に対する 0..1 の割合で表した矩形。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FracRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// ソケットパス。`$XDG_RUNTIME_DIR/touch-server.sock`、無ければ `/tmp/touch-server.sock`。
/// `TOUCH_SERVER_SOCK` で上書き可。
pub fn socket_path() -> String {
    if let Ok(p) = std::env::var("TOUCH_SERVER_SOCK") {
        if !p.is_empty() {
            return p;
        }
    }
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(d) if !d.is_empty() => format!("{d}/touch-server.sock"),
        _ => "/tmp/touch-server.sock".to_string(),
    }
}
