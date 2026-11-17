from __future__ import annotations

import os


BUILD_ENVIRONMENT_ALLOWLIST = {
    "APPDATA",
    "COMSPEC",
    "FLUTTER_ROOT",
    "HOMEDRIVE",
    "HOMEPATH",
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "LOCALAPPDATA",
    "NUMBER_OF_PROCESSORS",
    "OS",
    "PATH",
    "PATHEXT",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "PUBLIC",
    "SYSTEMDRIVE",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "UCRTVERSION",
    "UNIVERSALCRTSDKDIR",
    "USERPROFILE",
    "VCINSTALLDIR",
    "VCTOOLSINSTALLDIR",
    "VISUALSTUDIOVERSION",
    "VSCMD_ARG_HOST_ARCH",
    "VSCMD_ARG_TGT_ARCH",
    "VSCMD_VER",
    "WINDIR",
    "WINDOWSSDKDIR",
    "WINDOWSSDKVERSION",
    "RUSTUP_HOME",
}


def build_child_environment(
    overrides: dict[str, str], *, inherited: dict[str, str] | None = None
) -> dict[str, str]:
    """Build tools inherit only the approved host/toolchain variables."""
    source = os.environ if inherited is None else inherited
    result = {
        name: value
        for name, value in source.items()
        if name.upper() in BUILD_ENVIRONMENT_ALLOWLIST
    }
    for name, value in overrides.items():
        for existing in tuple(result):
            if existing.upper() == name.upper():
                del result[existing]
        result[name] = value
    return result
