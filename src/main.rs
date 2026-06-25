mod agent_context;
mod assert;
mod cli;
mod config;
mod daemon;
mod input;
mod ipc;
mod monitor;
mod protocol;
mod render;
mod shell;
mod terminal;
mod trace;

use std::path::Path;
use std::time::{Duration, Instant};

use clap::Parser;

use cli::{Cli, Command, DaemonCmd, ExpectCmd, GetArg, MouseCmd, WaitCmd};
use protocol::{GetField, MouseAction, Request, Response};

/// Long-form agent skill manifest, printed by `shell-use skill`.
const SKILL_MD: &str = include_str!("../SKILL.md");

fn main() {
    let cli = Cli::parse();
    let session = config::session_name_from_env(cli.session.clone());

    let code = match cli.command {
        Command::InternalDaemon => {
            if let Err(e) = daemon::run(session, cli.verbose) {
                eprintln!("daemon error: {e}");
                std::process::exit(5);
            }
            0
        }
        Command::Usage => {
            print!("{}", usage_text());
            0
        }
        Command::AgentContext => {
            println!("{}", agent_context::render());
            0
        }
        Command::Skill => {
            print!("{SKILL_MD}");
            0
        }
        Command::GetRecording { session: target } => get_recording(target.unwrap_or(session)),
        Command::Sessions => list_sessions(cli.json),
        Command::Close { all } if all => close_all(cli.json),
        Command::Monitor => monitor::run_client(&session),
        command => run_remote(&session, command, cli.json, cli.verbose),
    };
    std::process::exit(code);
}

/// Build the request for a daemon-backed command, then send it.
fn run_remote(session: &str, command: Command, json: bool, verbose: bool) -> i32 {
    let request = match build_request(command) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };

    if let Err(e) = ensure_daemon(session, verbose) {
        eprintln!("failed to start daemon: {e}");
        return 4;
    }

    let socket = config::socket_name(session);
    match ipc::send(&socket, &request) {
        Ok(resp) => print_response(&resp, json),
        Err(e) => {
            eprintln!("request failed: {e}");
            4
        }
    }
}

fn build_request(command: Command) -> anyhow::Result<Request> {
    let req = match command {
        Command::Open {
            shell,
            cols,
            rows,
            cwd,
            env,
        } => Request::Open {
            shell,
            program: None,
            cols,
            rows,
            cwd,
            env: parse_env(&env)?,
        },
        Command::Run {
            program,
            args,
            cols,
            rows,
            cwd,
            env,
        } => {
            let mut prog = vec![program];
            prog.extend(args);
            Request::Open {
                shell: None,
                program: Some(prog),
                cols,
                rows,
                cwd,
                env: parse_env(&env)?,
            }
        }
        Command::Close { .. } => Request::Close,
        Command::Daemon { cmd } => match cmd {
            DaemonCmd::Status => Request::Status,
            DaemonCmd::Stop => Request::Shutdown,
        },
        Command::State => Request::State,
        Command::Text { full } => Request::Text { full },
        Command::Screenshot { path, out, full } => Request::Screenshot {
            full,
            path: out.or(path),
        },
        Command::Cells { x, y, w, h } => Request::Cells { x, y, w, h },
        Command::Get { field } => Request::Get {
            field: map_field(field),
        },
        Command::Type { text } => Request::Write { data: text },
        Command::Submit { text } => Request::Submit { data: text },
        Command::Press { keys } => Request::Press { keys },
        Command::Keys { combo } => Request::Press { keys: vec![combo] },
        Command::Mouse { action } => Request::Mouse {
            action: map_mouse(action),
        },
        Command::Resize { cols, rows } => Request::Resize { cols, rows },
        Command::Write { data } => Request::Write { data },
        Command::Signal { name } => Request::Signal {
            name: name.as_str().to_string(),
        },
        Command::Kill => Request::Signal {
            name: "KILL".to_string(),
        },
        Command::Wait { what } => map_wait(what),
        Command::Expect { what } => map_expect(what),
        _ => anyhow::bail!("unsupported command"),
    };
    Ok(req)
}

fn map_field(f: GetArg) -> GetField {
    match f {
        GetArg::Command => GetField::Command,
        GetArg::Output => GetField::Output,
        GetArg::ExitCode => GetField::ExitCode,
        GetArg::Cwd => GetField::Cwd,
        GetArg::Cursor => GetField::Cursor,
        GetArg::Size => GetField::Size,
    }
}

fn map_mouse(action: MouseCmd) -> MouseAction {
    match action {
        MouseCmd::Click {
            x,
            y,
            on_text,
            button,
            clicks,
        } => MouseAction::Click {
            x,
            y,
            on_text,
            button,
            clicks,
        },
        MouseCmd::Move { x, y } => MouseAction::Move { x, y },
        MouseCmd::Down { x, y, button } => MouseAction::Down { x, y, button },
        MouseCmd::Up { x, y, button } => MouseAction::Up { x, y, button },
        MouseCmd::Drag {
            x1,
            y1,
            x2,
            y2,
            button,
        } => MouseAction::Drag {
            x1,
            y1,
            x2,
            y2,
            button,
        },
        MouseCmd::Scroll { direction, amount } => MouseAction::Scroll {
            direction: direction.as_str().to_string(),
            amount,
        },
    }
}

fn map_wait(what: WaitCmd) -> Request {
    match what {
        WaitCmd::Text {
            text,
            regex,
            full,
            not,
            timeout,
        } => Request::WaitText {
            text,
            regex,
            full,
            timeout_ms: timeout,
            not,
        },
        WaitCmd::Idle { timeout } => Request::WaitIdle {
            timeout_ms: timeout,
        },
        WaitCmd::Command { timeout } => Request::WaitCommand {
            timeout_ms: timeout,
        },
        WaitCmd::Exit { timeout } => Request::WaitExit {
            timeout_ms: timeout,
        },
    }
}

fn map_expect(what: ExpectCmd) -> Request {
    match what {
        ExpectCmd::Text {
            text,
            regex,
            full,
            no_strict,
            not,
            fg,
            bg,
            timeout,
        } => Request::ExpectText {
            text,
            regex,
            full,
            strict: !no_strict,
            not,
            fg,
            bg,
            timeout_ms: timeout,
        },
        ExpectCmd::ExitCode { code } => Request::ExpectExitCode { code },
        ExpectCmd::Output { text, regex } => Request::ExpectOutput { text, regex },
        ExpectCmd::Snapshot {
            name,
            update,
            include_colors,
        } => Request::Snapshot {
            name,
            update,
            include_colors,
        },
    }
}

fn parse_env(pairs: &[String]) -> anyhow::Result<Vec<(String, String)>> {
    pairs
        .iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow::anyhow!("invalid env (expected KEY=VALUE): {p}"))
        })
        .collect()
}

/// Spawn the daemon for this session if it is not already running.
fn ensure_daemon(session: &str, verbose: bool) -> anyhow::Result<()> {
    let socket = config::socket_name(session);
    if ipc::is_running(&socket) {
        if verbose {
            eprintln!(
                "note: daemon for session '{session}' is already running; verbose logging only \
                 applies to a freshly started daemon. Run `shell-use --session {session} close` \
                 first, then retry with --verbose."
            );
        }
        return Ok(());
    }
    config::ensure_home()?;
    let exe = std::env::current_exe()?;
    spawn_detached(&exe, session, verbose)?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if ipc::is_running(&socket) {
            if verbose {
                eprintln!("daemon logging to {}", config::log_file(session).display());
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    anyhow::bail!("daemon did not become ready")
}

#[cfg(windows)]
fn spawn_detached(exe: &Path, session: &str, verbose: bool) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("__daemon").arg("--session").arg(session);
    if verbose {
        cmd.arg("--verbose");
    }
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
fn spawn_detached(exe: &Path, session: &str, verbose: bool) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("__daemon").arg("--session").arg(session);
    if verbose {
        cmd.arg("--verbose");
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Stream a session's recording (asciinema v2 cast) to stdout.
fn get_recording(session: String) -> i32 {
    let path = config::recording_file(&session);
    match std::fs::read(&path) {
        Ok(bytes) => {
            use std::io::Write;
            let _ = std::io::stdout().write_all(&bytes);
            0
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("no recording for session '{session}'");
            3
        }
        Err(e) => {
            eprintln!("failed to read recording: {e}");
            5
        }
    }
}

fn list_sessions(json: bool) -> i32 {
    let home = config::home_dir();
    let mut sessions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&home) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stripped) = name.strip_suffix(".pid") {
                let socket = config::socket_name(stripped);
                if ipc::is_running(&socket) {
                    sessions.push(stripped.to_string());
                }
            }
        }
    }
    if json {
        println!("{}", serde_json::json!({ "sessions": sessions }));
    } else if sessions.is_empty() {
        println!("no active sessions");
    } else {
        for s in sessions {
            println!("{s}");
        }
    }
    0
}

fn close_all(json: bool) -> i32 {
    let home = config::home_dir();
    if let Ok(entries) = std::fs::read_dir(&home) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stripped) = name.strip_suffix(".pid") {
                let socket = config::socket_name(stripped);
                if ipc::is_running(&socket) {
                    let _ = ipc::send(&socket, &Request::Close);
                }
            }
        }
    }
    if json {
        println!("{}", serde_json::json!({ "ok": true }));
    } else {
        println!("closed all sessions");
    }
    0
}

fn print_response(resp: &Response, json: bool) -> i32 {
    if json {
        println!("{}", serde_json::to_string(resp).unwrap_or_default());
        return exit_code(resp);
    }
    if resp.ok {
        if let Some(data) = &resp.data {
            if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                println!("{text}");
            } else {
                println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            }
        }
        0
    } else {
        if let Some(msg) = &resp.message {
            eprintln!("{msg}");
        }
        exit_code(resp)
    }
}

/// Map a response to a stable process exit code (see the exit-code taxonomy).
fn exit_code(resp: &Response) -> i32 {
    if resp.ok {
        0
    } else {
        resp.kind.map(|k| k.exit_code()).unwrap_or(1)
    }
}

fn usage_text() -> &'static str {
    "shell-use: headless terminal CLI + daemon\n\
\n\
SESSION   open [--shell S] [--cols N --rows N] [--cwd D] [--env K=V]\n\
          run <program> [args...]\n\
          sessions | close [--all] | daemon status|stop\n\
INSPECT   state | text [--full] | screenshot [-o file.svg] [--full]\n\
          cells X Y [W H] | get command|output|exit-code|cwd|cursor|size\n\
INPUT     type \"text\" | submit [\"text\"] | press <Key...> | keys \"Ctrl+a\"\n\
          mouse click X Y | mouse click --on-text \"OK\" | mouse move|down|up|drag|scroll\n\
PTY       resize COLS ROWS | write <data> | signal INT|TERM|KILL|QUIT | kill\n\
WAIT      wait text \"T\" [--regex --full --not --timeout MS]\n\
          wait idle | wait command | wait exit\n\
EXPECT    expect text \"T\" [--regex --full --not --fg C --bg C --timeout MS]\n\
          expect exit-code N | expect output \"T\" [--regex]\n\
          expect snapshot NAME [-u] [--include-colors]\n\
RECORD    sessions auto-record; get-recording [session] > out.cast (asciinema v2)\n\
          play with `asciinema play out.cast`, render GIF with `agg out.cast out.gif`\n\
WATCH     monitor (live full-color view in another terminal; q/Esc/Ctrl-C to detach)\n\
AGENT     agent-context (JSON CLI schema) | skill (workflow guide)\n\
GLOBAL    --session NAME | --json | --verbose (log PTY traffic to ~/.shell-use/<session>.log)\n\
EXIT      0 ok | 1 assertion/wait failed | 2 usage | 3 no session | 4 daemon/IPC | 5 internal\n\
"
}
