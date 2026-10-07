"""開発専用。Simulator native XCTestへ一時招待を渡し、実Brokerとの接続を確認する。"""
from __future__ import annotations

import json
import os
import re
import signal
import socket
import subprocess
import sys
import threading
import uuid

from tooling.minidora_live_check import ROOT, 成功


def 検証(owner, binary, root, simulator, derived_data, result_bundle, product_ui=False):
    if sys.platform != "darwin" or not re.fullmatch(r"[A-Fa-f0-9-]{36}", simulator or ""):
        raise RuntimeError("iOS native試験は明示指定したmacOS Simulator専用")
    devices = json.loads(subprocess.check_output(
        ["xcrun", "simctl", "list", "devices", "available", "--json"], text=True))
    matches = [device for runtime, entries in devices["devices"].items() if ".iOS-" in runtime
               for device in entries if device.get("udid") == simulator and device.get("isAvailable")]
    if len(matches) != 1:
        raise RuntimeError("指定したiOS Simulatorが利用不能")
    if product_ui and not matches[0].get("name", "").startswith("D4PocketNativeProduct-"):
        raise RuntimeError("製品UI試験は新規作成した専用Simulatorだけを対象とする")

    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(1)
    stop = threading.Event()
    issued = {}
    bridge_errors = []
    invitation_path = root / ("ios-native-invitation-" + uuid.uuid4().hex + ".json")

    def serve_invitation():
        try:
            while not stop.is_set():
                try:
                    connection, _ = listener.accept()
                    break
                except socket.timeout:
                    continue
            else:
                return
            with connection:
                connection.settimeout(20)
                request = bytearray()
                while len(request) <= 40 and not request.endswith(b"\n"):
                    data = connection.recv(41 - len(request))
                    if not data:
                        break
                    request.extend(data)
                if not re.fullmatch(rb"[a-f0-9]{32}\n", request):
                    raise RuntimeError("device_id_frame")
                device_id = request[:-1].decode("ascii")
                issued["device_id"] = device_id
                created = subprocess.run(
                    [str(binary), "対話承認操作", "--session-file", str(root / "owner.json"),
                     "端末招待", device_id, "127.0.0.1", str(invitation_path)],
                    capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10)
                if created.returncode:
                    raise RuntimeError("owner_invitation")
                invitation = json.loads(invitation_path.read_text(encoding="utf-8"))
                invitation_path.unlink()
                secret = invitation.get("招待秘密")
                if not isinstance(secret, str) or secret in created.stdout + created.stderr:
                    raise RuntimeError("owner_output_boundary")
                issued["secret"] = secret
                connection.sendall(json.dumps(invitation, ensure_ascii=False, separators=(",", ":")).encode("utf-8") + b"\n")
        except Exception as error:
            if not stop.is_set():
                bridge_errors.append(type(error).__name__)

    bridge = threading.Thread(target=serve_invitation, name="ios-native-invitation", daemon=True)
    stage = "xctest"
    failure = None
    try:
        bridge.start()
        # AppleのTEST_RUNNER_転送に非秘密portだけを渡す。招待・端末資格はnative socket内に限定する。
        env = dict(os.environ)
        port_key = "TEST_RUNNER_D4_IOS_PRODUCT_BRIDGE_PORT" if product_ui else "TEST_RUNNER_D4_IOS_NATIVE_BRIDGE_PORT"
        env[port_key] = str(listener.getsockname()[1])
        command = ["xcodebuild", "test", "-project", str(ROOT / "apps/mobile_flutter/ios/Runner.xcodeproj"),
                   "-scheme", "Runner", "-destination", f"platform=iOS Simulator,id={simulator}",
                   "-parallel-testing-enabled", "NO", "-derivedDataPath", str(derived_data),
                   "-resultBundlePath", str(result_bundle), "CODE_SIGNING_ALLOWED=YES",
                   "CODE_SIGNING_REQUIRED=YES", "CODE_SIGN_IDENTITY=-", "CODE_SIGN_STYLE=Manual"]
        if product_ui:
            command.extend(["-only-testing:RunnerUITests/DeviceLinkProductUITests",
                            "SWIFT_ACTIVE_COMPILATION_CONDITIONS=$(inherited) D4_IOS_PRODUCT_TEST"])
        else:
            command.append("-skip-testing:RunnerUITests")
        process = subprocess.Popen(command, env=env, cwd=ROOT, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace",
                                   start_new_session=True)
        try:
            output, _ = process.communicate(timeout=900)
        except BaseException:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate(timeout=10)
            raise
        bridge.join(timeout=5)
        secret = issued.get("secret")
        if secret and secret in output:
            raise RuntimeError("invitation_output_boundary")
        marker = "D4_IOS_PRODUCT" if product_ui else "D4_IOS_NATIVE_LIVE"
        if process.returncode or marker + "_PASS" not in output or "** TEST SUCCEEDED **" not in output:
            match = re.search(marker + r"_FAIL ([a-z_]+)", output)
            if match:
                stage = match.group(1)
            elif not issued:
                # 秘密が未発行のcompile／起動失敗だけをboundedに表示する。
                diagnostics = [line for line in output.splitlines() if "error:" in line or "failed" in line][-12:]
                print("\n".join(diagnostics)[-3000:])
            raise RuntimeError("native_test_failed")
        if bridge_errors or not secret or bridge.is_alive():
            raise RuntimeError("invitation_bridge_failed")
        stage = "owner_revocation"
        state = 成功(owner, "端末一覧", {})
        for key in ("招待", "結合"):
            if any(item.get("端末ID") == issued["device_id"] for item in state.get(key, [])):
                raise RuntimeError("device_state_remained")
        stage = "secret_scan"
        for path in (root / "process.log", root / "store/audit.jsonl"):
            if secret in path.read_text(encoding="utf-8", errors="replace"):
                raise RuntimeError("invitation_broker_output_boundary")
        for line in output.splitlines():
            if re.search(r"Executed \d+ tests?, with \d+ failures?", line):
                print(line)
    except BaseException as error:
        failure = (stage, type(error).__name__)
    finally:
        stop.set()
        bridge.join(timeout=25)
        listener.close()
        if invitation_path.exists():
            invitation_path.unlink()
        if issued.get("device_id"):
            try:
                state = 成功(owner, "端末一覧", {})
                for item in state.get("招待", []):
                    if item.get("端末ID") == issued["device_id"]:
                        成功(owner, "端末招待取消", {"招待ID": item["招待ID"]})
                for item in state.get("結合", []):
                    if item.get("端末ID") == issued["device_id"]:
                        成功(owner, "端末失効", {"結合ID": item["結合ID"]})
            except Exception:
                if failure is None:
                    failure = ("owner_cleanup", "RuntimeError")
        if bridge.is_alive() and failure is None:
            failure = ("bridge_cleanup", "RuntimeError")
    if failure:
        raise RuntimeError(f"iOS native端末連携の失敗: 段階={failure[0]}、分類={failure[1]}")
    if product_ui:
        return {"result": "PASS", "evidence_source": "LIVE_RUNTIME",
                "target": "iOS Simulator製品UIのXCUITest操作",
                "product_pair_native_confirmation": "PASS", "runtime_projection": "PASS",
                "os_background_resume": "PASS", "product_disconnect": "PASS",
                "owner_revocation": "PASS",
                "secret_scan": "招待秘密はXCTest出力・Broker log・durable Auditに不存在",
                "scope": "新規専用Simulatorの基本製品経路。物理端末・全lifecycle timing・正式配布は未検証。"}
    return {"result": "PASS", "evidence_source": "LIVE_RUNTIME", "target": "iOS Simulator native XCTest",
            "native_tls_pair": "PASS", "keychain_store_readback_delete": "PASS",
            "wrong_certificate_pin_rejected": "PASS", "existing_broker_runtime_query": "PASS",
            "disconnect_and_revoked_credential": "PASS", "owner_revocation": "PASS",
            "secret_scan": "invitation absent from XCTest output, Broker log, and durable Audit",
            "scope": "既存Swift transport／保管の実Broker接続。製品UI／確認dialog／OS lifecycle／物理端末は未検証。"}
