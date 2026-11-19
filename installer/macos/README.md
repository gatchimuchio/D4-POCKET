# macOS Installer source

`tooling/export_macos_product.py` generates an install package and an identity-bound removal package from the same validated Export Manifest.

## Install, update, repair

- The macOS flat `.pkg` contains the exported App Bundle and directs Installer to `/Applications`. Its component rules disable relocation, require the same Bundle ID at the install path, and preserve a newer installed version.
- Its package identifier is derived from the persistent Export App bundle identifier. A later version for the same App identity keeps this identifier so macOS Installer can treat it as the same package.
- The package uses macOS system ownership for installed payload files. Installer replaces the App Bundle as a component, which removes files omitted from the new version. Repair uses the same package to restore a missing App Bundle.
- Packages remain unsigned candidates until the separately controlled Publisher identity and signing work is authorized and completed.

## Removal

- The generated `D4Pocket-<12 hex>-Uninstaller.pkg` is scripts-only and is applied through macOS Installer.
- `uninstall/postinstall.sh.in` is rendered with the exact App Bundle name and Bundle ID. It resolves the Installer target volume, rejects symlinked target directories, reads `Contents/Info.plist`, and removes only that matching App Bundle.
- A missing App Bundle is a successful no-op; a path type or identity mismatch fails closed.
- Application Support data, Keychain entries, and Installer package receipts are deliberately left untouched. The package script does not invoke `pkgutil --forget`.

## Current evidence boundary

The manual `macos_export` workflow installs the candidate on an ephemeral macOS runner, opens the installed app and its Broker, repairs the candidate after removing its App Bundle, and runs the matching uninstaller. The smoke checks root ownership, wheel group, and write permissions for the Bundle entry points after install and repair. A marker checks preservation of user data during removal. Formal signing, notarization, and distribution require separate evidence.
