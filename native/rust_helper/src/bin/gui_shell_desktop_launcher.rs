#![cfg_attr(windows, windows_subsystem = "windows")]
#![forbid(unsafe_code)]

#[cfg(windows)]
fn main() {
    match gui_shell_rust_helper::mcp_task_tool_stdio::maybe_run_stdio_from_process() {
        Ok(true) => return,
        Ok(false) => {}
        Err(_) => {
            eprintln!("D4 PocketのMCP Task要求受付を安全に終了しました。");
            std::process::exit(1);
        }
    }
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

#[cfg(target_os = "linux")]
fn main() {
    match gui_shell_rust_helper::mcp_task_tool_stdio::maybe_run_stdio_from_process() {
        Ok(true) => return,
        Ok(false) => {}
        Err(_) => {
            eprintln!("D4 PocketのMCP Task要求受付を安全に終了しました。");
            std::process::exit(1);
        }
    }
    if let Err(error) = gui_shell_rust_helper::linux_desktop_launcher::run() {
        eprintln!("D4 Pocketを起動できません: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn main() {
    eprintln!("この起動器は現在のOSでは利用できません。");
    std::process::exit(2);
}
