//! touch-paint: タッチで描けるペイントツール。
//! touch-server からタッチを受け取り、/dev/fb0 に直接描画する。

mod app;
mod canvas;
mod fb;
mod fb_client;
mod geom;
mod history;
mod png;
mod protocol;
mod text;
mod toolbar;
mod tools;
mod touch;

use anyhow::Result;
use app::App;
use protocol::Event;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let open_path = std::env::args().nth(1).map(PathBuf::from);

    let fb = fb::Framebuffer::open()?;
    let font = text::load_font()?;

    let term = Arc::new(AtomicBool::new(false));
    for sig in [SIGINT, SIGTERM, SIGHUP] {
        signal_hook::flag::register(sig, Arc::clone(&term))?;
    }

    let (tx, rx) = mpsc::channel::<Event>();
    thread::spawn(move || {
        if let Err(e) = touch::run(tx) {
            eprintln!("touch-paint: 入力スレッド終了 {e:#}");
        }
    });

    let mut app = App::new(fb, font, open_path, Arc::clone(&term))?;
    app.full_render()?;

    // fb-server クライアント起動(表示可否の通知を受け取る)
    let (vtx, vrx) = mpsc::channel::<bool>();
    fb_client::spawn("touch-paint", vtx);

    while !term.load(Ordering::Relaxed) {
        while let Ok(v) = vrx.try_recv() {
            app.set_visible(v)?;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => app.handle(ev)?,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    app.shutdown();
    Ok(())
}
