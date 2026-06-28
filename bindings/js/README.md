# @microsoft/shell-use

A Node client for the [`shell-use`](https://github.com/microsoft/shell-use) terminal daemon.

The `shell-use` binary must be on your `PATH` (or point to it with the `SHELL_USE_BIN` environment variable or the `binary` option). The client talks to the per-session daemon directly over its local socket (a named pipe on Windows, a Unix socket elsewhere) and starts the daemon automatically.

## Install

```sh
npm install @microsoft/shell-use # Node 20+

bun add @microsoft/shell-use # Bun

deno add npm:@microsoft/shell-use # Deno 2
```

The package is only ESM

## Runtime Requirements

- Node: 20+
- Deno: 2
- Bun: *

> Note: On Windows, Deno requires all permissions (`-A` / `--allow-all`) instead of just `--allow-read --allow-write` due to the use of named pipes for IPC with the daemon.

## Quick start

```js
import { ShellUse } from "@microsoft/shell-use";

const su = new ShellUse();
await su.open();
await su.submit("echo hello");
await su.waitCommand();
await su.expectText("hello");
await su.expectExitCode(0);
await su.close();
```

## Errors

Every failure maps to one of the daemon's exit codes:

| Class | `exitCode` | Meaning |
| --- | --- | --- |
| `ExpectationError` | 1 | an `expect`/`wait` condition was not met |
| `UsageError` | 2 | invalid argument (e.g. a bad regex) |
| `NoSessionError` | 3 | no active session |
| `DaemonError` | 4 | daemon could not be reached or started |
| `VersionMismatchError` | 4 | the daemon's version differs from this package |
| `InternalError` | 5 | internal daemon error |

All derive from `ShellUseError` and carry `kind` and `exitCode`. `waitX` and `expectX` reject with `ExpectationError` on failure.

On its first call, a client checks that the running daemon's version matches the
package version and throws `VersionMismatchError` if they differ. Stop the daemon
(`daemonStop`) so it restarts with the current binary, or point `SHELL_USE_BIN`
at a matching one.

## API

`new ShellUse(session?, { binary?, home? })` mirrors the CLI: `open` / `run`, `type` / `write`, `submit`, `press` / `keys`, `mouse.click|move|down|up|drag|scroll`, `resize`, `signal` / `kill`, `state`, `text`, `cells`, `get` (+ `getCommand` / `getOutput` / `getExitCode` / `getCwd` / `getCursor` / `getSize`), `screenshot`, `waitText` / `waitIdle` / `waitCommand` / `waitExit`, `expectText` / `expectExitCode` / `expectOutput` / `expectSnapshot`, and `close`.

Module-level helpers: `sessions()`, `closeAll()`, `daemonStatus()`, `daemonStop()`, `getRecording()`.

## Configuration

| Variable | Purpose |
| --- | --- |
| `SHELL_USE_BIN` | path to the `shell-use` binary |
| `SHELL_USE_SESSION` | default session name |
| `SHELL_USE_HOME` | daemon state directory (sockets, pids) |
