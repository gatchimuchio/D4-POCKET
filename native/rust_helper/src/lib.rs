#![forbid(unsafe_code)]

#[cfg(windows)]
pub mod desktop_launcher;
#[cfg(windows)]
pub(crate) mod product_bootstrapper;
#[cfg(windows)]
pub mod protected_store;

pub(crate) mod a2a;
pub mod adapters;
pub mod audit_hash;
pub mod broker;
pub mod checkpoint;
pub mod diagnostics;
pub mod filesystem;
pub mod ipc;
#[cfg(target_os = "linux")]
mod linux_credential_input;
#[cfg(target_os = "linux")]
pub mod linux_desktop_worker;
#[cfg(target_os = "macos")]
mod macos_credential_input;
#[cfg(any(target_os = "linux", target_os = "macos", test))]
mod macos_desktop_owner;
#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub mod macos_desktop_worker;
#[cfg(any(target_os = "macos", test))]
mod macos_workspace_selection;
pub(crate) mod mcp;
pub mod network;
pub mod process;
pub mod product_package;
pub mod update_verification;
pub mod workspace_diff;
pub mod workspace_reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperResponse<T> {
    pub ok: bool,
    pub operation: String,
    pub result: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<HelperError>,
}

pub fn helper_ok<T>(operation: &str, result: T, diagnostics: Vec<Diagnostic>) -> HelperResponse<T> {
    HelperResponse {
        ok: true,
        operation: operation.to_string(),
        result: Some(result),
        diagnostics,
        error: None,
    }
}

pub fn helper_error<T>(
    operation: &str,
    code: &str,
    message: &str,
    recoverable: bool,
    diagnostics: Vec<Diagnostic>,
) -> HelperResponse<T> {
    HelperResponse {
        ok: false,
        operation: operation.to_string(),
        result: None,
        diagnostics,
        error: Some(HelperError {
            code: code.to_string(),
            message: message.to_string(),
            recoverable,
        }),
    }
}

pub mod installed_artifact;
