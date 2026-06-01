use crate::{helper_ok, Diagnostic, HelperResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDiagnosticRequest {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub external_fetch_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDiagnosticResult {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub external_fetch_performed: bool,
}

pub fn diagnose_network(
    request: &NetworkDiagnosticRequest,
) -> HelperResponse<NetworkDiagnosticResult> {
    let mut diagnostics = Vec::new();
    if request.external_fetch_url.is_some() {
        diagnostics.push(Diagnostic {
            code: "external_fetch_ignored".to_string(),
            message: "network diagnostics do not perform arbitrary external fetches".to_string(),
        });
    }

    helper_ok(
        "network.diagnose",
        NetworkDiagnosticResult {
            host: request.host.clone(),
            port: request.port,
            external_fetch_performed: false,
        },
        diagnostics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_diagnostic_does_not_fetch_external_url_by_default() {
        let response = diagnose_network(&NetworkDiagnosticRequest {
            host: Some("localhost".to_string()),
            port: Some(8080),
            external_fetch_url: Some("https://example.com".to_string()),
        });
        assert!(response.ok);
        assert!(!response.result.unwrap().external_fetch_performed);
    }
}
