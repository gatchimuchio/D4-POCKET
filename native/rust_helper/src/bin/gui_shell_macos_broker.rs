#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
fn main() {
    if gui_shell_rust_helper::macos_desktop_worker::run().is_err() {
        eprintln!("macOS安全Broker接続を終了しました。自動再送せず接続状態を確認してください。");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("この同梱Broker接続はmacOS専用です。");
    std::process::exit(2);
}
