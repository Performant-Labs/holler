#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #196: test assertions

//! A minimal, hand-rolled fake OpenCode HTTP server for `attach_cli_test.rs`
//! (issue #196) — just enough of `GET /session` (and `/api/session`) to
//! drive `body attach sessions`/`body attach init`.
//!
//! `holler-body`'s own `http_attach_driver_test/fake_server.rs` (issue #194)
//! already hand-rolls a fake OpenCode server with a `GET /session` route, but
//! it lives under a *different crate's* `tests/` directory (not the
//! `holler-body` lib itself), so it is not importable from here — Cargo
//! integration-test binaries in one crate cannot `mod`/`use` a file living
//! under another crate's `tests/`. This is a smaller, purpose-built cousin
//! that follows the same conventions (hand-rolled, `std::net` only, no new
//! `axum`/`hyper` dev-dependency, `Connection: close` per response): plain
//! synchronous `std::thread`s rather than the other one's `tokio`, since
//! this fake never needs a long-lived SSE connection, only fixed-shape
//! request/response routes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct RouteReply {
    status: u16,
    body: String,
}

struct State {
    bare: RouteReply,
    api: RouteReply,
    requests: Vec<String>,
}

pub struct FakeServer {
    pub addr: std::net::SocketAddr,
    state: Arc<Mutex<State>>,
}

impl FakeServer {
    /// Starts a fake server whose bare `GET /session` answers `200 []` and
    /// whose `/api/session` answers `404` (the common case: the bare form is
    /// the one that resolves, matching `http_attach_driver`'s own existence
    /// check trying bare first).
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("fake server must bind a loopback port");
        let addr = listener.local_addr().expect("bound listener has a local addr");
        let state = Arc::new(Mutex::new(State {
            bare: RouteReply { status: 200, body: "[]".to_string() },
            api: RouteReply { status: 404, body: "{}".to_string() },
            requests: Vec::new(),
        }));
        let accept_state = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let conn_state = accept_state.clone();
                std::thread::spawn(move || {
                    let _ = handle_connection(stream, conn_state);
                });
            }
        });
        FakeServer { addr, state }
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Set the bare `GET /session` route's status + JSON body.
    pub fn set_bare(&self, status: u16, body: impl Into<String>) {
        let mut s = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        s.bare = RouteReply { status, body: body.into() };
    }

    /// Set the `GET /api/session` fallback route's status + JSON body.
    pub fn set_api(&self, status: u16, body: impl Into<String>) {
        let mut s = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        s.api = RouteReply { status, body: body.into() };
    }

    /// Every request path this server has served, in order (for assertions
    /// like "the bare form was tried before the `/api` fallback").
    pub fn requests(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .requests
            .clone()
    }
}

/// An address nothing is listening on: bind a real loopback port, then drop
/// the listener immediately — the OS releases the port, so a connect to it
/// reliably refuses (rather than picking an arbitrary fixed port that might
/// collide with something else on a CI runner).
pub fn unreachable_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a throwaway port");
    let addr = listener.local_addr().expect("bound listener has a local addr");
    drop(listener);
    format!("http://{addr}")
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<State>>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let mut parts = request_line.trim_end().splitn(3, ' ');
    let _method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default().to_string();

    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line)? == 0 {
            break;
        }
        let trimmed = header_line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let mut state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    state.requests.push(path.clone());
    let reply = match path.as_str() {
        "/session" => state.bare.clone(),
        "/api/session" => state.api.clone(),
        _ => RouteReply { status: 404, body: "{}".to_string() },
    };
    drop(state);

    let status_line = match reply.status {
        200 => "200 OK",
        404 => "404 Not Found",
        500 => "500 Internal Server Error",
        other => panic!("fake server asked to reply with an unmapped status {other}"),
    };
    let response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        reply.body.len(),
        reply.body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
