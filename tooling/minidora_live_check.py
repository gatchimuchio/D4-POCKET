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
REFERENCE = "3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8"


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


def file待機(path, process):
    end = time.monotonic() + 20
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


def 検証(reference, binary, dart_client=False):
    head = subprocess.check_output(["git", "-C", str(reference), "rev-parse", "HEAD"], text=True).strip()
    if head != REFERENCE:
        raise RuntimeError("MINIDORA参照commitが固定点と異なる")
    if subprocess.check_output(["git", "-C", str(reference), "status", "--porcelain"], text=True).strip():
        raise RuntimeError("MINIDORA参照に既存差分がある")
    # 参照製品の既存handlerと製品チャットをそのまま使う。基礎Core未接続を隠さない。
    server_code = '''import json,sys
from pathlib import Path
from http.server import ThreadingHTTPServer
from minidora.製品版.api import APIHandler
from minidora.製品版.製品チャット import 製品ミニドラ
server=ThreadingHTTPServer(("127.0.0.1",0),APIHandler)
server.app=製品ミニドラ()
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
                broker = subprocess.Popen([str(binary), "broker-server", "--store-dir", str(root / "store"),
                    "--session-file", str(normal_file), "--owner-session-file", str(owner_file),
                    "--minidora-runtime", f"left={addresses[0]}", "--minidora-runtime", f"right={addresses[1]}"], cwd=root, stdout=log, stderr=log)
                processes.append(broker)
                normal = file待機(normal_file, broker)
                owner = file待機(owner_file, broker)
                assert 操作(normal, "対話承認待ち", {})["status"] == "rejected"
                assert 成功(normal, "実行系列挙", {})["実行系"] == ["left", "right"]
                if dart_client:
                    dart = shutil.which("dart")
                    if dart is None:
                        raise RuntimeError("Dart実行環境がない")
                    driver = subprocess.Popen([dart, "run", str(ROOT / "apps/desktop_flutter/tool/dialogue_live_client.dart"),
                        str(normal_file), str(root)], cwd=ROOT / "apps/desktop_flutter", stdout=log, stderr=log)
                    processes.append(driver)
                    ready = file待機(root / "dart-ready.json", driver)
                    pending = 成功(owner, "対話承認待ち", {})["要求"]
                    selected = [v for v in pending if v["要求"]["要求ID"] in ready["requests"]]
                    assert len(selected) == 2
                    for item in selected:
                        成功(owner, "対話承認", {"要求ID": item["要求"]["要求ID"], "要求hash": item["要求hash"], "表示範囲": "full"})
                    driver.wait(timeout=25)
                    assert driver.returncode == 0, "Dart製品clientの試験失敗"
                    assert json.loads((root / "dart-result.json").read_text())["result"] == "PASS"

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
                a, b = 完了(p), 完了(q)
                assert left != right and p["要求ID"] != q["要求ID"]
                assert a["状態"] == b["状態"] == "成功", (a["失敗分類"], b["失敗分類"])
                assert a["本文"] and a["追跡ID"] and a["追跡hash"] and a["能力"]
                assert a["実行系ID"] == "left" and b["実行系ID"] == "right"
                assert b["本文"] == b["追跡ID"] == "" and b["能力"] == [] and b["応答hash"]
                # 基礎Core未接続に該当する要求を成功と誤射影しない。
                _, hold = 対話("left", "full", "存在論的な未知問題を解いて")
                assert 完了(hold)["状態"] == "保留"
                # 参照実process停止後も、他側の成功・表示資格・sessionを共有しない。
                終了(processes[1])
                _, p = 対話("left", "full")
                _, q = 対話("right", "full")
                assert 完了(p)["状態"] == "成功"
                assert 完了(q)["状態"] == "失敗"
                終了(processes[0])
                _, p = 対話("left", "full")
                _, q = 対話("right", "full")
                assert 完了(p)["状態"] == 完了(q)["状態"] == "失敗"
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
                        "tested": ["実API二実行系", "owner CLI承認", "通常資格拒否", "表示分離", "trace照合", "保留", "片側失敗", "両失敗", "監査chain再読取"],
                        "dart_product_client": "PASS" if dart_client else "未実行",
                        "scope": "MINIDORA製品チャットの基本会話と保留。基礎Core・外部検索の能力保証ではない。"}
            except Exception:
                log.flush()
                # ログには秘密資格を記録しない。エラー時も本文一括転送は行わない。
                raise
            finally:
                for process in reversed(processes):
                    終了(process)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--dart-client", action="store_true")
    parser.add_argument("--binary", type=Path, default=ROOT / "native/rust_helper/target/debug" / ("gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper"))
    args = parser.parse_args()
    print(json.dumps(検証(args.reference.resolve(), args.binary.resolve(), args.dart_client), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
