#![allow(dead_code)] // #839 (each test binary uses a different subset of these helpers)
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! A hub in process, with fake bodies (#839): what `driven_resolve_test.rs` and
//! `offline_reason_test.rs` drive the hub's control socket over.
//!
//! The control socket is the real one (`handle_control_conn` over a `UnixStream` pair) over a
//! real `Registry`, `Roster`, `Lockout`, hold registry and pane registry. A body is a live
//! registry entry (`Registry::insert`), its presence cached and advertised to the roster as the
//! circuit does on a `session/presence`, and a task that drains the entry's `LiveCommand` and
//! cancel channels and answers each as its [`Script`] says, logging what reached it. So "nothing
//! reached the body" is observable: the body's log is empty. No `holler` process, no socket to a
//! body, no sleep: every wait is bounded on a reply.
//!
//! `control/say` and `control/interrupt` resolve the hub's state dir for the TalkLog, so every test
//! binary that includes this module points `HOLLER_STATE_DIR` at one throwaway directory, once,
//! before its first connection ([`hub`] does it).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::time::Duration;

use holler_hub::control_server::handle_control_conn;
use holler_hub::holds::Holds;
use holler_hub::live::{CancelReply, LiveCommand, Registry, SayReply};
use holler_hub::lockout::Lockout;
use holler_hub::pane_dispatch::PaneDeps;
use holler_hub::panes::PaneState;
use holler_hub::profile::ProfileState;
use holler_hub::roster::{Config, Roster};
use holler_hub::state::HubState;
use holler_pane::{Pane, PaneStore as _};
use holler_proto::{Message, Mode, Part, Presence, Role, SessionAd, SessionState, WireError};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// How long a control reply may take: a bounded wait on the line, never a sleep.
pub const REPLY_WITHIN: Duration = Duration::from_secs(20);

static STATE_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
static SET_STATE_DIR: Once = Once::new();

/// The one throwaway state dir of this test binary, with `HOLLER_STATE_DIR` pointing at it.
pub fn state_dir() -> &'static std::path::Path {
    let dir = STATE_DIR.get_or_init(|| tempfile::tempdir().unwrap());
    SET_STATE_DIR.call_once(|| {
        // Edition 2021: `set_var` is safe. Set once, before any connection reads it, to the same
        // value for every test of the binary.
        std::env::set_var("HOLLER_STATE_DIR", dir.path());
    });
    dir.path()
}

/// A `ses_`-shaped id (D8): `ses_` and 26 hex digits.
pub fn id(n: u32) -> String {
    format!("ses_{n:026x}")
}

/// An idle attach-mode session `name` advertising `harness_session_id`.
pub fn attach_ad(name: &str, harness_session_id: &str) -> SessionAd {
    SessionAd {
        name: name.to_owned(),
        harness: "opencode".to_owned(),
        state: SessionState::Idle,
        mode: Mode::Attach,
        harness_session_id: Some(harness_session_id.to_owned()),
        turn_started_at: None,
        last_update_at: None,
        pending: None,
        turn_id: None,
        last_turn: None,
    }
}

/// How a fake body answers a `say`, a forwarded request and a cancel.
#[derive(Clone)]
pub struct Script {
    pub say: SayAnswer,
    pub query: QueryAnswer,
    pub cancel: CancelAnswer,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            say: SayAnswer::EndTurn,
            query: QueryAnswer::Echo,
            cancel: CancelAnswer::Applied,
        }
    }
}

#[derive(Clone)]
pub enum SayAnswer {
    /// The turn ends at once with `stop_reason`.
    Stop(&'static str),
    /// `Stop("end_turn")`.
    EndTurn,
    /// Never answers (the turn outlives every wait).
    Never,
}

#[derive(Clone)]
pub enum QueryAnswer {
    /// Answer `{session, harness_session_id}` from the params (a rebind that took), after `delay`
    /// when one is set by [`QueryAnswer::Slow`].
    Echo,
    /// [`QueryAnswer::Echo`], after `Duration`.
    Slow(Duration),
    /// Answer this wire error, verbatim.
    Refuse(WireError),
    /// Never answer.
    Never,
}

#[derive(Clone)]
pub enum CancelAnswer {
    Applied,
    Never,
}

/// What reached a fake body, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Seen {
    Say {
        session: String,
        replace: bool,
    },
    Query {
        method: String,
        params: Option<Value>,
    },
    Cancel {
        session: String,
    },
}

/// A fake body: its log, and the most forwarded requests it ever held unanswered at once.
#[derive(Clone)]
pub struct FakeBody {
    pub client_id: String,
    pub token_id: String,
    pub label: String,
    log: Arc<Mutex<Vec<Seen>>>,
    outstanding: Arc<AtomicUsize>,
    most_outstanding: Arc<AtomicUsize>,
}

impl FakeBody {
    pub fn seen(&self) -> Vec<Seen> {
        self.log.lock().unwrap().clone()
    }

    /// The most forwarded requests (`session/answer`, `query/*`, `session/rebind`) this body held
    /// unanswered at one moment.
    pub fn most_outstanding(&self) -> usize {
        self.most_outstanding.load(Ordering::SeqCst)
    }
}

/// The hub in process: its registries and the control socket's state.
pub struct TestHub {
    pub registry: Registry,
    pub roster: Arc<Roster>,
    pub lockout: Arc<Lockout>,
    pub holds: Holds,
    pub panes: Arc<PaneState>,
    deps: PaneDeps,
    _dir: tempfile::TempDir,
}

/// A hub with an empty pane registry and no holds.
pub fn hub() -> TestHub {
    state_dir();
    let dir = tempfile::tempdir().unwrap();
    let state = HubState::from_root(dir.path().to_path_buf());
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    let panes = Arc::new(PaneState::load(&state));
    let holds = Holds::in_memory();
    let registry = Registry::new()
        .with_holds(holds.clone())
        .with_panes(Arc::clone(&panes));
    let deps = PaneDeps {
        panes: Arc::clone(&panes),
        profiles: Arc::new(ProfileState::load(&state)),
    };
    let roster = Arc::new(Roster::with_system_clock(&Config::default()));
    TestHub {
        registry,
        roster,
        lockout: Lockout::new(),
        holds,
        panes,
        deps,
        _dir: dir,
    }
}

impl TestHub {
    /// Store `pane` in the hub's pane registry (a new record).
    pub fn put_pane(&self, pane: &Pane) {
        self.panes.cas_put(pane, 0).unwrap();
    }

    /// A live body `label`, token `tok_<label>`, advertising `ads`, answering as `script` says.
    pub async fn body(&self, label: &str, ads: Vec<SessionAd>, script: Script) -> FakeBody {
        let (client_id, token_id) = (format!("cli_{label}"), format!("tok_{label}"));
        let (rx, cancel_rx, _seq) = self
            .registry
            .insert(&client_id, label, &token_id, "127.0.0.1:1")
            .await;
        self.registry.update_presence(&client_id, ads.clone()).await;
        // As `circuit::handle_authenticated` does before any presence: rows are `<label>/<name>`.
        self.roster.set_token(&token_id, &client_id);
        self.roster.set_label(&token_id, label);
        self.roster.advertise(
            &token_id,
            &Presence {
                hostname: label.to_owned(),
                sessions: ads,
            },
        );
        let body = FakeBody {
            client_id,
            token_id,
            label: label.to_owned(),
            log: Arc::default(),
            outstanding: Arc::default(),
            most_outstanding: Arc::default(),
        };
        tokio::spawn(serve(rx, body.clone(), script.clone()));
        tokio::spawn(serve_cancels(cancel_rx, body.clone(), script));
        body
    }

    /// The body `label` goes offline: its entry is removed and its last presence kept, as a
    /// dropped connection leaves it.
    pub async fn disconnect(&self, body: &FakeBody) {
        self.registry.remove(&body.client_id).await;
    }

    /// A new control-socket connection to this hub.
    pub fn connect(&self) -> Conn {
        let (client, server) = UnixStream::pair().unwrap();
        let task = tokio::spawn(handle_control_conn(
            server,
            self.registry.clone(),
            Arc::clone(&self.roster),
            Arc::clone(&self.lockout),
            self.deps.clone(),
        ));
        Conn {
            client: BufReader::new(client),
            _server: task,
            next: 0,
        }
    }

    /// One control request on a fresh connection: its reply.
    pub async fn call(&self, method: &str, params: Value) -> Value {
        self.connect().call(method, params).await
    }
}

/// A control connection.
pub struct Conn {
    client: BufReader<UnixStream>,
    _server: tokio::task::JoinHandle<()>,
    next: u32,
}

impl Conn {
    /// Send one request and return its one reply line, parsed.
    pub async fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let req = json!({"jsonrpc": "2.0", "id": format!("b-test-{}", self.next), "method": method, "params": params});
        self.client
            .get_mut()
            .write_all(format!("{req}\n").as_bytes())
            .await
            .unwrap();
        let mut reply = String::new();
        tokio::time::timeout(REPLY_WITHIN, self.client.read_line(&mut reply))
            .await
            .unwrap_or_else(|_| panic!("no reply to {method} within {REPLY_WITHIN:?}"))
            .unwrap();
        serde_json::from_str(&reply)
            .unwrap_or_else(|e| panic!("{method}: not JSON ({e}): {reply:?}"))
    }
}

/// The error object of a reply that must be one.
pub fn error_of(reply: &Value) -> &Value {
    reply
        .get("error")
        .unwrap_or_else(|| panic!("an error reply: {reply}"))
}

/// `(code, data.reason)` of an error reply.
pub fn code_and_reason(reply: &Value) -> (i64, Option<String>) {
    let error = error_of(reply);
    let code = error["code"].as_i64().unwrap();
    let reason = error
        .pointer("/data/reason")
        .and_then(Value::as_str)
        .map(str::to_owned);
    (code, reason)
}

fn agent_message(text: &str) -> Message {
    Message {
        message_id: "m-1".to_owned(),
        context_id: None,
        task_id: None,
        role: Role::RoleAgent,
        parts: vec![Part::text_part(text)],
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

/// The body's command loop: answer each command as the script says, logging it first.
async fn serve(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
    body: FakeBody,
    script: Script,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            LiveCommand::Say {
                session,
                replace,
                reply,
                ..
            } => {
                body.log
                    .lock()
                    .unwrap()
                    .push(Seen::Say { session, replace });
                let stop = match &script.say {
                    SayAnswer::EndTurn => "end_turn",
                    SayAnswer::Stop(stop) => stop,
                    SayAnswer::Never => {
                        std::mem::forget(reply);
                        continue;
                    }
                };
                let message = Box::new(agent_message("ok"));
                let state = "completed".to_owned();
                let _ = reply.send(SayReply::Result {
                    message,
                    stop_reason: stop.to_owned(),
                    state,
                    updates: Vec::new(),
                });
            }
            LiveCommand::Query {
                method,
                params,
                reply,
            } => {
                body.log.lock().unwrap().push(Seen::Query {
                    method: method.clone(),
                    params: params.clone(),
                });
                let now = body.outstanding.fetch_add(1, Ordering::SeqCst) + 1;
                body.most_outstanding.fetch_max(now, Ordering::SeqCst);
                let answer = script.query.clone();
                let outstanding = Arc::clone(&body.outstanding);
                tokio::spawn(async move {
                    let result = match answer {
                        QueryAnswer::Never => {
                            std::future::pending::<()>().await;
                            return;
                        }
                        QueryAnswer::Refuse(error) => Err(error),
                        QueryAnswer::Echo => Ok(echo(&method, params.as_ref())),
                        QueryAnswer::Slow(delay) => {
                            tokio::time::sleep(delay).await;
                            Ok(echo(&method, params.as_ref()))
                        }
                    };
                    outstanding.fetch_sub(1, Ordering::SeqCst);
                    let _ = reply.send(result);
                });
            }
            LiveCommand::Ping { .. }
            | LiveCommand::Drop { .. }
            | LiveCommand::Supersede { .. }
            | LiveCommand::Revoke { .. } => {}
        }
    }
}

/// A rebind's result: `{session, harness_session_id}` as asked; an answer's `{applied: true}`;
/// any other request's `{}`.
fn echo(method: &str, params: Option<&Value>) -> Value {
    match (method, params) {
        ("session/rebind", Some(params)) => {
            json!({"session": params["session"], "harness_session_id": params["harness_session_id"]})
        }
        ("session/answer", _) => json!({"applied": true}),
        _ => json!({}),
    }
}

async fn serve_cancels(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<holler_hub::live::CancelCommand>,
    body: FakeBody,
    script: Script,
) {
    while let Some(cmd) = rx.recv().await {
        body.log.lock().unwrap().push(Seen::Cancel {
            session: cmd.session.clone(),
        });
        match script.cancel {
            CancelAnswer::Applied => {
                let _ = cmd.reply.send(CancelReply::Applied);
            }
            CancelAnswer::Never => std::mem::forget(cmd.reply),
        }
    }
}

/// A healthy pane `name` whose session of record and SHOWN are `sor`, DRIVEN `driven`.
pub fn pane(name: &str, sor: &str, driven: Option<&str>) -> Pane {
    let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
    pane.session_of_record = Some(sor.to_owned());
    pane.harness.health = holler_pane::pane::Health::Healthy;
    pane.last_observed = holler_pane::pane::LastObserved {
        shown: Some(sor.to_owned()),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}
