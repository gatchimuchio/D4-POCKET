#[cfg(any(windows, test))]
pub(crate) mod content_access;
pub(crate) mod history_access;
pub(crate) mod execution_history;
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
pub mod store;

pub use audit::{BrokerAuditEvent, BrokerAuditLog};
pub use ipc_server::{run_loopback_server, BrokerEndpoint, BrokerServerConfig};
pub use protocol::{
    Broker, BrokerError, BrokerHealth, BrokerMetadata, BrokerOperation, BrokerPersistenceMode,
    BrokerRequestEnvelope, BrokerResponse, BrokerStateStore, BrokerStatus,
};
pub use store::{BrokerPersistentState, BrokerPersistentStore, BrokerStoreError};
