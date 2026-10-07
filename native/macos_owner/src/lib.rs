//! Rust所有の期限付きmacOS確認画面。要求や資格を受け取らず、明示選択だけを返す。
#![deny(unsafe_op_in_unsafe_fn)]

use std::time::Duration;

pub const TITLE: &str = "D4 Pocket — 今回の操作を確認";
pub const DENY: &str = "承認しない";
pub const APPROVE: &str = "今回の操作を承認";

/// OSが選択したfolderへの起動中access。D4のPermission／Approvalではない。
pub struct SelectedWorkspace {
    path: std::path::PathBuf,
    #[cfg(target_os = "macos")]
    url: objc2::rc::Retained<objc2_foundation::NSURL>,
}

impl AsRef<std::path::Path> for SelectedWorkspace {
    fn as_ref(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(target_os = "macos")]
impl Drop for SelectedWorkspace {
    fn drop(&mut self) {
        // Open panelはOSがscopeを開始する。保持した同じURLで一度だけ終了する。
        unsafe {
            self.url.stopAccessingSecurityScopedResource();
        }
    }
}

/// Main threadでOS chooserを開く。bookmark、credential、任意初期pathを受け取らない。
pub fn select_workspace() -> Result<Option<SelectedWorkspace>, &'static str> {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSOpenPanel};
        use objc2_foundation::NSString;
        let mtm = MainThreadMarker::new().ok_or("OS選択をmain threadで開始できません")?;
        let app = NSApplication::sharedApplication(mtm);
        if !app.setActivationPolicy(NSApplicationActivationPolicy::Accessory) {
            return Err("OS選択の表示を開始できません");
        }
        let panel = NSOpenPanel::openPanel(mtm);
        panel.setTitle(Some(&NSString::from_str("D4 Pocket — 作業領域のOS選択")));
        panel.setPrompt(Some(&NSString::from_str("作業領域を選択")));
        panel.setMessage(Some(&NSString::from_str(
            "この選択はOSの起動中accessだけです。D4の登録・Permission・Approvalは別に必要です。",
        )));
        panel.setCanChooseFiles(false);
        panel.setCanChooseDirectories(true);
        panel.setAllowsMultipleSelection(false);
        panel.setResolvesAliases(false);
        panel.setCanCreateDirectories(false);
        if panel.runModal() != 1 {
            return Ok(None);
        }
        let url = panel.URL().ok_or("OS選択にURLがありません")?;
        let mut selected = SelectedWorkspace {
            path: std::path::PathBuf::new(),
            url,
        };
        selected.path = selected
            .url
            .path()
            .ok_or("OS選択のpathが不正です")?
            .to_string()
            .into();
        let path = selected
            .path
            .to_str()
            .ok_or("OS選択のpathを表示できません")?;
        if !selected.path.is_absolute() || path.len() > 1024 || path.chars().any(char::is_control) {
            return Err("OS選択のpath範囲が不正です");
        }
        Ok(Some(selected))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("このplatformのOS選択は未対応です")
    }
}

/// 既定ボタン、取消、期限超過、OS失敗は全て非承認。
pub fn confirm(summary: &str, timeout: Duration) -> bool {
    if summary.is_empty()
        || summary.len() > 16 * 1024
        || summary.chars().any(|c| c.is_control() && c != '\n')
        || timeout.is_zero()
        || timeout > Duration::from_secs(300)
    {
        return false;
    }
    native_confirm(summary, timeout)
}

#[cfg(target_os = "macos")]
fn native_confirm(summary: &str, timeout: Duration) -> bool {
    use core_foundation::{base::TCFType, string::CFString};
    use core_foundation_sys::user_notification::{
        kCFUserNotificationAlternateResponse, kCFUserNotificationCancelResponse,
        kCFUserNotificationCautionAlertLevel, CFUserNotificationDisplayAlert,
    };
    let title = CFString::new(TITLE);
    let message = CFString::new(summary);
    let deny = CFString::new(DENY);
    let approve = CFString::new(APPROVE);
    let mut response = kCFUserNotificationCancelResponse;
    let started = std::time::Instant::now();
    // SAFETY: CFStringは同期呼出しの終了まで保持する。全optional URLはNULL、
    // 出力先は初期化済みCFOptionFlags。OS error時に出力を承認として解釈しない。
    let result = unsafe {
        CFUserNotificationDisplayAlert(
            timeout.as_secs_f64(),
            kCFUserNotificationCautionAlertLevel,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            title.as_concrete_TypeRef(),
            message.as_concrete_TypeRef(),
            deny.as_concrete_TypeRef(),
            approve.as_concrete_TypeRef(),
            std::ptr::null(),
            &mut response,
        )
    };
    result == 0 && response == kCFUserNotificationAlternateResponse && started.elapsed() < timeout
}

#[cfg(not(target_os = "macos"))]
fn native_confirm(_: &str, _: Duration) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_confirmation_never_opens_a_dialog() {
        assert!(!confirm("", Duration::from_secs(1)));
        assert!(!confirm("synthetic\0value", Duration::from_secs(1)));
        assert!(!confirm("試験", Duration::ZERO));
        assert!(!confirm("試験", Duration::from_secs(301)));
        assert!(!confirm(&"a".repeat(16 * 1024 + 1), Duration::from_secs(1)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_confirmation_deadline_is_not_approval() {
        let started = std::time::Instant::now();
        assert!(!confirm(
            "合成試験: 何も操作せず期限超過を待ちます。製品操作は実行しません。",
            Duration::from_secs(1)
        ));
        assert!(
            started.elapsed() >= Duration::from_millis(800),
            "OSが確認画面を期限前に拒否した"
        );
    }
}
