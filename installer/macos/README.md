# macOS Installer source

`tooling/export_macos_product.py` generates an install package and an identity-bound removal package from the same validated Export Manifest.

## Install, update, repair

- The install package is a macOS flat `.pkg` that places the exported App Bundle under `/Applications` through the native Installer.
- Its package identifier is derived from the persistent Export App bundle identifier. A later version for the same App identity keeps this identifier so macOS Installer can treat it as the same package.
- Repair uses the same package to restore the App Bundle payload. The manual hosted smoke will verify reinstall after removing the candidate App Bundle.
- Packages remain unsigned candidates until the separately controlled Publisher identity and signing work is authorized and completed.

## Removal

- The generated `D4Pocket-<12 hex>-Uninstaller.pkg` is scripts-only and is applied through macOS Installer.
- `uninstall/postinstall.sh.in` is rendered with the exact App Bundle name and Bundle ID. It resolves the Installer target volume, rejects symlinked target directories, reads `Contents/Info.plist`, and removes only that matching App Bundle.
- A missing App Bundle is a successful no-op; a path type or identity mismatch fails closed.
- Application Support data, Keychain entries, and Installer package receipts are deliberately left untouched. The package script does not invoke `pkgutil --forget`.

## Current evidence boundary

The source recipe and build evidence are implemented. The next manual `workflow_dispatch` is intended to exercise install, repair, and removal on an ephemeral hosted macOS runner, using a fixed test identity and a user-data preservation marker. It does not launch the GUI or validate sandbox, Keychain use, permissions, formal signing/notarization, or public distribution. Those remain separate product requirements and evidence gates.
