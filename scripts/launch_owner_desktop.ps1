$ErrorActionPreference = "Stop"

# 境界記録:
# - 目的: Rust broker-server を稼働させた状態で、所有者のローカルデスクトップを起動する。
# - 標準機構: tooling/launch_owner_desktop_isolated.py が一時source snapshot内でRust helperとFlutterを実行する。
# - wrapper の理由: 所有者利用時の broker endpoint/session 接続を再現可能に保つ。
# - 出力境界: Cargo target、Flutter build、session fileはRepository／OneDrive外のsnapshotに置き、終了後に回収する。
# - 削除条件: インストール済み製品の launcher がこの接続を所有したときに削除する。
# - release 証拠: これは Windows インストール先の release 証拠ではない。
# - 権限: この wrapper はcapability、permission、approval、audit 権限を与えない。

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    $Python = Get-Command python3 -ErrorAction SilentlyContinue
}
if (-not $Python) {
    throw "Python 3 が必要です。AGENTS.md の開発環境手順を確認してください。"
}

& $Python.Source -X utf8 (Join-Path $Root "tooling\launch_owner_desktop_isolated.py")
exit $LASTEXITCODE
