# GUI Shell Operating Model

Status: Phase B owner-use complete operating model; completed product release not claimed  
Reference style: BLUE-TANUKI direct-main owner workflow  
Scope: repository flow, safety posture, validation, backup, and reporting

## 1. Core posture

GUI Shell is a control plane, not a visual wrapper.

The repository must move in this order:

```text
standard
  -> schema
  -> conformance
  -> Shell Core
  -> Runtime Catalog
  -> Agent Runtime Contract
  -> adapter
  -> Rust helper
  -> Shell Core persistence / audit chain
  -> desktop product UI
  -> installer / update
  -> v1.0 release gate
  -> post-v1.0 mobile companion
```

Later phases must not weaken earlier guarantees.

## 2. Priority order

1. Safety
2. Robustness
3. Operator clarity / UX
4. Product features
5. Convenience

Feature coverage and convenience are not valid reasons to weaken authority strip, content exposure, approval, audit, recovery, or schema validation.

## 3. Boundary model

```text
Runtime
  -> Adapter
      -> schema validation
      -> authority strip
      -> content exposure policy
  -> Shell Core
      -> permission
      -> approval
      -> audit
      -> recovery
  -> UI
      -> display
      -> operator input
  -> Rust helper
      -> bounded native operation
```

The UI can request and display. It cannot grant authority.

Adapter metadata can describe. It cannot grant permissions.

Memory, cache, and previous state can inform UX. They cannot grant authority by themselves.

## 4. Backup workflow

GUI Shell uses a two-generation direct-main backup flow. The backup refs are local recovery refs by default, not collaboration branches.

Before committing a completed work block on `main`:

```bash
# If codex/backup-main already exists:
git branch -f codex/backup-main-prev codex/backup-main

# Always update the latest backup to the current pre-commit main:
git branch -f codex/backup-main main
```

Then validate, commit directly on `main`, and push `main` when credentials allow.

Do not push `codex/backup-main` or `codex/backup-main-prev` during ordinary completed-work flow. GitHub treats pushed backup branches as normal branches and may present them as pull request candidates, which can create duplicate merge history when the backup branch is merged into `main`.

If remote backup branches already exist and the owner has not explicitly requested remote backup retention, delete them after `main` is clean and aligned:

```bash
git push origin --delete codex/backup-main codex/backup-main-prev
```

Push backup branches only when the owner explicitly requests off-machine backup retention or emergency recovery handoff. When this exception is used, report that GitHub may show those branches as pull request candidates, and do not open or merge pull requests from backup branches.

The repository keeps exactly two backup branches:

```text
codex/backup-main
codex/backup-main-prev
```

Do not create per-phase backup branches or extra backup generations.

## 5. Validation gates

Minimum validation:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

Fallback when `python` is unavailable:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

Conditional checks:

```bash
cd native/rust_helper && cargo test
cd apps/desktop_flutter && flutter analyze
```

Report every command as passed, failed, or not run.

## 6. Change report format

Every completed change report must include:

1. Summary
2. Changed files
3. Risk classification
4. Validation results
5. Remaining risks
6. Commit hash, or `not committed`

## 7. Release claim rule

Do not claim release readiness until all applicable validation gates pass and the owner explicitly approves the release claim.

Current claim boundary lives in:

```text
CLAIM.md
```
