#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() {
    if gui_shell_rust_helper::linux_desktop_worker::run().is_err() {
        eprintln!(
            "Linux安全Broker接続を終了しました。要求を再送せず、接続状態を確認してください。"
        );
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux専用Broker helperです。現在のOSでは起動できません。");
    std::process::exit(1);
}
