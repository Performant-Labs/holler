//! The rig the `holler pane launch` and `holler pane relaunch` tests run over (story #644,
//! brief decision 25, plan-review warn 4).
//!
//! Declared from `launch.rs` (`pub(crate) mod rig;`) and reached by `relaunch.rs` as
//! `crate::launch::rig`. It builds on #643's `crate::list::Rig` (its fakes, unchanged, with a
//! Herdr workspace `main` of 3 by 3) and adds only what a live verb needs:
//!
//! - a **linked host**: `stop_owned(name)` also kills the harness server of every rig port that
//!   was served for `name`, as the test kit recommends (its host and harness fakes share no
//!   state);
//! - **harness hooks**: a test closure run once, right after a named `HarnessPort` method
//!   returns (to freeze a server after `serve`, vanish a Herdr pane at `attach_tui`, make another
//!   writer move a record or a profile, or panic);
//! - #663's real `StoreScope` in place of the fake scope (AC 16k);
//! - a **call-log span** (`Calls`, as in doctor's rig): `run` returns the calls the verb run made
//!   and nothing else, so a test's own setup and post-run reads are never asserted;
//! - the checks every case ends with: `assert_matches` (AC 2), `assert_no_keystroke` (I4) and
//!   `assert_untouched` (nothing live, no store write).

use std::ops::Deref;
use std::sync::{Mutex, PoisonError};

use holler_cli::output::Format;
use holler_cli::pane::profile_scope::StoreScope;
use holler_pane::pane::{
    ContextCeilings, HarnessInfo, HarnessKind, Health, HostInfo, LastObserved, ModelSpec, PaneRole,
};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::{
    Actor, AgentKey, Argv, GridPos, HarnessPort, HerdrPane, HerdrPort, HerdrSpec, HostPort, Pane,
    PaneError, PaneId, PaneName, PaneStore, Ports, Profile, ProfileLogEntry, ProfileName,
    ProfileScope, ProfileSpec, ProfileStore,
};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::{HarnessOp, ServerState};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp, PROTOCOL_22_VERSION};
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::prober::ProbeCall;
use holler_pane_testkit::profile_store::ProfileStoreOp;

use crate::list::Rig as Fakes;
use crate::verb_harness::{run_verb_with, Outcome};

/// The brief's `LAUNCH`: `demo-c1r1` at `main r2c1`, port 48100, no profile, no command and
/// no check.
pub(crate) const LAUNCH: &[&str] = &[
    "pane",
    "launch",
    "demo-c1r1",
    "--herdr-session",
    "scratch",
    "--project",
    "/srv/demo",
    "--workspace",
    "main",
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

/// The pane every case launches.
pub(crate) const PANE: &str = "demo-c1r1";
/// The Herdr session the rig's Herdr serves.
pub(crate) const SESSION: &str = "scratch";
/// The one workspace of the rig's Herdr.
pub(crate) const WORKSPACE: &str = "main";
/// The harness port of `LAUNCH`.
pub(crate) const PORT: u16 = 48100;
/// The ports the linked host checks on `stop_owned` (the test kit's scratch range).
pub(crate) const RIG_PORTS: [u16; 3] = [48100, 48101, 48102];
/// The step a run that names no profile prints (#663's `reconcile_step(None)`).
pub(crate) const STEP: &str = "to reconcile, run holler pane doctor";

/// The health check of AC 10, as an argv.
pub(crate) const PROBE: [&str; 2] = ["curl", "http://127.0.0.1:8095/v1/models"];
/// AC 10a's flags: profile `demo`, the check `PROBE`, and the expected string `qwen38`.
pub(crate) const PROBE_FLAGS: [&str; 8] = [
    "--profile",
    "demo",
    "--check-arg",
    PROBE[0],
    "--check-arg",
    PROBE[1],
    "--expect",
    "qwen38",
];

/// `LAUNCH` followed by `extra`.
pub(crate) fn launch_with<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut argv = LAUNCH.to_vec();
    argv.extend_from_slice(extra);
    argv
}

/// `LAUNCH` with the value of `flag` replaced by `value`.
pub(crate) fn launch_replacing<'a>(flag: &str, value: &'a str) -> Vec<&'a str> {
    let mut argv = LAUNCH.to_vec();
    let at = argv.iter().position(|a| *a == flag).expect("a LAUNCH flag");
    argv[at + 1] = value;
    argv
}

/// `LAUNCH` without `flag` and its value.
pub(crate) fn launch_without(flag: &str) -> Vec<&'static str> {
    let mut argv = LAUNCH.to_vec();
    let at = argv.iter().position(|a| *a == flag).expect("a LAUNCH flag");
    argv.drain(at..at + 2);
    argv
}

/// A typed pane name (a test's own constant).
pub(crate) fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

/// A typed profile name (a test's own constant).
pub(crate) fn profile_name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

/// An OpenCode agent key (#700; a test's own constant).
pub(crate) fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// An argv (a test's own constant).
pub(crate) fn argv(elements: &[&str]) -> Argv {
    serde_json::from_value(serde_json::json!(elements)).unwrap()
}

/// How many times `needle` occurs in `hay`.
pub(crate) fn occurrences(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// One pane for [`Rig::seed_live`]: by default `LAUNCH`'s (`demo-c1r1` at `r2c1` on 48100, in
/// no profile, with no command).
#[derive(Debug, Clone)]
pub(crate) struct Seed {
    pub pane: &'static str,
    pub grid: GridPos,
    pub port: u16,
    pub profile: Option<&'static str>,
    pub command: Option<Argv>,
}

impl Default for Seed {
    fn default() -> Self {
        Self {
            pane: PANE,
            grid: GridPos { row: 2, col: 1 },
            port: PORT,
            profile: None,
            command: None,
        }
    }
}

/// What a hooked harness method was called with.
#[derive(Debug, Clone, Default)]
pub(crate) struct Called {
    pub pane: Option<PaneId>,
    pub port: Option<u16>,
}

/// A test closure run once, after a named harness method returns.
pub(crate) type Hook = Box<dyn FnOnce(&Fakes, &Called) + Send>;

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
    /// The Herdr pane ids the snapshot listed before the run.
    pub before: Vec<PaneId>,
}

impl Deref for Run {
    type Target = Outcome;
    fn deref(&self) -> &Outcome {
        &self.outcome
    }
}

/// The fakes of `crate::list::Rig`, a linked host, the harness hooks and an optional real
/// scope.
pub(crate) struct Rig {
    pub fakes: Fakes,
    /// Whether `stop_owned` also kills the pane's servers (AC 22 runs unlinked).
    pub linked: bool,
    /// #663's `StoreScope` over the rig's two stores, in place of the fake scope (AC 16k).
    pub real_scope: Option<StoreScope>,
    hooks: Mutex<Vec<(HarnessOp, Hook)>>,
}

impl Rig {
    /// An empty registry and the profile `demo` with no spec (stored at generation 1).
    pub fn new() -> Self {
        Self::with(Vec::new(), vec![sample_profile("demo", &[]).unwrap()])
    }

    /// A rig whose stores hold `panes` and `profiles` (each stored at generation 1).
    pub fn with(panes: Vec<Pane>, profiles: Vec<Profile>) -> Self {
        let fakes = Fakes {
            herdr: FakeHerdr::new(SESSION)
                .with_workspace(WORKSPACE, 3, 3)
                .unwrap(),
            ..Fakes::new(panes, profiles).unwrap()
        };
        Self {
            fakes,
            linked: true,
            real_scope: None,
            hooks: Mutex::new(Vec::new()),
        }
    }

    /// The same rig with a plain `FakeHost`: `stop_owned` stops no harness server.
    pub fn unlinked(mut self) -> Self {
        self.linked = false;
        self
    }

    /// The same rig with #663's `StoreScope` (actor `holler pane`) over its two stores.
    pub fn with_store_scope(mut self) -> Self {
        let actor = Actor::parse("holler pane").unwrap();
        let profiles: std::sync::Arc<dyn ProfileStore> = self.fakes.profiles.clone();
        let panes: std::sync::Arc<dyn PaneStore> = self.fakes.panes.clone();
        self.real_scope = Some(StoreScope::new(profiles, panes, actor));
        self
    }

    /// Run `hook` once, right after the next call of `op` returns.
    pub fn after(&self, op: HarnessOp, hook: impl FnOnce(&Fakes, &Called) + Send + 'static) {
        self.hooks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((op, Box::new(hook)));
    }

    /// Call `f` with the ports a verb sees: the linked host, the hooked harness and the
    /// chosen scope over the rig's fakes.
    pub fn with_ports<R>(&self, f: impl FnOnce(Ports<'_>) -> R) -> R {
        let host = LinkedHost {
            fakes: &self.fakes,
            linked: self.linked,
        };
        let harness = HookedHarness {
            fakes: &self.fakes,
            hooks: &self.hooks,
        };
        let scope: &dyn ProfileScope = match &self.real_scope {
            Some(real) => real,
            None => &self.fakes.scope,
        };
        f(Ports {
            pane_store: self.fakes.panes.as_ref(),
            profile_store: self.fakes.profiles.as_ref(),
            herdr: &self.fakes.herdr,
            host: &host,
            harness: &harness,
            scope,
            prober: &self.fakes.prober,
        })
    }

    /// Run `holler <argv...>` over the rig, recording the calls it made.
    pub fn run(&self, argv: &[&str], format: Format) -> Run {
        let before = self.herdr_ids();
        let mark = self.mark();
        let outcome = self.with_ports(|ports| run_verb_with(argv, format, ports));
        Run {
            outcome,
            calls: self.calls_since(&mark),
            before,
        }
    }

    /// Call the engine (`f`) over the rig's ports, returning its answer and the calls made.
    pub fn engine<R>(&self, f: impl FnOnce(Ports<'_>) -> R) -> (R, Calls) {
        let mark = self.mark();
        let answer = self.with_ports(f);
        (answer, self.calls_since(&mark))
    }

    /// The pane `LAUNCH` (plus `--profile demo` when `profile`) leaves, built through the fakes'
    /// own port methods and not the verb, so a relaunch case fails only on relaunch.
    pub fn live(&self, profile: bool) -> Pane {
        let seed = Seed::default();
        self.seed_live(&Seed {
            profile: profile.then_some("demo"),
            ..seed
        })
    }

    /// A pane as a successful launch of `seed` leaves it (see [`Rig::live`]): a Herdr pane at
    /// its cell, its tmux session (running its command), a server on its port, one session
    /// the TUI shows, its record at generation 1, and, with a profile, P's spec for it.
    pub fn seed_live(&self, seed: &Seed) -> Pane {
        let f = &self.fakes;
        let pane = name(seed.pane);
        let herdr = self.place_herdr_pane(seed.grid.row, seed.grid.col);
        f.host.ensure_session(&pane, "/srv/demo").unwrap();
        if let Some(command) = &seed.command {
            f.host.run(&pane, command).unwrap();
        }
        let pid = f.harness.serve(&pane, seed.port).unwrap();
        let sid = f.harness.create_session(seed.port).unwrap();
        f.harness
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
            last_observed: LastObserved {
                shown: Some(sid),
                driven: None,
                at: 0,
            },
            profile: seed.profile.map(profile_name),
            command: seed.command.clone(),
            ..sample
        };
        let stored = f.panes.concurrent_put(&record).unwrap();
        if let Some(profile) = seed.profile {
            let edited = Profile {
                panes: vec![spec_from_pane(&stored)],
                ..self.profile(profile)
            };
            let actor = Actor::parse("holler pane").unwrap();
            f.profiles.concurrent_put(&edited, &actor).unwrap();
        }
        stored
    }

    /// The stored record of `pane` (a `get` through the port, logged).
    pub fn record(&self, pane: &str) -> Option<Pane> {
        self.fakes.panes.get(&name(pane)).unwrap()
    }

    /// The stored profile `profile` (a `get` through the port, logged).
    pub fn profile(&self, profile: &str) -> Profile {
        self.fakes
            .profiles
            .get(&profile_name(profile))
            .unwrap()
            .expect("the profile exists")
    }

    /// The change log of `profile` (a `log` through the port, logged).
    pub fn profile_log(&self, profile: &str) -> Vec<ProfileLogEntry> {
        self.fakes.profiles.log(&profile_name(profile)).unwrap()
    }

    /// Every Herdr pane the snapshot lists (a `snapshot` through the port, logged).
    pub fn herdr_panes(&self) -> Vec<HerdrPane> {
        self.fakes.herdr.snapshot().unwrap().panes
    }

    /// The ids of [`Rig::herdr_panes`].
    pub fn herdr_ids(&self) -> Vec<PaneId> {
        self.herdr_panes().into_iter().map(|p| p.pane_id).collect()
    }

    /// The Herdr pane at `grid` in `main`, if any.
    pub fn herdr_pane_at(&self, grid: GridPos) -> Option<HerdrPane> {
        self.herdr_panes()
            .into_iter()
            .find(|p| p.workspace == WORKSPACE && p.grid == grid)
    }

    /// Place a Herdr pane at `main r<row>c<col>` (setup, through the port).
    pub fn place_herdr_pane(&self, row: u16, col: u16) -> HerdrPane {
        self.fakes
            .herdr
            .ensure_pane(&HerdrSpec {
                session: SESSION.to_owned(),
                workspace: WORKSPACE.to_owned(),
                grid: GridPos { row, col },
            })
            .unwrap()
    }

    /// Serve a harness server on `port` for `pane` (setup, through the port); its pid.
    pub fn serve(&self, pane: &str, port: u16) -> u32 {
        self.fakes.harness.serve(&name(pane), port).unwrap()
    }

    /// Every call made through every port so far.
    pub fn mark(&self) -> Calls {
        let f = &self.fakes;
        Calls {
            panes: f.panes.faults().calls(),
            profiles: f.profiles.faults().calls(),
            herdr: f.herdr.faults().calls(),
            host: f.host.faults().calls(),
            harness: f.harness.faults().calls(),
            probes: f.prober.calls(),
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

    /// I4: nothing was ever typed into a pane, not even an attempt that failed.
    pub fn assert_no_keystroke(&self) {
        let typed: Vec<HerdrOp> = self
            .fakes
            .herdr
            .faults()
            .calls()
            .into_iter()
            .filter(|op| matches!(op, HerdrOp::SendText | HerdrOp::SendKeys))
            .collect();
        assert_eq!(typed, vec![], "no keystroke reaches a pane (I4)");
    }

    /// AC 2: the registry equals what the fakes observe for `pane` after `run`.
    pub fn assert_matches(&self, run: &Run, pane: &str) {
        let f = &self.fakes;
        let Some(record) = self.record(pane) else {
            let stray: Vec<PaneId> = self
                .herdr_ids()
                .into_iter()
                .filter(|id| !run.before.contains(id))
                .collect();
            assert_eq!(stray, vec![], "no record: no Herdr pane this run created");
            for port in RIG_PORTS {
                let running = f
                    .harness
                    .server(port)
                    .is_some_and(|s| s.state == ServerState::Running && s.name.as_str() == pane);
                assert!(!running, "no record: no server runs for {pane} on {port}");
            }
            match f.host.ps(&name(pane)) {
                Ok(pids) => assert_eq!(pids, Vec::<u32>::new(), "no record: no process"),
                Err(e) => assert_eq!(e.code(), "pane-not-found", "{e}"),
            }
            return;
        };
        let listed = self.herdr_panes().into_iter().any(|p| {
            p.pane_id == record.herdr.pane_id
                && p.grid == record.herdr.grid
                && p.workspace == record.herdr.workspace
        });
        assert!(
            listed,
            "Herdr lists the record's pane at its cell: {record:?}"
        );
        let server = f
            .harness
            .server(record.harness.port)
            .expect("a server on the record's port");
        assert_eq!(server.state, ServerState::Running, "{server:?}");
        assert_eq!(Some(server.pid), record.harness.pid, "the recorded pid");
        assert_eq!(server.name.as_str(), pane, "the server is the pane's");
        let shown = f.harness.tui(&record.herdr.pane_id).and_then(|t| t.shown);
        assert_eq!(
            shown, record.session_of_record,
            "the TUI shows the session of record"
        );
        assert!(
            f.host.sessions().contains(&record.name),
            "the tmux session exists"
        );
    }
}

/// Nothing live was called and neither store was written, over `calls`.
pub(crate) fn assert_untouched(calls: &Calls) {
    assert_eq!(calls.herdr, vec![], "no Herdr call");
    assert_eq!(calls.host, vec![], "no host call");
    assert_eq!(calls.harness, vec![], "no harness call");
    assert_eq!(calls.probes, vec![], "no probe run");
    let pane_writes = calls
        .panes
        .iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
        .count();
    assert_eq!(pane_writes, 0, "no pane-store write: {:?}", calls.panes);
    let profile_writes = calls
        .profiles
        .iter()
        .filter(|op| {
            matches!(
                op,
                ProfileStoreOp::CasPut | ProfileStoreOp::Delete | ProfileStoreOp::Rename
            )
        })
        .count();
    assert_eq!(profile_writes, 0, "no profile write: {:?}", calls.profiles);
}

/// The `(code, message)` of a failed JSON run, read through the test kit's envelope checker
/// (which also checks the exit code against the code's class).
pub(crate) fn error_of(run: &Outcome) -> (String, String) {
    let envelope =
        check_envelope(&run.out, run.code).unwrap_or_else(|fault| panic!("{fault}: {run:?}"));
    let error = envelope
        .error
        .unwrap_or_else(|| panic!("the run failed: {run:?}"));
    (error.code, error.message)
}

/// The `data` of a successful JSON run.
pub(crate) fn data_of(run: &Outcome) -> serde_json::Value {
    assert_eq!(run.code, 0, "the run succeeds: {run:?}");
    check_envelope(&run.out, run.code)
        .unwrap_or_else(|fault| panic!("{fault}: {run:?}"))
        .data
}

/// The effective spec `LAUNCH` builds (brief AC 16a): `demo-c1r1` at `main r2c1`, `/srv/demo`,
/// OpenCode on `fixed:48100`, the sample model and ceilings, an agent, nothing else.
pub(crate) fn launch_spec() -> ProfileSpec {
    ProfileSpec {
        pane: PANE.to_owned(),
        herdr: SpecHerdr {
            workspace: WORKSPACE.to_owned(),
            grid: GridPos { row: 2, col: 1 },
        },
        host: SpecHost {
            cwd: "/srv/demo".to_owned(),
        },
        harness: SpecHarness {
            kind: HarnessKind::Opencode,
            port_policy: "fixed:48100".to_owned(),
        },
        model: ModelSpec {
            provider: "demo-provider".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "medium".to_owned(),
        },
        // #700: `LAUNCH` passes no `--agent`, so the base spec holds none.
        opencode_agent: None,
        role: PaneRole::Agent,
        env: Vec::new(),
        context: ContextCeilings {
            soft: 100_000,
            hard: 150_000,
        },
        command: None,
        check: None,
        expect: Vec::new(),
    }
}

/// A `HostPort` over the rig's `FakeHost` whose `stop_owned` also kills the servers served
/// for the pane, when linked.
struct LinkedHost<'a> {
    fakes: &'a Fakes,
    linked: bool,
}

impl HostPort for LinkedHost<'_> {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.fakes.host.ensure_session(name, cwd)
    }

    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
        self.fakes.host.run(name, argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.fakes.host.stop_owned(name)?;
        if self.linked {
            for port in RIG_PORTS {
                if self
                    .fakes
                    .harness
                    .server(port)
                    .is_some_and(|s| s.name == *name)
                {
                    self.fakes.harness.kill(port)?;
                }
            }
        }
        Ok(())
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.fakes.host.ps(name)
    }
}

/// A `HarnessPort` over the rig's `FakeHarness` that runs a test's hook after a method.
struct HookedHarness<'a> {
    fakes: &'a Fakes,
    hooks: &'a Mutex<Vec<(HarnessOp, Hook)>>,
}

impl HookedHarness<'_> {
    /// Run (and drop) the first hook armed for `op`. No lock is held while it runs.
    fn fire(&self, op: HarnessOp, called: Called) {
        let hook = {
            let mut hooks = self.hooks.lock().unwrap_or_else(PoisonError::into_inner);
            hooks
                .iter()
                .position(|(armed, _)| *armed == op)
                .map(|at| hooks.remove(at).1)
        };
        if let Some(hook) = hook {
            hook(self.fakes, &called);
        }
    }
}

impl HarnessPort for HookedHarness<'_> {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        let answer = self.fakes.harness.serve(name, port);
        self.fire(HarnessOp::Serve, port_only(port));
        answer
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let answer = self.fakes.harness.health(port);
        self.fire(HarnessOp::Health, port_only(port));
        answer
    }

    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        let answer = self.fakes.harness.create_session(port);
        self.fire(HarnessOp::CreateSession, port_only(port));
        answer
    }

    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        let answer = self.fakes.harness.list_sessions(port);
        self.fire(HarnessOp::ListSessions, port_only(port));
        answer
    }

    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        let answer = self.fakes.harness.abort(port, session);
        self.fire(HarnessOp::Abort, port_only(port));
        answer
    }

    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        let answer = self.fakes.harness.attach_tui(pane, port, session);
        let called = Called {
            pane: Some(pane.clone()),
            port: Some(port),
        };
        self.fire(HarnessOp::AttachTui, called);
        answer
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        let answer = self.fakes.harness.select_session(pane, session);
        self.fire(HarnessOp::SelectSession, pane_only(pane));
        answer
    }

    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        let answer = self.fakes.harness.shown_session(pane);
        self.fire(HarnessOp::ShownSession, pane_only(pane));
        answer
    }
}

fn port_only(port: u16) -> Called {
    Called {
        pane: None,
        port: Some(port),
    }
}

fn pane_only(pane: &PaneId) -> Called {
    Called {
        pane: Some(pane.clone()),
        port: None,
    }
}
