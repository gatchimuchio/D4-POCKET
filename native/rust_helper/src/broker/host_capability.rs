use serde_json::{json, Value};

const EVIDENCE_CONFIG: &str = "CONFIG";
const EVIDENCE_INTERNAL_STATE: &str = "INTERNAL_STATE";
const EVIDENCE_LIVE_RUNTIME: &str = "LIVE_RUNTIME";

fn capability(id: &str, status: &str, evidence: &str, reason: &str) -> Value {
    json!({
        "能力ID": id,
        "状態": status,
        "証拠種別": evidence,
        "理由": reason,
    })
}

/// Brokerが現在のhostで観測できる範囲だけを返す。
/// capabilityの観測結果はPermissionやApprovalを生成せず、UIの表示材料に限る。
pub(crate) fn current() -> Value {
    let platform = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        "android" => "android",
        "ios" => "ios",
        _ => "unknown",
    };
    let secure_storage = if platform == "windows" {
        capability(
            "secure_storage",
            "ready",
            EVIDENCE_CONFIG,
            "Windows安全保管境界が選択可能である",
        )
    } else {
        capability(
            "secure_storage",
            "unavailable",
            EVIDENCE_CONFIG,
            "このplatformの安全保管は現在のBroker経路へ接続されていない",
        )
    };
    let mut capabilities = vec![
        capability(
            "filesystem",
            "ready",
            EVIDENCE_INTERNAL_STATE,
            "Brokerが永続保管境界をreadyとして保持している",
        ),
        capability(
            "process",
            "ready",
            EVIDENCE_LIVE_RUNTIME,
            "認証済みBrokerプロセスが現在稼働している",
        ),
        capability(
            "network",
            "ready",
            EVIDENCE_LIVE_RUNTIME,
            "認証済みloopback IPCを現在の接続から観測できる",
        ),
        secure_storage,
        capability(
            "local_runtime",
            "ready",
            EVIDENCE_INTERNAL_STATE,
            "Runtime接続をBroker登録境界へ限定できる",
        ),
        capability(
            "native_notification",
            "unavailable",
            EVIDENCE_CONFIG,
            "このBroker経路ではnative通知接続をまだ提供していない",
        ),
        capability(
            "gpu_observation",
            "unavailable",
            EVIDENCE_CONFIG,
            "GPU観測collectorが接続されていないためunknownとして扱う",
        ),
        capability(
            "system_dialog",
            "unavailable",
            EVIDENCE_CONFIG,
            "権限付きsystem dialog経路が接続されていない",
        ),
        capability(
            "background_execution",
            "unavailable",
            EVIDENCE_CONFIG,
            "background executionの製品経路が接続されていない",
        ),
    ];
    if matches!(platform, "android" | "ios") {
        capabilities.push(capability(
            "mobile_background",
            "unavailable",
            EVIDENCE_CONFIG,
            "mobile background経路は実機検証とともに延期されている",
        ));
    }
    let overall_status = if capabilities.iter().any(|item| {
        matches!(
            item.get("状態").and_then(Value::as_str),
            Some("failed" | "suspended")
        )
    }) {
        "failed"
    } else if capabilities.iter().any(|item| {
        matches!(
            item.get("状態").and_then(Value::as_str),
            Some("degraded" | "unavailable")
        )
    }) {
        "degraded"
    } else {
        "ready"
    };
    json!({
        "版": 1,
        "ホストID": format!("gui-shell-local-{platform}"),
        "表示名": format!("ローカル {platform}"),
        "プラットフォーム": platform,
        "状態": overall_status,
        "能力": capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::current;

    #[test]
    fn 現在host能力は権限情報を含めず状態を返す() {
        let body = current();
        assert_eq!(body["版"], 1);
        assert!(body["ホストID"]
            .as_str()
            .unwrap()
            .starts_with("gui-shell-local-"));
        assert!(body["能力"].as_array().unwrap().iter().all(|item| {
            item.get("Permission").is_none()
                && item.get("Approval").is_none()
                && item.get("authority").is_none()
        }));
    }
}
