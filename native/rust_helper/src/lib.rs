#![forbid(unsafe_code)]

pub mod audit_hash;
pub mod broker;
pub mod diagnostics;
pub mod filesystem;
pub mod ipc;
pub mod network;
pub mod process;
pub mod update_verification;

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
