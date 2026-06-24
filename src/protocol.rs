use serde::{Deserialize, Serialize};

use crate::shell::Shell;

/// A request sent from a stateless CLI invocation to the session daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Open {
        shell: Option<Shell>,
        program: Option<Vec<String>>,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
        env: Vec<(String, String)>,
    },
    Close,
    Status,
    State,
    Text {
        full: bool,
    },
    Cells {
        x: u16,
        y: u16,
        w: u16,
        h: u16,
    },
    Get {
        field: GetField,
    },
    Write {
        data: String,
    },
    Submit {
        data: Option<String>,
    },
    Press {
        keys: Vec<String>,
    },
    Mouse {
        action: MouseAction,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Signal {
        name: String,
    },
    WaitText {
        text: String,
        regex: bool,
        full: bool,
        timeout_ms: u64,
        not: bool,
    },
    WaitIdle {
        timeout_ms: u64,
    },
    WaitCommand {
        timeout_ms: u64,
    },
    WaitExit {
        timeout_ms: u64,
    },
    ExpectText {
        text: String,
        regex: bool,
        full: bool,
        strict: bool,
        not: bool,
        fg: Option<String>,
        bg: Option<String>,
        timeout_ms: u64,
    },
    ExpectExitCode {
        code: i32,
    },
    ExpectOutput {
        text: String,
        regex: bool,
    },
    Snapshot {
        name: String,
        update: bool,
        include_colors: bool,
    },
    Screenshot {
        full: bool,
        path: Option<String>,
    },
    Monitor {
        cols: u16,
        rows: u16,
    },
    Shutdown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GetField {
    Command,
    Output,
    ExitCode,
    Cwd,
    Cursor,
    Size,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MouseAction {
    Click {
        x: Option<u16>,
        y: Option<u16>,
        on_text: Option<String>,
        button: u8,
        clicks: u8,
    },
    Move {
        x: u16,
        y: u16,
    },
    Down {
        x: u16,
        y: u16,
        button: u8,
    },
    Up {
        x: u16,
        y: u16,
        button: u8,
    },
    Drag {
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
        button: u8,
    },
    Scroll {
        direction: String,
        amount: u16,
    },
}

/// Classifies a failure so the CLI can map it to a stable process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// An assertion or wait condition was not met (e.g. `expect`/`wait`).
    Assertion,
    /// An invalid argument value reached the daemon (e.g. bad regex/color).
    Usage,
    /// No active session for the target (run `open`/`run` first).
    NoSession,
    /// An internal error (spawn, I/O, rendering, ...).
    Internal,
}

impl ErrorKind {
    /// Stable process exit code for this failure class.
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorKind::Assertion => 1,
            ErrorKind::Usage => 2,
            ErrorKind::NoSession => 3,
            ErrorKind::Internal => 5,
        }
    }
}

/// A response returned by the daemon to the CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    /// Human/JSON payload describing the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error or assertion-failure message when `ok` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Failure classification when `ok` is false; drives the CLI exit code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ErrorKind>,
}

impl Response {
    pub fn ok() -> Self {
        Response {
            ok: true,
            data: None,
            message: None,
            kind: None,
        }
    }

    pub fn with(data: serde_json::Value) -> Self {
        Response {
            ok: true,
            data: Some(data),
            message: None,
            kind: None,
        }
    }

    /// A failure of the given class.
    pub fn err(kind: ErrorKind, message: impl Into<String>) -> Self {
        Response {
            ok: false,
            data: None,
            message: Some(message.into()),
            kind: Some(kind),
        }
    }

    /// An assertion / wait failure (exit code 1).
    pub fn assertion(message: impl Into<String>) -> Self {
        Response::err(ErrorKind::Assertion, message)
    }

    /// An invalid-argument failure (exit code 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Response::err(ErrorKind::Usage, message)
    }

    /// A "no active session" failure (exit code 3).
    pub fn no_session() -> Self {
        Response::err(
            ErrorKind::NoSession,
            "no active session; run `shell-use open` (or `shell-use run <program>`) first",
        )
    }

    /// An internal failure (exit code 5).
    pub fn internal(message: impl Into<String>) -> Self {
        Response::err(ErrorKind::Internal, message)
    }
}
