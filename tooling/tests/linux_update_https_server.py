#!/usr/bin/env python3
"""Run-scoped HTTPS source for the installed Linux update acceptance path."""

from __future__ import annotations

import argparse
import http.server
import os
import socketserver
import ssl
import stat
import sys
from pathlib import Path

TEST_HOST = "updates.d4p-test.example"
ALLOWED_FILES = {
    "/d4/stable/updates.json": ("updates.json", "application/json"),
    "/d4/stable/linux-installed-catalog-display.pkg": (
        "linux-installed-catalog-display.pkg",
        "application/octet-stream",
    ),
}


class AcceptanceUpdateHandler(http.server.BaseHTTPRequestHandler):
    server_version = "D4PocketAcceptanceUpdate/1"
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:
        if self.path == "/d4/stable/updates.json":
            self.server.record_event("HTTP_GET_CATALOG")  # type: ignore[attr-defined]
        elif self.path == "/d4/stable/linux-installed-catalog-display.pkg":
            self.server.record_event("HTTP_GET_PACKAGE")  # type: ignore[attr-defined]
        else:
            self.server.record_event("HTTP_GET_OTHER")  # type: ignore[attr-defined]
        if self.headers.get("Host") != TEST_HOST or self.path not in ALLOWED_FILES:
            self._reply(404, b"not found\n", "text/plain")
            return
        filename, content_type = ALLOWED_FILES[self.path]
        path = self.server.content_root / filename  # type: ignore[attr-defined]
        try:
            metadata = path.lstat()
            if self.server.content_root.is_symlink():  # type: ignore[attr-defined]
                self._reply(404, b"not found\n", "text/plain")
                return
            if (
                not stat.S_ISREG(metadata.st_mode)
                or stat.S_ISLNK(metadata.st_mode)
                or metadata.st_nlink != 1
                or metadata.st_mode & 0o022
            ):
                self._reply(404, b"not found\n", "text/plain")
                return
            descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
            source = os.fdopen(descriptor, "rb")
        except OSError:
            self._reply(404, b"not found\n", "text/plain")
            return
        with source:
            opened = os.fstat(source.fileno())
            if (
                not stat.S_ISREG(opened.st_mode)
                or opened.st_ino != metadata.st_ino
                or opened.st_dev != metadata.st_dev
                or opened.st_size != metadata.st_size
            ):
                self._reply(404, b"not found\n", "text/plain")
                return
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(opened.st_size))
            self.send_header("Cache-Control", "no-store")
            self.send_header("Connection", "close")
            self.end_headers()
            while chunk := source.read(1024 * 1024):
                self.wfile.write(chunk)
            if filename == "updates.json":
                self.server.record_event("HTTP_RESPONSE_COMPLETE_CATALOG")  # type: ignore[attr-defined]
            else:
                self.server.record_event("HTTP_RESPONSE_COMPLETE_PACKAGE")  # type: ignore[attr-defined]
        self.close_connection = True

    def do_HEAD(self) -> None:
        self._reply(405, b"method not allowed\n", "text/plain")

    def _reply(self, status: int, body: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)
        self.close_connection = True

    def log_message(self, _format: str, *_args: object) -> None:
        return


class AcceptanceUpdateServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = False

    def record_event(self, event: str) -> None:
        if event in {
            "TLS_ACCEPT",
            "TLS_ACCEPT_FAILURE",
            "HTTP_GET_CATALOG",
            "HTTP_GET_PACKAGE",
            "HTTP_GET_OTHER",
            "HTTP_RESPONSE_COMPLETE_CATALOG",
            "HTTP_RESPONSE_COMPLETE_PACKAGE",
        }:
            print(f"UPDATE_SERVER_EVENT {event}", flush=True)

    def get_request(self):
        try:
            request, client_address = super().get_request()
        except OSError as error:
            reason = getattr(error, "reason", None)
            if (
                not isinstance(reason, str)
                or not reason.isascii()
                or not reason.replace("_", "").isalnum()
            ):
                reason = "other"
            self.record_event("TLS_ACCEPT_FAILURE")
            print(f"UPDATE_SERVER_TLS_REASON {reason[:80]}", flush=True)
            raise
        self.record_event("TLS_ACCEPT")
        return request, client_address

    def handle_error(self, _request, _client_address) -> None:
        error = sys.exc_info()[1]
        name = type(error).__name__ if error is not None else "unknown"
        if not name.isascii() or not name.isalnum():
            name = "other"
        print(f"UPDATE_SERVER_HANDLER_FAILURE {name}", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--certificate", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--pid-file", type=Path, required=True)
    parser.add_argument("--drop-uid", type=int, required=True)
    parser.add_argument("--drop-gid", type=int, required=True)
    parser.add_argument("--port", type=int, default=443)
    args = parser.parse_args()

    try:
        if args.port != 443:
            raise ValueError("acceptance HTTPSは標準TLS portだけを使用します")
        if args.root.is_symlink() or not args.root.is_dir():
            raise ValueError("配布rootが通常directoryではありません")
        root = args.root.resolve(strict=True)
        root_metadata = root.lstat()
        if args.pid_file.is_symlink() or args.certificate.is_symlink() or args.private_key.is_symlink():
            raise ValueError("TLS資材とPID記録にsymlinkを指定できません")
        pid_path = args.pid_file.absolute()
        certificate = args.certificate.resolve(strict=True)
        private_key = args.private_key.resolve(strict=True)
        if (
            not stat.S_ISDIR(root_metadata.st_mode)
            or stat.S_ISLNK(root_metadata.st_mode)
            or root_metadata.st_uid != args.drop_uid
            or root_metadata.st_mode & 0o022
            or pid_path.parent.resolve(strict=True) != root
            or root not in certificate.parents
            or root not in private_key.parents
        ):
            raise ValueError("TLS資材とPID記録は配布root直下の非symlink pathに限ります")
        private_key_metadata = private_key.lstat()
        certificate_metadata = certificate.lstat()
        if (
            not stat.S_ISREG(private_key_metadata.st_mode)
            or stat.S_ISLNK(private_key_metadata.st_mode)
            or private_key_metadata.st_uid != args.drop_uid
            or private_key_metadata.st_mode & 0o077
            or not stat.S_ISREG(certificate_metadata.st_mode)
            or stat.S_ISLNK(certificate_metadata.st_mode)
            or certificate_metadata.st_uid != args.drop_uid
        ):
            raise ValueError("TLS秘密鍵は権限を限定した通常fileに限ります")
        for route, (filename, _content_type) in ALLOWED_FILES.items():
            if route.endswith("updates.json") or route.endswith(".pkg"):
                path = root / filename
                metadata = path.lstat()
                if (
                    not stat.S_ISREG(metadata.st_mode)
                    or stat.S_ISLNK(metadata.st_mode)
                    or metadata.st_uid != args.drop_uid
                    or metadata.st_nlink != 1
                    or metadata.st_mode & 0o022
                ):
                    raise ValueError("配布対象は変更可能でない単一linkの通常fileに限ります")
        pid_descriptor = os.open(
            pid_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644
        )
        os.fchmod(pid_descriptor, 0o644)
        with os.fdopen(pid_descriptor, "w", encoding="ascii") as pid_record:
            pid_record.write(f"{os.getpid()}\n")
            pid_record.flush()
            os.fsync(pid_record.fileno())

        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.load_cert_chain(certificate, private_key)
        server = AcceptanceUpdateServer(("127.0.0.1", args.port), AcceptanceUpdateHandler)
        server.content_root = root  # type: ignore[attr-defined]
        server.socket = context.wrap_socket(server.socket, server_side=True)
        if os.geteuid() == 0:
            os.setgroups([])
            os.setgid(args.drop_gid)
            os.setuid(args.drop_uid)
        if os.geteuid() != args.drop_uid or os.getegid() != args.drop_gid:
            raise ValueError("HTTPS acceptance serverをrun所有者へ降格できません")
        print(f"READY {args.port}", flush=True)
        server.serve_forever(poll_interval=0.2)
    except Exception as error:
        print(f"Linux update HTTPS acceptance serverを開始できません: {type(error).__name__}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
