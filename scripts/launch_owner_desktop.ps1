$ErrorActionPreference = "Stop"

# Boundary record:
# - purpose: local owner desktop launch with the Rust broker-server active.
# - standard mechanism: native Rust helper broker-server plus Flutter desktop run.
# - wrapper reason: keep the broker endpoint/session wiring reproducible for owner-use.
# - deletion condition: remove when the installed product launcher owns this wiring.
# - release evidence: this is not Windows installed-path release evidence.
# - authority: this wrapper grants no capability, permission, approval, or audit authority.

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$BrokerRoot = Join-Path $Root ".gui_shell\broker"
$BrokerStore = Join-Path $BrokerRoot "store"
$BrokerSession = Join-Path $BrokerRoot "broker_session.json"
$Helper = Join-Path $Root "native\rust_helper\target\debug\gui_shell_rust_helper.exe"

Set-Location $Root
if (-not (Test-Path $Helper)) {
    cargo build --manifest-path native\rust_helper\Cargo.toml
}

New-Item -ItemType Directory -Force -Path $BrokerStore | Out-Null
if (Test-Path $BrokerSession) {
    Remove-Item $BrokerSession -Force
}

$Broker = Start-Process -FilePath $Helper -ArgumentList @(
    "broker-server",
    "--store-dir",
    $BrokerStore,
    "--session-file",
    $BrokerSession
) -PassThru

try {
    for ($Index = 0; $Index -lt 100; $Index += 1) {
        if (Test-Path $BrokerSession) {
            break
        }
        if ($Broker.HasExited) {
            throw "Rust broker exited before endpoint was ready: $($Broker.ExitCode)"
        }
        Start-Sleep -Milliseconds 50
    }

    if (-not (Test-Path $BrokerSession)) {
        throw "Rust broker endpoint file was not created: $BrokerSession"
    }

    $env:GUI_SHELL_BROKER_ENDPOINT_JSON = $BrokerSession
    Set-Location (Join-Path $Root "apps\desktop_flutter")
    flutter run -d windows
}
finally {
    if (-not $Broker.HasExited) {
        Stop-Process -Id $Broker.Id -Force
    }
}
