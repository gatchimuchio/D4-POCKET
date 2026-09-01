#!/usr/bin/env bash
set -euo pipefail

# 境界記録:
# - 目的: Rust broker-server を稼働させた状態で、所有者のローカルデスクトップを起動する。
# - 標準機構: native Rust helper broker-server と Flutter desktop run。
# - wrapper の理由: 所有者利用時の broker endpoint/session 接続を再現可能に保つ。
# - 削除条件: インストール済み製品の launcher がこの接続を所有したときに削除する。
# - release 証拠: これは Windows インストール先の release 証拠ではない。
# - 権限: この wrapper はcapability、permission、approval、audit 権限を与えない。

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BROKER_ROOT="$ROOT/.gui_shell/broker"
BROKER_STORE="$BROKER_ROOT/store"
BROKER_SESSION="$BROKER_ROOT/broker_session.json"
HELPER="$ROOT/native/rust_helper/target/debug/gui_shell_rust_helper"

cd "$ROOT"
if [[ ! -x "$HELPER" ]]; then
  cargo build --manifest-path native/rust_helper/Cargo.toml
fi

mkdir -p "$BROKER_STORE"
rm -f "$BROKER_SESSION"

"$HELPER" broker-server \
  --store-dir "$BROKER_STORE" \
  --session-file "$BROKER_SESSION" &
BROKER_PID="$!"
trap 'kill "$BROKER_PID" 2>/dev/null || true' EXIT

for _ in {1..100}; do
  if [[ -f "$BROKER_SESSION" ]]; then
    break
  fi
  if ! kill -0 "$BROKER_PID" 2>/dev/null; then
    echo "Rust broker が endpoint 準備前に終了しました" >&2
    exit 1
  fi
  sleep 0.05
done

if [[ ! -f "$BROKER_SESSION" ]]; then
  echo "Rust broker endpoint ファイルが作成されませんでした: $BROKER_SESSION" >&2
  exit 1
fi

export GUI_SHELL_BROKER_ENDPOINT_JSON="$BROKER_SESSION"
cd apps/desktop_flutter
flutter run -d linux
