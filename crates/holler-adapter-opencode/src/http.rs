//! The adapter's own HTTP/1.1 client for OpenCode on `127.0.0.1`: blocking, one request
//! per connection (`Connection: close`), every step bounded by one timeout.
//!
//! It is public so this crate's tests (and part 2's pins of raw OpenCode) reuse it instead
//! of writing a second client. It is not an interface: it has no per-call deadline, no
//! reply-shape check and no error mapping. Code outside this crate reaches OpenCode only
//! through `HarnessPort`.
//!
//! Why a client of its own: the only OpenCode clients in the repo are async (`reqwest` on
//! `tokio`, in `holler-body`), a layer an adapter must not depend on, and the port must
//! tell "nothing listens" ([`HttpError::Refused`]) from "connected, never answered"
//! ([`HttpError::TimedOut`], a frozen server: `docs/research/opencode-pane-spike.md`,
//! 189-192), which a blocking `TcpStream` with deadlines gives directly. The head is parsed
//! by `httparse`, which the hub's hand-rolled WebSocket handshake already uses.
//!
//! How a reply is read: the head is parsed with `httparse::Response::parse`, reading more
//! while it is partial. The request sends no `Expect`, but an interim `1xx` head is skipped
//! and the next one read (RFC 9110 section 15.2). The body is then read by
//! `Transfer-Encoding: chunked` (each size line through `httparse::parse_chunk_size`, up to
//! the `0` chunk, whose trailers are discarded), else by `Content-Length`, else to the end of
//! the stream. A `204` or `304` has no body. A version other than HTTP/1.x, a malformed head
//! or chunk, a connection closed before the declared length, or a reply longer than 64 MiB
//! is [`HttpError::Garbled`].

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use httparse::{Header, Status};
use serde_json::Value;

/// The most headers a reply head may carry.
const MAX_HEADERS: usize = 64;

/// The most bytes a reply may take, head and body together (64 MiB): a session list of
/// thousands of sessions is a few MiB.
const MAX_REPLY: usize = 64 << 20;

/// A reply: the status and the whole body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request got no reply the adapter can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// Nothing listens: the connection was refused.
    Refused,
    /// No whole reply within the timeout: the connect, the request or the reply did not
    /// finish (a frozen server accepts and never answers).
    TimedOut,
    /// No reply the client can read: not an HTTP/1.x response, a reply cut short, or a
    /// connection that failed other than by a refusal or a timeout. The text says which.
    Garbled(String),
}

/// One request to `http://127.0.0.1:<port><path>`, `Connection: close`, with `body` as JSON
/// if given; the connect, the request and the whole reply together within `timeout`.
///
/// `path` is sent as it is, so it must already be percent-encoded (the port methods encode
/// every session id).
pub fn request(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&Value>,
    timeout: Duration,
) -> Result<Reply, HttpError> {
    let deadline = crate::deadline_after(timeout);
    let mut stream = connect(port, deadline)?;
    send(&mut stream, &encode(port, method, path, body), deadline)?;
    let mut reader = Reader {
        stream,
        buf: Vec::new(),
        deadline,
    };
    let (status, framing) = reader.head()?;
    let body = reader.body(framing)?;
    Ok(Reply { status, body })
}

/// How a reply's body is delimited.
enum Framing {
    /// No body (`1xx`, `204`, `304`).
    Empty,
    /// `Content-Length`.
    Length(usize),
    /// `Transfer-Encoding: chunked`.
    Chunked,
    /// Up to the end of the stream (the request sent `Connection: close`).
    ToEnd,
}

/// What is left before `deadline`; nothing left is [`HttpError::TimedOut`].
fn left(deadline: Instant) -> Result<Duration, HttpError> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err(HttpError::TimedOut)
    } else {
        Ok(left)
    }
}

/// An I/O error while `doing` something: a timeout, or a connection that broke.
fn failed(doing: &str, error: &std::io::Error) -> HttpError {
    match error.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => HttpError::TimedOut,
        _ => HttpError::Garbled(format!("cannot {doing}: {error}")),
    }
}

fn connect(port: u16, deadline: Instant) -> Result<TcpStream, HttpError> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, left(deadline)?).map_err(|error| match error.kind() {
        ErrorKind::ConnectionRefused => HttpError::Refused,
        _ => failed("connect", &error),
    })
}

/// The request's bytes: the head, then the JSON body if there is one. A method that carries
/// content (`POST`, `PUT`, `PATCH`) states its length even when it sends none.
fn encode(port: u16, method: &str, path: &str, body: Option<&Value>) -> Vec<u8> {
    let body = body.map(Value::to_string);
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\n\
         Connection: close\r\n"
    );
    if body.is_some() {
        head.push_str("Content-Type: application/json\r\n");
    }
    if body.is_some() || matches!(method, "POST" | "PUT" | "PATCH") {
        let length = body.as_ref().map_or(0, String::len);
        head.push_str(&format!("Content-Length: {length}\r\n"));
    }
    head.push_str("\r\n");
    let mut bytes = head.into_bytes();
    if let Some(body) = body {
        bytes.extend_from_slice(body.as_bytes());
    }
    bytes
}

fn send(stream: &mut TcpStream, bytes: &[u8], deadline: Instant) -> Result<(), HttpError> {
    stream
        .set_write_timeout(Some(left(deadline)?))
        .map_err(|error| failed("set a send timeout", &error))?;
    stream
        .write_all(bytes)
        .map_err(|error| failed("send the request", &error))
}

/// The head at the start of `buf`: its length, status and framing, or `None` while partial.
fn parse_head(buf: &[u8]) -> Result<Option<(usize, u16, Framing)>, HttpError> {
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut head = httparse::Response::new(&mut headers);
    match head.parse(buf) {
        Ok(Status::Complete(len)) => {
            let status = head
                .code
                .ok_or_else(|| HttpError::Garbled("a reply head without a status".to_owned()))?;
            Ok(Some((len, status, framing(status, head.headers)?)))
        }
        Ok(Status::Partial) => Ok(None),
        Err(error) => Err(HttpError::Garbled(format!(
            "not an HTTP/1.x reply ({error})"
        ))),
    }
}

/// How the body after a head of `status` with `headers` is delimited (RFC 9112 section 6.3).
fn framing(status: u16, headers: &[Header<'_>]) -> Result<Framing, HttpError> {
    if (100..200).contains(&status) || status == 204 || status == 304 {
        return Ok(Framing::Empty);
    }
    let named = |name: &'static str| {
        headers
            .iter()
            .filter(move |header| header.name.eq_ignore_ascii_case(name))
    };
    if let Some(coding) = named("transfer-encoding").next_back() {
        // A body is chunked only when chunked is the final coding; otherwise it runs to
        // the end of the stream.
        let last = coding
            .value
            .rsplit(|&b| b == b',')
            .next()
            .unwrap_or_default();
        let chunked = last.trim_ascii().eq_ignore_ascii_case(b"chunked");
        return Ok(if chunked {
            Framing::Chunked
        } else {
            Framing::ToEnd
        });
    }
    let mut length = None;
    for header in named("content-length") {
        let value = parse_length(header.value)
            .ok_or_else(|| HttpError::Garbled("an invalid Content-Length".to_owned()))?;
        if length.is_some_and(|seen| seen != value) {
            return Err(HttpError::Garbled(
                "two different Content-Lengths".to_owned(),
            ));
        }
        length = Some(value);
    }
    Ok(length.map_or(Framing::ToEnd, Framing::Length))
}

/// A `Content-Length` value: one or more ASCII digits, nothing else.
fn parse_length(value: &[u8]) -> Option<usize> {
    let value = value.trim_ascii();
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse().ok()
}

fn too_long() -> HttpError {
    HttpError::Garbled(format!("the reply is longer than {MAX_REPLY} bytes"))
}

/// The reading half of one request: the connection, the bytes read and not yet used, and
/// the request's deadline.
struct Reader {
    stream: TcpStream,
    buf: Vec<u8>,
    deadline: Instant,
}

impl Reader {
    /// Read more of the reply onto `buf`; `Ok(0)` at the end of the stream.
    fn fill(&mut self) -> Result<usize, HttpError> {
        if self.buf.len() >= MAX_REPLY {
            return Err(too_long());
        }
        let mut chunk = [0u8; 8192];
        loop {
            self.stream
                .set_read_timeout(Some(left(self.deadline)?))
                .map_err(|error| failed("set a read timeout", &error))?;
            match self.stream.read(&mut chunk) {
                Ok(n) => {
                    self.buf.extend_from_slice(&chunk[..n]);
                    return Ok(n);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(failed("read the reply", &error)),
            }
        }
    }

    /// `fill`, where the end of the stream cuts the reply short inside `what`.
    fn more(&mut self, what: &str) -> Result<(), HttpError> {
        if self.fill()? == 0 {
            Err(HttpError::Garbled(format!(
                "the connection closed inside {what}"
            )))
        } else {
            Ok(())
        }
    }

    /// The status and framing of the final head; interim `1xx` heads are skipped.
    fn head(&mut self) -> Result<(u16, Framing), HttpError> {
        loop {
            match parse_head(&self.buf)? {
                Some((len, status, framing)) => {
                    self.buf.drain(..len);
                    if !(100..200).contains(&status) {
                        return Ok((status, framing));
                    }
                }
                None => {
                    let nothing_yet = self.buf.is_empty();
                    if self.fill()? == 0 {
                        return Err(HttpError::Garbled(if nothing_yet {
                            "the server closed the connection without a reply".to_owned()
                        } else {
                            "the connection closed inside the reply head".to_owned()
                        }));
                    }
                }
            }
        }
    }

    fn body(&mut self, framing: Framing) -> Result<Vec<u8>, HttpError> {
        match framing {
            Framing::Empty => Ok(Vec::new()),
            Framing::Length(length) => self.sized(length),
            Framing::Chunked => self.chunked(),
            Framing::ToEnd => {
                while self.fill()? > 0 {}
                Ok(std::mem::take(&mut self.buf))
            }
        }
    }

    fn sized(&mut self, length: usize) -> Result<Vec<u8>, HttpError> {
        if length > MAX_REPLY {
            return Err(too_long());
        }
        while self.buf.len() < length {
            if self.fill()? == 0 {
                return Err(HttpError::Garbled(format!(
                    "the connection closed before the declared length ({} of {length} bytes)",
                    self.buf.len()
                )));
            }
        }
        self.buf.truncate(length);
        Ok(std::mem::take(&mut self.buf))
    }

    fn chunked(&mut self) -> Result<Vec<u8>, HttpError> {
        let mut body = Vec::new();
        loop {
            let (line, size) = match httparse::parse_chunk_size(&self.buf) {
                Ok(Status::Complete(found)) => found,
                Ok(Status::Partial) => {
                    self.more("a chunk size line")?;
                    continue;
                }
                Err(_) => return Err(HttpError::Garbled("an invalid chunk size line".to_owned())),
            };
            if size == 0 {
                return Ok(body);
            }
            let size = usize::try_from(size)
                .ok()
                .filter(|&size| size <= MAX_REPLY - body.len())
                .ok_or_else(too_long)?;
            let end = line + size;
            while self.buf.len() < end + 2 {
                self.more("a chunk")?;
            }
            if self.buf[end..end + 2] != *b"\r\n" {
                return Err(HttpError::Garbled(
                    "a chunk that does not end with CRLF".to_owned(),
                ));
            }
            body.extend_from_slice(&self.buf[line..end]);
            self.buf.drain(..end + 2);
        }
    }
}
