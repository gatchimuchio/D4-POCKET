//! 公開metadataからnative秘密入力を開く。入力は承認ではなくprivate pipeだけへ配送する。
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;
#[cfg(all(not(debug_assertions), feature = "credential-ui-fixture"))]
compile_error!("資格情報UI fixtureはDebug専用");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    client: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    #[serde(rename = "版")]
    version: u8,
    #[serde(rename = "資格情報ID")]
    id: String,
    #[serde(rename = "用途")]
    purpose: String,
    #[serde(rename = "接続対象")]
    target: String,
    #[serde(rename = "種類")]
    kind: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicRequest {
    request_id: String,
    operation: String,
    payload_hash: String,
    nonce: String,
    issued_at: String,
    metadata: Metadata,
    payload: Payload,
}
pub fn valid_credential_input(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > 8192 || bytes.contains(&b'\n') || bytes.contains(&b'\r') {
        return false;
    }
    let Ok(v) = serde_json::from_slice::<PublicRequest>(bytes) else {
        return false;
    };
    let text = |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
    v.operation == "資格情報登録"
        && v.metadata.client == "desktop_flutter"
        && text(&v.request_id)
        && text(&v.nonce)
        && v.issued_at.len() == 20
        && v.issued_at.ends_with('Z')
        && v.payload_hash.len() == 71
        && v.payload_hash.starts_with("sha256:")
        && v.payload.version == 1
        && v.payload.id.len() == 32
        && v.payload
            .id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && matches!(
            v.payload.purpose.as_str(),
            "provider_api_key" | "mcp_transport"
        )
        && text(&v.payload.target)
        && v.payload.kind == "api_key"
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInput {
    pub request_json: String,
    pub state: String,
    pub secret: Option<String>,
}
impl Drop for NativeInput {
    fn drop(&mut self) {
        if let Some(secret) = self.secret.as_mut() {
            secret.zeroize();
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialPrivateFrame {
    pub native_credential_input: NativeInput,
}

/// # Safety
/// 固定Runnerが同期中公開byte列と同じhelperへのpipe fdを保持する。秘密・Approvalを返さない。
#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn d4_credential_input_and_write(
    bytes: *const u8,
    length: usize,
    fd: i32,
) -> i32 {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSAlert, NSApplication, NSSecureTextField};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSTimer};
    use std::os::{
        fd::{AsRawFd, BorrowedFd},
        unix::fs::FileTypeExt,
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return 0;
    };
    if bytes.is_null() || length == 0 || length > 8192 || fd <= 2 {
        return 0;
    }
    // SAFETY: 固定RunnerのwithUnsafeBytesが同期終了まで保持。
    let input = unsafe { std::slice::from_raw_parts(bytes, length) };
    if !valid_credential_input(input) {
        return 0;
    }
    // SAFETY: 借用fdを閉じず複製だけを所有し、file／socketは拒否する。
    let Ok(fd) = (unsafe { BorrowedFd::borrow_raw(fd) }).try_clone_to_owned() else {
        return 0;
    };
    let pipe = std::fs::File::from(fd);
    if !pipe.metadata().is_ok_and(|m| m.file_type().is_fifo()) {
        return 0;
    }
    // SAFETY: 所有する複製pipeだけ。Darwinの固定F_SETNOSIGPIPE。
    if unsafe { libc::fcntl(pipe.as_raw_fd(), 73, 1) } != 0 {
        return 0;
    }
    let Some(window) = NSApplication::sharedApplication(mtm).mainWindow() else {
        return 0;
    };
    let Ok(request) = std::str::from_utf8(input) else {
        return 0;
    };
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str("D4 Pocket — 資格情報を入力"));
    alert.setInformativeText(&NSString::from_str(
        "秘密値はRustから保管庫へだけ渡します。入力後に別個のOwner確認があります。",
    ));
    alert.addButtonWithTitle(&NSString::from_str("入力して確認へ"));
    alert.addButtonWithTitle(&NSString::from_str("取消"));
    let field = NSSecureTextField::initWithFrame(
        NSSecureTextField::alloc(mtm),
        NSRect::new(NSPoint::new(0., 0.), NSSize::new(320., 28.)),
    );
    alert.setAccessoryView(Some(&field));
    #[cfg(all(debug_assertions, feature = "credential-ui-fixture"))]
    {
        // 合成秘密値はnative内で生成し、XCTest入力・引数・環境変数・logへ出さない。
        let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
        if getrandom::getrandom(bytes.as_mut()).is_err() {
            return 0;
        }
        let value =
            zeroize::Zeroizing::new(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
        field.setStringValue(&NSString::from_str(&value));
    }
    let pending = std::cell::RefCell::new(Some((request.to_owned(), pipe)));
    let timed_window = window.clone();
    let timed_sheet = alert.window();
    let timer_handler = block2::RcBlock::new(move |_: std::ptr::NonNull<NSTimer>| {
        timed_window.endSheet_returnCode(&timed_sheet, 1001);
    });
    // SAFETY: Main threadのrun loopで一回のみ。blockはretainされ、300秒で入力取消。
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_repeats_block(300., false, &timer_handler)
    };
    let handler = block2::RcBlock::new(move |response: isize| {
        timer.invalidate();
        let Some((request_json, mut pipe)) = pending.borrow_mut().take() else {
            return;
        };
        let mut secret = if response == 1000 {
            Some(field.stringValue().to_string())
        } else {
            None
        };
        field.setStringValue(&NSString::from_str(""));
        let state = if response != 1000 {
            "cancelled"
        } else if secret
            .as_ref()
            .is_none_or(|s| s.is_empty() || s.len() > 16384 || s.chars().any(char::is_control))
        {
            if let Some(s) = secret.as_mut() {
                s.zeroize();
            }
            secret = None;
            "invalid"
        } else {
            "entered"
        };
        let frame = CredentialPrivateFrame {
            native_credential_input: NativeInput {
                request_json,
                state: state.into(),
                secret,
            },
        };
        use std::io::Write;
        if let Ok(mut encoded) = serde_json::to_vec(&frame).map(zeroize::Zeroizing::new) {
            encoded.push(b'\n');
            if encoded.len() < 65536 {
                let _ = pipe.write_all(&encoded).and_then(|_| pipe.flush());
            }
        }
    });
    alert.beginSheetModalForWindow_completionHandler(&window, Some(&handler));
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_native_input_has_no_secret_or_authority_in_public_request() {
        let v = serde_json::json!({"request_id":"input", "operation":"資格情報登録", "payload_hash":format!("sha256:{}", "a".repeat(64)), "nonce":"once", "issued_at":"2026-10-08T00:00:00Z", "metadata":{"client":"desktop_flutter"},
            "payload":{"版":1,"資格情報ID":"a".repeat(32),"用途":"mcp_transport","接続対象":"server", "種類":"api_key"}});
        assert!(valid_credential_input(v.to_string().as_bytes()));
        for key in [
            "秘密値",
            "承認",
            "Permission",
            "保管方式",
            "登録者種別",
            "登録経路",
        ] {
            let mut bad = v.clone();
            bad["payload"][key] = serde_json::json!("injected");
            assert!(!valid_credential_input(bad.to_string().as_bytes()));
        }
        let duplicate = v.to_string().replacen("\"版\":1", "\"版\":1,\"版\":1", 1);
        assert!(!valid_credential_input(duplicate.as_bytes()));
        let frame = CredentialPrivateFrame {
            native_credential_input: NativeInput {
                request_json: v.to_string(),
                state: "cancelled".into(),
                secret: None,
            },
        };
        assert!(serde_json::from_str::<CredentialPrivateFrame>(
            &serde_json::to_string(&frame).unwrap()
        )
        .is_ok());
    }
}
