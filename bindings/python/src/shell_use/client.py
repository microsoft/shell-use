from __future__ import annotations

import os
from typing import Any, Dict, List, Optional

from . import _config as cfg
from . import _transport as transport
from ._protocol import EnvLike, env_pairs, unwrap
from .errors import VersionMismatchError
from .types import Cell, State


def check_version(daemon_version: Optional[str]) -> None:
    if daemon_version != cfg.VERSION:
        raise VersionMismatchError(
            f"shell-use version mismatch: client {cfg.VERSION}, daemon "
            f"{daemon_version or 'unknown'}. Ensure the shell-use binary matches the "
            "shell-use package version, or stop the daemon (daemon_stop) so it "
            "restarts with the current binary."
        )


class _Mouse:
    def __init__(self, client: "ShellUse") -> None:
        self._c = client

    async def click(
        self,
        x: Optional[int] = None,
        y: Optional[int] = None,
        *,
        on_text: Optional[str] = None,
        button: int = 0,
        clicks: int = 1,
    ) -> None:
        await self._c.send(
            {
                "kind": "mouse",
                "action": {
                    "op": "click",
                    "x": x,
                    "y": y,
                    "on_text": on_text,
                    "button": button,
                    "clicks": clicks,
                },
            }
        )

    async def move(self, x: int, y: int) -> None:
        await self._c.send({"kind": "mouse", "action": {"op": "move", "x": x, "y": y}})

    async def down(self, x: int, y: int, *, button: int = 0) -> None:
        await self._c.send(
            {"kind": "mouse", "action": {"op": "down", "x": x, "y": y, "button": button}}
        )

    async def up(self, x: int, y: int, *, button: int = 0) -> None:
        await self._c.send(
            {"kind": "mouse", "action": {"op": "up", "x": x, "y": y, "button": button}}
        )

    async def drag(
        self, x1: int, y1: int, x2: int, y2: int, *, button: int = 0
    ) -> None:
        await self._c.send(
            {
                "kind": "mouse",
                "action": {
                    "op": "drag",
                    "x1": x1,
                    "y1": y1,
                    "x2": x2,
                    "y2": y2,
                    "button": button,
                },
            }
        )

    async def scroll(self, direction: str, *, amount: int = 3) -> None:
        await self._c.send(
            {
                "kind": "mouse",
                "action": {"op": "scroll", "direction": direction, "amount": amount},
            }
        )


class ShellUse:
    def __init__(
        self,
        session: Optional[str] = None,
        *,
        binary: Optional[str] = None,
        home: Optional[str] = None,
    ) -> None:
        self._session = cfg.resolve_session(session)
        self._binary = cfg.resolve_binary(binary)
        self._home = cfg.resolve_home(home)
        self._version_checked = False
        self.mouse = _Mouse(self)

    @property
    def session(self) -> str:
        return self._session

    async def send(self, payload: Dict[str, Any]) -> Any:
        await self._check_version()
        resp = await transport.request(self._session, self._home, self._binary, payload)
        return unwrap(resp)

    async def _check_version(self) -> None:
        if self._version_checked:
            return
        resp = await transport.request(
            self._session, self._home, self._binary, {"kind": "status"}
        )
        data = unwrap(resp)
        check_version(data.get("version") if isinstance(data, dict) else None)
        self._version_checked = True

    async def open(
        self,
        *,
        shell: Optional[str] = None,
        cols: int = cfg.DEFAULT_COLS,
        rows: int = cfg.DEFAULT_ROWS,
        cwd: Optional[str] = None,
        env: EnvLike = None,
    ) -> Dict[str, Any]:
        return await self.send(
            {
                "kind": "open",
                "shell": shell,
                "program": None,
                "cols": cols,
                "rows": rows,
                "cwd": cwd,
                "env": env_pairs(env),
            }
        )

    async def run(
        self,
        program: str,
        *args: str,
        cols: int = cfg.DEFAULT_COLS,
        rows: int = cfg.DEFAULT_ROWS,
        cwd: Optional[str] = None,
        env: EnvLike = None,
    ) -> Dict[str, Any]:
        return await self.send(
            {
                "kind": "open",
                "shell": None,
                "program": [program, *args],
                "cols": cols,
                "rows": rows,
                "cwd": cwd,
                "env": env_pairs(env),
            }
        )

    async def close(self) -> None:
        if not await transport.can_connect(self._session, self._home):
            return
        resp = await transport.request(
            self._session, self._home, self._binary, {"kind": "close"}, autostart=False
        )
        unwrap(resp)

    async def type(self, text: str) -> None:
        await self.send({"kind": "write", "data": text})

    async def write(self, data: str) -> None:
        await self.send({"kind": "write", "data": data})

    async def submit(self, text: Optional[str] = None) -> None:
        await self.send({"kind": "submit", "data": text})

    async def press(self, *keys: str) -> None:
        await self.send({"kind": "press", "keys": list(keys)})

    async def keys(self, combo: str) -> None:
        await self.send({"kind": "press", "keys": [combo]})

    async def resize(self, cols: int, rows: int) -> None:
        await self.send({"kind": "resize", "cols": cols, "rows": rows})

    async def signal(self, name: str) -> None:
        await self.send({"kind": "signal", "name": name})

    async def kill(self) -> None:
        await self.send({"kind": "signal", "name": "KILL"})

    async def state(self) -> State:
        return State.from_dict(await self.send({"kind": "state"}))

    async def text(self, *, full: bool = False) -> str:
        return (await self.send({"kind": "text", "full": full}))["text"]

    async def cells(self, x: int, y: int, w: int = 1, h: int = 1) -> List[Cell]:
        data = await self.send({"kind": "cells", "x": x, "y": y, "w": w, "h": h})
        return [Cell(**c) for c in data["cells"]]

    async def get(self, field: str) -> Any:
        return (await self.send({"kind": "get", "field": field}))["value"]

    async def get_command(self) -> Optional[str]:
        return await self.get("command")

    async def get_output(self) -> Optional[str]:
        return await self.get("output")

    async def get_exit_code(self) -> Optional[int]:
        return await self.get("exit-code")

    async def get_cwd(self) -> Optional[str]:
        return await self.get("cwd")

    async def get_cursor(self) -> Dict[str, int]:
        return await self.get("cursor")

    async def get_size(self) -> Dict[str, int]:
        return await self.get("size")

    async def screenshot(self, path: Optional[str] = None, *, full: bool = False) -> str:
        data = await self.send({"kind": "screenshot", "full": full, "path": path})
        return data.get("path") or data.get("text")

    async def wait_text(
        self,
        text: str,
        *,
        regex: bool = False,
        full: bool = False,
        not_: bool = False,
        timeout: int = 5000,
    ) -> None:
        await self.send(
            {
                "kind": "wait_text",
                "text": text,
                "regex": regex,
                "full": full,
                "timeout_ms": timeout,
                "not": not_,
            }
        )

    async def wait_idle(self, *, timeout: int = 5000) -> None:
        await self.send({"kind": "wait_idle", "timeout_ms": timeout})

    async def wait_command(self, *, timeout: int = 30000) -> None:
        await self.send({"kind": "wait_command", "timeout_ms": timeout})

    async def wait_exit(self, *, timeout: int = 30000) -> None:
        await self.send({"kind": "wait_exit", "timeout_ms": timeout})

    async def expect_text(
        self,
        text: str,
        *,
        regex: bool = False,
        full: bool = False,
        strict: bool = True,
        not_: bool = False,
        fg: Optional[str] = None,
        bg: Optional[str] = None,
        timeout: int = 5000,
    ) -> None:
        await self.send(
            {
                "kind": "expect_text",
                "text": text,
                "regex": regex,
                "full": full,
                "strict": strict,
                "not": not_,
                "fg": fg,
                "bg": bg,
                "timeout_ms": timeout,
            }
        )

    async def expect_exit_code(self, code: int) -> None:
        await self.send({"kind": "expect_exit_code", "code": code})

    async def expect_output(self, text: str, *, regex: bool = False) -> None:
        await self.send({"kind": "expect_output", "text": text, "regex": regex})

    async def expect_snapshot(
        self, name: str, *, update: bool = False, include_colors: bool = False
    ) -> str:
        return (
            await self.send(
                {
                    "kind": "snapshot",
                    "name": name,
                    "update": update,
                    "include_colors": include_colors,
                    "cwd": os.getcwd(),
                }
            )
        )["status"]

    async def __aenter__(self) -> "ShellUse":
        return self

    async def __aexit__(self, *exc: Any) -> None:
        await self.close()


async def sessions(*, home: Optional[str] = None) -> List[str]:
    h = cfg.resolve_home(home)
    directory = cfg.home_dir(h)
    out: List[str] = []
    if directory.is_dir():
        for entry in sorted(directory.iterdir()):
            if entry.suffix == ".pid":
                name = entry.stem
                if await transport.can_connect(name, h):
                    out.append(name)
    return out


async def close_all(*, binary: Optional[str] = None, home: Optional[str] = None) -> None:
    h = cfg.resolve_home(home)
    b = cfg.resolve_binary(binary)
    for name in await sessions(home=h):
        try:
            await transport.request(name, h, b, {"kind": "close"}, autostart=False)
        except Exception:
            pass


async def daemon_status(
    session: Optional[str] = None,
    *,
    binary: Optional[str] = None,
    home: Optional[str] = None,
) -> Dict[str, Any]:
    s = cfg.resolve_session(session)
    h = cfg.resolve_home(home)
    b = cfg.resolve_binary(binary)
    return unwrap(await transport.request(s, h, b, {"kind": "status"}))


async def daemon_stop(
    session: Optional[str] = None,
    *,
    binary: Optional[str] = None,
    home: Optional[str] = None,
) -> None:
    s = cfg.resolve_session(session)
    h = cfg.resolve_home(home)
    b = cfg.resolve_binary(binary)
    if not await transport.can_connect(s, h):
        return
    unwrap(await transport.request(s, h, b, {"kind": "shutdown"}, autostart=False))


async def get_recording(
    session: Optional[str] = None, *, home: Optional[str] = None
) -> str:
    import asyncio

    s = cfg.resolve_session(session)
    h = cfg.resolve_home(home)
    path = cfg.recording_path(s, h)
    loop = asyncio.get_running_loop()
    try:
        data = await loop.run_in_executor(None, path.read_bytes)
    except FileNotFoundError:
        from .errors import NoSessionError

        raise NoSessionError(f"no recording for session '{s}'")
    return data.decode("utf-8", errors="replace")
