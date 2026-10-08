//! Rust所有のmacOS確認UIとOS作業領域選択。公開要求の配送だけを扱い、D4権限を生成しない。
#![deny(unsafe_op_in_unsafe_fn)]

use std::time::Duration;

#[cfg(any(target_os = "macos", test))]
mod workspace_transport;
#[cfg(target_os = "macos")]
pub use workspace_transport::PrivateFrame;
#[cfg(any(target_os = "macos", test))]
mod credential_input;
#[cfg(target_os = "macos")]
pub use credential_input::{valid_credential_input, CredentialPrivateFrame};

pub const TITLE: &str = "D4 Pocket — 今回の操作を確認";
pub const DENY: &str = "承認しない";
pub const APPROVE: &str = "今回の操作を承認";

/// OSが選択したfolder／単一CLI fileへの起動中access。D4のPermission／Approvalではない。
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

/// Main threadでOS chooserを構成する。bookmark、credential、任意初期pathを受け取らない。
#[cfg(target_os = "macos")]
fn selection_panel(
    cli: bool,
) -> Result<objc2::rc::Retained<objc2_app_kit::NSOpenPanel>, &'static str> {
    use objc2::{msg_send, ClassType, MainThreadMarker};
    use objc2_app_kit::NSOpenPanel;
    use objc2_foundation::NSString;
    let _mtm = MainThreadMarker::new().ok_or("OS選択をmain threadで開始できません")?;
    // SAFETY: 固定NSOpenPanel factoryをmain threadで呼ぶ。実観測のNULLをOptionで拒否し、
    // 既存bindingのnonnull panicで製品processを終了させない。ABIとretain規則は同じ。
    let panel: Option<objc2::rc::Retained<NSOpenPanel>> =
        unsafe { msg_send![NSOpenPanel::class(), openPanel] };
    let panel = panel.ok_or("OS選択の表示を開始できません")?;
    panel.setTitle(Some(&NSString::from_str(if cli {
        "D4 Pocket — CLI実行fileのOS選択"
    } else {
        "D4 Pocket — 作業領域のOS選択"
    })));
    panel.setPrompt(Some(&NSString::from_str(if cli {
        "CLI fileを選択"
    } else {
        "作業領域を選択"
    })));
    panel.setMessage(Some(&NSString::from_str(
        "この選択はOSの起動中accessだけです。D4の登録・Permission・Approvalは別に必要です。",
    )));
    panel.setCanChooseFiles(cli);
    panel.setCanChooseDirectories(!cli);
    panel.setAllowsMultipleSelection(false);
    panel.setResolvesAliases(false);
    panel.setCanCreateDirectories(false);
    Ok(panel)
}

#[cfg(target_os = "macos")]
fn selected_workspace(
    panel: &objc2_app_kit::NSOpenPanel,
    response: isize,
) -> Result<Option<SelectedWorkspace>, &'static str> {
    if response == 0 {
        return Ok(None);
    }
    if response != 1 {
        return Err("OS選択の表示を開始できません");
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

/// 起動中のnative間OS bookmarkだけを解決する。D4権限・登録を生成しない。
#[cfg(target_os = "macos")]
pub fn resolve_workspace_bookmark(bookmark: &str) -> Result<SelectedWorkspace, &'static str> {
    use objc2::{runtime::Bool, AnyThread};
    use objc2_foundation::{
        NSData, NSDataBase64DecodingOptions, NSString, NSURLBookmarkResolutionOptions, NSURL,
    };
    if bookmark.is_empty() || bookmark.len() > 24 * 1024 {
        return Err("OS選択のpath範囲が不正です");
    }
    let data = NSData::initWithBase64EncodedString_options(
        NSData::alloc(),
        &NSString::from_str(bookmark),
        NSDataBase64DecodingOptions::empty(),
    )
    .ok_or("OS選択のpathが不正です")?;
    let mut stale = Bool::NO;
    // SAFETY: NSDataを保持し、staleは同期呼出し中有効。UIを禁止しimplicit scopeを解決する。
    let url = unsafe {
        NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &data,
            NSURLBookmarkResolutionOptions::WithoutUI,
            None,
            &mut stale,
        )
    }
    .map_err(|_| "OS選択のpathが不正です")?;
    let mut scope = SelectedWorkspace {
        path: std::path::PathBuf::new(),
        url,
    };
    if stale.as_bool() {
        return Err("OS選択のpathが不正です");
    }
    scope.path = scope
        .url
        .path()
        .ok_or("OS選択のpathが不正です")?
        .to_string()
        .into();
    if !scope.path.is_absolute()
        || scope
            .path
            .to_str()
            .is_none_or(|p| p.len() > 1024 || p.chars().any(char::is_control))
    {
        return Err("OS選択のpath範囲が不正です");
    }
    Ok(scope)
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
