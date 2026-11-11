#!/usr/bin/env bash
case "${D4P_ACCEPTANCE_SCOPE:-}" in
  product_build|update_lifecycle|credential_vault|mcp_tool_execution|mcp_credential_binding|a2a_metadata|host_registration|workspace_selection|agent_task_supervision|agent_mcp_followup|agent_task_normal_release|agent_task_crash_recovery|provider_credential_binding|runtime_provenance|wayland_rendering|wayland_workspace_selection|portable_archive|debian_setup_package|debian_setup_first_install|debian_setup_lifecycle) ;;
  *) printf '%s\n' 'unsupported Linux acceptance scope' >&2; exit 2 ;;
esac
set -euo pipefail
fixture_root="$D4P_EXPORT_ROOT/fixture"
output_root="$D4P_EXPORT_ROOT/output"
umask 077
# RFC 8032 public test vector only; this key is deliberately public and never used by a product release.
printf '%s' '302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60' | xxd -r -p > "$D4P_EXPORT_ROOT/test-update-private.der"
openssl pkey -inform DER -in "$D4P_EXPORT_ROOT/test-update-private.der" -pubout -outform DER -out "$D4P_EXPORT_ROOT/test-update-public.der"
python3 tooling/tests/create_linux_update_catalog_fixture.py --private-key "$D4P_EXPORT_ROOT/test-update-private.der"
git diff --exit-code -- tooling/tests/fixtures/linux-update-catalog/updates.json tooling/tests/fixtures/linux-update-catalog/linux-installed-catalog-display.pkg
catalog_base_url="https://raw.githubusercontent.com/gatchimuchio/D4-POCKET/${GITHUB_SHA}/tooling/tests/fixtures/linux-update-catalog"
if [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_lifecycle' ]; then
  catalog_base_url='https://updates.d4p-test.example/d4/stable'
fi
fixture_arguments=(
  --output-dir "$fixture_root"
  --update-trust-public-key-der "$D4P_EXPORT_ROOT/test-update-public.der"
  --update-trust-base-url "$catalog_base_url"
)
if [ "$D4P_ACCEPTANCE_SCOPE" = 'host_registration' ]; then
  fixture_arguments+=(--include-host-operations)
fi
python3 tooling/tests/create_linux_export_fixture.py "${fixture_arguments[@]}"
export_arguments=(
  --receipt "$fixture_root/d4_pocket_export_receipt.json"
  --manifest-file "$fixture_root/d4-pocket-app-11111111111111111111111111111111.json"
  --output-dir "$output_root"
  --allow-verification-branch
)
if [ "$D4P_ACCEPTANCE_SCOPE" = 'agent_task_supervision' ] || [ "$D4P_ACCEPTANCE_SCOPE" = 'agent_mcp_followup' ] || [ "$D4P_ACCEPTANCE_SCOPE" = 'agent_task_crash_recovery' ] || [ "$D4P_ACCEPTANCE_SCOPE" = 'provider_credential_binding' ] || [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_lifecycle' ]; then
  export_arguments+=(--r2-e2e-test-build)
fi
python3 tooling/export_linux_product.py "${export_arguments[@]}"
python3 tooling/package_linux_product.py \
  --source-dir "$output_root/bundle" \
  --output "$output_root/D4Pocket-linux-rollback.d4pkg" \
  --product-version 0.1.1
if [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_lifecycle' ]; then
  python3 tooling/tests/prepare_linux_same_source_update.py \
    --bundle "$output_root/bundle" \
    --output-root "$D4P_EXPORT_ROOT" \
    --private-key "$D4P_EXPORT_ROOT/test-update-private.der" \
    --build-evidence "$output_root/d4_pocket_linux_export_build.json" \
    --source-commit "$GITHUB_SHA" \
    | tee "$D4P_EXPORT_ROOT/update-candidate-result.json"
fi
python3 tooling/linux_export_workflow.py verify-build-result "$output_root" "$(git rev-parse HEAD)" "$D4P_ACCEPTANCE_SCOPE" | tee "$D4P_EXPORT_ROOT/build-result.json"
python3 -m unittest tooling.tests.test_linux_workflow_helpers
python3 -m unittest tooling.tests.test_linux_gtk_bundle_smoke
python3 -m unittest tooling.tests.test_linux_installed_product_smoke
python3 -m unittest tooling.tests.test_export_linux_product
python3 -m unittest tooling.tests.test_linux_portable_archive
if [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_package' ] || \
   [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_first_install' ] || \
   [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_lifecycle' ]; then
  python3 -m unittest tooling.tests.test_linux_deb_setup_package
fi
if [ "$D4P_ACCEPTANCE_SCOPE" = 'portable_archive' ]; then
  archive_name="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["portable_archive"]["file_name"])' "$output_root/d4_pocket_linux_export_build.json")"
  python3 tooling/linux_export_workflow.py verify-portable-archive "$output_root" "$archive_name"
  extracted_root="$D4P_EXPORT_ROOT/portable-extracted"
  mkdir -m 700 "$extracted_root"
  umask 077
  tar --no-same-owner --no-same-permissions -xzf "$output_root/$archive_name" -C "$extracted_root"
  test -f "$extracted_root/D4Pocket/gui_shell_desktop_launcher"
  test -x "$extracted_root/D4Pocket/gui_shell_desktop_launcher"
  dbus-run-session -- xvfb-run -a python3 tooling/tests/linux_gtk_bundle_smoke.py \
    --launcher "$extracted_root/D4Pocket/gui_shell_desktop_launcher" \
    --temp-root "$D4P_EXPORT_ROOT" \
    --backend x11
  dbus-run-session -- python3 tooling/tests/linux_gtk_bundle_smoke.py \
    --launcher "$extracted_root/D4Pocket/gui_shell_desktop_launcher" \
    --temp-root "$D4P_EXPORT_ROOT" \
    --backend wayland
elif [ "$D4P_ACCEPTANCE_SCOPE" != 'product_build' ] && \
   [ "$D4P_ACCEPTANCE_SCOPE" != 'runtime_provenance' ] && \
   [ "$D4P_ACCEPTANCE_SCOPE" != 'wayland_rendering' ] && \
   [ "$D4P_ACCEPTANCE_SCOPE" != 'debian_setup_package' ] && \
   [ "$D4P_ACCEPTANCE_SCOPE" != 'debian_setup_first_install' ] && \
   [ "$D4P_ACCEPTANCE_SCOPE" != 'debian_setup_lifecycle' ]; then
  dbus-run-session -- xvfb-run -a python3 tooling/tests/linux_gtk_bundle_smoke.py \
    --launcher "$output_root/bundle/gui_shell_desktop_launcher" \
    --temp-root "$D4P_EXPORT_ROOT" \
    --backend x11
  dbus-run-session -- python3 tooling/tests/linux_gtk_bundle_smoke.py \
    --launcher "$output_root/bundle/gui_shell_desktop_launcher" \
    --temp-root "$D4P_EXPORT_ROOT" \
    --backend wayland
fi
{
  printf '\n- Export evidence: `%s`\n' "$(cat "$output_root/d4_pocket_linux_export_build.json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["artifact_tree_sha256"])')"
  printf -- '- Package hash: `%s`\n' "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["package"]["sha256"])' "$output_root/d4_pocket_linux_export_build.json")"
  if [ "$D4P_ACCEPTANCE_SCOPE" = 'portable_archive' ]; then
    printf -- '- Portable archive hash: `%s`\n' "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["portable_archive"]["sha256"])' "$output_root/d4_pocket_linux_export_build.json")"
  fi
  printf -- '- Fixture: synthetic Broker-shaped Receipt／Manifest; no Owner authority or credentials.\n'
  if [ "$D4P_ACCEPTANCE_SCOPE" = 'runtime_provenance' ]; then
    printf -- '- Portable X11／Wayland smoke was skipped because this scope verifies the installed product provenance; previously accepted scopes are not replayed.\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'product_build' ]; then
    printf -- '- Build-only scope: Linux Release export／package compilation and export helper unit tests ran. Portable launcher smoke and every installed-product Acceptance scope were skipped. This does not establish runtime or product acceptance.\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'wayland_rendering' ]; then
    printf -- '- Portable smoke was skipped; the dedicated installed-product Wayland surface-rendering acceptance runs below.\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'portable_archive' ]; then
    printf -- '- The unsigned Ubuntu 24.04 x86_64 tar.gz was hash-checked and extracted in the isolated runner; its extracted launcher completed the portable X11／Wayland smoke. This is not a signed distribution or general Linux acceptance.\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_package' ]; then
    printf -- '- Debian Setup package acceptance follows in a separate installed-package step; the APT package is a first-run bootstrap and does not replace Broker-controlled per-user product install/update/rollback/uninstall.\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_first_install' ]; then
    printf -- '- APT-installed Setup UIからの初回Broker導入Acceptanceを専用stepで実行します。従来のbootstrap package更新／rollback Acceptanceは再実行しません。\n'
  elif [ "$D4P_ACCEPTANCE_SCOPE" = 'debian_setup_lifecycle' ]; then
    printf -- '- APT-installed Setup初回導入からinstalled Update Centerのdownload／stage／activation／rollback、Export、Uninstallまでを同じrunで接続するD4-LNX-T06統合acceptanceを専用stepで実行します。更新candidateは同じsource bundleから生成する実D4PKGです。\n'
  else
    printf -- '- Runtime smoke: portable product Rust launcher opened and normally closed its Flutter GTK window under Xvfb/X11; Wayland run used Weston headless/Pixman, verified the live launcher GTK process successfully connected to the isolated Wayland socket using external connect-syscall observation, then stopped the owned process group. No debug environment variable was required or forwarded through the frontend allowlist. Both used an initially absent XDG_DATA_HOME and verified cleanup.\n'
    printf -- '- Scope: portable first-launch, Linux GTK X11 window close, and Wayland socket-connect smoke only; the Wayland check does not prove surface rendering. This build is not signed Install/Owner activation, installed-version handoff, production compositor coverage, formal distribution, or runtime Manifest consumption.\n'
  fi
} >> "$GITHUB_STEP_SUMMARY"
