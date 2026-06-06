# Desktop Flutter App

Reserved desktop Flutter shell boundary.

classification: required_for_v1
reason: desktop Flutter is the replaceable operator UI surface and must remain UI-only.
blocks_release: no

This app must remain UI-only:

- render dashboard
- show runtime status
- show permission/approval/audit/recovery centers
- call generated contract clients
- call adapter/runtime APIs through explicit boundaries

It must not own authority decisions.
