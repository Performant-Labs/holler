//! The rig the `holler pane park` and `holler pane unpark` tests run over (story #646,
//! brief Decision 11).
//!
//! Declared from `park.rs` (`pub(crate) mod rig;`) and reached by `unpark.rs` as
//! `crate::park::rig`. Its names and shapes follow #662's `profile_verbs/rig.rs`
//! (`Rig::new`, `ports`, `run`, `run_both`, `assert_failure`, `pane`, `member`), so a
//! later consolidation of the rigs is mechanical. What it adds is what park needs: a real
//! `FakeProfileScope` over the two stores (behind `Arc`s), a pane store the verb sees that
//! a test can wrap ([`Rig::wrapped`], AC 8c), `record`, and the AC 9 check.
//!
//! The adapters and the prober are fakes kept only so their call logs prove the verbs
//! never call them: park and unpark change the record and nothing live.

use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::pane::Hold;
use holler_pane::{Actor, Pane, PaneName, PaneStore, Ports, Profile, ProfileName};
use holler_pane_testkit::envelope::{check_envelope, Envelope};
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

use crate::verb_harness::{run_verb_with, Outcome};

/// AC 1's reason (`R`).
pub(crate) const R: &str = "disk full";
/// AC 1's release condition (`W`).
pub(crate) const W: &str = "after the cleanup";

/// The fakes park and unpark run over.
pub(crate) struct Rig {
    /// The fake pane store under everything: the records and its call log.
    pub panes: Arc<FakePaneStore>,
    /// The pane store the verb and the scope see: `panes` itself, or a test's wrapper
    /// of it (AC 8c).
    pub store: Arc<dyn PaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
    /// Every seeded pane as the store holds it after seeding (generation 1).
    pub seeded: Vec<Pane>,
}

impl Rig {
    /// A rig whose pane store holds `panes` and whose profile store holds `profiles`,
    /// each seeded in order (so each record is stored at generation 1), with the scope
    /// over the two.
    pub fn new(
        panes: impl IntoIterator<Item = Pane>,
        profiles: impl IntoIterator<Item = Profile>,
    ) -> Self {
        Self::wrapped(panes, profiles, |fake| fake)
    }

    /// [`Rig::new`], with the pane store the verb and the scope see being `wrap` of the
    /// fake (the fake keeps the records and its call log).
    pub fn wrapped(
        panes: impl IntoIterator<Item = Pane>,
        profiles: impl IntoIterator<Item = Profile>,
        wrap: impl FnOnce(Arc<FakePaneStore>) -> Arc<dyn PaneStore>,
    ) -> Self {
        let actor = Actor::parse("test").unwrap();
        let panes: Vec<Pane> = panes.into_iter().collect();
        let seeded = panes
            .iter()
            .map(|pane| Pane {
                generation: 1,
                ..pane.clone()
            })
            .collect();
        let fake = Arc::new(FakePaneStore::seeded(panes).unwrap());
        let store = wrap(fake.clone());
        let profiles = Arc::new(FakeProfileStore::seeded(profiles, &actor).unwrap());
        let scope = FakeProfileScope::new(profiles.clone(), store.clone(), actor);
        Self {
            panes: fake,
            store,
            profiles,
            scope,
            herdr: FakeHerdr::new("scratch"),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
            seeded,
        }
    }

    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: &*self.store,
            profile_store: &*self.profiles,
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &self.scope,
            prober: &self.prober,
        }
    }

    /// Run `holler <argv...>` over this rig.
    pub fn run(&self, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(argv, format, self.ports())
    }

    /// The stored record of `name` (a `get` on the fake, so it is in the call log; a
    /// test that asserts an empty log asserts it first).
    pub fn record(&self, name: &str) -> Pane {
        let name = PaneName::parse(name).unwrap();
        self.panes.get(&name).unwrap().expect("the record exists")
    }

    /// The record of `name` as seeded (generation 1).
    pub fn seeded(&self, name: &str) -> Pane {
        self.seeded
            .iter()
            .find(|pane| pane.name.as_str() == name)
            .unwrap_or_else(|| panic!("no seeded pane {name}"))
            .clone()
    }

    /// Every seeded record is stored exactly as it was seeded.
    pub fn assert_all_unchanged(&self) {
        for seed in &self.seeded {
            assert_eq!(
                self.record(seed.name.as_str()),
                *seed,
                "{} is unchanged",
                seed.name
            );
        }
    }

    /// The number of `cas_put` calls the fake pane store saw.
    pub fn cas_puts(&self) -> usize {
        self.panes
            .faults()
            .calls()
            .iter()
            .filter(|op| **op == PaneStoreOp::CasPut)
            .count()
    }

    /// AC 9: no Herdr, host, harness or probe call, and no profile write (`CasPut`,
    /// `Delete`, `Rename`). The profile reads a `--profile` run makes are expected.
    pub fn assert_no_live_call_and_no_profile_write(&self) {
        assert_eq!(self.herdr.faults().calls(), vec![], "no Herdr call");
        assert_eq!(self.host.faults().calls(), vec![], "no host call");
        assert_eq!(self.harness.faults().calls(), vec![], "no harness call");
        assert_eq!(self.prober.calls(), vec![], "no probe run");
        let writes: Vec<ProfileStoreOp> = self
            .profiles
            .faults()
            .calls()
            .into_iter()
            .filter(|op| {
                matches!(
                    op,
                    ProfileStoreOp::CasPut | ProfileStoreOp::Delete | ProfileStoreOp::Rename
                )
            })
            .collect();
        assert_eq!(writes, vec![], "no profile write");
    }

    /// AC 7: no call at all through any port, the two stores included.
    pub fn assert_no_call_at_all(&self) {
        self.assert_no_live_call_and_no_profile_write();
        assert_eq!(self.panes.faults().calls(), vec![], "no pane store call");
        assert_eq!(
            self.profiles.faults().calls(),
            vec![],
            "no profile store call"
        );
    }
}

/// One argv run in both formats, each on its own freshly seeded rig.
pub(crate) struct Both {
    pub text: Outcome,
    pub json: Outcome,
    /// The JSON run's one envelope, checked against its exit code.
    pub envelope: Envelope,
    pub text_rig: Rig,
    pub json_rig: Rig,
}

impl Both {
    /// The JSON failure's message.
    pub fn json_message(&self) -> &str {
        &self
            .envelope
            .error
            .as_ref()
            .expect("a failure has an error")
            .message
    }

    /// Both rigs' every record is as seeded and no `cas_put` reached the fake (AC 5-7).
    pub fn assert_nothing_written(&self) {
        for rig in [&self.text_rig, &self.json_rig] {
            assert_eq!(rig.cas_puts(), 0, "no cas_put");
            rig.assert_all_unchanged();
        }
    }
}

/// Run `argv` with `Format::Text` on `seed()` and with `Format::Json` on another
/// `seed()`. Asserts (AC 9, AC 10) the exit codes are equal, the JSON run wrote nothing
/// on `err` and its `out` passes the test kit's envelope checker, and neither run called
/// an adapter or the prober or wrote a profile.
pub(crate) fn run_both(seed: impl Fn() -> Rig, argv: &[&str]) -> Both {
    let text_rig = seed();
    let text = text_rig.run(argv, Format::Text);
    text_rig.assert_no_live_call_and_no_profile_write();
    let json_rig = seed();
    let json = json_rig.run(argv, Format::Json);
    json_rig.assert_no_live_call_and_no_profile_write();
    assert_eq!(
        text.code, json.code,
        "{argv:?}: exit codes are equal across formats: text {text:?}, json {json:?}"
    );
    assert_eq!(json.err, "", "{argv:?}: json: nothing on err: {json:?}");
    let envelope = check_envelope(&json.out, json.code)
        .unwrap_or_else(|fault| panic!("{argv:?}: {fault}: {json:?}"));
    Both {
        text,
        json,
        envelope,
        text_rig,
        json_rig,
    }
}

/// Assert `both` is a failure coded `code` at exit `exit`: text mode prints nothing on
/// `out` and an `error: ` line on `err`; JSON mode prints the envelope with that code.
pub(crate) fn assert_failure(both: &Both, code: &str, exit: i32) {
    assert_eq!(both.text.code, exit, "{:?}", both.text);
    assert!(
        both.text.out.is_empty(),
        "text: nothing on out: {:?}",
        both.text
    );
    assert!(both.text.err.starts_with("error: "), "{:?}", both.text);
    let error = both
        .envelope
        .error
        .as_ref()
        .expect("a failure has an error");
    assert_eq!(error.code, code, "{:?}", both.envelope);
}

/// [`assert_failure`], and the message is exactly `message` in both formats.
pub(crate) fn assert_failure_message(both: &Both, code: &str, exit: i32, message: &str) {
    assert_failure(both, code, exit);
    assert_eq!(
        both.text.err,
        format!("error: {message}\n"),
        "{:?}",
        both.text
    );
    assert_eq!(both.json_message(), message, "{:?}", both.envelope);
}

/// The sample pane `name` (a valid name is a test's own constant).
pub(crate) fn pane(name: &str) -> Pane {
    sample_pane(name).unwrap()
}

/// The sample pane `name`, a member of `profile`.
pub(crate) fn member(name: &str, profile: &str) -> Pane {
    Pane {
        profile: Some(ProfileName::parse(profile).unwrap()),
        ..pane(name)
    }
}

/// `pane` with its hold replaced by `hold` (built before seeding, brief E-7).
pub(crate) fn held(mut pane: Pane, hold: Hold) -> Pane {
    pane.hold = hold;
    pane
}

/// AC 3's earlier park: reason `old`, release `later`, since 5.
pub(crate) fn old_park() -> Hold {
    Hold::Parked {
        reason: "old".into(),
        release_when: "later".into(),
        since: 5,
    }
}

/// The sample profile `name` naming `panes` as its specs.
pub(crate) fn profile(name: &str, panes: &[&str]) -> Profile {
    sample_profile(name, panes).unwrap()
}

/// AC 4's world: `Demo Alpha` (`demo-c1r1`, `demo-c2r1`, seeded in that reverse order),
/// `demo-c3r1` in no profile, `demo-c4r1` in `Demo Beta`, and the memberless
/// `Demo Empty`. Every pane has no hold.
pub(crate) fn profile_world() -> Rig {
    Rig::new(
        [
            member("demo-c2r1", "Demo Alpha"),
            member("demo-c1r1", "Demo Alpha"),
            pane("demo-c3r1"),
            member("demo-c4r1", "Demo Beta"),
        ],
        [
            profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]),
            profile("Demo Beta", &["demo-c4r1"]),
            profile("Demo Empty", &[]),
        ],
    )
}

/// The two verbs, for the cases they share (AC 5-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Park,
    Unpark,
}

impl Verb {
    /// `pane <verb> <target...>`, with park's `--reason R --release-when W`.
    pub fn argv<'a>(self, target: &[&'a str]) -> Vec<&'a str> {
        let mut argv = vec!["pane", self.word()];
        argv.extend_from_slice(target);
        if self == Verb::Park {
            argv.extend_from_slice(&["--reason", R, "--release-when", W]);
        }
        argv
    }

    /// The verb itself.
    pub fn word(self) -> &'static str {
        match self {
            Verb::Park => "park",
            Verb::Unpark => "unpark",
        }
    }

    /// The word of the partial-failure suffix (Decision 7).
    pub fn done(self) -> &'static str {
        match self {
            Verb::Park => "parked",
            Verb::Unpark => "unparked",
        }
    }

    /// The hold a pane this verb changes starts from (none for park, parked for unpark).
    pub fn changeable(self) -> Hold {
        match self {
            Verb::Park => Hold::None,
            Verb::Unpark => old_park(),
        }
    }

    /// The hold that this verb leaves alone as "already" in the asked state.
    pub fn already(self) -> Hold {
        match self {
            Verb::Park => old_park(),
            Verb::Unpark => Hold::None,
        }
    }

    /// The text line of a pane this run changed (park with `R` and `W`).
    pub fn done_line(self, name: &str) -> String {
        match self {
            Verb::Park => format!("{name}: parked (reason \"{R}\", release when \"{W}\")"),
            Verb::Unpark => format!("{name}: unparked"),
        }
    }

    /// The text line of a pane an earlier run of this verb changed, when this run
    /// leaves it alone.
    pub fn done_line_again(self, name: &str) -> String {
        match self {
            Verb::Park => format!("{name}: already parked (reason \"{R}\", release when \"{W}\")"),
            Verb::Unpark => format!("{name}: not parked"),
        }
    }

    /// Whether `hold` is the state this verb puts a pane in (park: `R`/`W`, unpark: none).
    pub fn is_done(self, hold: &Hold) -> bool {
        match (self, hold) {
            (
                Verb::Park,
                Hold::Parked {
                    reason,
                    release_when,
                    ..
                },
            ) => reason == R && release_when == W,
            (Verb::Unpark, Hold::None) => true,
            _ => false,
        }
    }
}
