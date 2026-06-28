import os from "node:os";
import path from "node:path";

export const DEFAULT_COLS = 80;
export const DEFAULT_ROWS = 30;

export const IS_WINDOWS = process.platform === "win32";

export function resolveSession(session?: string): string {
  return session || process.env.SHELL_USE_SESSION || "default";
}

export function resolveBinary(binary?: string): string {
  return binary || process.env.SHELL_USE_BIN || "shell-use";
}

export function resolveHome(home?: string): string | undefined {
  return home || process.env.SHELL_USE_HOME || undefined;
}

export function homeDir(home?: string): string {
  return home || path.join(os.homedir(), ".shell-use");
}

export function socketPath(session: string, home?: string): string {
  if (IS_WINDOWS) {
    return `\\\\.\\pipe\\shell-use-${session}.sock`;
  }
  return path.join(homeDir(home), `${session}.sock`);
}

function cacheDir(): string {
  if (IS_WINDOWS) {
    return process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local");
  }
  if (process.platform === "darwin") {
    return path.join(os.homedir(), "Library", "Caches");
  }
  return process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache");
}

export function recordingDir(home?: string): string {
  if (home) {
    return path.join(home, "recordings");
  }
  return path.join(cacheDir(), "shell-use");
}

export function recordingPath(session: string, home?: string): string {
  return path.join(recordingDir(home), `${session}.cast`);
}
