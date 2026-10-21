#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    if let Some(argument) = arguments.next() {
        if arguments.next().is_some()
            || argument
                != std::ffi::OsStr::new(
                    gui_shell_rust_helper::linux_process_group_guardian::INTERNAL_ARGUMENT,
                )
        {
            eprintln!("Linux Broker helperの起動argumentが不正です。");
            std::process::exit(1);
        }
        if gui_shell_rust_helper::linux_process_group_guardian::run_from_stdin().is_err() {
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = gui_shell_rust_helper::linux_desktop_worker::run() {
        let exit_code = gui_shell_rust_helper::linux_desktop_worker::helper_exit_code(&error);
        eprintln!(
            "Linux安全Broker接続を終了しました。要求を再送せず、接続状態を確認してください。分類code={exit_code}"
        );
        std::process::exit(exit_code);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux専用Broker helperです。現在のOSでは起動できません。");
    std::process::exit(1);
}
