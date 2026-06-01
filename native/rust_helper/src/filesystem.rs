use crate::{helper_ok, HelperResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemDiagnosticRequest {
    pub path: String,
    pub read_content: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemDiagnosticResult {
    pub path: String,
    pub content_read: bool,
}

pub fn diagnose_filesystem(
    request: &FilesystemDiagnosticRequest,
) -> HelperResponse<FilesystemDiagnosticResult> {
    helper_ok(
        "filesystem.diagnose",
        FilesystemDiagnosticResult {
            path: request.path.clone(),
            content_read: false,
        },
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_diagnostic_does_not_read_content_by_default() {
        let response = diagnose_filesystem(&FilesystemDiagnosticRequest {
            path: "Cargo.toml".to_string(),
            read_content: false,
        });
        assert!(response.ok);
        assert!(!response.result.unwrap().content_read);
    }
}
