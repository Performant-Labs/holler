//! `holler_adapter_opencode` — the OpenCode adapter: it implements
//! [`holler_pane::HarnessPort`] for the OpenCode harness (epic #633) over OpenCode's HTTP
//! API, on `127.0.0.1` only. The facts about real OpenCode it relies on come from the spike
//! #635 (`docs/research/opencode-pane-spike.md`, OpenCode 1.18.35) and, for the web app's
//! catch-all page, from `holler-body`'s OpenCode driver (`http_attach_driver.rs`).
//!
//! **Part 1 of #642: the server side.** `serve`, `health`, `create_session`,
//! `list_sessions` and `abort` are built. `attach_tui`, `select_session` and
//! `shown_session` answer `not-implemented`, the skeleton's answer for a method whose story
//! has not landed, until part 2 adds the TUI, which runs in the pane's tmux session. Until
//! then [`tui`] holds only the tmux configuration. Nothing wires the adapter in yet (#649).
//!
//! **Blocking**, as the port is: every method is synchronous (call it from `spawn_blocking`
//! or a thread in async code) and returns within [`Timeouts::call`] (I5, 10 s by default).
//!
//! How a call answers:
//!
//! - Every method takes a deadline of [`Timeouts::call`] when it is entered, and every
//!   request in it uses the smaller of its own bound and what is left. Past the deadline it
//!   answers `timeout`, whose `op` is `"harness.<method>"` (the test kit's `HarnessOp`
//!   strings).
//! - A refused connection is `unavailable` ("the harness server on port N"). A server that
//!   accepts and never answers, a frozen or wedged one that only a client timeout can see,
//!   is `timeout`.
//! - **A 200 is not enough.** OpenCode answers a route it does not serve with its web app's
//!   HTML page and a 200, so each step requires the JSON it relies on. Anything else is
//!   `unavailable`, in one line that names the route, the port and the status and quotes at
//!   most 60 bytes of the body. The message names an id route as `/session/:id`, so it never
//!   echoes the id.
//! - `abort` of an id the server does not know is `session-not-found`. Raw OpenCode
//!   acknowledges an abort of an unknown id, so `abort` asks `GET /session/:id` first; a 404
//!   (or a 400, for a malformed id) means no such session, and nothing is sent after it. A
//!   session id enters a URL path percent-encoded, so no id can change the request line.
//! - `health` is never an error: it is `false` when the server is down, frozen, or answers
//!   anything but `{"healthy": true}`.
//!
//! What the adapter decides (the decisions of `docs/handoffs/642-brief.md`):
//!
//! - **`serve` never adopts a server** (decision 2). It asks `GET /global/health` first: a
//!   port that answers is `unavailable` ("port N is in use"), and a frozen server that holds
//!   it is `timeout`. Otherwise it starts `opencode serve --port P --hostname 127.0.0.1` in
//!   the pane's project directory, in a process group of its own, and sends nothing but
//!   health checks until its first healthy answer: a request sent while the server boots can
//!   hang for good (the spike's boot race). It returns the server's pid, which is also its
//!   process group id, and the server outlives the adapter. `HarnessPort` has no stop: the
//!   host adapter stops the server by that pid, its process group and its descendants
//!   (#695).
//! - **The session of record is titled with its own id** (decision 1). `create_session`
//!   renames each new session to its id, so the TUI's title (part 2) maps back to it. When
//!   the rename fails, the session is deleted again, so none is left untitled.
//! - **`list_sessions` lists top-level sessions only** (decision 13). A child (subagent)
//!   session, one whose `parentID` is a string, is left out, because reconcile (#647) treats
//!   every listed session except the session of record as a stray.
//! - **The `workdir` lookup is wiring's job** (decision 14, see [`Resolver`]). It must answer
//!   for a pane that is being launched, before the pane's record exists, so #649 cannot back
//!   it with a registry lookup alone.
//! - **One divergence from the test kit's `FakeHarness`** (decision 15(a)): `serve` on a port
//!   that already serves the same pane is `unavailable` here, where the fake returns that
//!   server's pid. The conformance suite pins neither answer.
//! - **No per-pane environment.** The adapter applies no per-pane environment names, model
//!   or effort. Under [`ProcessEnv::Inherit`] the server sees the operator's own environment
//!   (ADR-0021 section 13).
//!
//! [`http`] is the adapter's own transport. It is public for this crate's tests and is not an
//! interface: code outside the crate reaches OpenCode only through `HarnessPort`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};
use serde_json::{json, Value};

use crate::http::{HttpError, Reply};

mod exec;
pub mod http;
mod server;
pub mod tui;

pub use tui::{TmuxConfig, TmuxSocket};

/// The `op` of each method's `timeout`: the strings of the test kit's `HarnessOp`, which the
/// tests pin equal to these.
const OP_SERVE: &str = "harness.serve";
const OP_CREATE_SESSION: &str = "harness.create_session";
const OP_LIST_SESSIONS: &str = "harness.list_sessions";
const OP_ABORT: &str = "harness.abort";

/// How often `abort` asks `GET /session/status` whether the session is still busy.
const SETTLE_POLL: Duration = Duration::from_millis(100);

/// How much of a reply's body a message quotes.
const EXCERPT_BYTES: usize = 60;

/// The longest bound a deadline is computed for (a year), so that no configured bound can
/// overflow an `Instant`.
const LONGEST: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// Maps a key to what the adapter needs; an `Err` is returned to the caller as it is.
///
/// Precondition (decision 14): a resolver must answer for a pane that is being launched,
/// before its registry record exists (#644 writes the record after the act). Meeting it is
/// wiring's job (#649), not a registry lookup's.
pub type Resolver<K, V> = Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

/// The environment a child process of the adapter is started with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEnv {
    /// The child inherits the environment the adapter runs in.
    Inherit,
    /// The child gets exactly these variables (`env -i` semantics). Tests use it for the
    /// scratch `HOME` and `XDG_*`.
    ///
    /// **Never put a secret here.** In part 2 every pair goes on the TUI's command line,
    /// which `ps` shows and tmux reports as `#{pane_start_command}`. No message of the
    /// adapter echoes a start command or an environment value.
    Isolated(Vec<(String, String)>),
}

/// The adapter's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Every port method returns within this (I5). Default 10 s.
    pub call: Duration,
    /// One HTTP request, cut to what is left of `call`. Default 5 s.
    pub request: Duration,
    /// One health GET: `health`, and the check `serve` makes before it starts a server.
    /// Default 2 s.
    pub health: Duration,
    /// One health GET while `serve` waits for the server it started. Default 500 ms.
    pub boot_try: Duration,
    /// How long a switch, an attach or an abort may take to be observed. Default 5 s.
    pub settle: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            call: Duration::from_secs(10),
            request: Duration::from_secs(5),
            health: Duration::from_secs(2),
            boot_try: Duration::from_millis(500),
            settle: Duration::from_secs(5),
        }
    }
}

/// How the adapter reaches OpenCode and tmux.
#[derive(Clone)]
pub struct OpenCodeConfig {
    /// The `opencode` binary; production: `"opencode"`, found on `PATH`.
    pub opencode_bin: PathBuf,
    /// Appended after `serve --port P --hostname 127.0.0.1`; the tests pass `["--pure"]`.
    pub serve_args: Vec<String>,
    /// The environment of the server (and, in part 2, of the TUI's `attach`).
    pub env: ProcessEnv,
    /// The tmux server the panes' TUIs run on (part 2).
    pub tmux: TmuxConfig,
    /// The project directory a pane's server runs in (#649: from `Pane.host.cwd`).
    pub workdir: Resolver<PaneName, PathBuf>,
    /// The tmux session name a pane's TUI runs in, never a tmux target (#649:
    /// `Pane.host.tmux`, which equals `Pane.name`, ADR-0021 line 37). Part 2.
    pub tui_session: Resolver<PaneId, PaneName>,
    /// The bounds.
    pub timeouts: Timeouts,
}

/// The `HarnessPort` over OpenCode.
pub struct OpenCodeHarness {
    config: OpenCodeConfig,
}

impl OpenCodeHarness {
    /// An adapter with `config`. It starts nothing until a method is called.
    pub fn new(config: OpenCodeConfig) -> Self {
        Self { config }
    }

    /// A call of the method `op` to the server on `port`, its deadline starting now.
    fn call(&self, port: u16, op: &'static str) -> Call {
        let timeouts = &self.config.timeouts;
        Call {
            port,
            op,
            deadline: deadline_after(timeouts.call),
            request: timeouts.request,
        }
    }
}

impl HarnessPort for OpenCodeHarness {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        server::serve(&self.config, name, port)
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let timeouts = &self.config.timeouts;
        Ok(server::healthy(port, timeouts.health.min(timeouts.call)))
    }

    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        let call = self.call(port, OP_CREATE_SESSION);
        let id = create(&call)?;
        if let Err(error) = retitle(&call, &id) {
            // Best effort, within what is left of the call: no untitled session is left
            // behind as a session of record.
            let _ = call.send(DELETE, &session_path(&id), None);
            return Err(error);
        }
        Ok(id)
    }

    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        let call = self.call(port, OP_LIST_SESSIONS);
        let reply = call.send(LIST, LIST.label, None)?;
        json_of(&reply)
            .as_ref()
            .and_then(top_level_ids)
            .ok_or_else(|| call.unexpected(LIST, &reply, "a list of sessions"))
    }

    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        let call = self.call(port, OP_ABORT);
        let path = session_path(session);
        known(&call, &path, session)?;
        let reply = call.send(ABORT, &format!("{path}/abort"), None)?;
        if json_of(&reply) != Some(Value::Bool(true)) {
            return Err(call.unexpected(ABORT, &reply, "true"));
        }
        settled(&call, session, self.config.timeouts.settle)
    }

    fn attach_tui(&self, _pane: &PaneId, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }

    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }

    fn shown_session(&self, _pane: &PaneId) -> Result<Option<String>, PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }
}

/// A route of OpenCode's API as a message names it: an id stands as `:id`.
#[derive(Clone, Copy)]
struct Route {
    method: &'static str,
    label: &'static str,
}

const CREATE: Route = Route {
    method: "POST",
    label: "/session",
};
const RETITLE: Route = Route {
    method: "PATCH",
    label: "/session/:id",
};
const DELETE: Route = Route {
    method: "DELETE",
    label: "/session/:id",
};
const LIST: Route = Route {
    method: "GET",
    label: "/session",
};
const GET_SESSION: Route = Route {
    method: "GET",
    label: "/session/:id",
};
const ABORT: Route = Route {
    method: "POST",
    label: "/session/:id/abort",
};
const STATUS: Route = Route {
    method: "GET",
    label: "/session/status",
};

/// One port call in flight: the server's port, the method's `op` and its deadline.
struct Call {
    port: u16,
    op: &'static str,
    /// [`Timeouts::call`] from the method's entry (I5).
    deadline: Instant,
    /// [`Timeouts::request`]: one request's own bound.
    request: Duration,
}

impl Call {
    /// One request of this call, within the smaller of `request` and what is left of the
    /// call.
    fn send(&self, route: Route, path: &str, body: Option<&Value>) -> Result<Reply, PaneError> {
        self.send_by(route, path, body, self.deadline)
    }

    /// [`Call::send`], also ended by `until` (a poll's own end inside the call's bound).
    /// Refused is `unavailable`; no reply in time is `timeout` with the call's `op`; a
    /// reply that is not HTTP is `unavailable` naming the route.
    fn send_by(
        &self,
        route: Route,
        path: &str,
        body: Option<&Value>,
        until: Instant,
    ) -> Result<Reply, PaneError> {
        let within = budget(self.request, until.min(self.deadline));
        match http::request(self.port, route.method, path, body, within) {
            Ok(reply) => Ok(reply),
            Err(HttpError::Refused) => Err(PaneError::Unavailable {
                what: format!("the harness server on port {}", self.port),
            }),
            Err(HttpError::TimedOut) => Err(self.timeout()),
            Err(HttpError::Garbled(why)) => Err(PaneError::Unavailable {
                what: format!(
                    "{} {} on port {}: {}",
                    route.method,
                    route.label,
                    self.port,
                    one_line(&why)
                ),
            }),
        }
    }

    fn timeout(&self) -> PaneError {
        PaneError::Timeout {
            op: self.op.to_owned(),
        }
    }

    /// The `unavailable` of a reply that is not the `wanted` answer of `route`: one line
    /// naming the route, the port and the status, quoting at most 60 bytes of the body.
    fn unexpected(&self, route: Route, reply: &Reply, wanted: &str) -> PaneError {
        PaneError::Unavailable {
            what: format!(
                "{} {} on port {} answered {}, not {wanted}: \"{}\"",
                route.method,
                route.label,
                self.port,
                reply.status,
                excerpt(&reply.body)
            ),
        }
    }
}

/// `POST /session` with `{}`: the new session's id, which must start with `ses`.
fn create(call: &Call) -> Result<String, PaneError> {
    let reply = call.send(CREATE, CREATE.label, Some(&json!({})))?;
    match string_at(json_of(&reply).as_ref(), "id") {
        Some(id) if id.starts_with("ses") => Ok(id.to_owned()),
        _ => Err(call.unexpected(CREATE, &reply, "a new session")),
    }
}

/// `PATCH /session/<id>` with `{"title": "<id>"}`: the session is titled with its own id
/// (unique, at most 40 characters, and it maps back to the id: spike 149-151), and the reply
/// must carry that title.
fn retitle(call: &Call, id: &str) -> Result<(), PaneError> {
    let reply = call.send(RETITLE, &session_path(id), Some(&json!({ "title": id })))?;
    if string_at(json_of(&reply).as_ref(), "title") == Some(id) {
        Ok(())
    } else {
        Err(call.unexpected(RETITLE, &reply, "the session titled with its id"))
    }
}

/// The existence check `GET /session/<id>`: `Ok` when the server holds that session. A 404,
/// or a 400 (a malformed id), is `session-not-found`. Any other answer, a 200 that is not
/// that session included, is `unavailable`.
fn known(call: &Call, path: &str, id: &str) -> Result<(), PaneError> {
    let reply = call.send(GET_SESSION, path, None)?;
    if matches!(reply.status, 400 | 404) {
        return Err(PaneError::SessionNotFound {
            what: id.to_owned(),
        });
    }
    if string_at(json_of(&reply).as_ref(), "id") == Some(id) {
        Ok(())
    } else {
        Err(call.unexpected(GET_SESSION, &reply, "that session"))
    }
}

/// Poll `GET /session/status` until `id` is absent from it or not `busy`, within `settle`:
/// an abort is trusted only once it is seen. Still busy at the end is `timeout`.
fn settled(call: &Call, id: &str, settle: Duration) -> Result<(), PaneError> {
    let until = deadline_after(settle).min(call.deadline);
    loop {
        let reply = call.send_by(STATUS, STATUS.label, None, until)?;
        let status = json_of(&reply)
            .filter(Value::is_object)
            .ok_or_else(|| call.unexpected(STATUS, &reply, "an object"))?;
        if string_at(status.get(id), "type") != Some("busy") {
            return Ok(());
        }
        if Instant::now() + SETTLE_POLL >= until {
            return Err(call.timeout());
        }
        std::thread::sleep(SETTLE_POLL);
    }
}

/// The ids of a session list's top-level sessions, in the order given: an element whose
/// `parentID` is a string is a child session and is left out (decision 13). `None` unless
/// the list is an array of objects that each have a string `id`.
fn top_level_ids(list: &Value) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    for session in list.as_array()? {
        let id = string_at(Some(session), "id")?;
        if !session.get("parentID").is_some_and(Value::is_string) {
            ids.push(id.to_owned());
        }
    }
    Some(ids)
}

/// `/session/<id>` with the id percent-encoded: every byte outside `[A-Za-z0-9_-]` becomes
/// `%XX`, so no id can change the request line.
fn session_path(id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut path = String::from("/session/");
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
            path.push(char::from(byte));
        } else {
            path.push('%');
            path.push(char::from(HEX[usize::from(byte >> 4)]));
            path.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    path
}

/// The JSON of a `200` reply; `None` for another status or a body that is not JSON.
fn json_of(reply: &Reply) -> Option<Value> {
    if reply.status == 200 {
        serde_json::from_slice(&reply.body).ok()
    } else {
        None
    }
}

/// The string `key` of a JSON object.
fn string_at<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a str> {
    value?.get(key)?.as_str()
}

/// At most 60 bytes of `body` as text on one line (control characters become spaces),
/// ending in `...` when it was cut.
fn excerpt(body: &[u8]) -> String {
    let head = body.get(..EXCERPT_BYTES).unwrap_or(body);
    let mut text = one_line(&String::from_utf8_lossy(head));
    // A cut multi-byte character decodes to U+FFFD, which is longer than what it replaced.
    while text.len() > EXCERPT_BYTES {
        text.pop();
    }
    if head.len() < body.len() {
        text.push_str("...");
    }
    text
}

/// `text` on one line: every control character becomes a space (ADR-0021 section 9).
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `now + after`, with `after` capped at a year.
fn deadline_after(after: Duration) -> Instant {
    Instant::now() + after.min(LONGEST)
}

/// `own`, cut to what is left before `deadline`.
fn budget(own: Duration, deadline: Instant) -> Duration {
    own.min(deadline.saturating_duration_since(Instant::now()))
}
