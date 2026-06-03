# GUI Shell Extended Standard v0.3.1

Status: Phase 0 Lock  
Scope: GUI Shell / Cross-device Runtime Operation Shell  
Primary decision: Flutter + Rust helper  
Reference runtime: BLUE-TANUKI  
Important note: BLUE-TANUKI is frozen as the Phase 0 reference runtime contract target.

## 1. Definition

GUI Shell is a generic runtime operation shell for multiple Runtime / Agent / Tool / Local Service targets.

GUI Shell is also an LLM-readable application responsibility substrate: LLM development / integration agents may read and consume GUI Shell contracts as first-class implementation and integration surfaces, but LLMs are never authority sources.

It provides:

- installation-to-start experience
- runtime launch and status monitoring
- capability and permission control
- approval workflow
- audit trail
- recovery workflow
- update policy
- device coordination
- adapter-based runtime integration

## 2. Non-goals

GUI Shell must not:

- become a BLUE-TANUKI-specific MVP
- embed BLUE-TANUKI-specific logic into Shell Core
- expose low-level CLI/WSL/npm/Git/runtime complexity to normal users
- become a terminal wrapper
- treat LLM/Agent output as authority
- treat LLM implementation work as proof of release readiness or external standard adoption
- store core assets in Flutter-only code

## 3. Core assets

The following assets must remain framework-independent:

- schema
- Adapter Contract
- Runtime Model
- Capability Model
- Permission Model
- Approval Model
- Audit Event Format
- Recovery Action Format
- Content Exposure Boundary
- Authority Strip Conformance
- Rust helper boundary
- conformance tests

## 4. UI framework boundary

Flutter may own:

- rendering
- user input
- UI state
- navigation
- theme
- localization
- accessibility

Flutter must not own:

- authority decisions
- permission semantics
- audit semantics
- adapter conformance
- recovery classification
- content visibility rules
- runtime trust rules

## 5. Phase 0 Lock decisions

- Generic GUI Shell direction: locked
- BLUE-TANUKI as reference runtime only: locked
- BLUE-TANUKI Phase 0 reference runtime contract target: locked
- Flutter + Rust helper as first candidate: locked
- Compose MP as watchlist candidate: locked
- Tauri as desktop-heavy fallback: locked
- Schema-first: locked
- Conformance-first: locked

## 6. Runtime safety invariants

The shell must enforce or test:

- inbound authority keys are stripped
- external metadata cannot escalate authority
- runtime-disallowed authority context cannot be created by GUI
- GUI input is not authority
- content visibility is respected
- approval edits are field-scoped
- edited payloads are rehashed and revalidated
- all sensitive actions create audit events
