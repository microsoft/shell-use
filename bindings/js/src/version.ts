import { VersionMismatchError } from "./errors.js";

export const VERSION = "0.0.1-beta.5";

export function checkVersion(daemonVersion: string | undefined): void {
  if (daemonVersion !== VERSION) {
    throw new VersionMismatchError(
      `shell-use version mismatch: client ${VERSION}, daemon ${daemonVersion ?? "unknown"}. ` +
        "Ensure the shell-use binary matches the @microsoft/shell-use package version, " +
        "or stop the daemon (daemonStop) so it restarts with the current binary.",
    );
  }
}
