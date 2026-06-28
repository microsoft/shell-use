import { makeError } from "./errors.js";
import type { Response } from "./types.js";

export function unwrap(resp: Response): unknown {
  if (resp.ok) {
    return resp.data;
  }
  throw makeError(resp.kind, resp.message || "shell-use error");
}

export function envPairs(
  env?: Record<string, string> | [string, string][],
): [string, string][] {
  if (!env) {
    return [];
  }
  if (Array.isArray(env)) {
    return env;
  }
  return Object.entries(env);
}
