//! The rig the `holler pane close` tests run over (story #646, part 2).
//!
//! Declared from `close.rs` (`pub(crate) mod rig;`). Its shape follows #644's
//! `launch/rig.rs`: the fakes of `holler-pane-testkit`, a Herdr workspace the seeded
//! panes really sit in, and a call-log span (`mark`/`calls_since`) so a case asserts
//! only the calls the verb run made — the rig's own seeding never reaches an
//! assertion. What it adds over the read-verb rig (`crate::list::Rig`) is what close
//! needs: panes with a live footprint ([`Rig::live`] — a Herdr pane at its own cell, a
//! tmux session running one command, a record whose session of record is observed
//! equal) and the per-run call lists the ordering cases read back.
//!
//! The seeded names are synthetic (`demo-c*`, sessions `ses-demo-*`): no name here can
//! name a real pane or session.

use std::ops::Deref;
use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::pane::{Health, LastObserved};
use holler_pane::{
    Argv, GridPos, HerdrPort, HerdrSpec, HostPort, Pane, PaneName, PaneStore, Ports, Profile,
    ProfileName, ProfileStore,
};
use holler_pane_testkit::envelope::{check_envelope, Envelope};
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp};
use holler_pane_testkit::host::{FakeHost, HostOp};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::{FakeProber, ProbeCall};
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

use crate::verb_harness::{run_verb_with, Outcome};

/// The Herdr session and its one workspace, the fixture's own scratch names.
pub(crate) const SESSION: &str = "scratch";
pub(crate) const WORKSPACE: &str = "scratch";

/// The calls made through every port, oldest first (a run reports only those made
/// after the mark it took).
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

impl Deref for Run {
    type Target = Outcome;

    fn deref(&self) -> &Outcome {
        &self.outcome
    }
}

/// The fakes `holler pane close` runs over.
pub(crate) struct Rig {
    pub panes: Arc<FakePaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
}

impl Rig {
    /// The rig of `profiles` (each stored at generation 1), with no pane yet.
    pub fn with_profiles(profiles: impl IntoIterator<Item = Profile>) -> Self {
        let actor = holler_pane::Actor::parse("test").unwrap();
        let panes = Arc::new(FakePaneStore::new());
        let profiles = Arc::new(FakeProfileStore::seeded(profiles, &actor).unwrap());
        let scope = FakeProfileScope::new(profiles.clone(), panes.clone(), actor);
        Self {
            panes,
            profiles,
            scope,
            herdr: FakeHerdr::new(SESSION)
                .with_workspace(WORKSPACE, 8, 3)
                .unwrap(),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
        }
    }

    /// The empty rig: no pane, no profile.
    pub fn new() -> Self {
        Self::with_profiles(Vec::new())
    }

    /// Seed a live pane `name`, a member of `profile` when given: a Herdr pane at the
    /// next free cell of `scratch`, a tmux session running one command, and a record
    /// (generation 1) that is healthy, unparked, and whose session of record
    /// `ses-<name>` is what it shows and drives. Everything a close should stop, close
    /// and delete; the seeding goes through the fakes' own port methods, before the
    /// mark every assertion reads.
    pub fn live(&self, name: &str, profile: Option<&str>) -> Pane {
        let pane_name = PaneName::parse(name).unwrap();
        let taken = u16::try_from(self.herdr.snapshot().unwrap().panes.len()).unwrap();
        let herdr = self
            .herdr
            .ensure_pane(&HerdrSpec {
                session: SESSION.to_owned(),
                workspace: WORKSPACE.to_owned(),
                grid: GridPos {
                    row: taken + 1,
                    col: 1,
                },
            })
            .unwrap();
        self.host.ensure_session(&pane_name, "/srv/demo").unwrap();
        self.host.run(&pane_name, &argv(&["opencode"])).unwrap();
        let session = format!("ses-{name}");
        let mut record = sample_pane(name).unwrap();
        record.herdr = herdr;
        record.harness.health = Health::Healthy;
        record.session_of_record = Some(session.clone());
        record.last_observed = LastObserved {
            shown: Some(session.clone()),
            driven: Some(session),
            at: 1,
        };
        record.profile = profile.map(|name| ProfileName::parse(name).unwrap());
        self.panes.concurrent_put(&record).unwrap()
    }

    /// The ports a verb sees over this rig.
    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: self.panes.as_ref(),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &self.scope,
            prober: &self.prober,
        }
    }

    /// Run `holler <argv...>` over this rig, recording the calls the run made.
    pub fn run(&self, argv: &[&str], format: Format) -> Run {
        let mark = self.mark();
        let outcome = run_verb_with(argv, format, self.ports());
        Run {
            outcome,
            calls: self.calls_since(&mark),
        }
    }

    /// The stored record of `pane`, if any (a `get` on the fake).
    pub fn record(&self, pane: &str) -> Option<Pane> {
        self.panes.get(&PaneName::parse(pane).unwrap()).unwrap()
    }

    /// The stored profile `profile`.
    pub fn profile(&self, profile: &str) -> Profile {
        self.profiles
            .get(&ProfileName::parse(profile).unwrap())
            .unwrap()
            .expect("the profile exists")
    }

    /// The change log of `profile`, oldest first.
    pub fn profile_log(&self, profile: &str) -> Vec<holler_pane::ProfileLogEntry> {
        self.profiles
            .log(&ProfileName::parse(profile).unwrap())
            .unwrap()
    }

    /// The pane `seeded` names is fully closed (the close acceptance): no record, no
    /// Herdr pane at its cell, no owned process left in its tmux session. Call this
    /// after asserting on a run's `calls`: its own reads land in the logs after them.
    pub fn assert_gone(&self, seeded: &Pane) {
        assert!(
            self.record(seeded.name.as_str()).is_none(),
            "the record is deleted"
        );
        let listed = self.herdr.snapshot().unwrap().panes;
        let ids: Vec<&str> = listed.iter().map(|pane| pane.pane_id.as_str()).collect();
        assert!(
            !ids.contains(&seeded.herdr.pane_id.as_str()),
            "the Herdr pane is closed: {ids:?}"
        );
        assert_eq!(
            self.host.ps(&seeded.name).unwrap(),
            Vec::<u32>::new(),
            "no owned process is left"
        );
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

/// One argv run in both formats, each on its own freshly seeded rig: the exit codes
/// are equal, JSON mode writes nothing on `err` and its `out` passes the test kit's
/// envelope checker (the format parity every close case is held to).
pub(crate) fn run_both(world: impl Fn() -> Rig, argv: &[&str]) -> Both {
    let text_rig = world();
    let text = text_rig.run(argv, Format::Text);
    let json_rig = world();
    let json = json_rig.run(argv, Format::Json);
    assert_eq!(
        text.code, json.code,
        "{argv:?}: exit codes are equal across formats: text {text:?}, json {json:?}"
    );
    assert_eq!(json.err, "", "{argv:?}: json: nothing on err: {json:?}");
    let envelope = check_envelope(&json.out, json.code)
        .unwrap_or_else(|fault| panic!("{argv:?}: {fault}: {json:?}"));
    Both {
        text_rig,
        text,
        json_rig,
        json,
        envelope,
    }
}

/// The two runs of [`run_both`]: each rig beside its run's outcome and calls, and the
/// JSON run's one envelope, already checked.
pub(crate) struct Both {
    pub text_rig: Rig,
    pub text: Run,
    pub json_rig: Rig,
    pub json: Run,
    pub envelope: Envelope,
}

impl Both {
    /// Each rig beside its run, text first.
    pub fn each(&self) -> impl Iterator<Item = (&Rig, &Run)> {
        [(&self.text_rig, &self.text), (&self.json_rig, &self.json)].into_iter()
    }

    /// The `(code, message)` of the JSON run's error.
    pub fn error(&self) -> (String, String) {
        let error = self
            .envelope
            .error
            .as_ref()
            .unwrap_or_else(|| panic!("a failure has an error: {}", self.json.out));
        (error.code.clone(), error.message.clone())
    }

    /// Both runs failed with `code` at exit `exit`: text mode prints one `error: `
    /// line on `err` and nothing on `out`, JSON mode the one failure envelope, the
    /// message equal in both formats. `step`, when given, is what the message must end
    /// with (ruling 7: the reconcile step of a failure after the first live call).
    pub fn assert_error(&self, code: &str, exit: i32, step: Option<&str>) {
        assert_eq!(self.text.code, exit, "{:?}", self.text);
        assert!(
            self.text.out.is_empty(),
            "text: nothing on out: {:?}",
            self.text
        );
        let (json_code, message) = self.error();
        assert_eq!(json_code, code, "{:?}", self.envelope);
        assert_eq!(
            self.text.err,
            format!("error: {message}\n"),
            "{:?}",
            self.text
        );
        if let Some(step) = step {
            assert!(
                message.ends_with(step),
                "{code}: the message ends with the reconcile step: {message}"
            );
            assert_eq!(
                message.matches("to reconcile").count(),
                1,
                "the step is never appended twice: {message}"
            );
        } else {
            assert!(
                !message.contains("to reconcile"),
                "{code}: a plan refusal carries no reconcile step: {message}"
            );
        }
    }

    /// Neither run called anything live, and neither store was written.
    pub fn assert_untouched(&self) {
        for (_, run) in self.each() {
            assert_untouched(&run.calls);
        }
    }
}

/// Nothing live was called and neither store was written, over `calls`.
pub(crate) fn assert_untouched(calls: &Calls) {
    assert_eq!(calls.herdr, vec![], "no Herdr call: {:?}", calls.herdr);
    assert_eq!(calls.host, vec![], "no host call: {:?}", calls.host);
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

/// The `data` of a successful JSON run (exit 0, nothing on `err`).
pub(crate) fn data_of(run: &Run) -> serde_json::Value {
    assert_eq!(run.code, 0, "the run succeeds: {run:?}");
    assert_eq!(run.err, "", "json: nothing on err: {run:?}");
    check_envelope(&run.out, run.code)
        .unwrap_or_else(|fault| panic!("{fault}: {run:?}"))
        .data
}

/// The sample pane `name` with no live footprint (a record only).
pub(crate) fn pane(name: &str) -> Pane {
    sample_pane(name).unwrap()
}

/// The sample profile `name` naming `panes` as its specs.
pub(crate) fn profile(name: &str, panes: &[&str]) -> Profile {
    sample_profile(name, panes).unwrap()
}

/// `to reconcile, run holler pane doctor` — the step a run that names no profile
/// prints (built by the one builder, `reconcile_step`, so no test spells it out).
pub(crate) fn bare_step() -> String {
    holler_cli::pane::profile_scope::reconcile_step(None)
}

/// The step of a run that names `profile` (doctor over P, then `profile show`).
pub(crate) fn profile_step(profile: &str) -> String {
    holler_cli::pane::profile_scope::reconcile_step(Some(&ProfileName::parse(profile).unwrap()))
}

/// `elements` as an `Argv`.
pub(crate) fn argv(elements: &[&str]) -> Argv {
    serde_json::from_value(serde_json::json!(elements)).unwrap()
}

/// The world of the `--profile` cases: `Demo Alpha` with a spec each for
/// `demo-c1r1` and `demo-c2r1`, and `demo-c1r1` live as a member.
pub(crate) fn alpha_world() -> Rig {
    let rig = Rig::with_profiles([profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"])]);
    rig.live("demo-c1r1", Some("Demo Alpha"));
    rig
}
