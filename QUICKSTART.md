# GUI Shell Quickstart

GUI Shell is currently a release-hardening skeleton, not a production runtime. The quickstart path validates contracts and conformance scaffolding before product claims.

## Prerequisites

- A POSIX-like shell
- Python available as `python` or `python3`
- Optional: Rust for `native/rust_helper`
- Optional: Flutter for `apps/desktop_flutter`
- Optional post-v1: Flutter for `apps/mobile_flutter`

## Contract validation

Preferred commands:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

Fallback when `python` is not on `PATH`:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

Expected successful output:

```text
schema check passed: 26 schemas, 26 examples, 28 negative fixtures
conformance skeleton passed: 139 checks
```

## Phase B owner launch

Generate a local owner-operation snapshot and launch the desktop shell without running strict release validation:

```bash
bash scripts/launch_owner_desktop.sh
```

On native Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\launch_owner_desktop.ps1
```

This path is for Phase B local owner operation. It does not create `release_evidence/windows_installed_smoke.json`, does not claim release readiness, and does not satisfy measured Windows installed-path release evidence.

## Optional Rust helper check

```bash
cd native/rust_helper
cargo test
```

The Rust helper contains the current Rust Security Broker skeleton for broker envelope validation and rejection audit. Real external command dispatch is disabled until authority migration and IPC integration evidence exists.

## Optional desktop Flutter check

```bash
cd apps/desktop_flutter
flutter analyze
```

Flutter is the replaceable UI layer. UI widgets may collect operator input and render status, but they must not define authority, permission, approval, audit, or recovery semantics.

Mobile Flutter is `post_v1_scope` and is excluded from the v1.0 release gate and CI product claim unless the owner explicitly changes scope.

## Next implementation order

1. Keep `docs/standards/gui-shell-extended-standard.md` authoritative.
2. Extend JSON Schemas under `specs/`.
3. Add or update conformance tests.
4. Generate or update contracts.
5. Keep claim documents aligned with actual validation evidence.
