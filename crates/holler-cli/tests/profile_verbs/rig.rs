//! The rig the `holler profile` verb tests run over (story #662, Decision 12).
//!
//! Declared from `list.rs` by `#[path]` (so `profile_verbs/main.rs`, #670's, is not
//! edited) and reached by the other verb files as `crate::list::rig`. #664 and #665
//! extend this file, not `list.rs`.
//!
//! The two stores are the test kit's fakes. The adapters and the prober are fakes too,
//! kept only so their call logs prove a profile verb never calls them; the scope is
//! `Unwired`, so a call to it fails the verb. Each format runs on its own freshly seeded
//! rig ([`run_both`]).

use holler_cli::output::Format;
use holler_cli::pane::wiring::Unwired;
use holler_pane::profile_snapshot::fixed_port_policy;
use holler_pane::{Actor, Pane, Ports, Profile, ProfileName, ProfileSpec};
use holler_pane_testkit::envelope::{check_envelope, Envelope};
use holler_pane_testkit::fixture::{sample_pane, sample_profile, sample_spec};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::FakePaneStore;
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_store::FakeProfileStore;

use crate::verb_harness::{run_verb_with, Outcome};

static UNWIRED: Unwired = Unwired;

/// The fakes a profile verb runs over.
pub(crate) struct Rig {
    pub panes: FakePaneStore,
    pub profiles: FakeProfileStore,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
}

impl Rig {
    /// A rig whose pane store holds `panes` and whose profile store holds `profiles`,
    /// each seeded in order (so each record is stored at generation 1).
    pub fn new(
        panes: impl IntoIterator<Item = Pane>,
        profiles: impl IntoIterator<Item = Profile>,
    ) -> Self {
        let actor = Actor::parse("test seed").unwrap();
        Self {
            panes: FakePaneStore::seeded(panes).unwrap(),
            profiles: FakeProfileStore::seeded(profiles, &actor).unwrap(),
            herdr: FakeHerdr::new("scratch"),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
        }
    }

    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: &self.panes,
            profile_store: &self.profiles,
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &UNWIRED,
            prober: &self.prober,
        }
    }

    /// Run `holler <argv...>` over this rig.
    pub fn run(&self, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(argv, format, self.ports())
    }

    /// No Herdr, host, harness or probe call was made through this rig (AC 3).
    pub fn assert_no_adapter_call(&self) {
        assert_eq!(self.herdr.faults().calls(), vec![], "no Herdr call");
        assert_eq!(self.host.faults().calls(), vec![], "no host call");
        assert_eq!(self.harness.faults().calls(), vec![], "no harness call");
        assert_eq!(self.prober.calls(), vec![], "no probe run");
    }
}

/// One argv run in both formats, each on its own freshly seeded rig.
pub(crate) struct Both {
    pub text: Outcome,
    pub json: Outcome,
    /// The JSON run's one envelope, checked against its exit code.
    pub envelope: Envelope,
}

/// Run `argv` with `Format::Text` on `seed()` and with `Format::Json` on another
/// `seed()`. Asserts the exit codes are equal, the JSON output passes the test kit's
/// envelope checker, and neither run called an adapter or the prober.
pub(crate) fn run_both(seed: impl Fn() -> Rig, argv: &[&str]) -> Both {
    let text_rig = seed();
    let text = text_rig.run(argv, Format::Text);
    text_rig.assert_no_adapter_call();
    let json_rig = seed();
    let json = json_rig.run(argv, Format::Json);
    json_rig.assert_no_adapter_call();
    assert_eq!(
        text.code, json.code,
        "{argv:?}: exit codes are equal across formats: text {text:?}, json {json:?}"
    );
    let envelope = check_envelope(&json.out, json.code)
        .unwrap_or_else(|fault| panic!("{argv:?}: {fault}: {json:?}"));
    Both {
        text,
        json,
        envelope,
    }
}

/// Assert `both` is a failure coded `code` at exit `exit`: text mode prints nothing on
/// `out` and the message on `err`; JSON mode prints the envelope with that code.
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

/// A spec equal on every compared field to a seeded `pane(name)`: the kit's
/// `sample_spec` with the port policy of the sample panes' one port (Decision 3). `name`
/// is plain text, as a spec's pane is, so it need not be a valid pane name.
pub(crate) fn matching_spec(name: &str) -> ProfileSpec {
    let mut spec = sample_spec(name);
    spec.harness.port_policy = fixed_port_policy(pane("demo-c1r1").harness.port);
    spec
}

/// A profile named `name` holding `specs`, in order.
pub(crate) fn profile(name: &str, specs: Vec<ProfileSpec>) -> Profile {
    Profile {
        panes: specs,
        ..sample_profile(name, &[]).unwrap()
    }
}
