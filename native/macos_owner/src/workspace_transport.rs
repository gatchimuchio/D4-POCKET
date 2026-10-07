//! 親GUI processのRust OS選択。scope実値は既存private pipeだけへ書く。
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const VERSION_ONLY_HASH: &str =
    "sha256:2430f1a2ad2982d0067885488a4c89e21ad1d7c83b115ba8f1b20acc88dfaea8";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    version: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Client {
    client: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicRequest {
    request_id: String,
    operation: String,
    payload_hash: String,
    nonce: String,
    issued_at: String,
    metadata: Client,
    payload: Version,
}
fn public_candidate(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > 8192 || bytes.contains(&b'\n') || bytes.contains(&b'\r') {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<PublicRequest>(bytes) else {
        return false;
    };
    value.operation == "作業領域OS選択"
        && value.payload.version == 1
        && value.metadata.client == "desktop_flutter"
        && !value.request_id.is_empty()
        && value.request_id.len() <= 256
        && !value.nonce.is_empty()
        && value.nonce.len() <= 256
        && value.issued_at.len() == 20
        && value.issued_at.ends_with('Z')
        && value.payload_hash == VERSION_ONLY_HASH
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSelection {
    pub request_json: String,
    pub selection_status: String,
    pub bookmark: Option<String>,
}
impl Drop for NativeSelection {
    fn drop(&mut self) {
        if let Some(bookmark) = self.bookmark.as_mut() {
            bookmark.zeroize();
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateFrame {
    pub native_workspace_selection: NativeSelection,
}

#[cfg(target_os = "macos")]
thread_local! {
    static SCOPES: std::cell::RefCell<Vec<super::SelectedWorkspace>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 0=配送失敗、1=scopeなしで配送済み、2=選択scopeを保持して配送済み。承認値ではない。
///
/// # Safety（安全な呼出し前提）
/// 固定Runnerが保持する公開要求byte列を長さ分保持し、同じhelperへの有効pipe fdを渡す。
#[no_mangle]
#[cfg(target_os = "macos")]
pub unsafe extern "C" fn d4_workspace_select_and_write(
    bytes: *const u8,
    length: usize,
    fd: i32,
) -> i32 {
    use objc2_foundation::{NSDataBase64EncodingOptions, NSURLBookmarkCreationOptions};
    use std::io::Write;
    use std::os::{
        fd::{AsRawFd, BorrowedFd},
        unix::fs::FileTypeExt,
    };
    if bytes.is_null()
        || length == 0
        || length > 8192
        || fd <= 2
        || objc2::MainThreadMarker::new().is_none()
    {
        return 0;
    }
    // SAFETY: 固定Swift呼出しのwithUnsafeBytesが同期終了までbyte列を保持する。
    let input = unsafe { std::slice::from_raw_parts(bytes, length) };
    if !public_candidate(input) || SCOPES.with(|v| v.borrow().len() >= 8) {
        return 0;
    }
    // SAFETY: 固定Runner所有pipeを閉じず、複製fdの所有権だけを使う。普通fileは拒否する。
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let Ok(owned) = borrowed.try_clone_to_owned() else {
        return 0;
    };
    let mut pipe = std::fs::File::from(owned);
    if !pipe.metadata().is_ok_and(|m| m.file_type().is_fifo()) {
        return 0;
    }
    // SAFETY: 自分が所有する複製pipe fdだけ。helper停止時のSIGPIPEでGUIを終了させない。
    if unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETNOSIGPIPE, 1) } != 0 {
        return 0;
    }
    let selection = super::select_workspace();
    let mut selected = None;
    let (state, bookmark) = match selection {
        Ok(Some(scope)) => {
            let Ok(data) = scope
                .url
                .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                    NSURLBookmarkCreationOptions::empty(),
                    None,
                    None,
                )
            else {
                return 0;
            };
            let token = data
                .base64EncodedStringWithOptions(NSDataBase64EncodingOptions::empty())
                .to_string();
            if token.len() > 24 * 1024 {
                return 0;
            }
            selected = Some(scope);
            ("selected", Some(token))
        }
        Ok(None) => ("cancelled", None),
        Err(_) => ("failed_native", None),
    };
    let Ok(request_json) = std::str::from_utf8(input) else {
        return 0;
    };
    let private = PrivateFrame {
        native_workspace_selection: NativeSelection {
            request_json: request_json.to_owned(),
            selection_status: state.into(),
            bookmark,
        },
    };
    let Ok(mut serialized) = serde_json::to_vec(&private).map(zeroize::Zeroizing::new) else {
        return 0;
    };
    serialized.push(b'\n');
    if serialized.len() >= 65536
        || pipe
            .write_all(&serialized)
            .and_then(|_| pipe.flush())
            .is_err()
    {
        return 0;
    }
    if let Some(scope) = selected {
        SCOPES.with(|v| v.borrow_mut().push(scope));
        2
    } else {
        1
    }
}

/// 拒否された直前のOS scopeだけを終了する。権限を追加できない。
#[no_mangle]
#[cfg(target_os = "macos")]
pub extern "C" fn d4_workspace_release_last() {
    if objc2::MainThreadMarker::new().is_some() {
        SCOPES.with(|v| {
            v.borrow_mut().pop();
        });
    }
}
/// helper停止後に親GUIのOS scopeを終了する。復元・永続化しない。
#[no_mangle]
#[cfg(target_os = "macos")]
pub extern "C" fn d4_workspace_release_all() {
    if objc2::MainThreadMarker::new().is_some() {
        SCOPES.with(|v| v.borrow_mut().clear());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_selection_has_no_path_bookmark_or_authority() {
        let base = serde_json::json!({"request_id":"public","operation":"作業領域OS選択", "nonce":"nonce",
            "issued_at":"2026-10-08T00:00:00Z","metadata":{"client":"desktop_flutter"},
            "payload":{"version":1},"payload_hash":VERSION_ONLY_HASH});
        assert!(public_candidate(base.to_string().as_bytes()));
        for key in [
            "workspace_root",
            "bookmark",
            "approval",
            "permission",
            "credential",
        ] {
            let mut value = base.clone();
            value["payload"][key] = serde_json::json!("injected");
            assert!(
                !public_candidate(value.to_string().as_bytes()),
                "UI注入を拒否する"
            );
        }
        let duplicated =
            base.to_string()
                .replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
        assert!(!public_candidate(duplicated.as_bytes()));
        let mut value = base.clone();
        value["native_workspace_selection"] = serde_json::json!({});
        assert!(!public_candidate(value.to_string().as_bytes()));
        let private = PrivateFrame {
            native_workspace_selection: NativeSelection {
                request_json: base.to_string(),
                selection_status: "cancelled".into(),
                bookmark: None,
            },
        };
        let serialized = serde_json::to_string(&private).unwrap();
        assert!(serde_json::from_str::<PrivateFrame>(&serialized).is_ok());
        let injected = serialized.replacen(
            "\"bookmark\":null",
            "\"bookmark\":null,\"approval\":true",
            1,
        );
        assert!(serde_json::from_str::<PrivateFrame>(&injected).is_err());
    }
}
