use crate::{helper_ok, Diagnostic, HelperResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessDiagnosticRequest {
    pub pid: Option<u32>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessDiagnosticResult {
    pub pid: Option<u32>,
    pub command_executed: bool,
    pub command_ignored: bool,
}

pub fn diagnose_process(
    request: &ProcessDiagnosticRequest,
) -> HelperResponse<ProcessDiagnosticResult> {
    let mut diagnostics = Vec::new();
    if request.command.is_some() {
        diagnostics.push(Diagnostic {
            code: "command_ignored".to_string(),
            message: "process diagnostics do not execute arbitrary commands".to_string(),
        });
    }

    helper_ok(
        "process.diagnose",
        ProcessDiagnosticResult {
            pid: request.pid,
            command_executed: false,
            command_ignored: request.command.is_some(),
        },
        diagnostics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_diagnostic_does_not_execute_arbitrary_commands() {
        let response = diagnose_process(&ProcessDiagnosticRequest {
            pid: None,
            command: Some("rm -rf /".to_string()),
        });
        let result = response.result.unwrap();
        assert!(response.ok);
        assert!(!result.command_executed);
        assert!(result.command_ignored);
    }
}
