# Changelog

## Unreleased

- Aligned top-level repository documentation format with the BLUE-TANUKI reference style while preserving GUI Shell boundaries.
- Added claim, configuration, audit, security, troubleshooting, and quickstart documents for Phase 0 / Phase 1 operation.
- Expanded README with TL;DR, locked surface, explicit boundaries, architecture, validation, and top-level references.
- Reworked `AGENTS.md` around BLUE-TANUKI-style priorities, direct-main backup flow, validation gates, and report format for GUI Shell.
- Added `docs/OPERATING_MODEL.md` to document the repository flow, two-generation backup model, and release claim rule.
- Added `ROADMAP.md` for Phase 0 through release hardening execution order.
- Added Authority Strip Conformance documentation.
- Strengthened conformance checks with failure-case coverage for authority strip, metadata escalation, GUI-created authority context, non-authority state, content exposure, protected approval fields, approval edit rehash/revalidation, and sensitive action audit/recovery mapping.
- Strengthened schema validation with valid contract examples and invalid fixture rejection.
- Added Tauri fallback research note.
- Added MIT license.
- Replaced ambiguous BLUE-TANUKI freeze wording with Phase 0 reference runtime contract target wording.
- Added negative contract fixtures for adapter authority escalation, unsafe update policy, approval bad hash, and content exposure default full.
- Added conformance coverage that drives checks from `examples/contracts/*.valid.json`.
- Documented that `full_payload` may exist in approval storage but must not be exposed by UI projection unless `content_visibility=full`.
- Expanded invalid contract fixtures to cover every schema and made fixture coverage part of schema and conformance validation.
- Started Phase 3 Shell Core skeleton with framework-independent contract loading, runtime registry, adapter loader, permission ledger, approval queue, audit store, recovery catalog, update policy store, content exposure projection, and sensitive action routing.
- Added conformance checks proving Shell Core ignores adapter metadata permissions, rejects memory/cache/previous-state authority, routes sensitive actions through required mappings, hides full payload until full visibility, and avoids Flutter / BLUE-TANUKI internal imports.

## 0.1.0-phase0

- Initialized generic GUI Shell repository skeleton.
- Added Phase 0 standard.
- Added framework risk register documents.
- Added JSON Schema contracts.
- Added conformance test skeleton.
- Added Flutter desktop/mobile reserved boundaries.
- Added Rust helper reserved boundary.
- Added BLUE-TANUKI adapter reserved boundary.
