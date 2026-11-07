use crate::{helper_ok, Diagnostic, HelperResponse};
use std::path::Path;

const SECRET_PATH_PARTS: &[&str] = &[".env", ".ssh", ".gnupg", "secrets"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemDiagnosticRequest {
    pub path: String,
    pub read_content: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemDiagnosticResult {
    pub path: String,
    pub content_read: bool,
    pub symlink_detected: bool,
    pub secret_path_detected: bool,
}

pub fn diagnose_filesystem(
    request: &FilesystemDiagnosticRequest,
) -> HelperResponse<FilesystemDiagnosticResult> {
    let path = Path::new(&request.path);
    let symlink_detected = path
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    let canonical_secret = path
        .canonicalize()
        .map(|canonical| path_has_secret_part(&canonical))
        .unwrap_or_else(|_| path_has_secret_part(path));
    let secret_path_detected = canonical_secret || path_has_secret_part(path);
    let diagnostics = if secret_path_detected {
        vec![Diagnostic {
            code: "filesystem_secret_path_diagnostic_blocked".to_string(),
            message:
                "filesystem diagnosticのtargetがsecret pathへ解決されたため、contentを読まなかった"
                    .to_string(),
        }]
    } else if symlink_detected {
        vec![Diagnostic {
            code: "filesystem_symlink_diagnostic_observed".to_string(),
            message: "filesystem diagnosticのtargetがsymlinkであるため、contentを読まなかった"
                .to_string(),
        }]
    } else {
        vec![]
    };
    helper_ok(
        "filesystem.diagnose",
        FilesystemDiagnosticResult {
            path: request.path.clone(),
            content_read: false,
            symlink_detected,
            secret_path_detected,
        },
        diagnostics,
    )
}

fn path_has_secret_part(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        SECRET_PATH_PARTS
            .iter()
            .any(|secret| value == *secret || value.ends_with(secret))
    })
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

    #[cfg(unix)]
    #[test]
    fn filesystem_diagnostic_detects_symlink_to_secret() {
        use std::fs;
        use std::os::unix::fs::symlink;

        let root =
            std::env::temp_dir().join(format!("gui-shell-fs-symlink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("public")).unwrap();
        fs::write(root.join(".env"), "TOKEN=secret\n").unwrap();
        let link = root.join("public").join("linked-config");
        symlink(root.join(".env"), &link).unwrap();

        let response = diagnose_filesystem(&FilesystemDiagnosticRequest {
            path: link.to_string_lossy().to_string(),
            read_content: true,
        });
        let result = response.result.unwrap();
        assert!(!result.content_read);
        assert!(result.symlink_detected);
        assert!(result.secret_path_detected);
        assert_eq!(
            response.diagnostics[0].code,
            "filesystem_secret_path_diagnostic_blocked"
        );
        let _ = fs::remove_dir_all(root);
    }
}
