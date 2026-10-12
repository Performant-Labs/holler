//! The live rig the `holler profile apply` tests run over (story #664).
//!
//! Declared from `apply.rs` (`pub(crate) mod rig;`). It is additive (plan review, warn 6):
//! `profile_verbs/rig.rs`, the read verbs' rig, keeps its `Unwired` scope and its
//! "no adapter call" helpers unchanged, and this rig copies by name what
//! `pane_verbs/launch/rig.rs` (#644) built for a live verb, so a later consolidation is
//! mechanical:
//!
//! - the test kit's fakes, with the fake scope over the two stores and a Herdr serving the
//!   session `scratch` with one workspace `main` of 3 by 3;
//! - a **linked host**: `stop_owned(name)` also kills the harness server of every rig port
//!   served for `name` (the host and harness fakes share no state, so a relaunch of a pane
//!   whose server still runs would otherwise fail in `stop_old`);
//! - **harness hooks**: a test closure run once, right after a named `HarnessPort` method
//!   returns (another writer editing P during apply's up-front observation);
//! - a **call-log span** (`Calls`): `run` and `engine` return the calls the run made and
//!   nothing else;
//! - the pane store's **feed**: `cursor` before a run and `writes_since` after it give every
//!   record write the run made, in order, each with the record as written (brief C7, R4 W-2).
//!
//! Servers are started only through `crate::list::rig::serve_server` (decision O3).

use std::sync::{Arc, Mutex, PoisonError};

use holler_cli::output::Format;
use holler_pane::pane::{HarnessInfo, Health, HostInfo, LastObserved, PaneRole};
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::tx_apply::{self, Applied, ApplyRequest, Outcome as StepOutcome, Step};
use holler_pane::tx_launch::TxOptions;
use holler_pane::{
    Actor, Argv, Cursor, GridPos, HarnessPort, HerdrPane, HerdrPort, HerdrSpec, HostPort, Pane,
    PaneError, PaneEvent, PaneId, PaneName, PaneStore, Ports, Profile, ProfileName, ProfileSpec,
    ProfileStore, ServeSpec,
};
use holler_pane_testkit::envelope::{check_envelope, Envelope};
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp, PROTOCOL_22_VERSION};
use holler_pane_testkit::host::{FakeHost, HostOp};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::{FakeProber, ProbeCall};
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

use crate::list::rig::serve_server;
use crate::verb_harness::{run_verb_with, Outcome};

/// The profile every case applies.
pub(crate) const P: &str = "demo";
/// Another profile, whose panes `--take-over` moves.
pub(crate) const Q: &str = "other";
/// The Herdr session the rig's Herdr serves.
pub(crate) const SESSION: &str = "scratch";
/// The one workspace of the rig's Herdr.
pub(crate) const WORKSPACE: &str = "main";
/// The ports the linked host checks on `stop_owned`, one per pane a case may hold.
pub(crate) const RIG_PORTS: [u16; 8] = [48100, 48101, 48102, 48103, 48104, 48105, 48106, 48107];

/// `holler profile apply demo` followed by `extra`.
pub(crate) fn apply_argv<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut argv = vec!["profile", "apply", P];
    argv.extend_from_slice(extra);
    argv
}

/// `--herdr-session scratch`, the flag a plan holding a `create` needs (§4).
pub(crate) const HS: [&str; 2] = ["--herdr-session", SESSION];

/// A typed pane name (a test's own constant).
pub(crate) fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

/// A typed profile name (a test's own constant).
pub(crate) fn profile_name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

/// An argv (a test's own constant).
pub(crate) fn argv(elements: &[&str]) -> Argv {
    serde_json::from_value(serde_json::json!(elements)).unwrap()
}

/// The cell `r<row>c<col>`.
pub(crate) const fn cell(row: u16, col: u16) -> GridPos {
    GridPos { row, col }
}

/// What the engine is asked: apply P, with the flags given.
pub(crate) fn request(dry_run: bool, take_over: bool, herdr_session: bool) -> ApplyRequest {
    ApplyRequest {
        profile: profile_name(P),
        herdr_session: herdr_session.then(|| SESSION.to_owned()),
        take_over,
        dry_run,
    }
}

/// The options a case runs the engine under: the default budget, a constant clock.
pub(crate) fn options() -> TxOptions {
    TxOptions {
        now_ms: || 0,
        ..TxOptions::default()
    }
}

/// One pane for [`Rig::seed_live`]: by default an agent of P.
#[derive(Debug, Clone)]
pub(crate) struct Seed {
    pub pane: &'static str,
    pub grid: GridPos,
    pub port: u16,
    pub profile: Option<&'static str>,
    pub role: PaneRole,
}

impl Seed {
    /// The agent `pane` of P at `grid`, served on `port`.
    pub fn at(pane: &'static str, grid: GridPos, port: u16) -> Self {
        Self {
            pane,
            grid,
            port,
            profile: Some(P),
            role: PaneRole::Agent,
        }
    }

    /// The same pane, recorded in `profile` (`None`: in no profile).
    pub fn in_profile(self, profile: Option<&'static str>) -> Self {
        Self { profile, ..self }
    }

    /// The same pane, as the orchestrator.
    pub fn orchestrator(self) -> Self {
        Self {
            role: PaneRole::Orchestrator,
            ..self
        }
    }
}

/// A test closure run once, after a named harness method returns.
pub(crate) type Hook = Box<dyn FnOnce(&Rig) + Send>;

/// The calls made through every port, oldest first.
#[derive(Debug, Clone, Default)]
pub(crate) struct Calls {
    pub panes: Vec<PaneStoreOp>,
    pub profiles: Vec<ProfileStoreOp>,
    pub herdr: Vec<HerdrOp>,
    pub host: Vec<HostOp>,
    pub harness: Vec<HarnessOp>,
    pub probes: Vec<ProbeCall>,
}

/// One verb run: what it printed and the calls it made.
#[derive(Debug)]
pub(crate) struct Run {
    pub outcome: Outcome,
    pub calls: Calls,
}

impl std::ops::Deref for Run {
    type Target = Outcome;
    fn deref(&self) -> &Outcome {
        &self.outcome
    }
}

/// The fakes `profile apply` runs over, a linked host and the harness hooks.
pub(crate) struct Rig {
    pub panes: Arc<FakePaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
    hooks: Mutex<Vec<(HarnessOp, Hook)>>,
}

impl Rig {
    /// An empty registry, and the profiles P and Q with no spec (each stored at generation 1).
    pub fn new() -> Self {
        let actor = Actor::parse("test seed").unwrap();
        let panes = Arc::new(FakePaneStore::new());
        let seeded = [P, Q].map(|p| sample_profile(p, &[]).unwrap());
        let profiles = Arc::new(FakeProfileStore::seeded(seeded, &actor).unwrap());
        let scope = FakeProfileScope::new(
            profiles.clone(),
            panes.clone(),
            Actor::parse("holler profile").unwrap(),
        );
        Self {
            panes,
            profiles,
            scope,
            herdr: FakeHerdr::new(SESSION)
                .with_workspace(WORKSPACE, 3, 3)
                .unwrap(),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
            hooks: Mutex::new(Vec::new()),
        }
    }

    /// Run `hook` once, right after the next call of `op` returns.
    pub fn after(&self, op: HarnessOp, hook: impl FnOnce(&Rig) + Send + 'static) {
        self.hooks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((op, Box::new(hook)));
    }

    /// Call `f` with the ports a verb sees, with `pane_store` in place of the rig's own when
    /// given (a seam over `self.panes`).
    pub fn with_ports<R>(
        &self,
        pane_store: Option<&dyn PaneStore>,
        f: impl FnOnce(Ports<'_>) -> R,
    ) -> R {
        let host = LinkedHost { rig: self };
        let harness = HookedHarness { rig: self };
        f(Ports {
            pane_store: pane_store.unwrap_or(self.panes.as_ref()),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: &host,
            harness: &harness,
            scope: &self.scope,
            prober: &self.prober,
        })
    }

    /// Run `holler <argv...>` over the rig, recording the calls it made.
    pub fn run(&self, argv: &[&str], format: Format) -> Run {
        self.run_over_opt(None, argv, format)
    }

    /// [`Rig::run`] with `pane_store` (a seam over `self.panes`) in place of the rig's own.
    pub fn run_over(&self, pane_store: &dyn PaneStore, argv: &[&str], format: Format) -> Run {
        self.run_over_opt(Some(pane_store), argv, format)
    }

    fn run_over_opt(&self, store: Option<&dyn PaneStore>, argv: &[&str], format: Format) -> Run {
        let mark = self.mark();
        let outcome = self.with_ports(store, |ports| run_verb_with(argv, format, ports));
        Run {
            outcome,
            calls: self.calls_since(&mark),
        }
    }

    /// Call the engine over the rig's ports, returning its answer and the calls it made.
    pub fn engine(
        &self,
        request: &ApplyRequest,
        options: &TxOptions,
    ) -> (Result<Applied, PaneError>, Calls) {
        self.engine_over_opt(None, request, options)
    }

    /// [`Rig::engine`] with `pane_store` (a seam over `self.panes`) in place of the rig's own.
    pub fn engine_over(
        &self,
        pane_store: &dyn PaneStore,
        request: &ApplyRequest,
        options: &TxOptions,
    ) -> (Result<Applied, PaneError>, Calls) {
        self.engine_over_opt(Some(pane_store), request, options)
    }

    fn engine_over_opt(
        &self,
        store: Option<&dyn PaneStore>,
        request: &ApplyRequest,
        options: &TxOptions,
    ) -> (Result<Applied, PaneError>, Calls) {
        let mark = self.mark();
        let answer = self.with_ports(store, |ports| tx_apply::apply(ports, request, options));
        (answer, self.calls_since(&mark))
    }

    /// A pane as a successful launch leaves it, built through the fakes' own methods and not
    /// a verb: a Herdr pane at its cell, its tmux session, a server on its port, one session
    /// the TUI shows, and its record (generation 1, in `seed.profile`). P is not edited: a case
    /// sets P's specs with [`Rig::set_specs`].
    pub fn seed_live(&self, seed: &Seed) -> Pane {
        let pane = name(seed.pane);
        let herdr = self.place_herdr_pane(seed.grid);
        self.host.ensure_session(&pane, "/srv/demo").unwrap();
        let pid = serve_server(&self.harness, &pane, seed.port);
        let sid = self.harness.create_session(seed.port).unwrap();
        self.harness
            .attach_tui(&herdr.pane_id, seed.port, &sid)
            .unwrap();
        let sample = sample_pane(seed.pane).unwrap();
        let record = Pane {
            herdr,
            host: HostInfo {
                herdr_api_version: Some(PROTOCOL_22_VERSION.to_owned()),
                ..sample.host.clone()
            },
            harness: HarnessInfo {
                port: seed.port,
                pid: Some(pid),
                health: Health::Healthy,
                ..sample.harness.clone()
            },
            session_of_record: Some(sid.clone()),
            role: seed.role,
            last_observed: LastObserved {
                shown: Some(sid),
                driven: None,
                at: 0,
            },
            profile: seed.profile.map(profile_name),
            ..sample
        };
        self.panes.concurrent_put(&record).unwrap()
    }

    /// Set the specs of `profile` to `specs`, in order, as another writer (unlogged).
    pub fn set_specs(&self, profile: &str, specs: Vec<ProfileSpec>) -> Profile {
        let stored = self.stored_profile(profile);
        let edited = Profile {
            panes: specs,
            ..stored
        };
        let actor = Actor::parse("test seed").unwrap();
        self.profiles.concurrent_put(&edited, &actor).unwrap()
    }

    /// The stored profile `profile` (a `get` through the port, logged).
    pub fn stored_profile(&self, profile: &str) -> Profile {
        self.profiles
            .get(&profile_name(profile))
            .unwrap()
            .expect("the profile exists")
    }

    /// The stored record of `pane` (a `get` through the port, logged).
    pub fn record(&self, pane: &str) -> Option<Pane> {
        self.panes.get(&name(pane)).unwrap()
    }

    /// Place a Herdr pane at `main <grid>` (setup, through the port, logged).
    pub fn place_herdr_pane(&self, grid: GridPos) -> HerdrPane {
        self.herdr
            .ensure_pane(&HerdrSpec {
                session: SESSION.to_owned(),
                workspace: WORKSPACE.to_owned(),
                grid,
            })
            .unwrap()
    }

    /// Every Herdr pane the snapshot lists (a `snapshot` through the port, logged).
    pub fn herdr_panes(&self) -> Vec<HerdrPane> {
        self.herdr.snapshot().unwrap().panes
    }

    /// Kill the harness server of `record` (it no longer answers: the pane is not healthy).
    pub fn kill(&self, record: &Pane) {
        self.harness.kill(record.harness.port).unwrap();
    }

    /// The pane store's head: hand it to [`Rig::writes_since`] after a run.
    pub fn cursor(&self) -> Cursor {
        let mut watch = self.panes.watch(Cursor(0)).unwrap();
        let mut head = Cursor(0);
        while let Some(event) = watch.next().unwrap().unwrap() {
            head = head.max(event.cursor);
        }
        head
    }

    /// Every record write after `since`, in order, each with the record as written.
    pub fn writes_since(&self, since: Cursor) -> Vec<PaneEvent> {
        let mut watch = self.panes.watch(since).unwrap();
        let mut events = Vec::new();
        while let Some(event) = watch.next().unwrap().unwrap() {
            events.push(event);
        }
        events
    }

    /// Every call made through every port so far.
    pub fn mark(&self) -> Calls {
        Calls {
            panes: self.panes.faults().calls(),
            profiles: self.profiles.faults().calls(),
            herdr: self.herdr.faults().calls(),
            host: self.host.faults().calls(),
            harness: self.harness.faults().calls(),
            probes: self.prober.calls(),
        }
    }

    /// The calls made after `mark`.
    pub fn calls_since(&self, mark: &Calls) -> Calls {
        let now = self.mark();
        Calls {
            panes: now.panes[mark.panes.len()..].to_vec(),
            profiles: now.profiles[mark.profiles.len()..].to_vec(),
            herdr: now.herdr[mark.herdr.len()..].to_vec(),
            host: now.host[mark.host.len()..].to_vec(),
            harness: now.harness[mark.harness.len()..].to_vec(),
            probes: now.probes[mark.probes.len()..].to_vec(),
        }
    }
}

/// The health check of the every-row fixture's `pane-b`, as an argv.
pub(crate) const PROBE: [&str; 2] = ["curl", "http://127.0.0.1:8095/v1/models"];

/// One spec pane for each row of §3, in grid order, with P's specs listed in the reverse
/// order (so a case sees that apply orders its steps itself, C6):
///
/// | pane | cell | row | state |
/// |---|---|---|---|
/// | `pane-a` | r1c1 | a | recorded in Q, healthy, matching |
/// | `pane-b` | r1c2 | b | no record; its spec has a check, scripted to pass |
/// | `pane-c` | r1c3 | c | recorded in P, healthy, matching |
/// | `pane-d` | r2c1 | d | recorded in P, healthy; its spec's effort is `high` |
/// | `pane-e` | r2c2 | e | the orchestrator, recorded in P, its server killed |
/// | `pane-f` | r2c3 | f | an agent, recorded in P, its server killed |
pub(crate) fn every_row() -> Rig {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    let mut b = new_spec("pane-b", cell(1, 2), 48101);
    b.check = Some(argv(&PROBE));
    b.expect = vec!["qwen38".to_owned()];
    rig.prober
        .script(argv(&PROBE), holler_pane::ProbeResult::Ok);
    let c = rig.seed_live(&Seed::at("pane-c", cell(1, 3), 48102));
    let d = rig.seed_live(&Seed::at("pane-d", cell(2, 1), 48103));
    let mut d_spec = spec_of(&d);
    d_spec.model.effort = "high".to_owned();
    let e = rig.seed_live(&Seed::at("pane-e", cell(2, 2), 48104).orchestrator());
    rig.kill(&e);
    let f = rig.seed_live(&Seed::at("pane-f", cell(2, 3), 48105));
    rig.kill(&f);
    let specs = vec![
        spec_of(&f),
        spec_of(&e),
        d_spec,
        spec_of(&c),
        b,
        spec_of(&a),
    ];
    rig.set_specs(P, specs);
    rig
}

/// The panes of [`every_row`], in grid order.
pub(crate) const EVERY_ROW: [&str; 6] =
    ["pane-a", "pane-b", "pane-c", "pane-d", "pane-e", "pane-f"];

/// The spec that matches `record` on every compared field (`diff_spec` is empty).
pub(crate) fn spec_of(record: &Pane) -> ProfileSpec {
    spec_from_pane(record)
}

/// A spec for a pane with no record: `pane` at `grid` in `main`, served on `port`.
pub(crate) fn new_spec(pane: &str, grid: GridPos, port: u16) -> ProfileSpec {
    let mut spec = holler_pane_testkit::fixture::sample_spec(pane);
    spec.herdr.workspace = WORKSPACE.to_owned();
    spec.herdr.grid = grid;
    spec.harness.port_policy = holler_pane::profile_snapshot::fixed_port_policy(port);
    spec
}

/// The step of `pane` in `applied`.
pub(crate) fn step<'a>(applied: &'a Applied, pane: &str) -> &'a Step {
    applied
        .steps
        .iter()
        .find(|s| s.pane == pane)
        .unwrap_or_else(|| panic!("a step for {pane}: {applied:?}"))
}

/// Each step's pane and outcome, in order.
pub(crate) fn outcomes(applied: &Applied) -> Vec<(&str, StepOutcome)> {
    applied
        .steps
        .iter()
        .map(|s| (s.pane.as_str(), s.outcome))
        .collect()
}

/// Nothing but apply's own observations (§2, C1 step 3): the Herdr snapshot, `health` and
/// `shown_session`. No other live call, no probe, and no write to either store.
pub(crate) fn assert_only_observed(calls: &Calls) {
    let herdr: Vec<&HerdrOp> = calls
        .herdr
        .iter()
        .filter(|op| **op != HerdrOp::Snapshot)
        .collect();
    assert_eq!(herdr, Vec::<&HerdrOp>::new(), "only Herdr's snapshot");
    let harness: Vec<&HarnessOp> = calls
        .harness
        .iter()
        .filter(|op| !matches!(op, HarnessOp::Health | HarnessOp::ShownSession))
        .collect();
    assert_eq!(
        harness,
        Vec::<&HarnessOp>::new(),
        "only health and shown_session"
    );
    assert_eq!(calls.host, vec![], "no host call");
    assert_eq!(calls.probes, vec![], "no probe run");
    assert_no_store_write(calls);
}

/// Neither store was written (no `cas_put` and no `delete`).
pub(crate) fn assert_no_store_write(calls: &Calls) {
    let pane_writes: Vec<&PaneStoreOp> = calls
        .panes
        .iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
        .collect();
    assert_eq!(
        pane_writes,
        Vec::<&PaneStoreOp>::new(),
        "no pane-store write"
    );
    let profile_writes: Vec<&ProfileStoreOp> = calls
        .profiles
        .iter()
        .filter(|op| {
            matches!(
                op,
                ProfileStoreOp::CasPut | ProfileStoreOp::Delete | ProfileStoreOp::Rename
            )
        })
        .collect();
    assert_eq!(
        profile_writes,
        Vec::<&ProfileStoreOp>::new(),
        "no profile write"
    );
}

/// The envelope of a JSON run, checked by the test kit against its exit code.
pub(crate) fn envelope_of(run: &Outcome) -> Envelope {
    check_envelope(&run.out, run.code).unwrap_or_else(|fault| panic!("{fault}: {run:?}"))
}

/// The `data` of a JSON run that exits 0.
pub(crate) fn data_of(run: &Outcome) -> serde_json::Value {
    assert_eq!(run.code, 0, "the run succeeds: {run:?}");
    envelope_of(run).data
}

/// The JSON step of `pane` in `data`.
pub(crate) fn json_step<'a>(data: &'a serde_json::Value, pane: &str) -> &'a serde_json::Value {
    data["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("steps is an array: {data}"))
        .iter()
        .find(|s| s["pane"] == pane)
        .unwrap_or_else(|| panic!("a step for {pane}: {data}"))
}

/// What a run stopped by a `failed` step prints (the decision on O6).
pub(crate) struct Stop<'a> {
    /// The failed step's pane.
    pub pane: &'a str,
    /// The failed step's error code, the envelope's `error.code`.
    pub code: &'a str,
    /// The failed step's action, when the case knows its row.
    pub action: Option<tx_apply::Action>,
    /// How many later steps were skipped.
    pub skipped: usize,
}

/// The decision on O6, at the verb: each format on its own fresh rig (`seed`), run by `run`,
/// exits 1 and prints only the failure. JSON: one failure envelope, `data` null, `error.code`
/// the failed step's code, `error.message` in the pinned form `apply stopped at <pane>
/// (<action>): <step error message>; <N> later step(s) skipped`. Text: nothing on stdout and one
/// `error: <same message>` line on stderr. Returns the message.
pub(crate) fn assert_stopped(
    seed: impl Fn() -> Rig,
    run: impl Fn(&Rig, Format) -> Run,
    stop: &Stop<'_>,
) -> String {
    let json = run(&seed(), Format::Json);
    assert_eq!(json.code, 1, "{json:?}");
    let envelope = envelope_of(&json);
    assert_eq!(envelope.data, serde_json::Value::Null, "no plan: {json:?}");
    let error = envelope.error.expect("a failure");
    assert_eq!(error.code, stop.code, "{json:?}");
    let message = error.message;
    assert!(message.starts_with("apply stopped at "), "{message}");
    assert!(
        message.contains(stop.pane),
        "names {}: {message}",
        stop.pane
    );
    if let Some(action) = stop.action {
        let part = format!(" ({}): ", wire(action));
        assert!(
            message.contains(&part),
            "names the action {part:?}: {message}"
        );
    }
    let tail = format!("; {} later step(s) skipped", stop.skipped);
    assert!(message.ends_with(&tail), "ends with {tail:?}: {message}");

    let text = run(&seed(), Format::Text);
    assert_eq!(text.code, 1, "{text:?}");
    assert_eq!(
        text.out, "",
        "no plan and no skipped list on stdout: {text:?}"
    );
    assert_eq!(text.err.lines().count(), 1, "one line: {text:?}");
    assert!(text.err.starts_with("error: apply stopped at "), "{text:?}");
    assert!(text.err.trim_end().ends_with(&tail), "{text:?}");
    message
}

/// A value's serde form as a string (an `Action`'s or an `Outcome`'s wire name).
pub(crate) fn wire<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .expect("a unit variant serializes as a string")
        .to_owned()
}

/// A `HostPort` over the rig's `FakeHost` whose `stop_owned` also kills the servers served for
/// the pane.
struct LinkedHost<'a> {
    rig: &'a Rig,
}

impl HostPort for LinkedHost<'_> {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.rig.host.ensure_session(name, cwd)
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.rig.host.run(name, cwd, argv)?;
        // The host starts the server the argv describes (#802): the fake harness does it for
        // the argv it built, and starts nothing for any other.
        if let Some(started) = self.rig.harness.start_server(argv) {
            started?;
        }
        Ok(())
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.rig.host.stop_owned(name)?;
        for port in RIG_PORTS {
            if self
                .rig
                .harness
                .server(port)
                .is_some_and(|s| s.name == *name)
            {
                self.rig.harness.kill(port)?;
            }
        }
        Ok(())
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.rig.host.ps(name)
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.rig.host.foreground_commands(name)
    }
}

/// A `HarnessPort` over the rig's `FakeHarness` that runs a test's hook after a method.
struct HookedHarness<'a> {
    rig: &'a Rig,
}

impl HookedHarness<'_> {
    /// Run (and drop) the first hook armed for `op`. No lock is held while it runs.
    fn fire(&self, op: HarnessOp) {
        let hook = {
            let mut hooks = self
                .rig
                .hooks
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            hooks
                .iter()
                .position(|(armed, _)| *armed == op)
                .map(|at| hooks.remove(at).1)
        };
        if let Some(hook) = hook {
            hook(self.rig);
        }
    }
}

impl HarnessPort for HookedHarness<'_> {
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        let answer = self.rig.harness.serve_command(name, port, spec);
        self.fire(HarnessOp::ServeCommand);
        answer
    }

    fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError> {
        let answer = self.rig.harness.verify_serving(port, spec);
        self.fire(HarnessOp::VerifyServing);
        answer
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let answer = self.rig.harness.health(port);
        self.fire(HarnessOp::Health);
        answer
    }

    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        let answer = self.rig.harness.create_session(port);
        self.fire(HarnessOp::CreateSession);
        answer
    }

    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        let answer = self.rig.harness.list_sessions(port);
        self.fire(HarnessOp::ListSessions);
        answer
    }

    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        let answer = self.rig.harness.abort(port, session);
        self.fire(HarnessOp::Abort);
        answer
    }

    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        let answer = self.rig.harness.attach_tui(pane, port, session);
        self.fire(HarnessOp::AttachTui);
        answer
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        let answer = self.rig.harness.select_session(pane, session);
        self.fire(HarnessOp::SelectSession);
        answer
    }

    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        let answer = self.rig.harness.shown_session(pane);
        self.fire(HarnessOp::ShownSession);
        answer
    }
}
