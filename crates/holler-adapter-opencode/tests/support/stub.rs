//! A hand-rolled stub of OpenCode's HTTP API for `hermetic_test.rs` (#642a).
//!
//! Its cousin is `holler-cli/tests/attach_cli_test/fake_server.rs`: the same conventions
//! (`std::net` and plain `std::thread`s, no new dev-dependency, `Connection: close` on every
//! reply), except for the refused port: that file binds a port and drops the listener, which
//! is racy under parallel tests (see `closed_port`). That file lives under another crate's `tests/`,
//! so it cannot be imported from here. This one adds what the adapter's tests need and that
//! one lacks: replies per method and path, a chunked reply, a raw non-HTTP reply, a frozen
//! mode (accept, read, never answer), and a record of every request's raw request line and
//! body exactly as received, before any URL decoding.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// How the stub frames a reply's body.
#[derive(Clone, Copy)]
pub enum Framing {
    ContentLength,
    /// `Transfer-Encoding: chunked`, the body split into two chunks.
    Chunked,
}

#[derive(Clone)]
struct Canned {
    status: u16,
    content_type: &'static str,
    body: String,
    framing: Framing,
}

/// One request as the stub received it.
#[derive(Clone, Debug)]
pub struct Seen {
    /// The raw request line, e.g. `GET /session/ses%20x HTTP/1.1` (CRLF removed).
    pub line: String,
    pub body: Vec<u8>,
}

#[derive(Default)]
struct State {
    routes: HashMap<(String, String), Canned>,
    frozen: bool,
    raw: Option<Vec<u8>>,
    seen: Vec<Seen>,
    held: Vec<TcpStream>,
}

/// The stub server, on an OS-assigned loopback port.
pub struct Stub {
    pub port: u16,
    state: Arc<Mutex<State>>,
}

/// Default reply of a route the test did not set: OpenCode's JSON 404.
pub const NOT_FOUND: &str = r#"{"name":"NotFoundError","data":{"message":"not found"}}"#;

/// What OpenCode's web app answers for a route it does not serve: `200` HTML, longer than
/// any one-line message may be, with line breaks in it.
pub const HTML: &str = "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
<title>OpenCode</title><script type=\"module\" src=\"/assets/index-abcdef.js\"></script></head>\n\
<body><div id=\"root\"></div><noscript>You need to enable JavaScript to run this app.</noscript>\
</body>\n</html>\n";

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Stub {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stub binds a loopback port");
        let port = listener
            .local_addr()
            .expect("a bound listener has an address")
            .port();
        let state = Arc::new(Mutex::new(State::default()));
        let accept_state = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let conn_state = Arc::clone(&accept_state);
                std::thread::spawn(move || {
                    let _ = serve_one(stream, &conn_state);
                });
            }
        });
        Stub { port, state }
    }

    /// `method path` answers `status` with a JSON body framed by `Content-Length`.
    pub fn json(&self, method: &str, path: &str, status: u16, body: &str) {
        self.set(
            method,
            path,
            status,
            "application/json",
            body,
            Framing::ContentLength,
        );
    }

    /// `method path` answers `200` `text/html` (the web app's catch-all).
    pub fn html(&self, method: &str, path: &str) {
        self.set(method, path, 200, "text/html", HTML, Framing::ContentLength);
    }

    pub fn set(
        &self,
        method: &str,
        path: &str,
        status: u16,
        content_type: &'static str,
        body: &str,
        framing: Framing,
    ) {
        let canned = Canned {
            status,
            content_type,
            body: body.to_string(),
            framing,
        };
        lock(&self.state)
            .routes
            .insert((method.to_string(), path.to_string()), canned);
    }

    /// Every connection gets these bytes as its whole answer, then a close.
    pub fn raw(&self, bytes: &[u8]) {
        lock(&self.state).raw = Some(bytes.to_vec());
    }

    /// Accept and read every request, and never answer it (a SIGSTOPped server's socket).
    pub fn freeze(&self) {
        lock(&self.state).frozen = true;
    }

    /// Every request received, in order.
    pub fn seen(&self) -> Vec<Seen> {
        lock(&self.state).seen.clone()
    }

    /// The raw request lines received, in order.
    pub fn lines(&self) -> Vec<String> {
        self.seen().into_iter().map(|s| s.line).collect()
    }
}

/// The connections whose client ends are the `closed_port`s, held until the process exits.
static HELD: Mutex<Vec<(TcpStream, TcpStream)>> = Mutex::new(Vec::new());

/// A loopback port nothing listens on, has ever listened on, or can be handed to a later bind.
///
/// Binding a port and dropping the listener is racy. A process that another test spawns at
/// that moment holds a copy of every descriptor between its `fork` and its `exec`, the listener
/// among them, so the listener can outlive the drop: a connect then completes against it and is
/// reset once the child's `exec` closes the last copy. A port that is free can also be handed
/// to a stub that a test running in parallel starts.
///
/// So the port is the client end of a held connection, an ephemeral port that `connect` picked
/// and that never had a listener. A new connect to it matches no socket (the held one is tied
/// to its own peer) and is answered with a reset, a refusal, on Linux and on macOS alike. The
/// held socket keeps the port in use for the life of the process, so Linux never hands it to a
/// `bind` of port 0.
pub fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a throwaway listener");
    let client = TcpStream::connect(listener.local_addr().expect("a bound listener's address"))
        .expect("connect to the throwaway listener");
    let (server, _) = listener.accept().expect("accept the held connection");
    let port = client
        .local_addr()
        .expect("a connected socket has an address")
        .port();
    HELD.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((client, server));
    port
}

fn serve_one(mut stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let seen = read_request(&stream)?;
    let (method, path) = {
        let mut parts = seen.line.splitn(3, ' ');
        let method = parts.next().unwrap_or_default().to_string();
        (method, parts.next().unwrap_or_default().to_string())
    };
    let mut st = lock(state);
    st.seen.push(seen);
    if st.frozen {
        st.held.push(stream);
        return Ok(());
    }
    if let Some(raw) = st.raw.clone() {
        drop(st);
        stream.write_all(&raw)?;
        return stream.flush();
    }
    let canned = st.routes.get(&(method, path)).cloned().unwrap_or(Canned {
        status: 404,
        content_type: "application/json",
        body: NOT_FOUND.to_string(),
        framing: Framing::ContentLength,
    });
    drop(st);
    stream.write_all(&encode(&canned))?;
    stream.flush()
}

fn read_request(stream: &TcpStream) -> std::io::Result<Seen> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim_end_matches(['\r', '\n']).to_string();
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim_end().is_empty() {
            break;
        }
        if let Some((key, value)) = header.trim_end().split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    Ok(Seen { line, body })
}

fn encode(canned: &Canned) -> Vec<u8> {
    let reason = match canned.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let mut out = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nConnection: close\r\n",
        canned.status, canned.content_type
    );
    let body = canned.body.as_bytes();
    match canned.framing {
        Framing::ContentLength => {
            out.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
            let mut bytes = out.into_bytes();
            bytes.extend_from_slice(body);
            bytes
        }
        Framing::Chunked => {
            out.push_str("Transfer-Encoding: chunked\r\n\r\n");
            let mut bytes = out.into_bytes();
            let (a, b) = body.split_at(body.len() / 2);
            for chunk in [a, b] {
                bytes.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
                bytes.extend_from_slice(chunk);
                bytes.extend_from_slice(b"\r\n");
            }
            bytes.extend_from_slice(b"0\r\n\r\n");
            bytes
        }
    }
}
