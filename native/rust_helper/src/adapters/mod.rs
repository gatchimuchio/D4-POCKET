pub mod codex_cli;
pub mod mcp_stdio;
pub mod minidora;
mod process_tree;

pub(crate) fn supports_cli_adapter(adapter_id: &str) -> bool {
    cli_adapter_confirmation_scope(adapter_id).is_some()
}

pub(crate) fn cli_adapter_confirmation_scope(adapter_id: &str) -> Option<&'static str> {
    match adapter_id {
        "codex-cli" => Some("--version と exec --help"),
        _ => None,
    }
}

pub(crate) fn create_cli_adapter(
    adapter_id: &str,
    executable: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<std::sync::Arc<dyn crate::broker::dialogue::実行系Adapter>, String> {
    match adapter_id {
        "codex-cli" => Ok(std::sync::Arc::new(codex_cli::CodexCliAdapter::new(
            executable, workspace,
        )?)
            as std::sync::Arc<dyn crate::broker::dialogue::実行系Adapter>),
        _ => Err("未対応のCLI Adapter ID".to_owned()),
    }
}

/// 登録失敗を、CLI出力・path・OS errorを含まない固定理由へ射影する。
pub(crate) fn cli_registration_error_reason(error: &str) -> &'static str {
    match error {
        "Codex CLIのversion interfaceを確認できない" => "Codex CLI version応答の検証に失敗",
        "Codex CLIのexec help interfaceを確認できない" => {
            "Codex CLI exec --help応答の検証に失敗"
        }
        "Codex CLI probeを起動できない" => "Codex CLI probeの起動に失敗",
        "Codex CLI probeが期限を超過した" => "Codex CLI probeが期限超過",
        "Codex CLI probeの出力が上限を超えた" => "Codex CLI probe出力が上限超過",
        _ => "Owner確認後のAgent CLI interface検査に失敗",
    }
}

#[cfg(test)]
mod cli_adapter_registry_tests {
    use super::{cli_registration_error_reason, create_cli_adapter, supports_cli_adapter};

    #[test]
    fn registration_error_reason_is_bounded_and_never_reflects_unknown_input() {
        assert_eq!(
            cli_registration_error_reason("Codex CLIのversion interfaceを確認できない"),
            "Codex CLI version応答の検証に失敗"
        );
        assert_eq!(
            cli_registration_error_reason("Codex CLIのexec help interfaceを確認できない"),
            "Codex CLI exec --help応答の検証に失敗"
        );
        assert_eq!(
            cli_registration_error_reason("Codex CLI probeを起動できない"),
            "Codex CLI probeの起動に失敗"
        );
        assert_eq!(
            cli_registration_error_reason("unexpected C:\\private\\token"),
            "Owner確認後のAgent CLI interface検査に失敗"
        );
    }

    #[test]
    fn cli_adapter_registry_rejects_unknown_vendor_without_constructing_an_adapter() {
        assert!(supports_cli_adapter("codex-cli"));
        assert!(!supports_cli_adapter("unknown-cli"));
        let result = create_cli_adapter(
            "unknown-cli",
            std::path::Path::new("C:\\missing\\unknown.exe"),
            std::path::Path::new("C:\\missing\\workspace"),
        );
        assert!(matches!(result, Err(message) if message == "未対応のCLI Adapter ID"));
    }
}
