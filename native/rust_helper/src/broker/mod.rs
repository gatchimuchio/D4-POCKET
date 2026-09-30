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
