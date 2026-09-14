"""開発専用。固定した参照版の実APIとRust brokerを起動して対話経路を検証する。"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
REFERENCE = "3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8"

from tooling.schema_check.check_schemas import validate_instance


def 正本化(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def 操作(endpoint, operation, payload):
    nonce = uuid.uuid4().hex
    request = {"request_id": nonce, "nonce": nonce, "session_id": endpoint["session_id"],
               "operation": operation, "payload": payload, "metadata": {},
               "payload_hash": "sha256:" + hashlib.sha256(正本化(payload).encode()).hexdigest(),
               "issued_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    with socket.create_connection(("127.0.0.1", endpoint["port"]), timeout=5) as stream:
        stream.sendall((endpoint["session_secret"] + "\n" + 正本化(request) + "\n").encode())
        with stream.makefile("rb") as reader:
            raw = reader.readline(4 * 1024 * 1024 + 1)
            if len(raw) > 4 * 1024 * 1024:
                raise RuntimeError("broker応答上限超過")
            return json.loads(raw)


def 成功(endpoint, operation, payload):
    response = 操作(endpoint, operation, payload)
    if response["status"] != "accepted":
        raise RuntimeError(f"{operation}を拒否: {response.get('error')}")
    return response["body"]


def file待機(path, process, timeout=20):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if process.poll() is not None:
            raise RuntimeError("検証processが起動中に終了")
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.05)
    raise RuntimeError("起動期限超過")


def 終了(process):
    if process.poll() is None:
        process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def 検証(reference, binary, dart_client=False, mobile_client=False, dart_mobile_client=False, mobile_simulator=None, android_emulator=None):
    if mobile_simulator is not None and sys.platform != "darwin":
        raise RuntimeError("iOS Simulator統合はMac host専用")
    if mobile_simulator and android_emulator:
        raise RuntimeError("仮想端末を同時指定できません")
    adb = None
    if android_emulator:
        if sys.platform != "linux" or android_emulator != "emulator-5554":
            raise RuntimeError("Android試験はLinux専用runnerのemulator-5554に限定")
        sdk = os.environ.get("ANDROID_HOME")
        adb = str(Path(sdk) / "platform-tools/adb") if sdk else None
        if not adb or not Path(adb).is_file():
            raise RuntimeError("Android SDKのADBがない")
        def device_output(*args):
            return subprocess.check_output([adb, "-s", android_emulator, *args], text=True, timeout=10).strip()
        if device_output("shell", "getprop", "ro.kernel.qemu") != "1":
            raise RuntimeError("仮想端末属性を確認できません")
        if device_output("emu", "avd", "name").splitlines()[0] != "gui_shell_native_test":
            raise RuntimeError("専用AVD名と不一致")
    virtual_device = mobile_simulator or android_emulator
    simulator_result = None
    head = subprocess.check_output(["git", "-C", str(reference), "rev-parse", "HEAD"], text=True).strip()
    if head != REFERENCE:
        raise RuntimeError("MINIDORA参照commitが固定点と異なる")
    if subprocess.check_output(["git", "-C", str(reference), "status", "--porcelain"], text=True).strip():
        raise RuntimeError("MINIDORA参照に既存差分がある")
    # 参照製品の既存handlerと製品チャットをそのまま使う。基礎Core未接続を隠さない。
    server_code = '''import json,sys,time,faulthandler
from pathlib import Path
started=time.monotonic()
phase_file=Path(sys.argv[1]+".startup.json")
phases=[]
def phase(name):
    phases.append({"phase":name,"elapsed_seconds":round(time.monotonic()-started,3)})
    phase_file.write_text(json.dumps(phases),encoding="utf-8")
stack_file=open(sys.argv[1]+".startup-stack.txt","w",encoding="utf-8")
faulthandler.dump_traceback_later(10,file=stack_file)
phase("import_http")
from http.server import ThreadingHTTPServer
from socketserver import TCPServer
phase("import_reference")
from minidora.製品版.api import APIHandler
from minidora.製品版.製品チャット import 製品ミニドラ
phase("bind_http")
# literal loopback専用の開発serverで、表示名の逆引きを実行前提にしない。
server=ThreadingHTTPServer(("127.0.0.1",0),APIHandler,bind_and_activate=False)
try:
    TCPServer.server_bind(server)
    server.server_name="127.0.0.1"
    server.server_port=server.server_address[1]
    server.server_activate()
except Exception:
    server.server_close()
    raise
phase("initialize_product")
server.app=製品ミニドラ()
phase("ready")
faulthandler.cancel_dump_traceback_later()
stack_file.close()
Path(sys.argv[1]).write_text(json.dumps({"port":server.server_port}),encoding="utf-8")
server.serve_forever()
'''
    processes = []
    with tempfile.TemporaryDirectory(prefix="gui-shell-minidora-live-") as directory:
        root = Path(directory)
        with (root / "process.log").open("w", encoding="utf-8") as log:
            try:
                env = dict(os.environ, PYTHONPATH=str(reference / "src"), PYTHONUTF8="1", MINIDORA_HTTP_LOG="0", PYTHONDONTWRITEBYTECODE="1")
                addresses = []
                for index in range(2):
                    endpoint_file = root / f"runtime-{index}.json"
                    process = subprocess.Popen([sys.executable, "-c", server_code, str(endpoint_file)], env=env, cwd=root, stdout=log, stderr=log)
                    processes.append(process)
                    addresses.append(f"127.0.0.1:{file待機(endpoint_file, process)['port']}")
                normal_file = root / "normal.json"
                owner_file = root / "owner.json"
                vault = root / "protected"
                vault.mkdir()
                protected_args = ["--protected-store-dir", str(vault)] if os.name == "nt" else []
                broker = subprocess.Popen([str(binary), "broker-server", "--store-dir", str(root / "store"),
                    "--session-file", str(normal_file), "--owner-session-file", str(owner_file), *protected_args,
                    "--minidora-runtime", f"left={addresses[0]}", "--minidora-runtime", f"right={addresses[1]}", *(["--mobile-bind", "127.0.0.1:0"] if mobile_client or dart_mobile_client or virtual_device else [])], cwd=root, stdout=log, stderr=log)
                processes.append(broker)
                normal = file待機(normal_file, broker)
                owner = file待機(owner_file, broker)
                assert 操作(normal, "対話承認待ち", {})["status"] == "rejected"
                assert 成功(normal, "実行系列挙", {})["実行系"] == ["left", "right"]
                if mobile_client:
                    from tooling.device_link_live_check import 検証 as 端末検証
                    assert 端末検証(normal, owner, binary, root) == "PASS"
                if dart_client or dart_mobile_client:
                    dart = shutil.which("dart")
                    if dart is None:
                        raise RuntimeError("Dart実行環境がない")
                    # Windowsのbatを親にすると失敗時の子Dartがlogを保持する。
                    # Flutter同梱の実executableを直接使い、終了責任をこのprocessへ結合する。
                    if os.name == "nt" and Path(dart).suffix.lower() == ".bat":
                        executable = Path(dart).parent / "cache/dart-sdk/bin/dart.exe"
                        if not executable.is_file():
                            raise RuntimeError("Flutter同梱Dart executableがない")
                        dart = str(executable)
                    drivers = []
                    if dart_client:
                        drivers.append(("desktop_flutter", "dialogue_live_client.dart", normal_file, "dart"))
                    if dart_mobile_client:
                        invitation_file = root / "mobile-dart-invitation.json"
                        subprocess.run([str(binary), "対話承認操作", "--session-file", str(owner_file),
                            "端末招待", uuid.uuid4().hex, "127.0.0.1", str(invitation_file)], check=True, stdout=log, stderr=log, timeout=10)
                        drivers.append(("mobile_flutter", "device_link_live_client.dart", invitation_file, "mobile-dart"))
                    for app, script, credential_file, prefix in drivers:
                        driver = subprocess.Popen([dart, "run", str(ROOT / "apps" / app / "tool" / script),
                            str(credential_file), str(root)], cwd=ROOT / "apps" / app, stdout=log, stderr=log)
                        processes.append(driver)
                        ready = file待機(root / f"{prefix}-ready.json", driver)
                        pending = 成功(owner, "対話承認待ち", {})["要求"]
                        selected = [v for v in pending if v["要求"]["要求ID"] in ready["requests"]]
                        assert len(selected) == 2
                        for item in selected:
                            成功(owner, "対話承認", {"要求ID": item["要求"]["要求ID"], "要求hash": item["要求hash"], "表示範囲": "full"})
                        driver.wait(timeout=25)
                        assert driver.returncode == 0, "Dart製品clientの試験失敗"
                        assert json.loads((root / f"{prefix}-result.json").read_text())["result"] == "PASS"

                if virtual_device:
                    flutter = shutil.which("flutter")
                    if flutter is None:
                        raise RuntimeError("Flutter executableがない")
                    invitation_file = root / "simulator-invitation.json"
                    subprocess.run([str(binary), "対話承認操作", "--session-file", str(owner_file),
                        "端末招待", uuid.uuid4().hex, "10.0.2.2" if android_emulator else "127.0.0.1", str(invitation_file)],
                        check=True, stdout=log, stderr=log, timeout=10)
                    simulator_env = dict(os.environ, GUI_SHELL_TEST_INVITATION_FILE=str(invitation_file),
                        GUI_SHELL_TEST_ROOT=str(root), GUI_SHELL_TEST_SIMULATOR=virtual_device,
                        GUI_SHELL_TEST_PLATFORM="android" if android_emulator else "ios")
                    if adb:
                        simulator_env["GUI_SHELL_TEST_ADB"] = adb
                    driver = subprocess.Popen([flutter, "drive", "--no-pub", "--driver=test_driver/device_link_driver.dart",
                        "--target=integration_test/device_link_native_test.dart", "-d", virtual_device],
                        cwd=ROOT / "apps/mobile_flutter", env=simulator_env, stdout=log, stderr=log)
                    processes.append(driver)
                    ready = file待機(root / "simulator-ready.json", driver, timeout=300)
                    pending = 成功(owner, "対話承認待ち", {})["要求"]
                    selected = [v for v in pending if v["要求"]["要求ID"] in ready["requests"]]
                    assert len(selected) == 2
                    for item in selected:
                        成功(owner, "対話承認", {"要求ID": item["要求"]["要求ID"],
                            "要求hash": item["要求hash"], "表示範囲": "full"})
                    driver.wait(timeout=60)
                    assert driver.returncode == 0, "Simulator製品clientの試験失敗"
                    simulator_result = json.loads((root / "simulator-result.json").read_text(encoding="utf-8"))
                    assert simulator_result["result"] == "PASS"
                    assert simulator_result["physical_device_verified"] is False

                def 対話(runtime, scope, message="こんにちは"):
                    session = 成功(normal, "対話開始", {"実行系ID": runtime})["対話セッションID"]
                    pending = 成功(normal, "対話送信", {"対話セッションID": session, "入力": message})
                    assert 成功(normal, "対話取得", {"要求ID": pending["要求ID"]})["状態"] == "承認待ち"
                    approval = {"要求ID": pending["要求ID"], "要求hash": pending["要求hash"], "表示範囲": scope}
                    assert 操作(normal, "対話承認", approval)["status"] == "rejected"
                    # productionのowner CLIを実行。検証自身の一時資格だけを使う。
                    subprocess.run([str(binary), "対話承認操作", "--session-file", str(owner_file), "承認",
                                    pending["要求ID"], pending["要求hash"], scope], check=True, stdout=log, stderr=log, timeout=10)
                    return session, pending

                def 完了(pending):
                    end = time.monotonic() + 15
                    while time.monotonic() < end:
                        result = 成功(normal, "対話取得", {"要求ID": pending["要求ID"]})
                        if result["状態"] == "完了":
                            return result["結果"]
                        time.sleep(0.02)
                    raise RuntimeError("対話期限超過")

                left, p = 対話("left", "full")
                right, q = 対話("right", "hash_only")
                initial_pending = (p, q)
                a, b = 完了(p), 完了(q)
                initial_results = (a, b)
                assert left != right and p["要求ID"] != q["要求ID"]
                assert a["状態"] == b["状態"] == "成功", (a["失敗分類"], b["失敗分類"])
                expected_history = {p["要求ID"]: "成功", q["要求ID"]: "成功"}
                assert a["本文"] and a["追跡ID"] and a["追跡hash"] and a["能力"]
                assert a["実行系ID"] == "left" and b["実行系ID"] == "right"
                assert b["本文"] == b["追跡ID"] == "" and b["能力"] == [] and b["応答hash"]
                # 実APIの完了対象をowner CLIから明示保存する。実資格や運用署名鍵は使用しない。
                save_select = {"要求ID": p["要求ID"], "要求hash": p["要求hash"]}
                assert 操作(normal, "対話内容保存", save_select)["status"] == "rejected"
                if os.name == "nt":
                    assert list(vault.iterdir()) == []
                    assert 操作(owner, "対話内容保存", {"要求ID": q["要求ID"], "要求hash": q["要求hash"]})["status"] == "rejected"
                    assert 操作(owner, "対話内容保存", {**save_select, "要求hash": "sha256:" + "0" * 64})["status"] == "rejected"
                    saved = subprocess.check_output([str(binary), "対話承認操作", "--session-file", str(owner_file), "内容保存", p["要求ID"], p["要求hash"]], timeout=10)
                    receipt = json.loads(saved)
                    receipt_schema = json.loads((ROOT / "specs/runtime_content_receipt.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(receipt, receipt_schema)
                    cipher = (vault / f"history-{p['要求ID']}.dpapi").read_bytes()
                    assert receipt["要求ID"] == p["要求ID"] and receipt["要求hash"] == p["要求hash"]
                    assert receipt["暗号文hash"] == "sha256:" + hashlib.sha256(cipher).hexdigest()
                    assert a["本文"].encode() not in cipher
                    assert 操作(owner, "対話内容保存", save_select)["status"] == "rejected"
                    assert (vault / f"history-{p['要求ID']}.dpapi").read_bytes() == cipher
                    save_events = [json.loads(line) for line in (root / "store/audit.jsonl").read_text(encoding="utf-8").splitlines()]
                    committed = [e for e in save_events if e["reason"].startswith("対話内容保存記録:")]
                    assert len(committed) == 1
                    assert json.loads(committed[0]["reason"].split(":", 1)[1]) == receipt
                    assert any(e["event_id"] == receipt["保存承認監査ID"] and e["reason"].startswith("対話内容保存承認 ") for e in save_events)
                    assert any(e["event_id"] == receipt["終了監査ID"] and e["request_id"] == p["要求ID"] for e in save_events)
                    receipt_page = 成功(owner, "対話履歴一覧", {"after": 0, "limit": 1, "latest_per_request": True, "include_content_receipt": True, "filter": {"要求ID": p["要求ID"]}})
                    page_schema = json.loads((ROOT / "specs/runtime_execution_history_page.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(receipt_page, page_schema)
                    entry_receipt = receipt_page["entries"][0]["content_receipt"]
                    assert entry_receipt["receipt"] == receipt
                    assert entry_receipt["audit_event_id"] == committed[0]["event_id"]
                    assert entry_receipt["event_hash"] == committed[0]["event_hash"]
                    grant = 成功(owner, "対話履歴承認", {"実行系ID": "left"})["grant"]["approval_id"]
                    receipt_query = {"approval_id": grant, "query": {"after": 0, "limit": 1, "latest_per_request": True, "include_content_receipt": True, "filter": {"実行系ID": "left", "要求ID": p["要求ID"]}}}
                    viewed_receipt = 成功(normal, "対話履歴閲覧", receipt_query)
                    assert viewed_receipt["page"]["entries"][0]["content_receipt"] == entry_receipt
                    成功(owner, "対話履歴失効", {})
                    assert 操作(normal, "対話履歴閲覧", receipt_query)["body"] is None
                    content_target = {"要求ID": p["要求ID"], "保存監査ID": entry_receipt["audit_event_id"], "保存監査hash": entry_receipt["event_hash"]}
                    assert 操作(normal, "対話内容承認", content_target)["status"] == "rejected"
                    current_content = json.loads(subprocess.check_output([str(binary), "対話承認操作", "--session-file", str(owner_file), "内容閲覧承認", p["要求ID"], entry_receipt["audit_event_id"], entry_receipt["event_hash"]], timeout=10))
                    content_query = {"approval_id": current_content["grant"]["approval_id"]}
                    opened = 成功(normal, "対話内容閲覧", content_query)
                    access_schema = json.loads((ROOT / "specs/runtime_content_access.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(opened, access_schema)
                    assert opened["content"]["結果"] == a
                    assert opened["content"]["要求"]["入力"] == "こんにちは"
                    assert opened["content"]["要求hash"] == p["要求hash"]
                    assert opened["content"]["実行記録"]["終了監査ID"] == receipt["終了監査ID"]
                    assert current_content["content"] is None
                    if dart_client:
                        成功(owner, "対話履歴承認", {"実行系ID": "left"})
                        subprocess.run([dart, "run", "tool/content_live_client.dart", str(normal_file), p["要求ID"]], cwd=ROOT / "apps/desktop_flutter", check=True, stdout=log, stderr=log, timeout=30)
                        成功(owner, "対話履歴失効", {})

                    成功(owner, "対話内容失効", {})
                    assert 操作(normal, "対話内容削除", content_target)["body"] is None
                    deleted = json.loads(subprocess.check_output([str(binary), "対話承認操作", "--session-file", str(owner_file), "内容削除", p["要求ID"], entry_receipt["audit_event_id"], entry_receipt["event_hash"]], timeout=10))
                    deletion = deleted
                    deletion_schema = json.loads((ROOT / "specs/runtime_content_deletion.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(deletion, deletion_schema)
                    assert deletion["暗号文hash"] == receipt["暗号文hash"]
                    assert 操作(normal, "対話保管状態", save_select)["body"] is None
                    observed = json.loads(subprocess.check_output([str(binary), "対話承認操作", "--session-file", str(owner_file), "保管状態", p["要求ID"], p["要求hash"]], timeout=10))
                    state_schema = json.loads((ROOT / "specs/runtime_content_state.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(observed, state_schema)
                    assert observed["状態"] == "削除確定・file不在" and observed["file存在"] is False
                    assert observed["暗号文hash"] is None and observed["bytes"] is None
                    recovery_request = {"要求ID": p["要求ID"], "要求hash": p["要求hash"], "削除承認監査ID": deletion["削除承認監査ID"]}
                    assert 操作(normal, "対話削除中断確認", recovery_request)["body"] is None
                    completed_recovery = subprocess.run([str(binary), "対話承認操作", "--session-file", str(owner_file), "削除中断確認", p["要求ID"], p["要求hash"], deletion["削除承認監査ID"]], stdout=log, stderr=log, timeout=10)
                    assert completed_recovery.returncode != 0
                    assert 成功(owner, "対話保管状態", save_select)["状態"] == "削除確定・file不在"
                    assert not (vault / f"history-{p['要求ID']}.dpapi").exists()
                    assert 操作(owner, "対話内容承認", content_target)["body"] is None
                    assert 操作(owner, "対話内容削除", content_target)["body"] is None
                    assert 操作(normal, "対話内容閲覧", content_query)["body"] is None

                    _, partial_pending = 対話("left", "full")
                    assert 完了(partial_pending)["状態"] == "成功"
                    expected_history[partial_pending["要求ID"]] = "成功"
                    partial_select = {"要求ID": partial_pending["要求ID"], "要求hash": partial_pending["要求hash"]}
                    partial_path = vault / f"history-{partial_pending['要求ID']}.dpapi"
                    partial_path.write_bytes(b"synthetic partial ciphertext")
                    assert 操作(owner, "対話内容保存", partial_select)["body"] is None
                    partial_state = 成功(owner, "対話保管状態", partial_select)
                    assert partial_state["状態"] == "保存記録なし・fileあり" and partial_state["保存試行監査ID"]
                    discard = {**partial_select, "保存試行監査ID": partial_state["保存試行監査ID"], "暗号文hash": partial_state["暗号文hash"]}
                    assert 操作(normal, "対話部分保存破棄", discard)["body"] is None
                    discarded = json.loads(subprocess.check_output([str(binary), "対話承認操作", "--session-file", str(owner_file), "部分保存破棄", partial_pending["要求ID"], partial_pending["要求hash"], discard["保存試行監査ID"], discard["暗号文hash"]], timeout=10))
                    discard_schema = json.loads((ROOT / "specs/runtime_content_discard.schema.json").read_text(encoding="utf-8"))
                    assert not validate_instance(discarded, discard_schema)
                    assert not partial_path.exists()
                    after_discard = 成功(owner, "対話保管状態", partial_select)
                    assert not validate_instance(after_discard, state_schema)
                    assert after_discard["状態"] == "部分保存破棄済み・file不在"
                    assert 操作(owner, "対話内容保存", partial_select)["body"] is None


                else:
                    assert 操作(owner, "対話内容保存", save_select)["status"] == "rejected"
                parent_page = 成功(owner, "対話履歴一覧", {"after": 0, "limit": 1, "latest_per_request": True, "filter": {"要求ID": p["要求ID"]}})
                parent = parent_page["entries"][0]
                replay_grant = 成功(owner, "対話履歴承認", {"実行系ID": "left"})["grant"]["approval_id"]
                replay_schema = json.loads((ROOT / "specs/runtime_replay_response.schema.json").read_text(encoding="utf-8"))
                for operation, replay_input in [("対話再実行", "こんにちは"), ("対話分岐", "ありがとう")]:
                    child = 成功(normal, operation, {"approval_id": replay_grant, "実行系ID": "left", "参照監査ID": parent["audit_event_id"], "参照event_hash": parent["event_hash"], "入力": replay_input})
                    assert not validate_instance(child, replay_schema)
                    assert child["要求ID"] != p["要求ID"] and child["対話セッションID"] != left
                    waiting = 成功(normal, "対話取得", {"要求ID": child["要求ID"]})
                    assert waiting["状態"] == "承認待ち" and waiting["実行記録"]["開始時刻"] is None
                    assert 操作(owner, "対話承認", {"要求ID": child["要求ID"], "要求hash": p["要求hash"], "表示範囲": "full"})["status"] != "accepted"
                    成功(owner, "対話承認", {"要求ID": child["要求ID"], "要求hash": child["要求hash"], "表示範囲": "full"})
                    child_result = 完了(child)
                    assert child_result["状態"] == "成功"
                    expected_history[child["要求ID"]] = "成功"
                成功(owner, "対話履歴失効", {})
                # 基礎Core未接続に該当する要求を成功と誤射影しない。
                _, hold = 対話("left", "full", "存在論的な未知問題を解いて")
                assert 完了(hold)["状態"] == "保留"
                expected_history[hold["要求ID"]] = "保留"
                # 参照実process停止後も、他側の成功・表示資格・sessionを共有しない。
                終了(processes[1])
                _, p = 対話("left", "full")
                _, q = 対話("right", "full")
                assert 完了(p)["状態"] == "成功"
                assert 完了(q)["状態"] == "失敗"
                expected_history.update({p["要求ID"]: "成功", q["要求ID"]: "失敗"})
                終了(processes[0])
                _, p = 対話("left", "full")
                _, q = 対話("right", "full")
                assert 完了(p)["状態"] == 完了(q)["状態"] == "失敗"
                expected_history.update({p["要求ID"]: "失敗", q["要求ID"]: "失敗"})
                history_schema = json.loads((ROOT / "specs/runtime_execution_history_page.schema.json").read_text(encoding="utf-8"))
                cursor, observed_history = 0, {}
                for _ in range(64):
                    page = 成功(owner, "対話履歴一覧", {"after": cursor, "limit": 100})
                    assert not validate_instance(page, history_schema)
                    for entry in page["entries"]:
                        record = entry["record"]
                        observed_history[record["実行記録"]["要求ID"]] = record["状態"]
                    if not page["has_more"]:
                        break
                    assert page["next_cursor"] > cursor
                    cursor = page["next_cursor"]
                else:
                    raise RuntimeError("実履歴ページの取得上限")
                assert all(observed_history.get(k) == v for k, v in expected_history.items())
                for request_id, state in expected_history.items():
                    selected = 成功(owner, "対話履歴一覧", {"after": 0, "limit": 1, "filter": {"要求ID": request_id, "状態": state}})
                    assert not validate_instance(selected, history_schema)
                    assert len(selected["entries"]) == 1 and not selected["has_more"]
                    assert selected["entries"][0]["record"]["実行記録"]["要求ID"] == request_id
                    assert selected["entries"][0]["record"]["状態"] == state
                grouped = 成功(owner, "対話履歴一覧", {"after": 0, "limit": 100, "latest_per_request": True})
                grouped_records = grouped["entries"]
                grouped_ids = [e["record"]["実行記録"]["要求ID"] for e in grouped_records]
                assert len(grouped_ids) == len(set(grouped_ids)) and not grouped["has_more"]
                grouped_states = {e["record"]["実行記録"]["要求ID"]: e["record"]["状態"] for e in grouped_records}
                assert all(grouped_states.get(k) == v for k, v in expected_history.items())
                audit_events = [json.loads(line) for line in (root / "store/audit.jsonl").read_text(encoding="utf-8").splitlines()]
                proof_schema = json.loads((ROOT / "specs/runtime_result_evidence.schema.json").read_text(encoding="utf-8"))
                for pending, result in zip(initial_pending, initial_results):
                    found = [e for e in audit_events if e["request_id"] == pending["要求ID"] and e["reason"].startswith("対話結果証跡:")]
                    assert len(found) == 1
                    event = found[0]; encoded = event["reason"].split(":", 1)[1]; proof = json.loads(encoded)
                    assert not validate_instance(proof, proof_schema)
                    assert event["payload_hash"] == "sha256:" + hashlib.sha256(encoded.encode()).hexdigest()
                    assert proof["要求hash"] == pending["要求hash"] and proof["応答hash"] == result["応答hash"]
                    assert any(e["event_id"] == proof["終了監査ID"] and e["request_id"] == pending["要求ID"] and e["reason"] == "対話完了" for e in audit_events[:audit_events.index(event)])
                    if proof["表示範囲"] == "full":
                        projections = {"能力申告hash": result["能力"], "経路申告hash": result["経路"],
                                       "追跡参照hash": {"追跡ID": result["追跡ID"], "追跡hash": result["追跡hash"]}}
                        for field, projection in projections.items():
                            canonical = json.dumps(projection, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
                            assert proof[field] == "sha256:" + hashlib.sha256(canonical.encode()).hexdigest()
                    else:
                        assert proof["能力申告hash"] is proof["経路申告hash"] is proof["追跡参照hash"] is None
                contextual = 成功(owner, "対話履歴一覧", {"after": 0, "limit": 100, "latest_per_request": True, "include_audit_context": True, "include_result_evidence": True})
                context_schema = json.loads((ROOT / "specs/runtime_execution_history_page.schema.json").read_text(encoding="utf-8"))
                assert not validate_instance(contextual, context_schema)
                proof_entries = {e["record"]["実行記録"]["要求ID"]: e["result_evidence"] for e in contextual["entries"]}
                for pending, result in zip(initial_pending, initial_results):
                    assert proof_entries[pending["要求ID"]]["応答hash"] == result["応答hash"]
                contexts = {entry["record"]["実行記録"]["要求ID"]: entry["audit_context"] for entry in contextual["entries"]}
                for pending in initial_pending:
                    context = contexts[pending["要求ID"]]
                    assert context["要求hash"] == pending["要求hash"] == context["承認"]["要求hash"]
                    assert context["承認能力"] == ["対話送信"] and context["復旧対応"] == "接続再確認"
                    assert context["現在権限"] is False
                assert contexts[initial_pending[0]["要求ID"]]["承認"]["内容表示範囲"] == "full"
                assert contexts[initial_pending[1]["要求ID"]]["承認"]["内容表示範囲"] == "hash_only"
                access_schema = json.loads((ROOT / "specs/runtime_history_access.schema.json").read_text(encoding="utf-8"))
                for runtime_id in ["left", "right"]:
                    approved = 成功(owner, "対話履歴承認", {"実行系ID": runtime_id})
                    selection = {"approval_id": approved["grant"]["approval_id"], "query": {"after": 0, "limit": 100, "filter": {"実行系ID": runtime_id}, "include_result_evidence": True}}
                    if dart_client:
                        subprocess.run([dart, "run", str(ROOT / "apps/desktop_flutter/tool/history_live_client.dart"), str(normal_file), runtime_id], cwd=ROOT / "apps/desktop_flutter", check=True, stdout=log, stderr=log, timeout=30)
                    viewed = 成功(normal, "対話履歴閲覧", selection)
                    assert not validate_instance(viewed, access_schema)
                    assert viewed["page"]["entries"] and not viewed["page"]["has_more"]
                    assert all(e["record"]["実行記録"]["実行系ID"] == runtime_id for e in viewed["page"]["entries"])
                    for pending in initial_pending:
                        if proof_entries[pending["要求ID"]]["実行系ID"] == runtime_id:
                            actual = [e["result_evidence"] for e in viewed["page"]["entries"] if e["record"]["実行記録"]["要求ID"] == pending["要求ID"] and e["record"]["状態"] == "成功"]
                            assert actual == [proof_entries[pending["要求ID"]]]
                    成功(owner, "対話履歴失効", {})
                    denied = 操作(normal, "対話履歴閲覧", selection)
                    assert denied["status"] != "accepted" and denied.get("body") is None
                assert 操作(normal, "shutdown", None)["status"] == "accepted"
                broker.wait(timeout=5)
                # 再起動は永続監査chainとnonceを読み、整合しなければ起動しない。
                normal_file.unlink()
                restart = subprocess.Popen([str(binary), "broker-server", "--store-dir", str(root / "store"), "--session-file", str(normal_file)], stdout=log, stderr=log)
                processes.append(restart)
                endpoint = file待機(normal_file, restart)
                assert 操作(endpoint, "health", None)["health"]["persistence_ready"] is True
                assert 操作(endpoint, "shutdown", None)["status"] == "accepted"
                restart.wait(timeout=5)
                return {"result": "PASS", "evidence_source": "LIVE_RUNTIME", "reference_commit": head,
                        "runtime_startup": [json.loads((root / f"runtime-{i}.json.startup.json").read_text(encoding="utf-8")) for i in range(2)],
                        "tested": ["実API二実行系", "owner CLI承認", "通常資格拒否", "表示分離", "trace照合", "保留", "片側失敗", "両失敗", "実履歴の状態整合", "実履歴の条件検索", "結果証跡の要求・応答・完了監査照合", "新承認による実再実行と分岐", "現在承認による通常IPC履歴閲覧と失効", "監査chain再読取"],
                        "dart_product_client": "PASS" if dart_client else "未実行",
                        "mobile_tls_path": "PASS" if mobile_client else "未実行",
                        "mobile_dart_product_client": "PASS" if dart_mobile_client else "未実行",
                        "mobile_simulator": simulator_result if mobile_simulator else "未実行",
                        "android_emulator": simulator_result if android_emulator else "未実行",
                        "protected_content_save": "PASS" if os.name == "nt" else "未対応拒否を確認",
                        "protected_content_delete": "PASS" if os.name == "nt" else "未実行",
                        "scope": "MINIDORA製品チャットの基本会話と保留。基礎Core・外部検索の能力保証ではない。"}
            except Exception:
                log.flush()
                # 起動前の段階名・経過秒とstackだけを出力する。資格・要求本文・localsを含めない。
                for index in range(2):
                    phase_path = root / f"runtime-{index}.json.startup.json"
                    stack_path = root / f"runtime-{index}.json.startup-stack.txt"
                    if phase_path.is_file():
                        try:
                            print(json.dumps({"runtime_startup": index,
                                "phases": json.loads(phase_path.read_text(encoding="utf-8"))}, ensure_ascii=False))
                        except (OSError, json.JSONDecodeError):
                            print(f"Runtime {index} 起動段階記録は読取未成立")
                    if stack_path.is_file():
                        print(stack_path.read_text(encoding="utf-8")[:8192])
                # ログには秘密資格を記録しない。エラー時も本文一括転送は行わない。
                if virtual_device:
                    diagnostic = root / "simulator-state.json"
                    if diagnostic.is_file():
                        state = json.loads(diagnostic.read_text(encoding="utf-8"))
                        print(json.dumps({"simulator_last_state": state}, ensure_ascii=False))
                raise
            finally:
                for process in reversed(processes):
                    終了(process)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--dart-client", action="store_true")
    parser.add_argument("--mobile-client", action="store_true")
    parser.add_argument("--dart-mobile-client", action="store_true")
    parser.add_argument("--mobile-simulator", help="手動起動済みiOS SimulatorのUDID。実機には使用しない")
    parser.add_argument("--android-emulator", choices=["emulator-5554"], help="専用Linux runnerで起動済みの専用AVDだけを使用")
    parser.add_argument("--binary", type=Path, default=ROOT / "native/rust_helper/target/debug" / ("gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper"))
    args = parser.parse_args()
    print(json.dumps(検証(args.reference.resolve(), args.binary.resolve(), args.dart_client, args.mobile_client, args.dart_mobile_client, args.mobile_simulator, args.android_emulator), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
