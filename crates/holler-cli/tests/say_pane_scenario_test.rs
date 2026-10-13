#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #850
//! #831's `say` step as a default-run test (#850, Acceptance 5; D10): `holler say --pane NAME TEXT`
//! reaches an OpenCode pane and a Claude Code pane both, through the hub's prompt path with its
//! holds and its pane gate in force. It replaces `say_delivers_a_message_to_each_pane_of_the_scenario`
//! of `pane_claude/mixed.rs`, an ignored test that named #839 and #850.
//!
//! One hub and two bodies. The OpenCode body's row attaches to a fake OpenCode server
//! (`holler-body`'s `fake_server.rs`, `#[path]`-included as `attach_mode_test.rs` does). The Claude
//! body's row attaches to the program that stands for `claude` and the listener that stands for its
//! inbox (`claude_rig`). Two pane records are written through the wiring's own client, each
//! `session_of_record` and DRIVEN the id its body advertises: the OpenCode pane's is `ses_`-shaped
//! and the Claude pane's is a UUID, so the file rests on #839's join and `first_refusal`.
//!
//! Each test serialises on [`SERIAL`] (each spawns a hub and two bodies). The in-process clients
//! read `HOLLER_STATE_DIR` on every call, so they run inside [`with_state_dir`].

#[path = "../../holler-body/tests/http_attach_driver_test/fake_server.rs"]
mod fake_server;
#[path = "claude_rig/mod.rs"]
mod claude_rig;
mod support;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Output, Stdio};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use claude_rig::{claude_row, frame_of, parsed, FakeProgram, Process, PID, SESSION};
use fake_server::{message_updated_assistant, part_updated_text, session_idle, FakeServer};
use holler_cli::pane::wiring::store::HubPaneStore;
use holler_pane::pane::{HarnessKind, Health, Hold, LastObserved};
use holler_pane::{Pane, PaneName, PaneStore};
use holler_pane_testkit::fixture::{sample_pane, session_id};
use serde_json::{json, Value};
use support::{holler_cmd, mint_token, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static STATE_DIR: Mutex<()> = Mutex::new(());

/// The two panes, the two bodies' labels and their rows.
const OC_PANE: &str = "demo-c1r1";
const CL_PANE: &str = "demo-c1r2";
const OC_ROW: &str = "kiwi/oc";
const CL_ROW: &str = "plum/cl";

fn with_state_dir<R>(dir: &Path, f: impl FnOnce() -> R) -> R {
    let _held = STATE_DIR.lock().unwrap_or_else(PoisonError::into_inner);
    // Edition 2021: `set_var` is safe; the lock and `SERIAL` keep it to one test at a time.
    std::env::set_var("HOLLER_STATE_DIR", dir);
    f()
}

struct Rig {
    hub_state: StateDir,
    hub: Option<Hub>,
    oc_state: StateDir,
    oc_body: Option<Body>,
    cl_state: StateDir,
    cl_body: Option<Body>,
    server: FakeServer,
    program: FakeProgram,
    oc_id: String,
}

impl Rig {
    async fn start() -> Rig {
        let server = FakeServer::start().await;
        let hub_state = StateDir::new();
        let hub = Hub::start_with_args(&hub_state, &[], &["--listen", "127.0.0.1:0"]);
        let oc_id = session_id(1);

        let oc_state = StateDir::new();
        let (token, secret) = mint_token(&hub_state, "kiwi");
        support::join(&oc_state, &hub_state, &hub.ws_url(), &token, &secret);
        std::fs::create_dir_all(oc_state.body()).unwrap();
        let oc_config = oc_state.body().join("sessions.toml");
        let oc_row = format!(
            "[[session]]\nname = \"oc\"\nharness = \"opencode\"\nmode = \"attach\"\nendpoint = \"{}\"\nsession_id = \"{oc_id}\"\n",
            server.endpoint()
        );
        std::fs::write(&oc_config, oc_row).unwrap();
        let oc_body = Body::start(&oc_state, &oc_config);

        let cl_state = StateDir::new();
        let program = FakeProgram::new(&cl_state);
        program.set_processes(&[Process::idle(PID, SESSION)]);
        program.register(PID);
        let (token, secret) = mint_token(&hub_state, "plum");
        support::join(&cl_state, &hub_state, &hub.ws_url(), &token, &secret);
        std::fs::create_dir_all(cl_state.body()).unwrap();
        let cl_config = cl_state.body().join("sessions.toml");
        std::fs::write(&cl_config, claude_row("cl", SESSION)).unwrap();
        let env = program.env();
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let cl_body = Body::start_with_env(&cl_state, &cl_config, &env);

        let rig = Rig {
            hub_state,
            hub: Some(hub),
            oc_state,
            oc_body: Some(oc_body),
            cl_state,
            cl_body: Some(cl_body),
            server,
            program,
            oc_id,
        };
        rig.wait_row(OC_ROW, |row| row["harness_session_id"] == rig.oc_id.as_str());
        rig.wait_row(CL_ROW, |row| row["harness_session_id"] == SESSION);
        rig.seed(&pane(OC_PANE, HarnessKind::Opencode, &rig.oc_id));
        rig.seed(&pane(CL_PANE, HarnessKind::Claude, SESSION));
        rig
    }

    fn root(&self) -> &Path {
        self.hub_state.path()
    }

    /// One control request, straight to the hub's control socket: its reply.
    fn control(&self, method: &str, params: Value) -> Value {
        let state = holler_hub::state::HubState::from_root(self.root().to_path_buf());
        let mut stream = UnixStream::connect(holler_hub::state::control_sock_path(&state)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        let req = json!({"jsonrpc": "2.0", "id": "b-850", "method": method, "params": params});
        stream.write_all(format!("{req}\n").as_bytes()).unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{method}: {e}: {line:?}"))
    }

    /// The roster row `name` (`label/session`) once `want` holds.
    fn wait_row(&self, name: &str, want: impl Fn(&Value) -> bool) -> Value {
        wait_for(STARTUP_WAIT, || {
            let roster = self.control("control/roster", json!({"all": false}));
            roster["result"]["rows"].as_array()?.iter().find(|r| r["name"] == name && want(r)).cloned()
        })
        .unwrap_or_else(|| {
            panic!(
                "no matching row {name}: {}\noc body log:\n{}\nclaude body log:\n{}",
                self.control("control/roster", json!({})),
                self.oc_body.as_ref().map(Body::log_text).unwrap_or_default(),
                self.cl_body.as_ref().map(Body::log_text).unwrap_or_default()
            )
        })
    }

    fn seed(&self, pane: &Pane) {
        with_state_dir(self.root(), || HubPaneStore::new().cas_put(pane, 0).unwrap());
    }

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
        holler_cmd(&self.hub_state).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).output().unwrap()
    }

    fn oc_prompts(&self) -> usize {
        let path = format!("/session/{}/prompt_async", self.oc_id);
        self.server.requests().iter().filter(|r| r.path == path).count()
    }

    /// Run `args` (a `say` to the OpenCode pane or row) while a helper ends the turn that reaches
    /// the fake server's `prompt_async`.
    fn holler_oc(&self, args: &[&str]) -> Output {
        let (since, id) = (self.server.requests().len(), self.oc_id.clone());
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let path = format!("/session/{id}/prompt_async");
                let seen = wait_for(Duration::from_secs(10), || {
                    self.server.requests().iter().skip(since).any(|r| r.path == path).then_some(())
                });
                if seen.is_some() {
                    self.server.push_event(message_updated_assistant("m1", &id));
                    self.server.push_event(part_updated_text("m1", &id, "hello from the pane"));
                    self.server.push_event(session_idle(&id));
                }
            });
            self.holler(args)
        })
    }

    /// Run `args` (a `say` to the Claude pane or row) while a helper plays the process: it waits
    /// for one more inbox line than there was, shows the process busy until the body has listed it
    /// twice, then idle.
    fn holler_claude(&self, args: &[&str]) -> Output {
        let before = self.program.frames().len();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                self.program.wait_frames(before + 1, Duration::from_secs(20));
                self.program.set_processes(&[Process::busy(PID, SESSION)]);
                assert!(self.program.until_listed(2, Duration::from_secs(20)), "the body never polled the busy process");
                self.program.set_processes(&[Process::idle(PID, SESSION)]);
            });
            self.holler(args)
        })
    }

    fn stop_oc(&mut self) {
        if let Some(body) = self.oc_body.take() {
            body.stop(&self.oc_state, Duration::from_secs(5));
        }
    }

    fn stop_claude(&mut self) {
        if let Some(body) = self.cl_body.take() {
            body.stop(&self.cl_state, Duration::from_secs(5));
        }
    }

    /// Wait until the hub marks the id of `row`'s body offline (a check-only `say` is refused with
    /// the offline reason), delivering nothing.
    fn wait_offline(&self, id: &str) {
        let marked = wait_for(STARTUP_WAIT, || {
            let reply = self.control("control/say", json!({"session": id, "text": "x", "check_only": true}));
            (reply.pointer("/error/data/reason") == Some(&json!("offline"))).then_some(())
        });
        assert!(marked.is_some(), "the hub never marked {id} offline");
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.stop_oc();
        self.stop_claude();
        if let Some(hub) = self.hub.take() {
            hub.stop(Duration::from_secs(5));
        }
    }
}

/// A healthy pane `name` of `kind` whose session of record, SHOWN and DRIVEN are `id`.
fn pane(name: &str, kind: HarnessKind, id: &str) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    if kind == HarnessKind::Claude {
        pane.harness.kind = HarnessKind::Claude;
        pane.harness.port = None;
        pane.harness.pid = Some(PID);
    }
    pane.session_of_record = Some(id.to_owned());
    pane.harness.health = Health::Healthy;
    pane.last_observed = LastObserved { shown: Some(id.to_owned()), driven: Some(id.to_owned()), at: 1 };
    pane
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn say_pane_is_delivered_to_each_pane_of_the_scenario() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start().await;

    let to_oc = rig.holler_oc(&["say", "--pane", OC_PANE, "hi opencode"]);
    assert!(to_oc.status.success(), "say --pane {OC_PANE}: {}", stderr(&to_oc));
    assert!(String::from_utf8_lossy(&to_oc.stdout).contains("hello from the pane"));
    assert_eq!(rig.oc_prompts(), 1, "the fake server saw prompt_async");

    let to_claude = rig.holler_claude(&["say", "--pane", CL_PANE, "hi claude"]);
    assert!(to_claude.status.success(), "say --pane {CL_PANE}: {}", stderr(&to_claude));
    assert_eq!(parsed(&rig.program.frames()), vec![frame_of("hi claude")], "the listener read one frame");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_hold_on_a_row_refuses_say_pane_and_writes_nothing_for_both_harnesses() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start().await;
    for (row, pane) in [(OC_ROW, OC_PANE), (CL_ROW, CL_PANE)] {
        let held = rig.holler(&["hold", row]);
        assert!(held.status.success(), "hold {row}: {}", stderr(&held));
        let out = rig.holler(&["say", "--pane", pane, "should not arrive"]);
        assert_eq!(out.status.code(), Some(4), "{pane}: {}", stderr(&out));
        assert!(stderr(&out).contains("session_held"), "{pane}: {}", stderr(&out));
        assert!(
            !stderr(&out).contains("pane-not-driven"),
            "a hold is not 'not driven': {}",
            stderr(&out)
        );
    }
    assert_eq!(rig.oc_prompts(), 0, "nothing reached the OpenCode session");
    assert!(rig.program.frames().is_empty(), "nothing reached the Claude inbox");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_row_s_body_stops_say_pane_is_pane_not_driven_and_writes_nothing_for_both_harnesses() {
    let _guard = SERIAL.lock().await;
    let mut rig = Rig::start().await;

    rig.stop_oc();
    rig.wait_offline(&rig.oc_id.clone());
    let out = rig.holler(&["say", "--pane", OC_PANE, "hi"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("pane-not-driven"), "{}", stderr(&out));
    assert_eq!(rig.oc_prompts(), 0);

    rig.stop_claude();
    rig.wait_offline(SESSION);
    let out = rig.holler(&["say", "--pane", CL_PANE, "hi"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("pane-not-driven"), "{}", stderr(&out));
    assert!(rig.program.frames().is_empty());
}

/// A parked pane's record refuses a bare `say <label>/<name>` at the hub's gate (`pane-parked`) and
/// writes nothing, for both harnesses; after the park is lifted the same `say` is delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_parked_pane_refuses_a_bare_say_at_the_hub_s_gate_and_delivers_once_the_park_is_lifted() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start().await;
    let parked = || Hold::Parked { reason: "disk full".to_owned(), release_when: "later".to_owned(), since: 5 };
    rig.rewrite(OC_PANE, |p| p.hold = parked());
    rig.rewrite(CL_PANE, |p| p.hold = parked());

    let out = rig.holler(&["say", OC_ROW, "hi"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("pane-parked"), "{}", stderr(&out));
    let out = rig.holler(&["say", CL_ROW, "hi"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("pane-parked"), "{}", stderr(&out));
    assert_eq!(rig.oc_prompts(), 0, "nothing reached the OpenCode session");
    assert!(rig.program.frames().is_empty(), "nothing reached the Claude inbox");

    rig.rewrite(OC_PANE, |p| p.hold = Hold::None);
    rig.rewrite(CL_PANE, |p| p.hold = Hold::None);
    let out = rig.holler_oc(&["say", OC_ROW, "hi"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(rig.oc_prompts(), 1);
    let out = rig.holler_claude(&["say", CL_ROW, "hi"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(parsed(&rig.program.frames()), vec![frame_of("hi")]);
}
