//! `holler_adapter_opencode` — the OpenCode adapter: it implements
//! [`holler_pane::HarnessPort`] for the OpenCode harness (epic #633) over OpenCode's HTTP
//! API, on `127.0.0.1` only, and over tmux for a pane's TUI. The facts about real OpenCode it
//! relies on come from the spike #635 (`docs/research/opencode-pane-spike.md`, OpenCode
//! 1.18.35), for the web app's catch-all page and the prompt route from `holler-body`'s
//! OpenCode driver (`http_attach_driver.rs`), and for the agent and a session's status from
//! probes of OpenCode 1.19.0 (#642 part 3).
//!
//! **Every method of the port is built** (#642): the server side (`health`,
//! `create_session`, `list_sessions` and `abort`, part 1; `serve_command` and
//! `verify_serving`, #802), the TUI side (`attach_tui`, `select_session` and
//! `shown_session`, part 2), whose TUI runs in the pane's tmux session, as the server does,
//! and part 3: the pane's OpenCode agent (`Pane.opencode_agent`, #700), which
//! `serve_command` applies and `verify_serving` checks on the running server, and the prompt
//! seam of #645, `send_prompt` and `session_activity`. Nothing wires the adapter in yet
//! (#649). #647 part 2 adds the read `agent_view` (`agents.rs`): the agents of a session's
//! recent user turns and the agents that can dispatch, from `GET /session/:id/message` and
//! `GET /agent`.
//!
//! **Blocking**, as the port is: every method is synchronous (call it from `spawn_blocking`
//! or a thread in async code) and returns within [`Timeouts::call`] (I5, 10 s by default),
//! except `compact` (#878, `compact.rs`), whose model call may take [`Timeouts::compact`].
//!
//! How a call answers:
//!
//! - Every method takes a deadline of [`Timeouts::call`] when it is entered, and every
//!   request and tmux call in it uses the smaller of its own bound and what is left. Past
//!   the deadline it answers `timeout`, whose `op` is `"harness.<method>"` (the test kit's
//!   `HarnessOp` strings).
//! - A refused connection is `unavailable` ("the harness server on port N"). A server that
//!   accepts and never answers, a frozen or wedged one that only a client timeout can see,
//!   is `timeout`.
//! - **A 200 is not enough.** OpenCode answers a route it does not serve with its web app's
//!   HTML page and a 200, so each step requires the JSON it relies on. Anything else is
//!   `unavailable`, in one line that names the route, the port and the status and quotes at
//!   most 60 bytes of the body; the reply to a prompt is quoted not at all, since it could
//!   echo the prompt. The message names an id route as `/session/:id`, so it never echoes
//!   the id.
//! - `abort`, `attach_tui`, `select_session` and `send_prompt` of an id the server does not
//!   know are `session-not-found`. Raw OpenCode acknowledges an abort of an unknown id, so
//!   each asks `GET /session/:id` first; a 404 (or a 400, for a malformed id) means no such
//!   session, and nothing is sent or changed after it. A session id enters a URL path
//!   percent-encoded, so no id can change the request line.
//! - `health` is never an error for a server: it is `false` when the server is down, frozen, or
//!   answers anything but `{"healthy": true}`.
//! - **Every call is keyed by an endpoint** (#831): the adapter reaches a server on the
//!   endpoint's port (`HarnessEndpoint::port`, the one conversion), and an endpoint that names a
//!   single process is the refusal `process-unsupported`, before anything is sent. The four
//!   process methods keep their `process-unsupported` defaults: the router of the wiring never
//!   sends them here.
//! - A tmux call that fails because the pane's tmux session, its pane or the tmux server is
//!   missing reads as no TUI. Any other failure is `unavailable` with tmux's first stderr
//!   line. A message never echoes an argument the adapter passed, a directory, an
//!   environment value or a raw title.
//!
//! What the adapter decides (ADR-0021 section 2, "`HarnessPort` as built (#642)", whose facts
//! the numbers below cite):
//!
//! - **The adapter starts no server** (fact 2, #802). `serve_command` only builds the
//!   server's argv, `<env prefix> <opencode_bin> serve --port P --hostname 127.0.0.1
//!   <serve_args>`, behind the TUI's own `env` prefix, plus one pair for a spec with an agent
//!   (below): it starts nothing, binds nothing and sends nothing. The host adapter's `run`
//!   starts that argv in the pane's tmux session, in the pane spec's directory, and its
//!   `stop_owned` stops it (TERM, a grace, then KILL), so the adapter owns no process and
//!   keeps no pid. Never adopting a running server is the
//!   launch engine's: its plan refuses a port that already answers, and it asks again just
//!   before the run. The engine also waits for the server's first healthy answer, sending
//!   nothing but health checks until then: a request sent while the server boots can hang
//!   for good (the spike's boot race).
//! - **The pane's agent is its server's `default_agent`** (fact 7, #642 part 3), set once,
//!   when the server starts, never per prompt. `opencode serve` has no agent option, so for a
//!   spec with an agent `serve_command` adds one pair to the server's `env` prefix:
//!   `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}`. OpenCode starts healthy whatever the
//!   key, so once the server answers, `verify_serving` asks it: `GET /config` must name the key
//!   as its `default_agent` (the variable reached the server), and `GET /agent` must list it as
//!   an agent OpenCode takes as a default (a `mode` other than `subagent`, not hidden). Anything
//!   else is the adapter's own refusal [`AGENT_UNUSABLE`], and the launch engine's rollback
//!   then stops the server. A spec without an agent adds nothing to the argv, and
//!   `verify_serving` answers it `Ok` with nothing sent.
//! - **A prompt is queued through the API, never typed** (I4, ADR-0021 section 8).
//!   `send_prompt` checks the session as `abort` does, then posts the text part alone to the
//!   bare `POST /session/:id/prompt_async`, `Ok` exactly on its `204`. It is pure transport:
//!   a hold, a limit or a park is the hub's. `session_activity` is `HoldingQuestion` while a
//!   question or a permission request of the session, or of a direct child (subagent) session
//!   of it, is pending (`GET /question` and `GET /permission`, read best-effort: a list that
//!   cannot be read holds nothing), else `Busy` while `GET /session/status` has an entry for
//!   it that is not `idle`, else `Idle`. Only a server whose status route answers no JSON
//!   object is read from the session's last assistant message instead.
//! - **The session of record is titled with its own id** (fact 1). `create_session` renames
//!   each new session to its id, so the TUI's title maps back to it. When the rename fails,
//!   the session is deleted again, so none is left untitled.
//! - **SHOWN is read from the TUI's terminal title, through tmux** (fact 1): the
//!   `#{pane_title}` of the pane's tmux session, addressed by its exact name as `=<name>:`,
//!   which tmux resolves to the active pane of the session's current window. That pane is the
//!   TUI's: the session's current window must be the TUI's. Since #802 every pane's session
//!   also holds its server's window, which the host's `run` creates detached; a client that
//!   switches the session to that window would move `attach_tui` and `shown_session` onto the
//!   server's pane. `shown_session` is `None` whenever it cannot tell (fact 3): the home
//!   screen, a deleted session, a title that is not a whole id, a dead pane, another program,
//!   no tmux session or server. It never guesses and never asks the server.
//! - **A TUI's port is observed, not remembered**: `select_session` reads it from the
//!   pane's `#{pane_start_command}`, so a pane with no TUI is `unavailable` before anything
//!   is sent. `attach_tui` keeps the pane when its program exits (`remain-on-exit on`) and
//!   replaces that program with the TUI. Both answer `Ok` only once the title shows the
//!   session; the TUI is never sent a keystroke.
//! - **One TUI per server is the caller's precondition** (fact 4): `select-session` reaches
//!   every TUI of a server, and the registry gives each pane its own port.
//! - **`list_sessions` lists top-level sessions only** (fact 5). A child (subagent)
//!   session, one whose `parentID` is a string, is left out, because reconcile (#647) treats
//!   every listed session except the session of record as a stray.
//! - **The lookup is wiring's job** (fact 6, see [`Resolver`]). `tui_session` must answer
//!   for a pane that is being launched, before the pane's record exists, so #649 cannot back
//!   it with a registry lookup alone. The server's directory is no lookup (#802): it comes
//!   from the pane spec, through the host's `run`.
//! - **One divergence from the test kit's `FakeHarness`** (fact 6), which the conformance
//!   suite pins neither way: a `PaneId` the `tui_session` lookup does not know is the
//!   lookup's own error (`pane-not-found`) from `select_session` and `shown_session`, where
//!   the fake answers `unavailable` and `None`.
//! - **No per-pane environment, and none of the adapter's own environment on an argv**
//!   (`ps` and tmux show it): no per-pane environment names, model or effort are applied.
//!   tmux starts both the server and the TUI, so under [`ProcessEnv::Inherit`] both see the
//!   tmux server's environment: a provider credential reaches the server only through the
//!   pane's tmux session (#649's to set, ADR-0021 section 2), never through an argv. The one
//!   variable the adapter adds is `OPENCODE_CONFIG_CONTENT`, on the server's argv only (never
//!   the TUI's) and only for a spec with an agent, carrying only `{"default_agent":KEY}`,
//!   never an agent's definition (tmux limits a command's size: 3.7c refused a 20,000-byte
//!   argument). It comes after the prefix's own words, and `env` applies its assignments in
//!   order, so it replaces a same-named [`ProcessEnv::Isolated`] pair and a value inherited
//!   from the tmux server.
//!
//! [`http`] is the adapter's own transport, and [`tui`] holds the tmux configuration and the
//! pure builders and parsers of the tmux calls. Both are public for this crate's tests and
//! are not an interface: code outside the crate reaches OpenCode only through
//! `HarnessPort`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_pane::ports::{Activity, CompactModel, CompactOutcome};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
};
use serde_json::{json, Value};

use crate::http::{HttpError, Reply};

mod agents;
mod attach;
mod compact;
pub mod http;
mod prompt;
mod server;
pub mod tui;

pub use server::AGENT_UNUSABLE;
pub use tui::{TmuxConfig, TmuxSocket};

/// The `op` of each method's `timeout`: the strings of the test kit's `HarnessOp`, which the
/// tests pin equal to these. `OP_SEND_PROMPT` and `OP_SESSION_ACTIVITY` have no `HarnessOp`
/// counterpart yet (the fake's recording of the two is #684's), so the tests pin them as
/// literals.
const OP_VERIFY_SERVING: &str = "harness.verify_serving";
const OP_CREATE_SESSION: &str = "harness.create_session";
const OP_LIST_SESSIONS: &str = "harness.list_sessions";
const OP_ABORT: &str = "harness.abort";
const OP_ATTACH_TUI: &str = "harness.attach_tui";
const OP_SELECT_SESSION: &str = "harness.select_session";
const OP_SHOWN_SESSION: &str = "harness.shown_session";
const OP_SEND_PROMPT: &str = "harness.send_prompt";
const OP_SESSION_ACTIVITY: &str = "harness.session_activity";
const OP_AGENT_VIEW: &str = "harness.agent_view";

/// How often a switch, an attach or an abort is checked, within [`Timeouts::settle`], for
/// having been seen: `abort` asks `GET /session/status` whether the session is still running
/// there (its entry is there and not `idle`), and `select_session` and `attach_tui` ask tmux
/// what the pane's TUI shows.
const SETTLE_POLL: Duration = Duration::from_millis(100);

/// How much of a reply's body a message quotes.
const EXCERPT_BYTES: usize = 60;

/// The longest bound a deadline is computed for (a year), so that no configured bound can
/// overflow an `Instant`.
const LONGEST: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// The variable that turns the TUI's title off, which the adapter reads: under
/// [`ProcessEnv::Inherit`] every argv's `env` prefix removes it.
const TITLE_OFF: &str = "OPENCODE_DISABLE_TERMINAL_TITLE";

/// Maps a key to what the adapter needs; an `Err` is returned to the caller as it is.
///
/// Precondition (ADR-0021 section 2, "`HarnessPort` as built (#642)", fact 6): a resolver
/// must answer for a pane that is being launched, before its registry record exists (#644
/// writes the record after the act). Meeting it is wiring's job (#649), not a registry
/// lookup's.
pub type Resolver<K, V> = Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

/// The environment the adapter's OpenCode processes, the server and the TUI, start with.
/// Both are started by tmux (the server by the host adapter's `run`, #802), and both argvs
/// begin with the `env` prefix it gives (`ProcessEnv::prefix`). For a spec with an agent the
/// server's prefix is the TUI's plus one pair, `OPENCODE_CONFIG_CONTENT`
/// (`ProcessEnv::prefix_with`, #642 part 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEnv {
    /// Each inherits the tmux server's environment (tmux starts it, so not the adapter's),
    /// with `OPENCODE_DISABLE_TERMINAL_TITLE` removed (`env -u`), because the adapter reads
    /// the TUI's title. A provider credential reaches the server only that way, through the
    /// pane's tmux session (#649), never through an argv.
    Inherit,
    /// Each gets exactly these variables (`env -i` semantics). Tests use it for the scratch
    /// `HOME` and `XDG_*`. It must not set `OPENCODE_DISABLE_TERMINAL_TITLE`, which turns
    /// off the title the adapter reads.
    ///
    /// **Never put a secret here.** Every pair goes on the server's and the TUI's command
    /// lines, which `ps` shows and tmux reports as `#{pane_start_command}`. No message of
    /// the adapter echoes a start command or an environment value.
    Isolated(Vec<(String, String)>),
}

impl ProcessEnv {
    /// The `env` prefix of the TUI's argv, and of the server's for a spec without an agent:
    /// [`ProcessEnv::prefix_with`] with no extra pair.
    pub(crate) fn prefix(&self) -> Vec<String> {
        self.prefix_with(&[])
    }

    /// The `env` prefix of an OpenCode argv: `env -u OPENCODE_DISABLE_TERMINAL_TITLE` under
    /// [`ProcessEnv::Inherit`], and `env -i K=V ...` with exactly the configured pairs under
    /// [`ProcessEnv::Isolated`], then each of `extra` as `K=V`. The extra pairs come after the
    /// prefix's own words, and `env` applies its assignments in order, so one of them replaces
    /// a same-named configured pair and a value inherited from the tmux server. Nothing of the
    /// adapter's own environment goes into it. The server's argv for a spec with an agent is
    /// the one caller with a pair (`OPENCODE_CONFIG_CONTENT`, #642 part 3).
    ///
    /// **Never put a secret here**, the rule of [`ProcessEnv::Isolated`]: every pair goes on
    /// an argv, which `ps` shows and tmux reports as `#{pane_start_command}`.
    pub(crate) fn prefix_with(&self, extra: &[(String, String)]) -> Vec<String> {
        let mut prefix = vec!["env".to_owned()];
        match self {
            ProcessEnv::Inherit => prefix.extend(["-u".to_owned(), TITLE_OFF.to_owned()]),
            ProcessEnv::Isolated(vars) => {
                prefix.push("-i".to_owned());
                prefix.extend(vars.iter().map(assignment));
            }
        }
        prefix.extend(extra.iter().map(assignment));
        prefix
    }
}

/// A pair as `env` takes it: `K=V`.
fn assignment((key, value): &(String, String)) -> String {
    format!("{key}={value}")
}

/// The adapter's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Every port method returns within this (I5). Default 10 s.
    pub call: Duration,
    /// One HTTP request, cut to what is left of `call`. Default 5 s.
    pub request: Duration,
    /// One health GET (`health`, which the launch engine's wait also asks). Default 2 s.
    pub health: Duration,
    /// How long a switch, an attach or an abort may take to be observed. Default 5 s.
    pub settle: Duration,
    /// `compact`'s summarize and the token reads around it, each request included (#878).
    /// Default 120 s.
    pub compact: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            call: Duration::from_secs(10),
            request: Duration::from_secs(5),
            health: Duration::from_secs(2),
            settle: Duration::from_secs(5),
            compact: Duration::from_secs(120),
        }
    }
}

/// How the adapter reaches OpenCode and tmux.
#[derive(Clone)]
pub struct OpenCodeConfig {
    /// The `opencode` binary, for the server and the TUI. Its path must be valid UTF-8,
    /// since it goes on their command lines. tmux starts both, so a bare name is looked up
    /// on the `PATH` of the tmux server; wiring (#649) passes an absolute path.
    pub opencode_bin: PathBuf,
    /// Appended after `serve --port P --hostname 127.0.0.1`; the tests pass `["--pure"]`.
    pub serve_args: Vec<String>,
    /// The environment of the server and of the TUI's `attach` (see [`ProcessEnv`]).
    pub env: ProcessEnv,
    /// The tmux server the panes' TUIs run on.
    pub tmux: TmuxConfig,
    /// The tmux session name a pane's TUI runs in, never a tmux target (#649:
    /// `Pane.host.tmux`, which equals `Pane.name`, ADR-0021 line 37). The adapter alone
    /// turns it into a target.
    pub tui_session: Resolver<PaneId, PaneName>,
    /// The bounds.
    pub timeouts: Timeouts,
}

impl OpenCodeConfig {
    /// The `opencode` binary as an argv's text; `unavailable` when it is not valid UTF-8.
    pub(crate) fn bin(&self) -> Result<&str, PaneError> {
        self.opencode_bin
            .to_str()
            .ok_or_else(|| PaneError::Unavailable {
                what: "the path of the opencode binary is not valid UTF-8".to_owned(),
            })
    }
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
        self.call_until(port, op, deadline_after(self.config.timeouts.call))
    }

    /// A call of the method `op` to the server on `port` that must end by `deadline`, the
    /// one the method took at its entry: `select_session` learns its port only from tmux,
    /// after its deadline has started.
    fn call_until(&self, port: u16, op: &'static str, deadline: Instant) -> Call {
        Call {
            port,
            op,
            deadline,
            request: self.config.timeouts.request,
        }
    }
}

impl HarnessPort for OpenCodeHarness {
    fn serve_command(
        &self,
        _name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        server::serve_command(&self.config, port, spec)
    }

    /// A spec without an agent sends nothing (#802 decision 5): the argv `serve_command` built
    /// applies none, so the server keeps its own default, and the engine's health wait has
    /// just seen the server answer. A spec with an agent is checked on the running server,
    /// `agent-unusable` when it does not run that agent (#642 part 3, `server::verify_serving`).
    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        server::verify_serving(self, at.port()?, spec)
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        let port = at.port()?;
        let timeouts = &self.config.timeouts;
        Ok(server::healthy(port, timeouts.health.min(timeouts.call)))
    }

    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        let call = self.call(at.port()?, OP_CREATE_SESSION);
        let id = create(&call)?;
        if let Err(error) = retitle(&call, &id) {
            // Best effort, within what is left of the call: no untitled session is left
            // behind as a session of record.
            let _ = call.send(DELETE, &session_path(&id), None);
            return Err(error);
        }
        Ok(id)
    }

    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        let call = self.call(at.port()?, OP_LIST_SESSIONS);
        let reply = call.send(LIST, LIST.label, None)?;
        json_of(&reply)
            .as_ref()
            .and_then(top_level_ids)
            .ok_or_else(|| call.unexpected(LIST, &reply, "a list of sessions"))
    }

    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        let call = self.call(at.port()?, OP_ABORT);
        let path = session_path(session);
        known(&call, &path, session)?;
        let reply = call.send(ABORT, &format!("{path}/abort"), None)?;
        if json_of(&reply) != Some(Value::Bool(true)) {
            return Err(call.unexpected(ABORT, &reply, "true"));
        }
        settled(&call, session, self.config.timeouts.settle)
    }

    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        attach::attach_tui(self, pane, at.port()?, session)
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        attach::select_session(self, pane, session)
    }

    /// The TUI's title is read through tmux, never the server (fact 3), so the endpoint is
    /// checked to be a server's and is not otherwise used.
    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        at.port()?;
        attach::shown_session(self, pane)
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        prompt::send_prompt(self, at.port()?, session, text)
    }

    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        prompt::session_activity(self, at.port()?, session)
    }

    fn compact(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        model: Option<&CompactModel>,
    ) -> Result<CompactOutcome, PaneError> {
        compact::compact(self, at.port()?, session, model)
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        agents::agent_view(self, at.port()?, session, max_turns)
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
const SELECT: Route = Route {
    method: "POST",
    label: "/tui/select-session",
};
const GET_CONFIG: Route = Route {
    method: "GET",
    label: "/config",
};
const GET_AGENT: Route = Route {
    method: "GET",
    label: "/agent",
};
const PROMPT_ASYNC: Route = Route {
    method: "POST",
    label: "/session/:id/prompt_async",
};
const GET_QUESTION: Route = Route {
    method: "GET",
    label: "/question",
};
const GET_PERMISSION: Route = Route {
    method: "GET",
    label: "/permission",
};
const GET_MESSAGES: Route = Route {
    method: "GET",
    label: "/session/:id/message",
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
                "{}: \"{}\"",
                self.answered(route, reply, wanted),
                excerpt(&reply.body)
            ),
        }
    }

    /// [`Call::unexpected`] quoting no part of the body: one line naming the route, the port
    /// and the status only. For the reply to a request that carries what no message may hold
    /// (a prompt's text, #642 part 3): a server that echoes the request in its error body, a
    /// validation error quoting its input say, cannot put it in a message.
    fn unexpected_status(&self, route: Route, reply: &Reply, wanted: &str) -> PaneError {
        PaneError::Unavailable {
            what: self.answered(route, reply, wanted),
        }
    }

    /// `<method> <route> on port <port> answered <status>, not <wanted>`.
    fn answered(&self, route: Route, reply: &Reply, wanted: &str) -> String {
        format!(
            "{} {} on port {} answered {}, not {wanted}",
            route.method, route.label, self.port, reply.status
        )
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
    session_reply(call, path, id).map(drop)
}

/// [`known`]'s request and rules, also returning the reply and its JSON object, the session,
/// for a step that reads more of it (`attach_tui` reads its `directory`).
fn session_reply(call: &Call, path: &str, id: &str) -> Result<(Reply, Value), PaneError> {
    let reply = call.send(GET_SESSION, path, None)?;
    if matches!(reply.status, 400 | 404) {
        return Err(PaneError::SessionNotFound {
            what: id.to_owned(),
        });
    }
    match json_of(&reply) {
        Some(session) if string_at(Some(&session), "id") == Some(id) => Ok((reply, session)),
        _ => Err(call.unexpected(GET_SESSION, &reply, "that session")),
    }
}

/// Poll `GET /session/status` until `id` no longer runs there ([`running`]: its entry is gone
/// or `idle`, so a session OpenCode is retrying is not settled yet), within `settle`: an abort
/// is trusted only once it is seen. Still running at the end is `timeout`; a reply that is not
/// a JSON object is `unavailable`.
fn settled(call: &Call, id: &str, settle: Duration) -> Result<(), PaneError> {
    poll(settle, call.deadline, call.op, |until| {
        let reply = call.send_by(STATUS, STATUS.label, None, until)?;
        let status =
            status_object(&reply).ok_or_else(|| call.unexpected(STATUS, &reply, "an object"))?;
        Ok((!running(&status, id)).then_some(()))
    })
}

/// A reply of `GET /session/status` as the server's account of its sessions: the JSON object
/// of a `200` ([`json_of`]), and `None` for anything else, an error status whose body is an
/// object included. `settled` and `session_activity` read the route only through this and
/// [`running`], so the crate answers "is this session running?" one way.
fn status_object(reply: &Reply) -> Option<Value> {
    json_of(reply).filter(Value::is_object)
}

/// Whether `status` (a [`status_object`]) says `id` is running a turn: it has an entry for
/// `id` whose `type` is not `idle`. OpenCode's statuses are `idle`, `retry` and `busy`, and
/// setting a session `idle` deletes its entry, so no entry is idle (spike line 73), while
/// `retry` (a turn waiting to ask its provider again) and an entry with no `type` run.
fn running(status: &Value, id: &str) -> bool {
    status
        .get(id)
        .is_some_and(|entry| string_at(Some(entry), "type") != Some("idle"))
}

/// The adapter's one poll, for whatever must be seen within [`Timeouts::settle`] (an
/// abort, a switch, an attach): `step` runs every [`SETTLE_POLL`] until it answers, within
/// `settle` and `deadline`, the method's own, whichever ends first. `step` gets that end, to
/// bound its own work. The poll sleeps only while a whole interval is left; when the end
/// comes first, the answer is `timeout` with `op`.
fn poll<T>(
    settle: Duration,
    deadline: Instant,
    op: &str,
    mut step: impl FnMut(Instant) -> Result<Option<T>, PaneError>,
) -> Result<T, PaneError> {
    let until = deadline_after(settle).min(deadline);
    loop {
        if let Some(seen) = step(until)? {
            return Ok(seen);
        }
        if Instant::now() + SETTLE_POLL >= until {
            return Err(PaneError::Timeout { op: op.to_owned() });
        }
        std::thread::sleep(SETTLE_POLL);
    }
}

/// The ids of a session list's top-level sessions, in the order given: an element whose
/// `parentID` is a string is a child session and is left out (ADR-0021's `HarnessPort` note,
/// fact 5). `None` unless the list is an array of objects that each have a string `id`.
fn top_level_ids(list: &Value) -> Option<Vec<String>> {
    ids_where(list, |parent| parent.is_none())
}

/// The ids of a session list's direct children of `parent`, in the order given: the elements
/// whose `parentID` is `parent`, the scope `holler-body` gives a session's held questions. A
/// grandchild is not one. `None`, as for [`top_level_ids`], unless the list is an array of
/// objects that each have a string `id`.
fn child_ids(list: &Value, parent: &str) -> Option<Vec<String>> {
    ids_where(list, |of| of == Some(parent))
}

/// The ids, in the order given, of a session list's elements whose parent passes `keep`: the
/// parent is the element's `parentID` when that is a string, else none. The crate's one reading
/// of a session's `parentID`. `None` unless the list is an array of objects that each have a
/// string `id`.
fn ids_where(list: &Value, keep: impl Fn(Option<&str>) -> bool) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    for session in list.as_array()? {
        let id = string_at(Some(session), "id")?;
        if keep(string_at(Some(session), "parentID")) {
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
