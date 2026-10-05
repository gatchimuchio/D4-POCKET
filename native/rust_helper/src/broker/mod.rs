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
pub(crate) mod product_install;
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
    KNOWN_CREDENTIAL_MARKERS
        .iter()
        .any(|(prefix, suffix)| contains_split_marker(&lower, prefix, suffix))
}

// ASCII値をu32配列で保持し、検査対象文字列と同形の連続byte列を実行fileへ埋め込まない。
const KNOWN_CREDENTIAL_MARKERS: &[(&[u32], &[u32])] = &[
    (&[0x61, 0x70, 0x69, 0x5f], &[0x6b, 0x65, 0x79, 0x3d]),
    (&[0x61, 0x70, 0x69, 0x2d], &[0x6b, 0x65, 0x79, 0x3d]),
    (&[0x74, 0x6f, 0x6b], &[0x65, 0x6e, 0x3d]),
    (&[0x70, 0x61, 0x73, 0x73], &[0x77, 0x6f, 0x72, 0x64, 0x3d]),
    (&[0x62, 0x65, 0x61, 0x72], &[0x65, 0x72, 0x20]),
    (
        &[0x6f, 0x70, 0x65, 0x6e, 0x61, 0x69, 0x5f],
        &[0x61, 0x70, 0x69, 0x5f, 0x6b, 0x65, 0x79],
    ),
    (
        &[0x63, 0x6f, 0x64, 0x65, 0x78, 0x5f],
        &[0x61, 0x70, 0x69, 0x5f, 0x6b, 0x65, 0x79],
    ),
    (
        &[0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x5f],
        &[0x70, 0x61, 0x74, 0x5f],
    ),
    (&[0x67, 0x68], &[0x70, 0x5f]),
    (
        &[0x2d, 0x2d],
        &[0x2d, 0x2d, 0x2d, 0x62, 0x65, 0x67, 0x69, 0x6e],
    ),
    (&[0x61, 0x69], &[0x7a, 0x61]),
    (&[0x73], &[0x6b, 0x2d]),
    (&[0x78, 0x6f, 0x78], &[0x62, 0x2d]),
    (&[0x78, 0x6f, 0x78], &[0x70, 0x2d]),
];

fn contains_split_marker(value: &str, prefix: &[u32], suffix: &[u32]) -> bool {
    let marker_length = prefix.len() + suffix.len();
    value.as_bytes().windows(marker_length).any(|window| {
        prefix
            .iter()
            .zip(&window[..prefix.len()])
            .all(|(marker, byte)| *marker == u32::from(*byte))
            && suffix
                .iter()
                .zip(&window[prefix.len()..])
                .all(|(marker, byte)| *marker == u32::from(*byte))
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
