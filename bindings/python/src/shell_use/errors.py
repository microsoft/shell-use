from __future__ import annotations

from typing import Optional


class ShellUseError(Exception):
    kind: str = "internal"
    exit_code: int = 5

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message


class ExpectationError(ShellUseError):
    kind = "assertion"
    exit_code = 1


class UsageError(ShellUseError):
    kind = "usage"
    exit_code = 2


class NoSessionError(ShellUseError):
    kind = "no_session"
    exit_code = 3


class DaemonError(ShellUseError):
    kind = "daemon"
    exit_code = 4


class VersionMismatchError(ShellUseError):
    kind = "version_mismatch"
    exit_code = 4


class InternalError(ShellUseError):
    kind = "internal"
    exit_code = 5


_BY_KIND = {
    "assertion": ExpectationError,
    "usage": UsageError,
    "no_session": NoSessionError,
    "daemon": DaemonError,
    "internal": InternalError,
}


def make_error(kind: Optional[str], message: str) -> ShellUseError:
    """Construct the typed error for a daemon ``kind`` string."""
    return _BY_KIND.get(kind or "", InternalError)(message)
