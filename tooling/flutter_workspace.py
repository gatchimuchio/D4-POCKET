from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import subprocess
import tarfile
import tempfile
from pathlib import Path, PurePosixPath


class FlutterWorkspaceError(RuntimeError):
    pass


def _inside(path: Path, root: Path) -> bool:
    return path == root or path.is_relative_to(root)


def _known_onedrive_roots() -> list[Path]:
    roots: list[Path] = []
    for variable in ("OneDrive", "OneDriveCommercial", "OneDriveConsumer"):
        value = os.environ.get(variable)
        if value:
            roots.append(Path(value).expanduser().resolve(strict=False))
    return roots


def safe_flutter_temp_parent(source_root: Path, candidate: Path | None = None) -> Path:
    source = source_root.expanduser().resolve(strict=True)
    parent = (candidate or Path(tempfile.gettempdir())).expanduser().resolve(strict=True)
    if not parent.is_dir():
        raise FlutterWorkspaceError("一時配置先がdirectoryではない")
    if _inside(parent, source):
        raise FlutterWorkspaceError("一時配置先がsource repository内にある")

    cloud_roots = _known_onedrive_roots()
    parts = parent.parts
    for index, part in enumerate(parts):
        if part.casefold().startswith("onedrive"):
            cloud_roots.append(Path(*parts[: index + 1]))
    if any(_inside(parent, cloud_root) for cloud_root in cloud_roots):
        raise FlutterWorkspaceError("一時配置先がOneDrive配下にある")

    if os.name == "nt":
        attributes = getattr(os.stat(parent), "st_file_attributes", 0)
        reparse_point = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        if attributes & reparse_point:
            raise FlutterWorkspaceError("一時配置先自体がreparse pointである")
    return parent


def _git(root: Path, *arguments: str, input_bytes: bytes | None = None) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", errors="replace").strip()
        raise FlutterWorkspaceError(
            f"Git操作が終了値{result.returncode}で失敗しました: {detail}"
        )
    return result.stdout


def _relative_source_path(raw_path: bytes) -> tuple[str, Path]:
    name = os.fsdecode(raw_path)
    relative = PurePosixPath(name)
    if relative.is_absolute() or not relative.parts or ".." in relative.parts or ".git" in relative.parts:
        raise FlutterWorkspaceError("source file listに許可されないpathがある")
    if "\\" in name:
        raise FlutterWorkspaceError("Windows source pathに不正なseparatorがある")
    return name, Path(*relative.parts)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _untracked_files(root: Path) -> list[dict[str, object]]:
    raw = _git(root, "ls-files", "--others", "--exclude-standard", "-z")
    records: list[dict[str, object]] = []
    for raw_path in sorted(filter(None, raw.split(b"\0"))):
        name, relative = _relative_source_path(raw_path)
        source = root / relative
        try:
            metadata = source.lstat()
        except OSError as exc:
            raise FlutterWorkspaceError(f"untracked sourceが読めない: {name}: {exc}") from exc
        if not stat.S_ISREG(metadata.st_mode):
            raise FlutterWorkspaceError(f"untracked sourceが通常fileではない: {name}")
        records.append(
            {
                "path": name,
                "mode": stat.S_IMODE(metadata.st_mode),
                "sha256": _sha256_file(source),
            }
        )
    return records


def _source_state(root: Path) -> tuple[str, str, bytes, list[dict[str, object]]]:
    repository = Path(
        os.fsdecode(_git(root, "rev-parse", "--show-toplevel")).strip()
    ).resolve(strict=True)
    if repository != root.resolve(strict=True):
        raise FlutterWorkspaceError("source rootがGit repository rootではない")
    commit = os.fsdecode(_git(root, "rev-parse", "HEAD")).strip()
    patch = _git(root, "diff", "--binary", "--no-ext-diff", "HEAD", "--")
    untracked = _untracked_files(root)
    identity = json.dumps(
        {
            "commit": commit,
            "tracked_worktree_diff_sha256": hashlib.sha256(patch).hexdigest(),
            "untracked": untracked,
        },
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return commit, hashlib.sha256(identity).hexdigest(), patch, untracked


def _extract_source_archive(archive_path: Path, destination: Path) -> None:
    with tarfile.open(archive_path, mode="r:") as archive:
        members = archive.getmembers()
        for member in members:
            relative = PurePosixPath(member.name)
            if relative.is_absolute() or ".." in relative.parts:
                raise FlutterWorkspaceError("Git archiveにroot外pathがある")
            if not (member.isdir() or member.isfile()):
                raise FlutterWorkspaceError("Git archiveにsymlinkまたは特殊fileがある")
        archive.extractall(destination, members=members, filter="data")


def _copy_untracked_files(
    source_root: Path,
    destination_root: Path,
    records: list[dict[str, object]],
) -> None:
    for record in records:
        name = str(record["path"])
        _, relative = _relative_source_path(os.fsencode(name))
        source = source_root / relative
        destination = destination_root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists() or destination.is_symlink():
            raise FlutterWorkspaceError(f"snapshotとuntracked sourceが衝突する: {name}")
        shutil.copy2(source, destination, follow_symlinks=False)
        if _sha256_file(destination) != record["sha256"]:
            raise FlutterWorkspaceError(f"untracked sourceのcopy hashが不一致: {name}")


def _pub_cache_path(source_root: Path, temporary_root: Path) -> tuple[Path, str]:
    configured = os.environ.get("PUB_CACHE")
    if configured:
        cache = Path(configured).expanduser().resolve(strict=False)
    elif os.name == "nt":
        local_app_data = os.environ.get("LOCALAPPDATA")
        base = Path(local_app_data) if local_app_data else Path.home() / "AppData" / "Local"
        cache = (base / "Pub" / "Cache").resolve(strict=False)
    else:
        cache = (Path.home() / ".pub-cache").resolve(strict=False)

    cloud_roots = _known_onedrive_roots()
    if _inside(cache, source_root) or any(_inside(cache, root) for root in cloud_roots):
        isolated = temporary_root / "pub-cache"
        isolated.mkdir(parents=True, exist_ok=True)
        return isolated, "isolated_temporary"
    return cache, "shared_outside_source_and_OneDrive"


def _cargo_home_path(source_root: Path, temporary_root: Path) -> tuple[Path, str]:
    configured = os.environ.get("CARGO_HOME")
    cargo_home = (
        Path(configured).expanduser().resolve(strict=False)
        if configured
        else (Path.home() / ".cargo").resolve(strict=False)
    )
    cloud_roots = _known_onedrive_roots()
    if _inside(cargo_home, source_root) or any(_inside(cargo_home, root) for root in cloud_roots):
        isolated = temporary_root / "cargo-home"
        isolated.mkdir(parents=True, exist_ok=True)
        return isolated, "isolated_temporary"
    return cargo_home, "shared_outside_source_and_OneDrive"


class IsolatedFlutterWorkspace:
    """Flutter検証と生成物を隔離する一時Git source snapshot。"""

    def __init__(self, source_root: Path, temp_parent: Path | None = None):
        self.source_root = source_root.expanduser().resolve(strict=True)
        self.temp_parent = safe_flutter_temp_parent(self.source_root, temp_parent)
        self.root: Path | None = None
        self.temporary_root: Path | None = None
        self.source_commit: str | None = None
        self.source_sha256: str | None = None
        self.pub_cache_mode: str | None = None
        self.cargo_home_mode: str | None = None
        self.environment: dict[str, str] = {}
        self._temporary: tempfile.TemporaryDirectory[str] | None = None
        self._expected_identity: str | None = None

    def __enter__(self) -> IsolatedFlutterWorkspace:
        self._temporary = tempfile.TemporaryDirectory(
            prefix="d4-pocket-flutter-validation-", dir=self.temp_parent
        )
        self.temporary_root = Path(self._temporary.name).resolve(strict=True)
        self.root = self.temporary_root / "source"
        self.root.mkdir()
        try:
            commit, identity, patch, untracked = _source_state(self.source_root)
            archive_path = self.temporary_root / "source.tar"
            _git(
                self.source_root,
                "archive",
                "--format=tar",
                f"--output={archive_path}",
                commit,
            )
            _extract_source_archive(archive_path, self.root)
            archive_path.unlink()
            if patch:
                _git(
                    self.root,
                    "apply",
                    "--binary",
                    "--whitespace=nowarn",
                    "-",
                    input_bytes=patch,
                )
            _copy_untracked_files(self.source_root, self.root, untracked)
            commit_after, identity_after, _, _ = _source_state(self.source_root)
            if (commit_after, identity_after) != (commit, identity):
                raise FlutterWorkspaceError("snapshot作成中にsource worktreeが変化した")

            self.source_commit = commit
            self.source_sha256 = identity
            self._expected_identity = identity
            self._configure_environment()
            return self
        except Exception:
            self._temporary.cleanup()
            self._temporary = None
            self.root = None
            self.temporary_root = None
            raise

    def _configure_environment(self) -> None:
        assert self.temporary_root is not None
        assert self.source_root is not None
        environment = os.environ.copy()
        temporary_io = self.temporary_root / "io-temp"
        temporary_io.mkdir()
        app_data = self.temporary_root / "flutter-config"
        app_data.mkdir()

        if os.name == "nt":
            environment["APPDATA"] = str(app_data)
            environment["TEMP"] = str(temporary_io)
            environment["TMP"] = str(temporary_io)
        else:
            environment["XDG_CONFIG_HOME"] = str(app_data)
            environment["TMPDIR"] = str(temporary_io)

        cache, cache_mode = _pub_cache_path(self.source_root, self.temporary_root)
        cargo_home, cargo_home_mode = _cargo_home_path(self.source_root, self.temporary_root)
        environment["PUB_CACHE"] = str(cache)
        environment["CARGO_HOME"] = str(cargo_home)
        environment["CARGO_TARGET_DIR"] = str(
            self.root / "native" / "rust_helper" / "target"
        )
        environment["FLUTTER_SUPPRESS_ANALYTICS"] = "true"
        self.environment = environment
        self.pub_cache_mode = cache_mode
        self.cargo_home_mode = cargo_home_mode

    def __exit__(self, exc_type, exc, traceback) -> bool:
        source_error: Exception | None = None
        if exc_type is None and self._expected_identity is not None:
            try:
                _, identity, _, _ = _source_state(self.source_root)
                if identity != self._expected_identity:
                    source_error = FlutterWorkspaceError(
                        "Flutter validation中に元source worktreeが変化した"
                    )
            except Exception as state_error:
                source_error = state_error

        cleanup_error: Exception | None = None
        if self._temporary is not None:
            try:
                self._temporary.cleanup()
                if self.temporary_root is not None and self.temporary_root.exists():
                    raise OSError("Flutter temporary workspaceがcleanup後も残っている")
            except Exception as error:
                cleanup_error = error
            finally:
                self._temporary = None

        if cleanup_error is not None:
            if source_error is not None:
                cleanup_error.add_note(f"source確認も失敗: {source_error}")
            raise FlutterWorkspaceError(f"一時作業領域の後片付けに失敗しました: {cleanup_error}") from cleanup_error
        if source_error is not None:
            raise FlutterWorkspaceError(f"元sourceの同一性を確認できませんでした: {source_error}") from source_error
        return False
