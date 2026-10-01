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

#[cfg(test)]
mod cli_adapter_registry_tests {
    use super::{create_cli_adapter, supports_cli_adapter};

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
