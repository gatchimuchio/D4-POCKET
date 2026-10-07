//! Rust所有の期限付きmacOS確認画面。要求や資格を受け取らず、明示選択だけを返す。
#![deny(unsafe_op_in_unsafe_fn)]

use std::time::Duration;

pub const TITLE: &str = "D4 Pocket — 今回の操作を確認";
pub const DENY: &str = "承認しない";
pub const APPROVE: &str = "今回の操作を承認";

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
