use std::path::PathBuf;

pub const DEFAULT_COLS: u16 = 80;
pub const DEFAULT_ROWS: u16 = 30;
pub const DEFAULT_EXPECT_TIMEOUT_MS: u64 = 5_000;
pub const SHELL_READY_TIMEOUT_MS: u64 = 30_000;
pub const POLL_DELAY_MS: u64 = 50;
/// Monitor refresh interval (~20fps).
pub const MONITOR_FRAME_MS: u64 = 50;
/// Idle timeout: the daemon shuts itself down after this long without
/// servicing a request.
pub const IDLE_TIMEOUT_MS: u64 = 4 * 60 * 60 * 1_000;
/// How often the idle watchdog checks for inactivity.
pub const IDLE_CHECK_INTERVAL_MS: u64 = 5 * 60 * 1_000;

/// Root directory for all daemon state (sockets, pids, logs).
/// Override with `SHELL_USE_HOME`.
pub fn home_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SHELL_USE_HOME") {
        return PathBuf::from(dir);
    }
    let base = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    base.join(".shell-use")
}

pub fn ensure_home() -> std::io::Result<PathBuf> {
    let dir = home_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn pid_file(session: &str) -> PathBuf {
    home_dir().join(format!("{session}.pid"))
}

pub fn log_file(session: &str) -> PathBuf {
    home_dir().join(format!("{session}.log"))
}

/// Directory for session recordings, in the user's XDG cache. Recordings
/// persist after a session ends (so they remain retrievable) and are cleared
/// per-session when that session's daemon next starts.
///
/// Honors `SHELL_USE_HOME` for test isolation.
pub fn recording_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SHELL_USE_HOME") {
        return PathBuf::from(dir).join("recordings");
    }
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("shell-use")
}

/// Always-on session recording in asciinema v2 cast format, stored in the XDG
/// cache by session name (e.g. `<cache>/shell-use/<session>.cast`).
pub fn recording_file(session: &str) -> PathBuf {
    recording_dir().join(format!("{session}.cast"))
}

/// Platform-appropriate socket name for a session.
///
/// On Windows this is a namespaced pipe name; on Unix it is a filesystem path
/// inside the home directory.
pub fn socket_name(session: &str) -> String {
    if cfg!(windows) {
        format!("shell-use-{session}.sock")
    } else {
        home_dir()
            .join(format!("{session}.sock"))
            .to_string_lossy()
            .into_owned()
    }
}

pub fn session_name_from_env(explicit: Option<String>) -> String {
    explicit
        .or_else(|| std::env::var("SHELL_USE_SESSION").ok())
        .unwrap_or_else(|| "default".to_string())
}
