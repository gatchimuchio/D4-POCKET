# Runtime の目録（Runtime Catalog）

Runtime Catalog は manifest によって制御する。GUI-Shell は Runtime の挙動を Shell Core に hard-code してはならない。

Catalog entry は、次から構成する。

- `RuntimeManifest`
- `AdapterManifest`
- `PlatformProfile`
- `TrustProfile`
- `CapabilityProfile`
- `PermissionProfile`
- `RecoveryProfile`

Manifest は説明用の input である。それ自体では権限を付与できない。

`RuntimeManifest` の最小 field:

- `runtime_id`
- `display_name`
- `runtime_type`
- `supported_platforms`
- `required_tools`
- `required_ports`
- `storage_paths`
- `network_policy`
- `capabilities`
- `permissions`
- `audit_profile`
- `recovery_profile`
- `trust_profile`
- `signed_manifest`
