# Mobile Status

Mobile is not part of v1.0 completed product release unless owner explicitly changes scope.

Mobile is `post_v1_scope`. The mobile Flutter app may remain as a bounded companion surface, but this cleanup does not improve mobile and does not track `apps/mobile_flutter/pubspec.lock`; local Flutter tooling may regenerate it during mobile-only work.

Mobile is excluded from the v1.0 release gate, CI product claim, and advertised support surface unless the owner explicitly opts mobile into the release scope.

## Implemented Areas

- item: mobile dashboard
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  blocks_release: no

- item: approval review surface
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  blocks_release: no

- item: notifications surface
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  blocks_release: no

- item: runtime status surface
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  blocks_release: no

- item: emergency stop request surface
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  blocks_release: no

## Post-v1 Scope

- item: real device pairing
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  required_action: Complete during mobile release phase.
  blocks_release: no

- item: push notifications
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  required_action: Complete during mobile release phase.
  blocks_release: no

- item: Shell Core IPC for mobile
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  required_action: Complete during mobile release phase.
  blocks_release: no

- item: cryptographic device binding
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  required_action: Complete during mobile release phase.
  blocks_release: no

- item: mobile release packaging
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 desktop scope.
  required_action: Complete during mobile release phase.
  blocks_release: no
