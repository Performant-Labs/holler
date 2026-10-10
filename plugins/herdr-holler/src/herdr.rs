//! The plugin's own minimal Herdr client (A-warn 1): the Herdr adapter's `Request`
//! enum is closed over the hub adapter's methods, so the plugin cannot reuse it, and
//! instead mirrors its transport discipline (`holler-adapter-herdr`'s `transport.rs`,
//! after the Herdr spike, section 3):
//!
//! - **Two methods, and no way to build a third.** [`Method`] is closed over
//!   `pane.report_metadata` and `workspace.report_metadata`; the plugin sends nothing
//!   else.
//! - **One request per connection.** Connect, write the request's one line, read to
//!   the first newline or to the end of the stream, close.
//! - **One deadline bounds the whole exchange.** The connect, the write and the read
//!   run on a short-lived worker thread that sets the socket's timeout to the time
//!   left before each read and write, and the caller waits for its answer only until
//!   the deadline, so a wedged Herdr costs one deadline.
//! - **No error carries the request's line or the reply's bytes.** Messages name the
//!   method, the socket and, for Herdr's own refusal, its error code (quoted and cut
//!   short).

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// How long one exchange may take, from the connect to the reply line.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(2);

/// The most bytes a reply line may hold; a report's reply is a few dozen.
const MAX_REPLY_BYTES: usize = 64 * 1024;

/// The most bytes one read of a reply asks for.
const CHUNK_BYTES: usize = 4 * 1024;

/// The most characters of a Herdr error code a message quotes.
const CODE_EXCERPT_CHARS: usize = 64;

/// The name of the thread that runs one exchange.
const WORKER_NAME: &str = "herdr-report";

/// The only Herdr methods the plugin can send: the two display reports.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Method {
    /// `pane.report_metadata`: one pane's tokens, labels and TTL.
    PaneReport,
    /// `workspace.report_metadata`: one workspace's tokens.
    WorkspaceReport,
}

impl Method {
    /// The method's wire name.
    fn as_str(self) -> &'static str {
        match self {
            Method::PaneReport => "pane.report_metadata",
            Method::WorkspaceReport => "workspace.report_metadata",
        }
    }
}

/// Why one report did not land.
#[derive(Debug)]
pub(crate) enum Failure {
    /// Herdr answered with an error: this report did not land, but Herdr is there,
    /// so the other reports of the refresh may.
    Refused(String),
    /// The exchange itself failed (no socket, the deadline, a garbled reply): the
    /// refresh stops, so a wedged Herdr costs one deadline, not one per report.
    Fault(String),
}

/// Herdr's session socket, one report per connection.
pub(crate) struct Client {
    socket: PathBuf,
}

impl Client {
    /// The client of the socket at `socket`.
    pub(crate) fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// Send one report and read Herdr's answer: `Ok` when Herdr answered `{"type":
    /// "ok"}` to this request's id.
    pub(crate) fn send(&self, method: Method, params: Value) -> Result<(), Failure> {
        let id = format!("holler:{}", method.as_str());
        let request = json!({"id": id, "method": method.as_str(), "params": params});
        let exchange = Exchange {
            socket: self.socket.clone(),
            method,
            deadline: Instant::now() + EXCHANGE_TIMEOUT,
        };
        let reply = exchange.on_worker(format!("{request}\n"))?;
        exchange.decode(&id, &reply)
    }
}

/// One exchange: the socket, the method (for messages) and the deadline. The worker
/// runs it; the caller keeps a copy to answer with.
#[derive(Clone)]
struct Exchange {
    socket: PathBuf,
    method: Method,
    deadline: Instant,
}

impl Exchange {
    /// Run the exchange on a worker thread and wait for its one answer until the
    /// deadline. A worker the caller stopped waiting for ends when its own socket
    /// timeout fires, and its answer is dropped.
    fn on_worker(&self, line: String) -> Result<Vec<u8>, Failure> {
        let worker = self.clone();
        let (answer, answered) = mpsc::channel();
        let _detached = thread::Builder::new()
            .name(WORKER_NAME.to_owned())
            .spawn(move || {
                // At most one answer; when the caller has stopped waiting the send
                // fails and the answer is dropped.
                let _ = answer.send(worker.run(line.as_bytes()));
            })
            .map_err(|error| {
                self.fault(&format!("could not start its thread ({})", error.kind()))
            })?;
        match answered.recv_timeout(self.remaining()) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(self.timed_out()),
            Err(RecvTimeoutError::Disconnected) => Err(self.fault("ended without an answer")),
        }
    }

    /// Connect, write `line`, and read the reply line; on the worker thread.
    fn run(&self, line: &[u8]) -> Result<Vec<u8>, Failure> {
        let mut stream = self.connect()?;
        self.write(&mut stream, line)?;
        self.read(&mut stream)
    }

    /// A connection to the socket, unless the deadline has passed.
    fn connect(&self) -> Result<UnixStream, Failure> {
        loop {
            self.left()?;
            match UnixStream::connect(&self.socket) {
                Ok(stream) => return Ok(stream),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => {
                    return Err(self.fault(&format!("could not connect ({})", error.kind())))
                }
            }
        }
    }

    /// Write all of `line`, each write bounded by the time left.
    fn write(&self, stream: &mut UnixStream, mut line: &[u8]) -> Result<(), Failure> {
        const DOING: &str = "writing the request";
        while !line.is_empty() {
            self.bound(DOING, |left| stream.set_write_timeout(left))?;
            match stream.write(line) {
                Ok(0) => return Err(self.failed(DOING, ErrorKind::WriteZero)),
                Ok(written) => line = line.get(written..).unwrap_or_default(),
                Err(error) => self.retry(DOING, error.kind())?,
            }
        }
        Ok(())
    }

    /// The reply line: the bytes before the first newline, or every byte when the
    /// stream ends first. No byte at all, or more than [`MAX_REPLY_BYTES`] before a
    /// newline, is a fault.
    fn read(&self, stream: &mut UnixStream) -> Result<Vec<u8>, Failure> {
        const DOING: &str = "reading the reply";
        let mut reply = Vec::new();
        let mut chunk = [0_u8; CHUNK_BYTES];
        loop {
            self.bound(DOING, |left| stream.set_read_timeout(left))?;
            let read = match stream.read(&mut chunk) {
                Ok(read) => read,
                Err(error) => {
                    self.retry(DOING, error.kind())?;
                    continue;
                }
            };
            let got = chunk.get(..read).unwrap_or_default();
            if got.is_empty() {
                return if reply.is_empty() {
                    Err(self.fault("closed without a reply"))
                } else {
                    Ok(reply)
                };
            }
            let line_end = got.iter().position(|byte| *byte == b'\n');
            reply.extend_from_slice(got.get(..line_end.unwrap_or(got.len())).unwrap_or_default());
            if reply.len() > MAX_REPLY_BYTES {
                return Err(
                    self.fault(&format!("sent a reply longer than {MAX_REPLY_BYTES} bytes"))
                );
            }
            if line_end.is_some() {
                return Ok(reply);
            }
        }
    }

    /// Herdr's reply to the request `id`: `Ok` for an `ok` result, `Refused` for
    /// Herdr's own error, and a fault for anything else.
    fn decode(&self, id: &str, reply: &[u8]) -> Result<(), Failure> {
        let Ok(Value::Object(reply)) = serde_json::from_slice::<Value>(reply) else {
            return Err(self.fault("sent a reply that is not a JSON object"));
        };
        if reply.get("id").and_then(Value::as_str) != Some(id) {
            return Err(self.fault("sent a reply that does not carry the request's id"));
        }
        match (reply.get("result"), reply.get("error")) {
            (Some(result), None) if result.get("type").and_then(Value::as_str) == Some("ok") => {
                Ok(())
            }
            (None, Some(error)) => Err(Failure::Refused(
                match error.get("code").and_then(Value::as_str) {
                    Some(code) => format!(
                        "Herdr refused {} with the error {}",
                        self.method.as_str(),
                        excerpt(code)
                    ),
                    None => format!(
                        "Herdr refused {} with an error that has no code",
                        self.method.as_str()
                    ),
                },
            )),
            _ => Err(self.fault("sent a reply that is neither an ok result nor an error")),
        }
    }

    /// Set a socket timeout (`set`) to the time left.
    ///
    /// macOS refuses `SO_RCVTIMEO`/`SO_SNDTIMEO` with `EINVAL` (`InvalidInput`) on a
    /// socket whose peer has already closed it (the adapter's rule, seen on CI's macOS
    /// runner). That is not the exchange failing: the read or write that follows
    /// returns at once, and its own result is the answer.
    fn bound(
        &self,
        doing: &str,
        set: impl FnOnce(Option<Duration>) -> std::io::Result<()>,
    ) -> Result<(), Failure> {
        match set(Some(self.left()?)) {
            Err(error) if error.kind() != ErrorKind::InvalidInput => {
                Err(self.failed(doing, error.kind()))
            }
            _ => Ok(()),
        }
    }

    /// What a failed read or write means: `Interrupted` is tried again (`Ok`), a
    /// socket timeout is the deadline, and anything else is a fault.
    fn retry(&self, doing: &str, kind: ErrorKind) -> Result<(), Failure> {
        match kind {
            ErrorKind::Interrupted => Ok(()),
            ErrorKind::WouldBlock | ErrorKind::TimedOut => Err(self.timed_out()),
            kind => Err(self.failed(doing, kind)),
        }
    }

    /// The time left before the deadline: zero once it has passed.
    fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    /// The time left before the next socket call, or the deadline's fault when there
    /// is none (std refuses a zero socket timeout).
    fn left(&self) -> Result<Duration, Failure> {
        let left = self.remaining();
        if left.is_zero() {
            return Err(self.timed_out());
        }
        Ok(left)
    }

    /// The deadline ran out.
    fn timed_out(&self) -> Failure {
        self.fault(&format!(
            "did not answer within {} ms",
            EXCHANGE_TIMEOUT.as_millis()
        ))
    }

    /// The socket failed after the connect, while `doing`.
    fn failed(&self, doing: &str, kind: ErrorKind) -> Failure {
        self.fault(&format!("failed while {doing} ({kind})"))
    }

    /// A fault of this exchange: what `went` wrong, naming the method and the socket.
    fn fault(&self, went: &str) -> Failure {
        Failure::Fault(format!(
            "the {} exchange on the Herdr socket {:?} {went}",
            self.method.as_str(),
            self.socket
        ))
    }
}

/// `text` quoted and escaped in Rust's `{:?}` form, cut to [`CODE_EXCERPT_CHARS`].
fn excerpt(text: &str) -> String {
    let cut: String = text.chars().take(CODE_EXCERPT_CHARS).collect();
    format!("{cut:?}")
}
