//! D2, D8 and A's W-4, W-5 and W-12: the wiring's own port types, each built from injected
//! parts over the test kit's fakes, so the default run tests them with no Herdr, tmux or
//! OpenCode.
//!
//! - [`LazyHerdr`]: built on its first use, the result kept (D2).
//! - [`WiredHarness`]: forwards every method of `HarnessPort`, the two default-armed ones
//!   included, or answers its misconfiguration from each (W-4).
//! - [`BindingHerdr`] and [`BindingHost`]: the decorators that bind the pane id `ensure_pane`
//!   returned to the name the next `ensure_session` names (D8 (a)). They forward faithfully:
//!   each passes its port's conformance suite over the fake (W-12).
//! - [`tui_session`]: the resolver, answering from the bindings, then the registry by the
//!   join key (the configured Herdr session and the pane id, W-5), then the pane's label in
//!   Herdr's snapshot, else `pane-not-found` (D8).
//! - The pin of D8's coupling to the engine's call order (W-12): `pane launch` over the
//!   decorated fakes resolves `attach_tui`'s pane from the bindings, before the record
//!   exists.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use holler_cli::output::Format;
use holler_cli::pane::wiring::bind::{
    tui_session, BindingHerdr, BindingHost, Bindings, LazyHerdr, WiredHarness, WiredHost,
};
use holler_pane::ports::{Activity, LoginStatus, SessionChoice};
use holler_pane::{
    Actor, Argv, GridPos, HarnessEndpoint, HarnessPort, HerdrPane, HerdrPort, HerdrSpec, HostPort, Pane,
    PaneError, PaneId, PaneName, PaneStore, Ports, ServeSpec,
};
use holler_pane_testkit::conformance::host::run_host_conformance;
use holler_pane_testkit::fixture::sample_pane;
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::FakePaneStore;
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;

use crate::verb_harness::linked_host::{FakeHarnessLink, LinkedHost, OnStart, OnStop};
use crate::verb_harness::run_verb_with;
use crate::wrappers::{herdr_suite_over, unbuildable_lazy_herdr, CallLog};

/// The Herdr session the fakes serve and the wiring is configured with.
const SESSION: &str = "scratch";
const WORKSPACE: &str = "main";

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn herdr_fake() -> Arc<FakeHerdr> {
    Arc::new(FakeHerdr::new(SESSION).with_workspace(WORKSPACE, 3, 3).unwrap())
}

fn spec(row: u16, col: u16) -> HerdrSpec {
    HerdrSpec {
        session: SESSION.to_owned(),
        workspace: WORKSPACE.to_owned(),
        grid: GridPos { row, col },
    }
}

/// A record of `pane_name` placed on the Herdr pane `pane_id` of `session`.
fn record(pane_name: &str, session: &str, pane_id: &PaneId) -> Pane {
    let sample = sample_pane(pane_name).unwrap();
    Pane {
        herdr: HerdrPane {
            session: session.to_owned(),
            workspace: WORKSPACE.to_owned(),
            pane_id: pane_id.clone(),
            grid: GridPos { row: 1, col: 1 },
            label: None,
        },
        ..sample
    }
}

/// The wiring's parts over fresh fakes: the bindings, the two decorators, the registry and
/// the resolver over them.
struct Bound {
    fake: Arc<FakeHerdr>,
    herdr: BindingHerdr,
    host: BindingHost,
    panes: Arc<FakePaneStore>,
    resolve: holler_adapter_opencode::Resolver<PaneId, PaneName>,
}

impl Bound {
    fn new() -> Self {
        let fake = herdr_fake();
        let bindings = Bindings::new();
        let inner: Arc<dyn HerdrPort> = fake.clone();
        let panes = Arc::new(FakePaneStore::new());
        let store: Arc<dyn PaneStore> = panes.clone();
        Self {
            herdr: BindingHerdr::new(Arc::clone(&inner), bindings.clone()),
            host: BindingHost::new(Arc::new(FakeHost::new()), bindings.clone()),
            resolve: tui_session(SESSION, bindings, store, inner),
            fake,
            panes,
        }
    }

    /// `ensure_pane` at `(row, col)` then `ensure_session(pane_name)`: the engine's order.
    fn launch(&self, row: u16, col: u16, pane_name: &str) -> PaneId {
        let pane = self.herdr.ensure_pane(&spec(row, col)).unwrap();
        self.host.ensure_session(&name(pane_name), "/tmp").unwrap();
        pane.pane_id
    }

    fn answer(&self, pane: &PaneId) -> Result<PaneName, PaneError> {
        (self.resolve)(pane)
    }
}

// --- the lazy Herdr (D2) -------------------------------------------------------------------

#[test]
fn the_lazy_herdr_is_built_on_its_first_use_and_kept() {
    let built = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&built);
    let lazy = LazyHerdr::new(move || {
        count.fetch_add(1, Ordering::SeqCst);
        let port: Arc<dyn HerdrPort> = herdr_fake();
        Ok(port)
    });
    assert_eq!(built.load(Ordering::SeqCst), 0, "building the wiring touches no Herdr");
    lazy.version().unwrap();
    lazy.snapshot().unwrap();
    assert_eq!(built.load(Ordering::SeqCst), 1, "built once, on the first use");
}

#[test]
fn a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method() {
    let (lazy, built) = unbuildable_lazy_herdr();
    let pane = PaneId::new("w1:p1");
    let codes = [
        lazy.ensure_pane(&spec(1, 1)).err(),
        lazy.send_text(&pane, "x").err(),
        lazy.send_keys(&pane, &[]).err(),
        lazy.read(&pane, 1).err(),
        lazy.close(&pane).err(),
        lazy.snapshot().err(),
        lazy.version().err(),
        lazy.observe_pane(&pane).err(),
    ];
    for error in codes {
        let error = error.expect("every method fails");
        assert_eq!(error.code(), "usage", "{error}");
        assert!(error.to_string().contains("HOLLER_HERDR_SESSION"), "{error}");
    }
    assert_eq!(built.load(Ordering::SeqCst), 1, "the failed build is kept, not retried");
}

// --- the wired harness (W-4) --------------------------------------------------------------

/// A harness that records which method was called and answers each distinctively: the
/// default-armed methods answer what their defaults never would (#645's two; #831's four
/// process methods, whose defaults refuse `process-unsupported`).
#[derive(Default)]
struct Recording {
    calls: CallLog,
}

impl HarnessPort for Recording {
    fn serve_command(&self, _: &PaneName, _: u16, _: &ServeSpec) -> Result<Argv, PaneError> {
        self.calls.note("serve_command");
        Ok(Argv::new(vec!["serve".to_owned()]))
    }
    fn verify_serving(&self, _: &HarnessEndpoint, _: &ServeSpec) -> Result<(), PaneError> {
        self.calls.note("verify_serving");
        Ok(())
    }
    fn health(&self, _: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.calls.note("health");
        Ok(true)
    }
    fn create_session(&self, _: &HarnessEndpoint) -> Result<String, PaneError> {
        self.calls.note("create_session");
        Ok("ses_1".to_owned())
    }
    fn list_sessions(&self, _: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.calls.note("list_sessions");
        Ok(Vec::new())
    }
    fn abort(&self, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        self.calls.note("abort");
        Ok(())
    }
    fn attach_tui(&self, _: &PaneId, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        self.calls.note("attach_tui");
        Ok(())
    }
    fn select_session(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
        self.calls.note("select_session");
        Ok(())
    }
    fn shown_session(&self, _: &PaneId, _: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
        self.calls.note("shown_session");
        Ok(None)
    }
    fn send_prompt(&self, _: &HarnessEndpoint, _: &str, _: &str) -> Result<(), PaneError> {
        self.calls.note("send_prompt");
        Ok(())
    }
    fn session_activity(&self, _: &HarnessEndpoint, _: &str) -> Result<Activity, PaneError> {
        self.calls.note("session_activity");
        Ok(Activity::Busy)
    }
    fn process_command(
        &self,
        _: &PaneName,
        _: &ServeSpec,
        _: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        self.calls.note("process_command");
        Ok(Argv::new(vec!["claude".to_owned()]))
    }
    fn new_session_id(&self) -> Result<String, PaneError> {
        self.calls.note("new_session_id");
        Ok("s-1".to_owned())
    }
    fn locate_process(&self, _: &PaneName, _: &str) -> Result<Option<u32>, PaneError> {
        self.calls.note("locate_process");
        Ok(Some(4242))
    }
    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        self.calls.note("login_status");
        Ok(LoginStatus::Missing)
    }
}

/// Call every method of `harness` once; each answer, as `Ok` or the error's code.
fn call_every_method(harness: &dyn HarnessPort) -> Vec<Result<String, String>> {
    let (pane, port, spec) = (
        PaneId::new("w1:p1"),
        HarnessEndpoint::Port(48100),
        ServeSpec::default(),
    );
    let choice = SessionChoice::Fresh("s-1".to_owned());
    let code = |e: PaneError| e.code().to_owned();
    vec![
        harness
            .serve_command(&name("demo-c1r1"), 48100, &spec)
            .map(|_| "serve_command".into())
            .map_err(code),
        harness
            .verify_serving(&port, &spec)
            .map(|()| "verify_serving".into())
            .map_err(code),
        harness.health(&port).map(|_| "health".into()).map_err(code),
        harness
            .create_session(&port)
            .map(|_| "create_session".into())
            .map_err(code),
        harness
            .list_sessions(&port)
            .map(|_| "list_sessions".into())
            .map_err(code),
        harness
            .abort(&port, "ses_1")
            .map(|()| "abort".into())
            .map_err(code),
        harness
            .attach_tui(&pane, &port, "ses_1")
            .map(|()| "attach_tui".into())
            .map_err(code),
        harness
            .select_session(&pane, "ses_1")
            .map(|()| "select_session".into())
            .map_err(code),
        harness
            .shown_session(&pane, &port)
            .map(|_| "shown_session".into())
            .map_err(code),
        harness
            .send_prompt(&port, "ses_1", "hello")
            .map(|()| "send_prompt".into())
            .map_err(code),
        harness
            .session_activity(&port, "ses_1")
            .map(|a| format!("{a:?}"))
            .map_err(code),
        harness
            .process_command(&name("demo-c1r1"), &spec, &choice)
            .map(|_| "process_command".into())
            .map_err(code),
        harness
            .new_session_id()
            .map(|_| "new_session_id".into())
            .map_err(code),
        harness
            .locate_process(&name("demo-c1r1"), "s-1")
            .map(|p| format!("{p:?}"))
            .map_err(code),
        harness
            .login_status()
            .map(|s| format!("{s:?}"))
            .map_err(code),
    ]
}

#[test]
fn the_wired_harness_forwards_every_method_the_default_armed_ones_included() {
    let inner = Arc::new(Recording::default());
    let port: Arc<dyn HarnessPort> = inner.clone();
    let wired = WiredHarness::new(Ok(port));
    let answers = call_every_method(&wired);
    assert_eq!(
        answers[10],
        Ok("Busy".to_owned()),
        "session_activity is the inner one's, not the default Idle"
    );
    assert_eq!(
        answers[9],
        Ok("send_prompt".to_owned()),
        "send_prompt is the inner one's, not the default prompt-unsupported"
    );
    assert_eq!(
        answers[11..],
        [
            Ok("process_command".to_owned()),
            Ok("new_session_id".to_owned()),
            Ok("Some(4242)".to_owned()),
            Ok("Missing".to_owned()),
        ],
        "#831's four process methods are the inner one's, not the process-unsupported defaults"
    );
    assert_eq!(
        inner.calls.list(),
        vec![
            "serve_command",
            "verify_serving",
            "health",
            "create_session",
            "list_sessions",
            "abort",
            "attach_tui",
            "select_session",
            "shown_session",
            "send_prompt",
            "session_activity",
            "process_command",
            "new_session_id",
            "locate_process",
            "login_status",
        ],
        "each method reached the inner harness once"
    );
}

#[test]
fn a_misconfigured_wired_harness_answers_its_error_from_every_method() {
    let wired = WiredHarness::new(Err(PaneError::Unavailable {
        what: "no opencode binary: set HOLLER_OPENCODE_BIN".to_owned(),
    }));
    for answer in call_every_method(&wired) {
        assert_eq!(answer, Err("unavailable".to_owned()));
    }
}

// --- the decorators forward faithfully (W-12) ---------------------------------------------

#[test]
fn the_binding_herdr_passes_the_herdr_conformance_suite() {
    let result = herdr_suite_over(|inner| BindingHerdr::new(inner, Bindings::new()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_binding_host_passes_the_host_conformance_suite() {
    let result = run_host_conformance(|| {
        (BindingHost::new(Arc::new(FakeHost::new()), Bindings::new()), ())
    });
    assert_eq!(result, Ok(()));
}

/// #835 (A's W6): the production host is `BindingHost` over `WiredHost` over the adapter, so
/// `WiredHost` forwards every `HostPort` method too; the suite's `foreground_commands` case
/// pins the method #835 adds.
#[test]
fn the_wired_host_passes_the_host_conformance_suite() {
    let result = run_host_conformance(|| (WiredHost::new(Ok(Arc::new(FakeHost::new()))), ()));
    assert_eq!(result, Ok(()));
}

// --- the resolver (D8, W-5) ---------------------------------------------------------------

#[test]
fn the_resolver_answers_a_pane_being_launched_from_the_bindings() {
    let bound = Bound::new();
    let pane = bound.launch(1, 1, "hlr649-c1r1");
    assert!(bound.panes.list().unwrap().is_empty(), "no record exists yet");
    assert_eq!(bound.answer(&pane), Ok(name("hlr649-c1r1")));
}

#[test]
fn each_ensure_session_claims_the_pane_the_ensure_pane_before_it_returned() {
    let bound = Bound::new();
    let first = bound.launch(1, 1, "hlr649-c1r1");
    let second = bound.launch(2, 1, "hlr649-c1r2");
    assert_ne!(first, second);
    assert_eq!(bound.answer(&first), Ok(name("hlr649-c1r1")));
    assert_eq!(bound.answer(&second), Ok(name("hlr649-c1r2")));
}

#[test]
fn an_ensure_session_with_no_pane_before_it_binds_nothing() {
    let bound = Bound::new();
    bound.host.ensure_session(&name("hlr649-c1r1"), "/tmp").unwrap();
    let unknown = PaneId::new("w1:p1");
    assert_eq!(
        bound.answer(&unknown).map_err(|e| e.code().to_owned()),
        Err("pane-not-found".to_owned())
    );
}

#[test]
fn the_resolver_answers_from_the_registry_by_session_and_pane_id() {
    let bound = Bound::new();
    let pane = bound.fake.ensure_pane(&spec(1, 1)).unwrap().pane_id;
    bound.panes.concurrent_put(&record("hlr649-c1r1", SESSION, &pane)).unwrap();
    assert_eq!(bound.answer(&pane), Ok(name("hlr649-c1r1")));
}

/// W-5: Herdr numbers pane ids within a session, so a record of another session with the
/// same id is not this pane.
#[test]
fn a_record_of_another_herdr_session_with_the_same_id_is_not_a_match() {
    let bound = Bound::new();
    let pane = bound.fake.ensure_pane(&spec(1, 1)).unwrap().pane_id;
    bound.panes.concurrent_put(&record("hlr649-c1r1", "elsewhere", &pane)).unwrap();
    assert_eq!(
        bound.answer(&pane).map_err(|e| e.code().to_owned()),
        Err("pane-not-found".to_owned())
    );
}

/// W-5: two records naming the same pane is never a pick.
#[test]
fn two_records_on_one_pane_are_unavailable() {
    let bound = Bound::new();
    let pane = bound.fake.ensure_pane(&spec(1, 1)).unwrap().pane_id;
    bound.panes.concurrent_put(&record("hlr649-c1r1", SESSION, &pane)).unwrap();
    bound.panes.concurrent_put(&record("hlr649-c2r1", SESSION, &pane)).unwrap();
    assert_eq!(
        bound.answer(&pane).map_err(|e| e.code().to_owned()),
        Err("unavailable".to_owned())
    );
}

#[test]
fn the_resolver_answers_a_labelled_pane_from_herdrs_snapshot() {
    let bound = Bound::new();
    let pane = bound.fake.ensure_pane(&spec(1, 1)).unwrap().pane_id;
    assert_eq!(
        bound.answer(&pane).map_err(|e| e.code().to_owned()),
        Err("pane-not-found".to_owned()),
        "an unlabelled pane with no record and no binding is not found"
    );
    bound.fake.rename(&pane, "hlr649-c1r1").unwrap();
    assert_eq!(bound.answer(&pane), Ok(name("hlr649-c1r1")));
}

#[test]
fn the_bindings_come_before_the_registry_and_the_registry_before_the_label() {
    let bound = Bound::new();
    let pane = bound.launch(1, 1, "hlr649-bound");
    bound.panes.concurrent_put(&record("hlr649-stored", SESSION, &pane)).unwrap();
    bound.fake.rename(&pane, "hlr649-label").unwrap();
    assert_eq!(bound.answer(&pane), Ok(name("hlr649-bound")), "the bindings first");

    let other = Bound::new();
    let pane = other.fake.ensure_pane(&spec(1, 1)).unwrap().pane_id;
    other.panes.concurrent_put(&record("hlr649-stored", SESSION, &pane)).unwrap();
    other.fake.rename(&pane, "hlr649-label").unwrap();
    assert_eq!(other.answer(&pane), Ok(name("hlr649-stored")), "then the registry");
}

// --- the pin of D8's call order (W-12) ----------------------------------------------------

/// What `attach_tui` saw: the resolver's answer for its pane, and whether the pane store
/// held any record at that moment.
type Seen = Mutex<Vec<(Result<PaneName, String>, bool)>>;

/// The fake harness, whose `attach_tui` first asks the wiring's resolver, as the OpenCode
/// adapter's does.
struct Resolving {
    inner: Arc<FakeHarness>,
    resolve: holler_adapter_opencode::Resolver<PaneId, PaneName>,
    panes: Arc<FakePaneStore>,
    seen: Seen,
}

impl HarnessPort for Resolving {
    fn serve_command(&self, n: &PaneName, port: u16, s: &ServeSpec) -> Result<Argv, PaneError> {
        self.inner.serve_command(n, port, s)
    }
    fn verify_serving(&self, at: &HarnessEndpoint, s: &ServeSpec) -> Result<(), PaneError> {
        self.inner.verify_serving(at, s)
    }
    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.inner.health(at)
    }
    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.inner.create_session(at)
    }
    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(at)
    }
    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.inner.abort(at, session)
    }
    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        let answer = (self.resolve)(pane).map_err(|e| e.code().to_owned());
        let stored = !self.panes.list().unwrap_or_default().is_empty();
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((answer, stored));
        self.inner.attach_tui(pane, at, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.inner.select_session(pane, session)
    }
    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane, at)
    }
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.inner.send_prompt(at, session, text)
    }
    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        self.inner.session_activity(at, session)
    }
}

/// `pane launch` (the engine, unchanged) over the wiring's decorators wrapping the fakes:
/// the resolver answers `attach_tui`'s pane from the bindings, while no record exists yet.
/// A change to the engine's order (`ensure_pane`, `ensure_session`, `attach_tui`, E34)
/// fails here, in the default run.
#[test]
fn launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists() {
    let fake = herdr_fake();
    let inner: Arc<dyn HerdrPort> = fake.clone();
    let bindings = Bindings::new();
    let panes = Arc::new(FakePaneStore::new());
    let profiles = Arc::new(FakeProfileStore::new());
    let harness = Arc::new(FakeHarness::new());
    let store: Arc<dyn PaneStore> = panes.clone();
    let herdr = BindingHerdr::new(Arc::clone(&inner), bindings.clone());
    let host = BindingHost::new(
        // The fake host, linked to the fake harness: running the argv the harness built starts
        // its server (as the launch rig of `pane_verbs` links them, #802); stopping kills none.
        Arc::new(LinkedHost {
            host: FakeHost::new(),
            link: FakeHarnessLink {
                harness: Arc::clone(&harness),
                on_start: OnStart::Propagate,
                on_stop: OnStop::KillNothing,
            },
        }),
        bindings.clone(),
    );
    let resolving = Resolving {
        inner: harness,
        resolve: tui_session(SESSION, bindings, store, inner),
        panes: Arc::clone(&panes),
        seen: Mutex::new(Vec::new()),
    };
    // `Ports` takes all seven ports. The profile store, the scope and the prober only fill it:
    // the test asserts on the resolver's answer and the pane store.
    let scope = FakeProfileScope::new(
        profiles.clone(),
        panes.clone(),
        Actor::parse("holler pane").unwrap(),
    );
    let prober = FakeProber::new();
    let ports = Ports {
        pane_store: panes.as_ref(),
        profile_store: profiles.as_ref(),
        herdr: &herdr,
        host: &host,
        harness: &resolving,
        scope: &scope,
        prober: &prober,
        hub: &holler_cli::pane::wiring::Unwired,
    };
    let out = run_verb_with(LAUNCH, Format::Json, ports);
    assert_eq!(out.code, 0, "the launch succeeds: {out:?}");
    let seen = resolving.seen.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![(Ok(name("demo-c1r1")), false)],
        "attach_tui's pane resolved from the bindings, before the record was written"
    );
    assert_eq!(panes.list().unwrap().len(), 1, "the record was written after");
}

/// The launch of the `pane_verbs` launch rig (`demo-c1r1` at `main r2c1`, port 48100).
const LAUNCH: &[&str] = &[
    "pane",
    "launch",
    "demo-c1r1",
    "--herdr-session",
    SESSION,
    "--project",
    "/srv/demo",
    "--workspace",
    WORKSPACE,
    "--grid",
    "r2c1",
    "--model",
    "demo-provider/demo-model",
    "--effort",
    "medium",
    "--ctx-soft",
    "100000",
    "--ctx-hard",
    "150000",
    "--port-policy",
    "fixed:48100",
];
