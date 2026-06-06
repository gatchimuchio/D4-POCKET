#!/usr/bin/env bash
set -euo pipefail

# Boundary record:
# - purpose: local owner desktop launch with the Rust broker-server active.
# - standard mechanism: native Rust helper broker-server plus Flutter desktop run.
# - wrapper reason: keep the broker endpoint/session wiring reproducible for owner-use.
# - deletion condition: remove when the installed product launcher owns this wiring.
# - release evidence: this is not Windows installed-path release evidence.
# - authority: this wrapper grants no capability, permission, approval, or audit authority.

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
    echo "Rust broker exited before endpoint was ready" >&2
    exit 1
  fi
  sleep 0.05
done

if [[ ! -f "$BROKER_SESSION" ]]; then
  echo "Rust broker endpoint file was not created: $BROKER_SESSION" >&2
  exit 1
fi

export GUI_SHELL_BROKER_ENDPOINT_JSON="$BROKER_SESSION"
cd apps/desktop_flutter
flutter run -d linux
