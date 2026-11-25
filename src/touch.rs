//! touch-server への接続とイベント受信。

use crate::protocol::{socket_path, Event, FracRect, Hello};
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::Sender;

/// socket に接続して Hello を送り、受信した Event を tx へ流す。
/// メイン側が終了して tx がクローズされたらループを抜ける。
pub fn run(tx: Sender<Event>) -> Result<()> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).with_context(|| format!("touch-server接続失敗: {path}"))?;
    let hello = Hello {
        hello: "touch-paint".into(),
        pane: None,
        // 画面全体を占有し、全タッチを最優先で受け取る
        region: Some(FracRect { left: 0.0, top: 0.0, right: 1.0, bottom: 1.0 }),
    };
    let mut w = stream.try_clone().context("ソケット複製失敗")?;
    writeln!(w, "{}", serde_json::to_string(&hello)?).context("Hello送信失敗")?;

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(&line) {
            Ok(ev) => {
                if tx.send(ev).is_err() {
                    break; // メイン終了
                }
            }
            Err(_) => continue, // 不正な行は無視
        }
    }
    Ok(())
}
