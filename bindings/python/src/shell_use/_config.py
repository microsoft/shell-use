from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Optional

VERSION = "0.0.1-beta.5"

DEFAULT_COLS = 80
DEFAULT_ROWS = 30

IS_WINDOWS = sys.platform == "win32"


def resolve_session(session: Optional[str]) -> str:
    return session or os.environ.get("SHELL_USE_SESSION") or "default"


def resolve_binary(binary: Optional[str]) -> str:
    return binary or os.environ.get("SHELL_USE_BIN") or "shell-use"


def resolve_home(home: Optional[str]) -> Optional[str]:
    return home or os.environ.get("SHELL_USE_HOME") or None


def home_dir(home: Optional[str]) -> Path:
    return Path(home) if home else Path.home() / ".shell-use"


def socket_path(session: str, home: Optional[str]) -> str:
    if IS_WINDOWS:
        return rf"\\.\pipe\shell-use-{session}.sock"
    return str(home_dir(home) / f"{session}.sock")


def _cache_dir() -> Path:
    if IS_WINDOWS:
        base = os.environ.get("LOCALAPPDATA")
        return Path(base) if base else Path.home() / "AppData" / "Local"
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Caches"
    xdg = os.environ.get("XDG_CACHE_HOME")
    return Path(xdg) if xdg else Path.home() / ".cache"


def recording_dir(home: Optional[str]) -> Path:
    if home:
        return Path(home) / "recordings"
    return _cache_dir() / "shell-use"


def recording_path(session: str, home: Optional[str]) -> Path:
    return recording_dir(home) / f"{session}.cast"
