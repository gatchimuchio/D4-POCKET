pub(crate) mod a2a_center;
pub(crate) mod adapter_center;
pub(crate) mod agent_task_scratch;
pub(crate) mod ai_edit_center;
pub mod audit;
pub mod authority;
pub(crate) mod compose_center;
#[cfg(any(windows, test))]
pub(crate) mod content_access;
#[cfg(windows)]
pub(crate) mod credential_vault;
pub(crate) mod device_link;
pub(crate) mod device_transport;
pub mod dialogue;
pub(crate) mod evaluation_lab;
pub(crate) mod execution_history;
pub(crate) mod export_center;
pub(crate) mod history_access;
pub(crate) mod host_capability;
pub(crate) mod host_center;
pub mod ipc_server;
pub(crate) mod json_input;
pub(crate) mod mcp_center;
pub(crate) mod notification_center;
pub(crate) mod observation_center;
pub(crate) mod profile_center;
pub mod protocol;
pub(crate) mod runtime_lifecycle;
pub(crate) mod runtime_registry;
pub mod store;
pub(crate) mod update_center;
pub(crate) mod update_download;
pub(crate) mod workspace;
pub(crate) mod workspace_root;

/// 既知形式の資格情報マーカーを拒否する。未知形式の秘密値不存在までは証明しない。
pub(crate) fn contains_known_credential_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        ("api_", "key="),
        ("api-", "key="),
        ("tok", "en="),
        ("pass", "word="),
        ("bear", "er "),
        ("openai_", "api_key"),
        ("codex_", "api_key"),
        ("github_", "pat_"),
        ("gh", "p_"),
        ("--", "---begin"),
        ("ai", "za"),
        ("s", "k-"),
        ("xox", "b-"),
        ("xox", "p-"),
    ]
    .iter()
    .any(|(prefix, suffix)| contains_split_marker(&lower, prefix, suffix))
}

fn contains_split_marker(value: &str, prefix: &str, suffix: &str) -> bool {
    value.match_indices(prefix).any(|(index, _)| {
        value
            .get(index + prefix.len()..)
            .is_some_and(|tail| tail.starts_with(suffix))
    })
}

#[cfg(test)]
mod credential_marker_tests {
    use super::contains_known_credential_marker;

    #[test]
    fn 既知credential形式を大小文字を問わず拒否する() {
        for value in [
            "api_key=SYNTHETIC",
            "api-key=SYNTHETIC",
            "TOKEN=SYNTHETIC",
            "PASSWORD=SYNTHETIC",
            "Bearer SYNTHETIC",
            "OPENAI_API_KEY",
            "codex_api_key",
            "github_pat_",
            "ghp_",
            "-----BEGIN",
            "AIza",
            "sk-",
            "xoxb-SYNTHETIC",
            "XoXp-SYNTHETIC",
        ] {
            assert!(contains_known_credential_marker(value), "{value}");
        }
        assert!(!contains_known_credential_marker(
            "作業状態を要約してください"
        ));
    }
}

pub use audit::{BrokerAuditEvent, BrokerAuditLog};
pub use ipc_server::{
    run_loopback_server, run_loopback_server_cancellable, BrokerCredentialRole, BrokerEndpoint,
    BrokerServerConfig,
};
pub use protocol::{
    Broker, BrokerError, BrokerHealth, BrokerMetadata, BrokerOperation, BrokerPersistenceMode,
    BrokerRequestEnvelope, BrokerResponse, BrokerStateStore, BrokerStatus,
};
pub use store::{BrokerPersistentState, BrokerPersistentStore, BrokerStoreError};

/// development buildでのみ、固定lifecycle fixture childのstdin/stdout protocolを実行する。
#[cfg(debug_assertions)]
pub fn run_development_lifecycle_fixture_child() -> i32 {
    runtime_lifecycle::run_development_fixture_child()
}
