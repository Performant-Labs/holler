#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #839
#![allow(dead_code)] // #839
//! DRIVEN end to end (#839): a real hub, a real body in attach mode against the fake OpenCode
//! server (`holler-body`'s `fake_server.rs`, `#[path]`-included as `attach_mode_test.rs` does),
//! and pane records written through the wiring's own client. Every id is `ses_`-shaped (D8).
//!
//! - Acceptance 3: `say --pane` reaches the pane's real session of record, and `interrupt --pane`
//!   and `answer --pane` resolve the same id; a pane no body drives is `pane-not-driven`.
//! - Acceptance 18: a typed bare id is not a SESSION; a turn that outlives its wait is not
//!   `pane-not-driven`; `reset --first`'s `Send` is accepted and returns while the turn runs on.
//! - Acceptance 19: the `HubPort` conformance suite against the real client over this hub, and
//!   the hub's gate refusing a body on a pane's recorded DRIVEN.
//! - Acceptance 12, 13 and 21 against the real circuit: the hold key across a rebind, the closed
//!   generic forward, and one forwarded request at a time on one connection.
//!
//! Each test serialises on [`SERIAL`] (each spawns a hub and a body). The in-process clients
//! read `HOLLER_STATE_DIR` on every call, so they run inside [`with_state_dir`].

#[path = "../../holler-body/tests/http_attach_driver_test/fake_server.rs"]
mod fake_server;
mod support;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Output;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use fake_server::{message_updated_assistant, part_updated_text, session_idle, FakeServer};
use holler_cli::pane::wiring::hub::HubClient;
use holler_cli::pane::wiring::store::HubPaneStore;
use holler_pane::hub::{HubPort, PromptMode};
use holler_pane::pane::{Health, LastObserved};
use holler_pane::{Pane, PaneName, PaneStore};
use holler_pane_testkit::conformance::hub::{run_hub_conformance, HubFixture};
use holler_pane_testkit::fixture::{sample_pane, session_id};
use serde_json::{json, Value};

use support::{holler_cmd, mint_token, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static STATE_DIR: Mutex<()> = Mutex::new(());

/// Run `f` with `HOLLER_STATE_DIR` set to `dir` (the in-process clients read it on every call).
fn with_state_dir<R>(dir: &Path, f: impl FnOnce() -> R) -> R {
    let _held = STATE_DIR.lock().unwrap_or_else(PoisonError::into_inner);
    // Edition 2021: `set_var` is safe; the lock and `SERIAL` keep it to one test at a time.
    std::env::set_var("HOLLER_STATE_DIR", dir);
    f()
}

/// A hub, one body `kiwi` with an attach session per `(name, id)` on one fake OpenCode server.
struct Rig {
    state: StateDir,
    hub: Option<Hub>,
    body: Option<Body>,
    server: FakeServer,
    token: String,
}

impl Rig {
    async fn start(sessions: &[(&str, &str)], hub_args: &[&str]) -> Rig {
        let server = FakeServer::start().await;
        let state = StateDir::new();
        let hub = Hub::start_with_args(&state, &[], hub_args);
        let (token, secret) = mint_token(&state, "kiwi");
        support::join(&state, &state, &hub.ws_url(), &token, &secret);
        let blocks: String = sessions
            .iter()
            .map(|(name, id)| {
                format!(
                    "[[session]]\nname = \"{name}\"\nharness = \"opencode\"\nmode = \"attach\"\nendpoint = \"{}\"\nsession_id = \"{id}\"\n\n",
                    server.endpoint()
                )
            })
            .collect();
        std::fs::create_dir_all(state.body()).unwrap();
        let config = state.body().join("sessions.toml");
        std::fs::write(&config, blocks).unwrap();
        let body = Body::start(&state, &config);
        let rig = Rig {
            state,
            hub: Some(hub),
            body: Some(body),
            server,
            token,
        };
        for (name, id) in sessions {
            rig.wait_row(name, |row| row["harness_session_id"] == *id);
        }
        rig
    }

    fn root(&self) -> &Path {
        self.state.path()
    }

    /// One control request, straight to the hub's control socket: its reply.
    fn control(&self, method: &str, params: Value) -> Value {
        let state = holler_hub::state::HubState::from_root(self.root().to_path_buf());
        let mut stream = UnixStream::connect(holler_hub::state::control_sock_path(&state)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let req = json!({"jsonrpc": "2.0", "id": "b-e2e", "method": method, "params": params});
        stream.write_all(format!("{req}\n").as_bytes()).unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{method}: {e}: {line:?}"))
    }

    /// The roster row of `kiwi/<session>` once it satisfies `want`.
    fn wait_row(&self, session: &str, want: impl Fn(&Value) -> bool) -> Value {
        let name = format!("kiwi/{session}");
        wait_for(STARTUP_WAIT, || {
            let roster = self.control("control/roster", json!({"all": false}));
            roster["result"]["rows"]
                .as_array()?
                .iter()
                .find(|r| r["name"] == name.as_str() && want(r))
                .cloned()
        })
        .unwrap_or_else(|| {
            panic!(
                "no matching row {name}: {}",
                self.control("control/roster", json!({}))
            )
        })
    }

    /// Store `pane` (a new record) through the wiring's own pane client.
    fn seed(&self, pane: &Pane) {
        with_state_dir(self.root(), || {
            HubPaneStore::new().cas_put(pane, 0).unwrap()
        });
    }

    /// Rewrite the stored record of `name` with `change`.
    fn rewrite(&self, name: &str, change: impl FnOnce(&mut Pane)) {
        with_state_dir(self.root(), || {
            let store = HubPaneStore::new();
            let mut pane = store.get(&PaneName::parse(name).unwrap()).unwrap().unwrap();
            let generation = pane.generation;
            change(&mut pane);
            store.cas_put(&pane, generation).unwrap();
        });
    }

    fn holler(&self, args: &[&str]) -> Output {
        holler_cmd(&self.state).args(args).output().unwrap()
    }

    /// Run `args` while a helper thread ends the turn that reaches `id`'s `prompt_async`.
    fn holler_answered(&self, args: &[&str], id: &str) -> Output {
        let since = self.server.requests().len();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let path = format!("/session/{id}/prompt_async");
                let seen = wait_for(Duration::from_secs(10), || {
                    self.server
                        .requests()
                        .iter()
                        .skip(since)
                        .any(|r| r.path == path)
                        .then_some(())
                });
                if seen.is_some() {
                    self.server.push_event(message_updated_assistant("m1", id));
                    self.server
                        .push_event(part_updated_text("m1", id, "hello from the pane"));
                    self.server.push_event(session_idle(id));
                }
            });
            self.holler(args)
        })
    }

    fn prompts_to(&self, id: &str) -> usize {
        let path = format!("/session/{id}/prompt_async");
        self.server
            .requests()
            .iter()
            .filter(|r| r.path == path)
            .count()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(body) = self.body.take() {
            body.stop(&self.state, Duration::from_secs(5));
        }
        if let Some(hub) = self.hub.take() {
            hub.stop(Duration::from_secs(5));
        }
    }
}

/// A healthy pane `name` whose session of record and SHOWN are `sor` and DRIVEN `driven`.
fn pane(name: &str, sor: &str, driven: Option<&str>) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    pane.session_of_record = Some(sor.to_owned());
    pane.harness.health = Health::Healthy;
    pane.last_observed = LastObserved {
        shown: Some(sor.to_owned()),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

const P: &str = "demo-c1r1";
const Q: &str = "demo-c2r1";

// --- Acceptance 3 -------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn say_pane_reaches_the_pane_s_real_session_of_record() {
    let _guard = SERIAL.lock().await;
    let id = session_id(1);
    let rig = Rig::start(&[("oc", &id)], &[]).await;
    rig.seed(&pane(P, &id, Some(&id)));
    rig.seed(&pane(Q, &session_id(2), None));

    let said = rig.holler_answered(&["say", "--pane", P, "hi"], &id);
    assert!(said.status.success(), "say --pane: {}", stderr(&said));
    assert!(String::from_utf8_lossy(&said.stdout).contains("hello from the pane"));
    assert_eq!(
        rig.prompts_to(&id),
        1,
        "delivered to the session of record's prompt_async"
    );

    let lone = rig.holler(&["say", "--pane", Q, "hi"]);
    assert_eq!(lone.status.code(), Some(3), "{}", stderr(&lone));
    assert!(
        stderr(&lone).contains("pane-not-driven"),
        "{}",
        stderr(&lone)
    );

    let answered = rig.holler(&["answer", "--pane", P, "0"]);
    let text = stderr(&answered);
    assert!(
        !text.contains("pane-not-driven") && !text.contains("unknown session"),
        "the id resolved: {text}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupt_pane_resolves_the_same_id() {
    let _guard = SERIAL.lock().await;
    let id = session_id(3);
    let rig = Rig::start(&[("oc", &id)], &[]).await;
    rig.seed(&pane(P, &id, Some(&id)));
    let root = rig.root().to_path_buf();
    let long = std::thread::spawn(move || {
        let mut say = std::process::Command::new(support::holler_bin());
        say.env("HOLLER_STATE_DIR", root)
            .env("HOLLER_DEBUG", "quiet");
        say.args(["say", "--pane", P, "a long thing"])
            .output()
            .unwrap()
    });
    rig.wait_row("oc", |row| row["state"] == "working");
    let since = rig.server.requests().len();
    let out = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_for(Duration::from_secs(10), || {
                rig.server
                    .requests()
                    .iter()
                    .skip(since)
                    .any(|r| r.path.contains(&id) && r.method == "POST")
                    .then_some(())
            });
            rig.server.push_event(session_idle(&id));
        });
        rig.holler(&["interrupt", "--pane", P])
    });
    assert!(out.status.success(), "interrupt --pane: {}", stderr(&out));
    let _ = long.join();
}

// --- Acceptance 18 -------------------------------------------------------------------------

/// A typed bare id is refused before any hub is contacted (there is none here): `unknown
/// session`, exit 1, never `no live holler hub reachable`.
#[test]
fn a_typed_bare_id_is_unknown_with_no_hub() {
    let state = StateDir::new();
    let id = session_id(4);
    for args in [
        vec!["say", id.as_str(), "hi"],
        vec!["interrupt", id.as_str()],
        vec!["answer", id.as_str(), "0"],
    ] {
        let out = holler_cmd(&state).args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{args:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains(&format!("unknown session: {id}")),
            "{args:?}: {}",
            stderr(&out)
        );
        assert!(
            !stderr(&out).contains("no live holler hub"),
            "{args:?}: no hub was contacted"
        );
    }
}

/// (i) A `say --pane` whose turn outlives `--timeout` is exit 1 with the hub's text, never
/// `pane-not-driven`; (ii) an `interrupt --pane` whose cancel the body never confirms is exit 1
/// "interrupt sent but not confirmed", as today.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_turn_that_outlives_its_wait_is_not_pane_not_driven() {
    let _guard = SERIAL.lock().await;
    let id = session_id(5);
    let rig = Rig::start(&[("oc", &id)], &[]).await;
    rig.seed(&pane(P, &id, Some(&id)));
    let timed_out = rig.holler(&["say", "--pane", P, "--timeout", "1s", "never ends"]);
    assert_eq!(timed_out.status.code(), Some(1), "{}", stderr(&timed_out));
    assert!(
        stderr(&timed_out).contains("no reply from") && stderr(&timed_out).contains("within 1s"),
        "{}",
        stderr(&timed_out)
    );
    assert!(!stderr(&timed_out).contains("pane-not-driven"));

    rig.wait_row("oc", |row| row["state"] == "working");
    let unconfirmed = rig.holler(&["interrupt", "--pane", P]);
    assert_eq!(
        unconfirmed.status.code(),
        Some(1),
        "{}",
        stderr(&unconfirmed)
    );
    assert!(
        stderr(&unconfirmed).contains("interrupt sent but not confirmed within"),
        "{}",
        stderr(&unconfirmed)
    );
    assert!(!stderr(&unconfirmed).contains("pane-not-driven"));
    rig.server.push_event(session_idle(&id));
}

/// (iii) `reset --first`'s `Send` through the real client: accepted and returned within the
/// accept wait while the turn runs on, never reported as a failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_send_returns_once_accepted_while_the_turn_runs_on() {
    let _guard = SERIAL.lock().await;
    let id = session_id(6);
    let rig = Rig::start(&[("oc", &id)], &[]).await;
    rig.seed(&pane(P, &id, Some(&id)));
    let started = Instant::now();
    let sent = with_state_dir(rig.root(), || {
        HubClient::new().prompt(&id, "first message", PromptMode::Send)
    });
    assert_eq!(sent, Ok(()), "accepted is success");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    assert!(
        wait_for(Duration::from_secs(10), || (rig.prompts_to(&id) == 1)
            .then_some(()))
        .is_some()
    );
    rig.server.push_event(session_idle(&id));
}

// --- Acceptance 19 -------------------------------------------------------------------------

/// The `HubPort` suite against the real client over a real hub and body: the same cases
/// `FakeHub` passes (the recorded DRIVEN key, fail closed across panes, the first bad pane
/// named).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_real_client_passes_the_hub_conformance_suite() {
    let _guard = SERIAL.lock().await;
    let ids = [session_id(7), session_id(8)];
    let rig = Rig::start(&[("oa", &ids[0]), ("ob", &ids[1])], &[]).await;
    let result = with_state_dir(rig.root(), || {
        run_hub_conformance(|| {
            let store = HubPaneStore::new();
            for pane in store.list().unwrap() {
                store.delete(&pane.name, pane.generation).unwrap();
            }
            let panes: Arc<dyn PaneStore> = Arc::new(store);
            (
                HubClient::new(),
                HubFixture {
                    panes,
                    ids: ids.clone(),
                },
                (),
            )
        })
    });
    assert_eq!(result, Ok(()));
}

/// The gate sees a body that is behind, on the real circuit: a body on `old`, a pane whose record
/// and SHOWN are `new`. With no DRIVEN recorded a bare `say` is delivered (the stated window);
/// once `old` is the pane's recorded DRIVEN (what `pane doctor` writes) it is refused
/// `pane-shown-driven-mismatch` and reaches nothing; after the body is re-pointed and DRIVEN is
/// `new` (what `pane switch` does), it is delivered to `new`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_gate_refuses_a_body_on_a_pane_s_recorded_driven() {
    let _guard = SERIAL.lock().await;
    let (old, new) = (session_id(9), session_id(10));
    let rig = Rig::start(&[("oc", &old)], &[]).await;
    rig.seed(&pane(P, &new, None));
    let window = rig.holler_answered(&["say", "kiwi/oc", "hi"], &old);
    assert!(window.status.success(), "the window: {}", stderr(&window));

    rig.rewrite(P, |p| p.last_observed.driven = Some(old.clone()));
    let before = rig.prompts_to(&old);
    let refused = rig.holler(&["say", "kiwi/oc", "hi"]);
    assert_eq!(refused.status.code(), Some(3), "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("pane-shown-driven-mismatch"),
        "{}",
        stderr(&refused)
    );
    assert_eq!(rig.prompts_to(&old), before, "nothing reached the body");

    let rebound = rig.control(
        "control/rebind",
        json!({"token_id": rig.token, "session": "oc", "harness_session_id": new}),
    );
    assert_eq!(
        rebound["result"]["harness_session_id"],
        new.as_str(),
        "{rebound}"
    );
    rig.rewrite(P, |p| p.last_observed.driven = Some(new.clone()));
    let delivered = rig.holler_answered(&["say", "kiwi/oc", "hi"], &new);
    assert!(delivered.status.success(), "{}", stderr(&delivered));
}

// --- Acceptance 12, 13 and 21 on the real circuit --------------------------------------------

/// D4: a hub started with `--join-held` holds the body's session on join; the hold key is the
/// session's name, so it holds across a rebind (which itself sets no hold), the hub's check
/// refuses `session-held` until `release`, and then the first message is sent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through() {
    let _guard = SERIAL.lock().await;
    let (old, new) = (session_id(11), session_id(12));
    let rig = Rig::start(&[("oc", &old)], &["--join-held", "kiwi/*"]).await;
    rig.seed(&pane(P, &old, Some(&old)));
    let client = || HubClient::new();
    let held = with_state_dir(rig.root(), || {
        client().prompt(&old, "first", PromptMode::Check)
    });
    assert_eq!(
        held.map_err(|e| e.code().to_string()),
        Err("session-held".to_owned())
    );
    let body = with_state_dir(rig.root(), || client().advertised().unwrap()).remove(0);
    let acknowledged = with_state_dir(rig.root(), || client().rebind(&body, &new));
    assert_eq!(
        acknowledged,
        Ok(new.clone()),
        "a hold does not stop a rebind"
    );
    rig.rewrite(P, |p| {
        p.session_of_record = Some(new.clone());
        p.last_observed = LastObserved {
            shown: Some(new.clone()),
            driven: Some(new.clone()),
            at: 2,
        };
    });
    let still = with_state_dir(rig.root(), || {
        client().prompt(&new, "first", PromptMode::Check)
    });
    assert_eq!(
        still.map_err(|e| e.code().to_string()),
        Err("session-held".to_owned()),
        "same key"
    );
    assert!(rig.holler(&["release", "kiwi/oc"]).status.success());
    let sent = with_state_dir(rig.root(), || {
        client().prompt(&new, "first", PromptMode::Send)
    });
    assert_eq!(sent, Ok(()));
}

/// P1: `control/query_remote` carrying `session/rebind` is refused `-32601`, and the body's
/// binding is unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_generic_forward_cannot_carry_a_rebind() {
    let _guard = SERIAL.lock().await;
    let (old, new) = (session_id(13), session_id(14));
    let rig = Rig::start(&[("oc", &old)], &[]).await;
    let params = json!({"session": "oc", "harness_session_id": new});
    let reply = rig.control(
        "control/query_remote",
        json!({"target": "kiwi", "method": "session/rebind", "params": params}),
    );
    assert_eq!(
        reply.pointer("/error/code").and_then(Value::as_i64),
        Some(-32601),
        "{reply}"
    );
    assert_eq!(
        rig.wait_row("oc", |_| true)["harness_session_id"],
        old.as_str()
    );
}

/// One forwarded request at a time per connection, on the real circuit: a rebind and several
/// `hub query` requests at once each get their own reply, and the rebind applies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rebind_and_queries_at_once_each_get_their_own_reply() {
    let _guard = SERIAL.lock().await;
    let (old, new) = (session_id(15), session_id(16));
    let rig = Rig::start(&[("oc", &old)], &[]).await;
    let query = json!({"target": "kiwi", "method": "query/status"});
    let replies: Vec<Value> = std::thread::scope(|scope| {
        let mut threads = vec![scope.spawn(|| {
            rig.control(
                "control/rebind",
                json!({"token_id": rig.token, "session": "oc", "harness_session_id": new}),
            )
        })];
        for _ in 0..3 {
            threads.push(scope.spawn(|| rig.control("control/query_remote", query.clone())));
        }
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    for reply in &replies {
        assert!(
            reply.get("result").is_some(),
            "its own reply, never not connected: {reply}"
        );
    }
    assert_eq!(replies[0]["result"]["harness_session_id"], new.as_str());
    rig.wait_row("oc", |row| row["harness_session_id"] == new.as_str());
}

/// The bounds nest strictly, so each layer sees the answer of the one below it: the body's task
/// deadline (5 s) and its reply wait (6 s), the hub's rebind forward (8 s), the CLI's call (10 s).
#[test]
fn the_rebind_bounds_nest() {
    use holler_body::prompt_dispatch::{REBIND_DEADLINE, REBIND_REPLY_WAIT};
    use holler_hub::control_rebind::REBIND_FORWARD;
    let cli = holler_hub::control::ControlCall::rebind("tok_kiwi", "oc", &session_id(17)).timeout;
    assert!(
        REBIND_DEADLINE < REBIND_REPLY_WAIT
            && REBIND_REPLY_WAIT < REBIND_FORWARD
            && REBIND_FORWARD < cli
    );
    assert_eq!(
        [REBIND_DEADLINE, REBIND_REPLY_WAIT, REBIND_FORWARD, cli].map(|d| d.as_secs()),
        [5, 6, 8, 10]
    );
}
