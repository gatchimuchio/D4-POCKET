# LLM-Readable Extension Surface Standard

Status: definition lock workstream
Scope: GUI Shell / Runtime Operation Shell / LLM-readable application responsibility substrate
Reference runtime: BLUE-TANUKI through adapter only

## 1. Purpose

GUI Shell is a generic Runtime Operation Shell and LLM-readable application responsibility substrate.

It provides a stable, machine-readable responsibility structure into which LLM development / integration agents can implement or connect applications, runtimes, tools, services, and adapters without reinventing or bypassing:

- capability boundaries;
- permission boundaries;
- approval boundaries;
- authority stripping;
- content exposure controls;
- audit evidence;
- recovery behavior;
- update and install responsibility;
- runtime trust boundaries;
- validation and conformance requirements.

This document does not activate new runtime authority, add a module loader, add an SDK, or claim external standard adoption.

## 2. Role Model

Human operator / owner:

- observes runtime and shell state;
- grants or denies approval;
- authorizes or performs recovery;
- accepts or rejects release claims;
- remains the final responsibility holder.

LLM development / integration agent:

- reads architecture, standards, schemas, conformance rules, and operating documents;
- proposes or produces bounded code and documentation changes;
- connects new runtimes, tools, services, or adapters through declared contracts;
- runs validation and reports evidence;
- must not create authority, approve its own sensitive operations, widen permissions silently, bypass conformance, or convert generated output into trusted truth.

Runtime / tool / service target:

- exposes behavior only through declared runtime or adapter contracts;
- must not use metadata, diagnostics, or self-reporting to create authority.

Adapter:

- normalizes target state into GUI Shell schemas;
- strips authority;
- applies content exposure boundaries;
- maps diagnostics and failures without granting permission.

Rust Security Broker:

- remains the intended authority-sensitive production boundary for IPC, approval eligibility, audit, recovery classification, command-envelope gating, and sensitive native operations.

UI layer:

- renders operator surfaces and collects operator input;
- must not own authority, approval semantics, audit semantics, recovery classification, content visibility rules, or runtime trust rules.

## 3. Authority Model

LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.

LLM agents may implement, propose, explain, and validate. They may not grant permission, approve sensitive actions, replace human responsibility, or convert generated state into trusted runtime truth.

Human operators retain final approval, recovery, responsibility, and release-claim authority.

Broker, contract, and conformance paths govern sensitive execution. Any sensitive action must remain mapped to capability, permission, approval state, AuditEvent, and RecoveryAction on failure.

## 4. Connection Model

New functions must connect through existing contracts or through an explicitly introduced contract reviewed under the schema-first and conformance-first order.

New authority-relevant behavior must not be introduced as an undocumented shortcut, generated configuration side effect, UI-only decision, adapter metadata claim, memory/cache inference, diagnostic observation, or tool-response assertion.

Runtime-specific behavior must remain behind the adapter boundary. BLUE-TANUKI remains the first reference runtime and must not be pulled into Shell Core for GUI Shell convenience.

If a proposed integration cannot be represented by existing contracts, the correct result is a contract-design task, not an improvised production path.

## 5. Required Contract Families

LLM-built modules and integrations must account for these contract families before completion can be claimed:

- capability;
- permission;
- approval;
- audit;
- recovery;
- content exposure;
- adapter;
- runtime;
- update / install.

An explicit LLM extension submission or module integration contract may be introduced later only if contract design proves that existing schemas cannot represent bounded module onboarding safely.

## 6. Conformance Target Model

Future proof target:

```text
A third-party LLM development agent can read GUI Shell repository contracts and add a bounded reference module or adapter without breaking authority, approval, audit, recovery, content exposure, or runtime neutrality constraints.
```

The proof target requires negative cases, not only successful fixture examples. A valid harness must prove that the extension cannot escalate authority, cannot bypass approval, cannot expose disallowed content, emits required audit evidence, and maps failure to RecoveryAction or SUSPEND where required.

## 7. Non-Claims and Deferred Evidence

This definition update does not prove cross-agent implementation success.

It does not prove external standard adoption.

It does not activate new runtime authority.

It does not close current Windows-first release blockers.

It does not complete Rust Security Broker production convergence.

It does not authorize a plugin registry, module loader, SDK, marketplace, or broad runtime implementation.

Cross-agent reproduction and ecosystem claims remain deferred until measured evidence exists and the owner approves claim promotion.
