"""開発専用の実TLS・端末資格・実MINIDORA経路検証。試験自身の一時資格だけを使う。"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import socket
import ssl
import subprocess
import time
import uuid

from tooling.minidora_live_check import 操作, 成功, 正本化


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
