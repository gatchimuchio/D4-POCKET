#![cfg_attr(windows, windows_subsystem = "windows")]
#![forbid(unsafe_code)]

#[cfg(windows)]
fn main() {
    if let Err(error) = gui_shell_rust_helper::desktop_launcher::run() {
        if error.requires_immediate_exit() {
            // Broker異常終了時はowner操作待ちのdialogでAgent Task childを生存させない。
            std::process::exit(1);
        }
        use winsafe::{co, prelude::*, HWND};
        let _ = HWND::NULL.MessageBox(
            &error.dialog_text(),
            "D4 Pocket 起動エラー",
            co::MB::OK | co::MB::ICONERROR,
        );
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("この起動器はWindows専用です。");
    std::process::exit(2);
}
