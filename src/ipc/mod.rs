//! CLI ↔ daemon transport over an `interprocess` local socket.
//! One JSON request line per connection, one JSON response line back.

use std::io::{BufRead, BufReader, Write};

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, ListenerOptions, Stream};

use crate::protocol::{Request, Response};

fn to_name(raw: &str) -> std::io::Result<interprocess::local_socket::Name<'_>> {
    if cfg!(windows) {
        raw.to_ns_name::<GenericNamespaced>()
    } else {
        raw.to_fs_name::<GenericFilePath>()
    }
}

/// Connect to a running daemon and exchange a single request/response.
pub fn send(socket: &str, req: &Request) -> anyhow::Result<Response> {
    let name = to_name(socket)?;
    let conn = Stream::connect(name)?;
    let mut reader = BufReader::new(conn);
    let mut line = serde_json::to_string(req)?;
    line.push('\n');
    reader.get_mut().write_all(line.as_bytes())?;
    reader.get_mut().flush()?;

    let mut response = String::new();
    reader.read_line(&mut response)?;
    let resp: Response = serde_json::from_str(response.trim())?;
    Ok(resp)
}

/// Is a daemon currently accepting connections on this socket?
pub fn is_running(socket: &str) -> bool {
    match to_name(socket) {
        Ok(name) => Stream::connect(name).is_ok(),
        Err(_) => false,
    }
}

/// Open a raw connection to a running daemon (for streaming, e.g. the monitor).
pub fn connect(socket: &str) -> std::io::Result<Stream> {
    let name = to_name(socket)?;
    Stream::connect(name)
}

/// Bind the daemon listener, removing any stale Unix socket file first.
pub fn listen(socket: &str) -> anyhow::Result<interprocess::local_socket::Listener> {
    if !cfg!(windows) {
        let _ = std::fs::remove_file(socket);
    }
    let name = to_name(socket)?;
    let listener = ListenerOptions::new().name(name).create_sync()?;
    Ok(listener)
}

/// Read one request line from an accepted connection.
pub fn read_request(conn: &Stream) -> anyhow::Result<Request> {
    let mut reader = BufReader::new(conn);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let req: Request = serde_json::from_str(line.trim())?;
    Ok(req)
}

/// Write one response line to an accepted connection.
pub fn write_response(conn: &mut Stream, resp: &Response) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(resp)?;
    line.push('\n');
    conn.write_all(line.as_bytes())?;
    conn.flush()?;
    Ok(())
}
