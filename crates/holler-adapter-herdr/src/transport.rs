//! How the adapter reaches Herdr: one request out, one reply line back, by a deadline
//! (the Herdr spike, `docs/research/herdr-api-spike.md`, section 3: one request per
//! connection on a local Unix socket).
//!
//! - **One request per connection.** [`UnixSocketTransport`] connects for every
//!   exchange, writes the request's line, reads to the first newline or to the end of
//!   the stream, and closes. It never half-closes its side: the spike's client did not,
//!   and whether Herdr answers after a half-close is unverified. Bytes after the first
//!   newline are ignored, and a reply that ends without one is returned as it came.
//! - **One deadline bounds the whole exchange.** The connect, the write and the read
//!   run on a short-lived worker thread, which sets the socket's timeout to the time
//!   left before each read and write. The caller waits for the worker's one answer only
//!   until the deadline, so a call returns by then even when the connect blocks (the
//!   full listen backlog of a wedged server, which std cannot bound) or the server drips
//!   its reply a byte at a time. A worker the caller stopped waiting for ends by itself
//!   when its socket timeout fires (stuck in a connect, when the server accepts or
//!   dies), and its answer is dropped.
//! - **Every fault is `timeout` or `unavailable`.** The deadline running out, on either
//!   side, is `timeout`, its `op` naming the method (`herdr.ping`). A socket that
//!   cannot be connected to, a failed write or read, and a reply that is empty, longer
//!   than [`MAX_REPLY_BYTES`] or not UTF-8 are `unavailable`, naming the socket. No
//!   message carries the request's line or the reply's bytes, so text typed into a pane
//!   never reaches an error.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::PaneError;

use crate::protocol::Request;

/// The most bytes a reply line may hold; a longer one is `unavailable`.
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

/// The most bytes read of one reply: one past the longest line, so a reply that has no
/// newline by then is too long.
const READ_LIMIT: usize = MAX_REPLY_BYTES + 1;

/// The most bytes one read of a reply asks for.
const CHUNK_BYTES: usize = 64 * 1024;

/// The name of the thread that runs one exchange.
const WORKER_NAME: &str = "herdr-exchange";

/// How the adapter reaches Herdr: one request out, one reply line back, by a deadline.
///
/// [`UnixSocketTransport`] is the one production implementation, and
/// `HerdrAdapter::connect` the one production way to build an adapter on it: the
/// adapter has no remote path. Any other implementation is a test seam, a simulated
/// Herdr handed to `HerdrAdapter::connect_with`.
pub trait Transport: Send + Sync {
    /// Send `request.to_line()` and return Herdr's reply line without its newline, or
    /// `timeout` (`op` = `herdr.<method>`) once `deadline` passes, or `unavailable`.
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError>;
}

impl<T: Transport + ?Sized> Transport for Arc<T> {
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError> {
        (**self).exchange(request, deadline)
    }
}

/// Herdr's local Unix socket: a new connection per request (spike section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixSocketTransport {
    socket: PathBuf,
}

impl UnixSocketTransport {
    /// The transport to the socket at `socket`.
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    /// The socket's path.
    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Transport for UnixSocketTransport {
    /// The exchange runs on a worker thread, and the caller waits for its answer until
    /// `deadline`. A deadline already passed is `timeout`, and no connection is made.
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError> {
        let exchange = Exchange {
            socket: self.socket.clone(),
            method: request.method(),
            deadline,
        };
        exchange.left()?;
        let line = request.to_line();
        let worker = exchange.clone();
        let (answer, answered) = mpsc::channel();
        let _detached = thread::Builder::new()
            .name(WORKER_NAME.to_owned())
            .spawn(move || {
                // At most one answer. When the caller has stopped waiting, the send
                // fails and the answer is dropped: no panic, no retry.
                let _ = answer.send(worker.run(line.as_bytes()));
            })
            .map_err(|error| {
                exchange.unavailable(&format!("could not start its thread ({})", error.kind()))
            })?;
        match answered.recv_timeout(exchange.remaining()) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(exchange.timeout()),
            Err(RecvTimeoutError::Disconnected) => {
                Err(exchange.unavailable("ended without an answer"))
            }
        }
    }
}

/// One exchange: the socket, the method (for messages and the `timeout`'s `op`) and
/// the deadline. The worker runs it; the caller keeps a copy to answer with.
#[derive(Clone)]
struct Exchange {
    socket: PathBuf,
    method: &'static str,
    deadline: Instant,
}

impl Exchange {
    /// Connect, write `line`, and read the reply line; on the worker thread.
    fn run(&self, line: &[u8]) -> Result<String, PaneError> {
        let mut stream = self.connect()?;
        self.write(&mut stream, line)?;
        let reply = self.read(&mut stream)?;
        String::from_utf8(reply).map_err(|_| self.bad_reply("is not UTF-8"))
    }

    /// A connection to the socket, unless the deadline has passed.
    fn connect(&self) -> Result<UnixStream, PaneError> {
        loop {
            self.left()?;
            match UnixStream::connect(&self.socket) {
                Ok(stream) => return Ok(stream),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => {
                    return Err(PaneError::Unavailable {
                        what: format!(
                            "the Herdr socket {:?} cannot be connected to ({})",
                            self.socket,
                            error.kind()
                        ),
                    })
                }
            }
        }
    }

    /// Write all of `line`, each write bounded by the time left.
    fn write(&self, stream: &mut UnixStream, mut line: &[u8]) -> Result<(), PaneError> {
        const DOING: &str = "writing the request";
        while !line.is_empty() {
            stream
                .set_write_timeout(Some(self.left()?))
                .map_err(|error| self.failed(DOING, error.kind()))?;
            match stream.write(line) {
                Ok(0) => return Err(self.failed(DOING, ErrorKind::WriteZero)),
                Ok(written) => line = line.get(written..).unwrap_or_default(),
                Err(error) => self.retry(DOING, error.kind())?,
            }
        }
        Ok(())
    }

    /// The reply line: the bytes before the first newline, or every byte when the
    /// stream ends first. No byte at all, or more than [`MAX_REPLY_BYTES`] with no
    /// newline, is `unavailable`; reading stops at the limit.
    fn read(&self, stream: &mut UnixStream) -> Result<Vec<u8>, PaneError> {
        let mut reply = Vec::new();
        let mut chunk = [0_u8; CHUNK_BYTES];
        loop {
            let room = READ_LIMIT.saturating_sub(reply.len()).min(CHUNK_BYTES);
            if room == 0 {
                return Err(self.bad_reply(&format!(
                    "passed {MAX_REPLY_BYTES} bytes before its newline"
                )));
            }
            let read = self.read_some(stream, chunk.get_mut(..room).unwrap_or_default())?;
            let got = chunk.get(..read).unwrap_or_default();
            if got.is_empty() {
                return if reply.is_empty() {
                    Err(PaneError::Unavailable {
                        what: format!(
                            "Herdr closed the socket {:?} without replying to {}",
                            self.socket, self.method
                        ),
                    })
                } else {
                    Ok(reply)
                };
            }
            if let Some(end) = got.iter().position(|byte| *byte == b'\n') {
                reply.extend_from_slice(got.get(..end).unwrap_or_default());
                return Ok(reply);
            }
            reply.extend_from_slice(got);
        }
    }

    /// One read into `into`, bounded by the time left: the bytes read, 0 at the end of
    /// the stream.
    fn read_some(&self, stream: &mut UnixStream, into: &mut [u8]) -> Result<usize, PaneError> {
        const DOING: &str = "reading the reply";
        loop {
            stream
                .set_read_timeout(Some(self.left()?))
                .map_err(|error| self.failed(DOING, error.kind()))?;
            match stream.read(into) {
                Ok(read) => return Ok(read),
                Err(error) => self.retry(DOING, error.kind())?,
            }
        }
    }

    /// What a failed read or write means: `Interrupted` is tried again (`Ok`), a socket
    /// timeout is `timeout`, and anything else is `unavailable`.
    fn retry(&self, doing: &str, kind: ErrorKind) -> Result<(), PaneError> {
        match kind {
            ErrorKind::Interrupted => Ok(()),
            ErrorKind::WouldBlock | ErrorKind::TimedOut => Err(self.timeout()),
            kind => Err(self.failed(doing, kind)),
        }
    }

    /// The time left before the deadline: zero once it has passed.
    fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    /// The time left before the next socket call; `timeout` when there is none, and the
    /// call is not made (std refuses a zero socket timeout).
    fn left(&self) -> Result<Duration, PaneError> {
        let left = self.remaining();
        if left.is_zero() {
            return Err(self.timeout());
        }
        Ok(left)
    }

    /// `timeout`, its `op` naming the method.
    fn timeout(&self) -> PaneError {
        PaneError::Timeout {
            op: format!("herdr.{}", self.method),
        }
    }

    /// `unavailable`: the socket failed after it was connected to, while `doing`.
    fn failed(&self, doing: &str, kind: ErrorKind) -> PaneError {
        PaneError::Unavailable {
            what: format!(
                "the Herdr socket {:?} failed while {doing} of the {} exchange ({kind})",
                self.socket, self.method
            ),
        }
    }

    /// `unavailable`: Herdr's reply, which `is` says what is wrong with.
    fn bad_reply(&self, is: &str) -> PaneError {
        PaneError::Unavailable {
            what: format!(
                "Herdr's reply to {} on the socket {:?} {is}",
                self.method, self.socket
            ),
        }
    }

    /// `unavailable`: the exchange itself, which `went` says what happened to.
    fn unavailable(&self, went: &str) -> PaneError {
        PaneError::Unavailable {
            what: format!(
                "the {} exchange on the Herdr socket {:?} {went}",
                self.method, self.socket
            ),
        }
    }
}
