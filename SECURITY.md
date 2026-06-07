# GUI Shell Security

## Security posture

GUI Shell is a control plane. Security decisions must be made in schema, conformance, Shell Core, adapter contracts, and bounded native helper surfaces, not in Flutter widgets.

## Authority rules

- Default deny for sensitive actions.
- Adapter metadata is untrusted.
- UI input is never authority.
- Memory, cache, and previous state are never authority by themselves.
- Runtime permissions must not be broadened silently.
- Full content display requires `content_visibility=full`.
- `full_payload` may exist in storage, but UI projection must not expose it unless `content_visibility=full`.

## Sensitive surfaces

The following surfaces require explicit capability, permission, approval, audit, and recovery treatment:

- filesystem access;
- process execution or process control;
- network access;
- credential access;
- IPC;
- update verification;
- runtime adapter actions;
- approval payload edits;
- audit export or inspection.

## Reporting security issues

Do not place secrets, tokens, private keys, raw approval payloads, or full hidden content in issue text, logs, screenshots, or audit examples.

Preferred route: use GitHub Security Advisories for this repository for private vulnerability reports. If GitHub does not expose a private advisory path and a public issue is necessary, include only a redacted reproduction and do not attach private evidence, secrets, tokens, raw approval payloads, or full hidden content.

Report the affected boundary, expected invariant, observed behavior, reproduction steps with redacted data, and validation commands run.
