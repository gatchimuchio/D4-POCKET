"""開発専用の実TLS・端末資格・実MINIDORA経路検証。試験自身の一時資格だけを使う。"""
from __future__ import annotations
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import ssl
import subprocess
import time
import threading
import uuid

from tooling.minidora_live_check import 操作, 成功, 正本化

ROOT = Path(__file__).resolve().parents[1]


def 要求(資格, operation, payload=None):
    return {"版": 1, "HostID": 資格["HostID"], "端末ID": 資格["端末ID"],
            "資格ID": 資格.get("結合ID", 資格.get("招待ID")),
            "資格秘密": 資格.get("端末秘密", 資格.get("招待秘密")),
            "nonce": uuid.uuid4().hex, "発行時刻": int(time.time()), "操作": operation, "内容": payload or {}}


def 通信(資格, request, *, expected_hash=None):
    # private証明書は招待で固定する。OS trustの代わりに送信前のpeer hash一致を必須にする。
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE
    with socket.create_connection((資格["接続先Host"], 資格["port"]), timeout=5) as tcp:
        with context.wrap_socket(tcp, server_hostname="gui-shell.local") as stream:
            actual = hashlib.sha256(stream.getpeercert(binary_form=True)).hexdigest()
            if actual != (expected_hash or 資格["証明書hash"]):
                raise ValueError("Host証明書不一致。application資格は送信しない")
            stream.sendall((正本化(request) + "\n").encode())
            with stream.makefile("rb") as reader:
                raw = reader.readline(4 * 1024 * 1024 + 1)
                if len(raw) > 4 * 1024 * 1024:
                    raise ValueError("端末応答上限")
                return json.loads(raw)


def AndroidNative検証(owner, binary, root, mobile_port, serial):
    """製品Flutter UIからAndroid native Device Linkを実Rust Brokerへ接続する。"""
    if not isinstance(mobile_port, int) or not 1 <= mobile_port <= 65535:
        raise RuntimeError("Android native試験のBroker portが不正")
    adb = shutil.which("adb")
    if adb is None:
        sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
        if sdk:
            candidate = Path(sdk) / "platform-tools" / ("adb.exe" if os.name == "nt" else "adb")
            if candidate.is_file():
                adb = str(candidate)
    if adb is None:
        raise RuntimeError("Android SDK platform-toolsがない")

    def ADB(*args, timeout=30, allow_failure=False):
        result = subprocess.run([adb, "-s", serial, *args], capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=timeout)
        if result.returncode and not allow_failure:
            raise RuntimeError(f"adb操作失敗: {args[0]} (exit {result.returncode})")
        return result

    if ADB("get-state").stdout.strip() != "device":
        raise RuntimeError("指定Android deviceがonlineではない")
    if ADB("shell", "getprop", "ro.kernel.qemu").stdout.strip() != "1":
        raise RuntimeError("対象がAndroid Emulatorではない")
    avd_name = ADB("emu", "avd", "name").stdout.splitlines()
    if "gui_shell_native_test" not in avd_name:
        raise RuntimeError("専用gui_shell_native_test AVDではない")

    mobile = ROOT / "apps/mobile_flutter"
    android = mobile / "android"
    wrapper = android / ("gradlew.bat" if os.name == "nt" else "gradlew")
    target_apk = mobile / "build/app/outputs/flutter-apk/app-debug.apk"
    if not target_apk.is_file():
        raise RuntimeError("現行sourceのdebug APKがない")
    gradle = subprocess.run([str(wrapper), ":app:assembleDebugAndroidTest", "--no-daemon", "--console=plain"],
                            cwd=android, capture_output=True, text=True, encoding="utf-8", errors="replace",
                            timeout=600)
    if gradle.returncode:
        tail = (gradle.stdout + "\n" + gradle.stderr)[-2400:]
        raise RuntimeError("Android instrumentation APK build失敗:\n" + tail)
    test_apk = mobile / "build/app/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
    if not test_apk.is_file():
        raise RuntimeError("Android instrumentation APKが見つからない")

    # AVDを専用のfresh app stateにし、既存Android Keystore資格をテストへ持ち込まない。
    if ADB("shell", "pm", "path", "com.example.gui_shell_mobile", allow_failure=True).stdout.strip().startswith("package:"):
        ADB("uninstall", "com.example.gui_shell_mobile")
    if ADB("shell", "pm", "path", "com.example.gui_shell_mobile.test", allow_failure=True).stdout.strip().startswith("package:"):
        ADB("uninstall", "com.example.gui_shell_mobile.test")
    ADB("install", "-r", str(target_apk), timeout=120)
    ADB("install", "-r", str(test_apk), timeout=120)

    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(240)
    bridge_port = listener.getsockname()[1]
    issued = {}
    server_errors = []
    server_finished = threading.Event()
    invitation_path = root / ("android-native-invitation-" + uuid.uuid4().hex + ".json")

    def serve_invitation():
        try:
            with listener.accept()[0] as connection:
                connection.settimeout(20)
                request = bytearray()
                while len(request) <= 40 and not request.endswith(b"\n"):
                    part = connection.recv(41 - len(request))
                    if not part:
                        break
                    request.extend(part)
                if not request.endswith(b"\n") or not re.fullmatch(rb"[a-f0-9]{32}\n", request):
                    raise RuntimeError("device ID frame invalid")
                device_id = request[:-1].decode("ascii")
                result = subprocess.run(
                    [str(binary), "対話承認操作", "--session-file", str(root / "owner.json"),
                     "端末招待", device_id, "127.0.0.1", str(invitation_path)],
                    capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10,
                )
                if result.returncode != 0:
                    raise RuntimeError("owner invitation failed")
                invitation = json.loads(invitation_path.read_text(encoding="utf-8"))
                invitation_path.unlink()
                secret = invitation.get("招待秘密")
                if not isinstance(secret, str) or secret in result.stdout + result.stderr:
                    raise RuntimeError("owner invitation output boundary failed")
                issued.update({"device_id": device_id, "invitation": invitation, "secret": secret})
                connection.sendall(json.dumps(invitation, ensure_ascii=False, separators=(",", ":")).encode("utf-8") + b"\n")
        except BaseException as error:
            server_errors.append(type(error).__name__)
        finally:
            server_finished.set()

    server_thread = threading.Thread(target=serve_invitation, name="android-native-invitation-bridge", daemon=True)
    reverse_ports = []
    stage = "ADB reverse"
    caught = None
    try:
        server_thread.start()
        for port in sorted({bridge_port, mobile_port}):
            ADB("reverse", f"tcp:{port}", f"tcp:{port}")
            reverse_ports.append(port)
        stage = "instrumentation"
        runner = ADB("shell", "am", "instrument", "-w", "-r", "-e", "bridge_port", str(bridge_port),
                     "-e", "class", "com.example.gui_shell_mobile.NativeDeviceLinkProductFlowTest",
                     "com.example.gui_shell_mobile.test/androidx.test.runner.AndroidJUnitRunner", timeout=360)
        instrument_output = runner.stdout + "\n" + runner.stderr
        if server_thread.is_alive():
            server_thread.join(timeout=10)
        invitation_secret = issued.get("secret")
        if invitation_secret and invitation_secret in instrument_output:
            raise RuntimeError("instrumentation output contained invitation secret")
        if runner.returncode != 0 or not re.search(r"OK \(\s*1 test\)", instrument_output) or "FAILURES!!!" in instrument_output:
            safe_stage = re.search(r"Android native Device Link E2E failed at ([^\s(]+)", instrument_output)
            raise RuntimeError("instrumentation failed at " + (safe_stage.group(1) if safe_stage else "runner"))
        if server_errors:
            raise RuntimeError("invitation bridge failed: " + server_errors[0])
        if not issued.get("device_id") or not server_finished.is_set():
            raise RuntimeError("native test did not request a broker invitation")

        # Native結合が実Brokerで失効したこと、未消費招待が残らないことをOwner経路で照合する。
        state = 成功(owner, "端末一覧", {})
        device_id = issued["device_id"]
        assert not any(item.get("端末ID") == device_id for item in state.get("結合", [])), "Android test pairing remained active"
        assert not any(item.get("端末ID") == device_id for item in state.get("招待", [])), "Android test invitation remained active"
        audit_path = root / "store/audit.jsonl"
        audit = audit_path.read_text(encoding="utf-8")
        if invitation_secret in audit:
            raise RuntimeError("Rust Broker Audit contained invitation secret")
        broker_log = (root / "process.log").read_text(encoding="utf-8", errors="replace")
        if invitation_secret in broker_log:
            raise RuntimeError("Rust Broker log contained invitation secret")
    except BaseException as error:
        detail = str(error)
        if any(marker in detail for marker in ("招待秘密", "端末秘密", "資格秘密")):
            detail = ""
        if stage == "instrumentation":
            audit_path = root / "store/audit.jsonl"
            try:
                relevant = []
                for line in audit_path.read_text(encoding="utf-8").splitlines():
                    event = json.loads(line)
                    operation = event.get("operation")
                    if operation not in {"実行系列挙", "実行系ライフサイクル状態", "端末招待", "端末結合", "端末確認", "端末離脱"}:
                        continue
                    relevant.append({
                        "operation": operation,
                        "decision": event.get("decision") if event.get("decision") in {"accepted", "rejected", "suspended", "received", "recorded"} else "unknown",
                    })
                if relevant:
                    detail += "; broker_audit=" + json.dumps(relevant[-12:], ensure_ascii=False, separators=(",", ":"))
            except (OSError, ValueError, TypeError):
                pass
        caught = (stage, type(error).__name__, detail)
    finally:
        try:
            listener.close()
        except OSError:
            pass
        if server_thread.is_alive():
            server_thread.join(timeout=5)
        if invitation_path.exists():
            invitation_path.unlink()
        for port in reversed(reverse_ports):
            ADB("reverse", "--remove", f"tcp:{port}", allow_failure=True)
        # 失敗時もtest invitation／pairingをOwner操作で回収し、AVD上のAPKを除去する。
        if issued.get("device_id"):
            try:
                state = 成功(owner, "端末一覧", {})
                device_id = issued["device_id"]
                for item in state.get("招待", []):
                    if item.get("端末ID") == device_id:
                        成功(owner, "端末招待取消", {"招待ID": item["招待ID"]})
                for item in state.get("結合", []):
                    if item.get("端末ID") == device_id:
                        成功(owner, "端末失効", {"結合ID": item["結合ID"]})
            except Exception:
                if caught is None:
                    caught = ("Owner cleanup", "RuntimeError", "")
        ADB("uninstall", "com.example.gui_shell_mobile.test", allow_failure=True)
        ADB("uninstall", "com.example.gui_shell_mobile", allow_failure=True)

    if caught:
        stage_name, error_type, safe_detail = caught
        raise RuntimeError(f"Android native Device Link LIVE_RUNTIME failed at {stage_name} ({error_type})" + (f": {safe_detail}" if safe_detail else ""))
    return {
        "result": "PASS",
        "evidence_source": "LIVE_RUNTIME",
        "target": "Android 15 API 35 gui_shell_native_test Emulator",
        "product_ui_method_channel": "PASS",
        "native_tls_pair_and_keystore": "PASS",
        "wrong_certificate_pin_rejected": "PASS",
        "existing_broker_runtime_query": "PASS",
        "background_resume": "PASS",
        "product_disconnect_and_owner_revocation": "PASS",
        "secret_scan": "invitation absent from instrumentation output, broker log, and durable Audit",
    }


def 検証(normal, owner, binary, root):
    for op in ("端末一覧", "端末招待", "端末失効", "端末招待取消"):
        assert 操作(normal, op, {})["status"] == "rejected"
    invites = []
    def 招待():
        target = root / ("invitation-" + uuid.uuid4().hex + ".json")
        result = subprocess.run([str(binary), "対話承認操作", "--session-file", str(root / "owner.json"),
                                 "端末招待", uuid.uuid4().hex, "127.0.0.1", str(target)],
                                capture_output=True, text=True, encoding="utf-8", timeout=10)
        assert result.returncode == 0, "owner招待CLI失敗"
        invitation = json.loads(target.read_text(encoding="utf-8"))
        assert invitation["招待秘密"] not in result.stdout + result.stderr
        invites.append(invitation)
        return invitation
    invitation = 招待()
    frame = 要求(invitation, "端末結合")
    response = 通信(invitation, frame)
    assert response["status"] == "accepted"
    credential = response["body"]
    assert 通信(invitation, frame)["status"] == "rejected"
    frame = 要求(credential, "端末確認")
    assert 通信(credential, frame)["status"] == "accepted"
    assert 通信(credential, frame)["status"] == "rejected"
    assert 通信(credential, 要求(credential, "端末確認"))["status"] == "accepted"
    for change in ({"HostID": "0" * 32}, {"端末ID": "0" * 32}, {"資格秘密": "0" * 64},
                   {"発行時刻": int(time.time()) - 61}, {"操作": "対話承認"}, {"操作": "shutdown"}, {"owner": True}):
        assert 通信(credential, {**要求(credential, "端末確認"), **change})["status"] == "rejected"
    try:
        通信(credential, 要求(credential, "端末確認"), expected_hash="0" * 64)
        raise AssertionError("別証明書を受理した")
    except ValueError as error:
        assert "Host証明書不一致" in str(error)
    def 端末成功(op, payload=None, pair=credential):
        result = 通信(pair, 要求(pair, op, payload))
        assert result["status"] == "accepted", (op, result.get("error"))
        return result["body"]
    session = 端末成功("対話開始", {"実行系ID": "left"})["対話セッションID"]
    pending = 端末成功("対話送信", {"対話セッションID": session, "入力": "こんにちは"})
    other_invite = 招待()
    other = 通信(other_invite, 要求(other_invite, "端末結合"))["body"]
    for op, payload in (("対話送信", {"対話セッションID": session, "入力": "他端末"}),
                        ("対話取得", {"要求ID": pending["要求ID"]}), ("対話中止", {"要求ID": pending["要求ID"]}),
                        ("対話終了", {"対話セッションID": session})):
        assert 通信(other, 要求(other, op, payload))["status"] == "rejected"
    成功(owner, "対話承認", {"要求ID": pending["要求ID"], "要求hash": pending["要求hash"], "表示範囲": "full"})
    deadline = time.monotonic() + 15
    while True:
        progress = 端末成功("対話取得", {"要求ID": pending["要求ID"]})
        if progress["状態"] == "完了":
            assert progress["結果"]["状態"] == "成功" and progress["結果"]["本文"] and progress["結果"]["追跡ID"]
            break
        assert time.monotonic() < deadline
        time.sleep(.05)
    second = 端末成功("対話送信", {"対話セッションID": session, "入力": "こんにちは"})
    成功(owner, "端末失効", {"結合ID": credential["結合ID"]})
    assert 通信(credential, 要求(credential, "端末確認"))["status"] == "rejected"
    assert 操作(owner, "対話承認", {"要求ID": second["要求ID"], "要求hash": second["要求hash"], "表示範囲": "full"})["status"] == "rejected"
    cancelled = 招待()
    成功(owner, "端末招待取消", {"招待ID": cancelled["招待ID"]})
    assert 通信(cancelled, 要求(cancelled, "端末結合"))["status"] == "rejected"
    assert 通信(other, 要求(other, "端末離脱"))["status"] == "accepted"
    assert 通信(other, 要求(other, "端末確認"))["status"] == "rejected"
    audit = (root / "store/audit.jsonl").read_text(encoding="utf-8")
    for secret in [credential["端末秘密"], other["端末秘密"], *[v["招待秘密"] for v in invites]]:
        assert secret not in audit
    return "PASS"
