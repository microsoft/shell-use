# shell-use (Python)

A Python client for the [`shell-use`](https://github.com/microsoft/shell-use) terminal daemon.

The `shell-use` binary must be on your `PATH` (or point to it with the `SHELL_USE_BIN` environment variable or the `binary=` argument). The client talks to the per-session daemon directly over its local socket (a named pipe on Windows, a Unix socket elsewhere) and starts the daemon automatically.

## Install

```sh
pip install shell-use
```

Requires Python 3.8+.

## Quick start

```python
import asyncio
from shell_use import ShellUse

async def main():
    async with ShellUse() as su:
        await su.open()
        await su.submit("echo hello")
        await su.wait_command()
        await su.expect_text("hello")
        await su.expect_exit_code(0)

asyncio.run(main())
```

Drive a full-screen TUI:

```python
async with ShellUse("vim-session") as su:
    await su.run("vim", "file.txt")
    await su.wait_idle()
    await su.press("i")
    await su.type("some text")
    await su.press("Escape", ":", "w", "q", "Enter")
    await su.wait_exit()
```

## Errors

Every failure maps to one of the daemon's exit codes:

| Exception | Exit code | Meaning |
| --- | --- | --- |
| `ExpectationError` | 1 | an `expect`/`wait` condition was not met |
| `UsageError` | 2 | invalid argument (e.g. a bad regex) |
| `NoSessionError` | 3 | no active session |
| `DaemonError` | 4 | daemon could not be reached or started |
| `VersionMismatchError` | 4 | the daemon's version differs from this package |
| `InternalError` | 5 | internal daemon error |

All derive from `ShellUseError`. `wait_*` and `expect_*` raise `ExpectationError` on failure. Assertion errors include the current visible terminal content.

On its first call, a client checks that the running daemon's version matches the
package version and raises `VersionMismatchError` if they differ. Stop the daemon
(`daemon_stop`) so it restarts with the current binary, or point `SHELL_USE_BIN`
at a matching one.

## API

`ShellUse(session="default", *, binary=None, home=None)` mirrors the CLI: `open` / `run`, `type` / `write`, `submit`, `press` / `keys`, `mouse.click|move|down|up|drag|scroll`, `resize`, `signal` / `kill`, `state`, `text`, `cells`, `get` (+ `get_command` / `get_output` / `get_exit_code` / `get_cwd` / `get_cursor` / `get_size`), `screenshot`, `wait_text` / `wait_idle` / `wait_command` / `wait_exit`, `expect_text` / `expect_exit_code` / `expect_output` / `expect_snapshot`, and `close`.

Module-level helpers: `sessions()`, `close_all()`, `daemon_status()`, `daemon_stop()`, `get_recording()`.

## Configuration

| Variable | Purpose |
| --- | --- |
| `SHELL_USE_BIN` | path to the `shell-use` binary |
| `SHELL_USE_SESSION` | default session name |
| `SHELL_USE_HOME` | daemon state directory (sockets, pids) |
