export { ShellUse } from "./client.js";
export type {
  ExpectTextOptions,
  MouseButtonOptions,
  WaitTextOptions,
} from "./client.js";
export {
  closeAll,
  daemonStatus,
  daemonStop,
  getRecording,
  sessions,
} from "./sessions.js";
export {
  DaemonError,
  ExpectationError,
  InternalError,
  NoSessionError,
  ShellUseError,
  UsageError,
  VersionMismatchError,
} from "./errors.js";
export type { ErrorKind } from "./errors.js";
export { VERSION } from "./version.js";
export type {
  Cell,
  ClientOptions,
  Color,
  Cursor,
  DaemonStatus,
  OpenResult,
  Shell,
  Size,
  SpawnOptions,
  State,
} from "./types.js";
