#[cfg(any(windows, test))]
pub(crate) mod content_access;
pub(crate) mod history_access;
pub(crate) mod execution_history;
pub(crate) mod evaluation_lab;
pub mod audit;
pub(crate) mod workspace;
pub(crate) mod workspace_root;
pub(crate) mod json_input;
pub mod dialogue;
pub(crate) mod device_link;
pub(crate) mod device_transport;
pub mod authority;
pub mod ipc_server;
pub mod protocol;
pub(crate) mod runtime_registry;
pub(crate) mod runtime_lifecycle;
pub mod store;

pub use audit::{BrokerAuditEvent, BrokerAuditLog};
pub use ipc_server::{
    run_loopback_server, BrokerCredentialRole, BrokerEndpoint, BrokerServerConfig,
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
