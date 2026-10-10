//! The doctor rig (#647, brief Decision 12): a live world on the test kit's fakes, with one
//! record per pane that matches it.
//!
//! For each seeded pane it makes a Herdr pane (`ensure_pane`), a tmux session
//! (`ensure_session`), a harness server on its own port (`serve`, 48100 + index), a
//! session on it (`create_session`) and a TUI showing that session (`attach_tui`), then
//! stores a `sample_pane` naming all of them, with that session as its session of record.
//! A freshly built rig is a healthy fleet: doctor reports nothing about it.
//!
//! The setup goes through the ports, so it is in the fakes' call logs: a test that asserts
//! on the calls doctor made takes a [`Rig::mark`] first and reads [`Rig::calls_since`].

use std::collections::BTreeMap;
use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::findings::{Finding, FindingKind};
use holler_pane::pane::PaneRole;
use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
use holler_pane::{
    Actor, GridPos, HarnessPort, HerdrPane, HerdrPort, HerdrSpec, HostPort, Pane, PaneId, PaneName,
    PaneStore, Ports, ProfileName,
};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp};
use holler_pane_testkit::host::{FakeHost, HostOp};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};
use serde_json::Value;

use crate::verb_harness::{run_verb_with, Outcome};

/// The clock every direct `reconcile` call of these tests passes as `now_ms`.
pub const NOW: i64 = 1_760_000_000_000;

/// The port of the first seeded pane; the pane at index `i` is on `FIRST_PORT + i`.
pub const FIRST_PORT: u16 = 48100;

/// A port no record names, for a server the record does not know (incident 3).
pub const FOREIGN_PORT: u16 = 48150;

/// The Herdr session and workspace of the rig (the test kit's scratch name).
const SCRATCH: &str = "scratch";

/// One pane to seed.
#[derive(Debug, Clone)]
pub struct Seed {
    pub name: &'static str,
    pub grid: GridPos,
    pub role: PaneRole,
    pub profile: Option<&'static str>,
    /// A data directory of the pane's server's own (default: the shared `"default"`).
    pub data_dir: Option<&'static str>,
}

impl Seed {
    /// An agent pane at `r<row>c<col>`, in no profile, on the shared data directory.
    pub fn new(name: &'static str, row: u16, col: u16) -> Self {
        Self {
            name,
            grid: GridPos { row, col },
            role: PaneRole::Agent,
            profile: None,
            data_dir: None,
        }
    }

    pub fn orchestrator(mut self) -> Self {
        self.role = PaneRole::Orchestrator;
        self
    }

    pub fn in_profile(mut self, profile: &'static str) -> Self {
        self.profile = Some(profile);
        self
    }

    pub fn data_dir(mut self, dir: &'static str) -> Self {
        self.data_dir = Some(dir);
        self
    }
}

/// What the rig made live for one pane.
#[derive(Debug, Clone)]
pub struct Live {
    pub name: PaneName,
    pub herdr: HerdrPane,
    pub port: u16,
    /// The session the rig created and recorded as the pane's session of record.
    pub session: String,
}

/// The fakes, the records and the live world of each pane.
pub struct Rig {
    pub panes: Arc<FakePaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
    pub live: BTreeMap<&'static str, Live>,
}

/// The calls made through every port, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Calls {
    pub panes: Vec<PaneStoreOp>,
    pub profiles: Vec<ProfileStoreOp>,
    pub herdr: Vec<HerdrOp>,
    pub host: Vec<HostOp>,
    pub harness: Vec<HarnessOp>,
    pub probes: usize,
}

impl Rig {
    /// The healthy fleet of `seeds` (see the module docs).
    pub fn new(seeds: &[Seed]) -> Self {
        let herdr = FakeHerdr::new(SCRATCH)
            .with_workspace(SCRATCH, 3, 3)
            .expect("the rig's workspace");
        let (host, harness) = (FakeHost::new(), FakeHarness::new());
        let mut live = BTreeMap::new();
        let mut records = Vec::new();
        for (index, seed) in seeds.iter().enumerate() {
            let port = FIRST_PORT + u16::try_from(index).expect("a small fleet");
            let made = make_live(&herdr, &host, &harness, seed, port);
            records.push(record_of(seed, &made));
            live.insert(seed.name, made);
        }
        let actor = Actor::parse("doctor-test").expect("an actor");
        let panes = Arc::new(FakePaneStore::seeded(records).expect("seed the panes"));
        let profiles = Arc::new(
            FakeProfileStore::seeded(profiles_of(seeds), &actor).expect("seed the profiles"),
        );
        let scope = FakeProfileScope::new(profiles.clone(), panes.clone(), actor);
        Self {
            panes,
            profiles,
            scope,
            herdr,
            host,
            harness,
            prober: FakeProber::new(),
            live,
        }
    }

    /// The ports over the rig's fakes.
    pub fn ports(&self) -> Ports<'_> {
        self.ports_with(&self.harness)
    }

    /// The ports over the rig's fakes, with `harness` in place of the rig's own.
    pub fn ports_with<'a>(&'a self, harness: &'a dyn HarnessPort) -> Ports<'a> {
        Ports {
            pane_store: &*self.panes,
            profile_store: &*self.profiles,
            herdr: &self.herdr,
            host: &self.host,
            harness,
            scope: &self.scope,
            prober: &self.prober,
        }
    }

    pub fn live(&self, name: &str) -> &Live {
        self.live
            .get(name)
            .unwrap_or_else(|| panic!("no seeded pane {name}"))
    }

    pub fn pane_id(&self, name: &str) -> PaneId {
        self.live(name).herdr.pane_id.clone()
    }

    /// The stored record of `name` (a `get` through the port, so it is in the log).
    pub fn record(&self, name: &str) -> Pane {
        let name = PaneName::parse(name).expect("a pane name");
        self.panes
            .get(&name)
            .expect("the store answers")
            .expect("the record exists")
    }

    /// Another writer changes the record of `name` (outside the call log).
    pub fn rewrite(&self, name: &str, change: impl FnOnce(&mut Pane)) {
        let mut pane = self.record(name);
        change(&mut pane);
        self.panes
            .concurrent_put(&pane)
            .expect("rewrite the record");
    }

    /// Incident 1's move: a fresh session on the pane's own server, and the pane's TUI
    /// moved to it by hand. Returns the new session.
    pub fn move_tui_to_new_session(&self, name: &str) -> String {
        let live = self.live(name);
        let fresh = self.harness.seed_session(live.port);
        self.harness
            .navigate(&live.herdr.pane_id, Some(&fresh))
            .expect("move the TUI");
        fresh
    }

    /// Incident 3's move: a bare harness server that no record names, on
    /// [`FOREIGN_PORT`] with a data directory of its own, and the TUI of `name` attached
    /// to a session there. Returns that session.
    pub fn attach_foreign_tui(&self, name: &str) -> String {
        let bare = PaneName::parse("bare-opencode").expect("a pane name");
        self.harness.set_data_dir(FOREIGN_PORT, "foreign");
        self.harness.serve(&bare, FOREIGN_PORT).expect("serve");
        let foreign = self
            .harness
            .create_session(FOREIGN_PORT)
            .expect("a session");
        self.harness
            .attach_tui(&self.pane_id(name), FOREIGN_PORT, &foreign)
            .expect("attach the TUI");
        foreign
    }

    /// A Herdr pane that no record names, at `r<row>c<col>`.
    pub fn unregistered_herdr_pane(&self, row: u16, col: u16) -> HerdrPane {
        self.herdr
            .ensure_pane(&HerdrSpec {
                session: SCRATCH.to_owned(),
                workspace: SCRATCH.to_owned(),
                grid: GridPos { row, col },
            })
            .expect("place the unregistered pane")
    }

    /// One reconcile pass over the rig's ports; the pass must complete.
    pub fn run(&self, request: &ReconcileRequest<'_>) -> Report {
        run_over(self.ports(), request)
    }

    /// `holler <argv...>` through the verb, over the rig's ports.
    pub fn verb(&self, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(argv, format, self.ports())
    }

    /// Every call made through every port so far.
    pub fn mark(&self) -> Calls {
        Calls {
            panes: self.panes.faults().calls(),
            profiles: self.profiles.faults().calls(),
            herdr: self.herdr.faults().calls(),
            host: self.host.faults().calls(),
            harness: self.harness.faults().calls(),
            probes: self.prober.calls().len(),
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
            probes: now.probes - mark.probes,
        }
    }
}

/// One reconcile pass over `ports`; the pass must complete.
pub fn run_over(ports: Ports<'_>, request: &ReconcileRequest<'_>) -> Report {
    reconcile(ports, request)
        .unwrap_or_else(|e| panic!("reconcile must complete the pass and report: {e}"))
}

/// A whole-fleet request at [`NOW`].
pub fn whole(fix: bool) -> ReconcileRequest<'static> {
    ReconcileRequest {
        profile: None,
        pane: None,
        fix,
        now_ms: NOW,
    }
}

/// Each finding as `(kind, pane, session)`, in report order.
pub fn keys(report: &Report) -> Vec<(&'static str, Option<String>, Option<String>)> {
    report
        .findings
        .iter()
        .map(|f| {
            (
                f.kind.code(),
                f.pane.as_ref().map(ToString::to_string),
                f.session.clone(),
            )
        })
        .collect()
}

/// The kinds of the findings, in report order.
pub fn kinds(report: &Report) -> Vec<&'static str> {
    report.findings.iter().map(|f| f.kind.code()).collect()
}

/// The findings of `kind`.
pub fn of_kind(report: &Report, kind: FindingKind) -> Vec<&Finding> {
    report.findings.iter().filter(|f| f.kind == kind).collect()
}

/// The one finding of `kind` about the pane `pane`; fails unless there is exactly one.
pub fn one<'r>(report: &'r Report, kind: FindingKind, pane: &str) -> &'r Finding {
    let found: Vec<&Finding> = of_kind(report, kind)
        .into_iter()
        .filter(|f| f.pane.as_ref().map(PaneName::as_str) == Some(pane))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one {} for {pane}; findings: {:?}",
        kind.code(),
        keys(report)
    );
    found[0]
}

/// The `data` of a JSON-mode run that must have succeeded (exit 0, a valid envelope).
pub fn json_data(run: &Outcome) -> Value {
    assert_eq!(run.code, 0, "the pass completes: exit 0: {run:?}");
    let envelope = check_envelope(&run.out, run.code)
        .unwrap_or_else(|e| panic!("a valid envelope: {e}; out {:?}", run.out));
    assert!(
        run.err.is_empty(),
        "JSON mode writes nothing on err: {run:?}"
    );
    envelope.data
}

/// The JSON findings of `data` whose `kind` is `code`.
pub fn json_findings<'v>(data: &'v Value, code: &str) -> Vec<&'v Value> {
    data["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("data.findings is an array: {data}"))
        .iter()
        .filter(|f| f["kind"] == code)
        .collect()
}

fn make_live(
    herdr: &FakeHerdr,
    host: &FakeHost,
    harness: &FakeHarness,
    seed: &Seed,
    port: u16,
) -> Live {
    let name = PaneName::parse(seed.name).expect("a pane name");
    let spec = HerdrSpec {
        session: SCRATCH.to_owned(),
        workspace: SCRATCH.to_owned(),
        grid: seed.grid,
    };
    let pane = herdr.ensure_pane(&spec).expect("place the pane");
    host.ensure_session(&name, "/srv/demo")
        .expect("the tmux session");
    if let Some(dir) = seed.data_dir {
        harness.set_data_dir(port, dir);
    }
    harness.serve(&name, port).expect("serve");
    let session = harness.create_session(port).expect("a session");
    harness
        .attach_tui(&pane.pane_id, port, &session)
        .expect("attach the TUI");
    Live {
        name,
        herdr: pane,
        port,
        session,
    }
}

fn record_of(seed: &Seed, live: &Live) -> Pane {
    let mut pane = sample_pane(seed.name).expect("a sample pane");
    pane.herdr = live.herdr.clone();
    pane.harness.port = live.port;
    pane.session_of_record = Some(live.session.clone());
    pane.role = seed.role;
    pane.profile = seed
        .profile
        .map(|p| ProfileName::parse(p).expect("a profile name"));
    pane
}

fn profiles_of(seeds: &[Seed]) -> Vec<holler_pane::Profile> {
    let mut members: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for seed in seeds {
        if let Some(profile) = seed.profile {
            members.entry(profile).or_default().push(seed.name);
        }
    }
    members
        .iter()
        .map(|(name, panes)| sample_profile(name, panes).expect("a sample profile"))
        .collect()
}
