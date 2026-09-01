$ErrorActionPreference = "Stop"

# 境界記録:
# - 目的: Rust broker-server を稼働させた状態で、所有者のローカルデスクトップを起動する。
# - 標準機構: native Rust helper broker-server と Flutter desktop run。
# - wrapper の理由: 所有者利用時の broker endpoint/session 接続を再現可能に保つ。
# - 削除条件: インストール済み製品の launcher がこの接続を所有したときに削除する。
# - release 証拠: これは Windows インストール先の release 証拠ではない。
# - 権限: この wrapper はcapability、permission、approval、audit 権限を与えない。

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
      throw "Rust broker が endpoint 準備前に終了しました: $($Broker.ExitCode)"
        }
        Start-Sleep -Milliseconds 50
    }

    if (-not (Test-Path $BrokerSession)) {
    throw "Rust broker endpoint ファイルが作成されませんでした: $BrokerSession"
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
