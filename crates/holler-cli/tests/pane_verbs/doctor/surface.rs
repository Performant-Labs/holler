//! `holler pane doctor`'s scope, output and remedies (#647, ACs 14-24 and 26).

use std::collections::BTreeSet;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use holler_cli::output::Format;
use holler_pane::findings::FindingKind as K;
use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
use holler_pane::{Argv, EnvVarName, HarnessPort, PaneError, PaneId, PaneName, ProfileName};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::{HerdrVersion, PROTOCOL_22_VERSION, SUPPORTED_VERSIONS};
use holler_pane_testkit::host::HostOp;
use serde_json::{json, Value};

use super::rig::{json_data, json_findings, keys, of_kind, one, whole, Rig, Seed};
use crate::verb_harness::parse::try_parse;
use crate::verb_harness::Outcome;

const P: &str = "demo";
const Q: &str = "qa";
const P_PANE: &str = "demo-c1r1";
const Q_PANE: &str = "demo-c2r1";

fn profile(name: &str) -> ProfileName {
    ProfileName::parse(name).expect("a profile name")
}

fn in_profile(profile: &ProfileName) -> ReconcileRequest<'_> {
    ReconcileRequest {
        profile: Some(profile),
        ..whole(false)
    }
}

/// Every pane a finding names, and the panes the report checked.
fn panes_named(report: &Report) -> (BTreeSet<String>, Vec<String>) {
    let named = report
        .findings
        .iter()
        .filter_map(|f| f.pane.as_ref().map(ToString::to_string))
        .collect();
    let checked = report.panes.iter().map(|p| p.name.to_string()).collect();
    (named, checked)
}

fn stray_sessions(report: &Report) -> Vec<String> {
    of_kind(report, K::StraySession)
        .iter()
        .filter_map(|f| f.session.clone())
        .collect()
}

// --- Scope ----------------------------------------------------------------------------

/// AC 14 (a): P's run sees P's panes and P's servers only.
#[test]
fn doctor_profile_reports_only_its_panes() {
    let rig = Rig::new(&[
        Seed::new(P_PANE, 1, 1).in_profile(P),
        Seed::new(Q_PANE, 1, 2).in_profile(Q).data_dir("qa-dir"),
    ]);
    rig.move_tui_to_new_session(P_PANE);
    let q_moved = rig.move_tui_to_new_session(Q_PANE);
    let q_stray = rig.harness.seed_session(rig.live(Q_PANE).port);
    rig.unregistered_herdr_pane(3, 3);

    let p = profile(P);
    let report = rig.run(&in_profile(&p));

    let (named, checked) = panes_named(&report);
    assert_eq!(
        named,
        BTreeSet::from([P_PANE.to_owned()]),
        "{:?}",
        keys(&report)
    );
    assert_eq!(checked, [P_PANE]);
    one(&report, K::ShownDrivenMismatch, P_PANE);
    assert!(
        of_kind(&report, K::UnregisteredHerdrPane).is_empty(),
        "{:?}",
        keys(&report)
    );
    let strays = stray_sessions(&report);
    assert!(
        !strays.contains(&q_stray) && !strays.contains(&q_moved),
        "{strays:?}"
    );
}

/// AC 14 (b): on a shared data directory, another profile's session of record is not a
/// stray, though P's server lists it.
#[test]
fn another_profiles_session_of_record_is_not_a_stray() {
    let rig = Rig::new(&[
        Seed::new(P_PANE, 1, 1).in_profile(P),
        Seed::new(Q_PANE, 1, 2).in_profile(Q),
    ]);
    let q_session = rig.live(Q_PANE).session.clone();
    let listed = rig
        .harness
        .list_sessions(rig.live(P_PANE).port)
        .expect("list");
    assert!(
        listed.contains(&q_session),
        "precondition: P's server lists Q's session"
    );

    let p = profile(P);
    let report = rig.run(&in_profile(&p));

    assert!(
        !stray_sessions(&report).contains(&q_session),
        "{:?}",
        keys(&report)
    );
}

fn two_profiles() -> Rig {
    Rig::new(&[
        Seed::new(P_PANE, 1, 1).in_profile(P),
        Seed::new(Q_PANE, 1, 2).in_profile(Q),
    ])
}

/// Run `argv` in both formats over `rig`: the JSON envelope is valid and both formats
/// exit alike (AC 16). Returns `(text, json)`.
fn both_formats(rig: &Rig, argv: &[&str]) -> (Outcome, Outcome) {
    let text = rig.verb(argv, Format::Text);
    let json = rig.verb(argv, Format::Json);
    check_envelope(&json.out, json.code)
        .unwrap_or_else(|e| panic!("{argv:?}: a valid envelope: {e}; {json:?}"));
    assert_eq!(
        text.code, json.code,
        "{argv:?}: text {text:?} vs json {json:?}"
    );
    (text, json)
}

/// A refusal in both formats: its exit code, text on `err` only, the envelope's code.
fn assert_refused(rig: &Rig, argv: &[&str], code: &str, exit: i32) {
    let (text, json) = both_formats(rig, argv);
    assert_eq!(json.code, exit, "{argv:?}: {json:?}");
    assert!(
        text.out.is_empty() && text.err.starts_with("error: "),
        "{argv:?}: {text:?}"
    );
    let envelope = check_envelope(&json.out, json.code).expect("checked above");
    let error = envelope.error.expect("a failure has an error");
    assert_eq!(error.code, code, "{argv:?}: {json:?}");
}

/// AC 15: a named pane scopes the run; bad names and scopes are refused.
#[test]
fn doctor_named_pane_scopes() {
    let rig = two_profiles();
    rig.move_tui_to_new_session(P_PANE);
    rig.move_tui_to_new_session(Q_PANE);
    rig.unregistered_herdr_pane(3, 3);
    let name = PaneName::parse(P_PANE).expect("a pane name");

    let report = rig.run(&ReconcileRequest {
        pane: Some(&name),
        ..whole(false)
    });

    let (named, checked) = panes_named(&report);
    assert_eq!(
        named,
        BTreeSet::from([P_PANE.to_owned()]),
        "{:?}",
        keys(&report)
    );
    assert_eq!(checked, [P_PANE]);
    assert!(of_kind(&report, K::UnregisteredHerdrPane).is_empty());

    assert_refused(&rig, &["pane", "doctor", "demo-c9r9"], "pane-not-found", 3);
    assert_refused(
        &rig,
        &["pane", "doctor", "--profile", P, Q_PANE],
        "pane-not-in-profile",
        3,
    );
    let missing = ["pane", "doctor", "--profile", "missing"];
    assert_refused(&rig, &missing, "profile-not-found", 3);
    assert_refused(&rig, &["pane", "doctor", "BAD_NAME"], "usage", 2);
}

/// AC 16: the rigs of ACs 1, 2, 4 and 14 complete with exit 0 in both formats.
#[test]
fn doctor_envelope_and_exit_codes_match_across_formats() {
    let mismatch = two_profiles();
    mismatch.move_tui_to_new_session(P_PANE);
    let wedged = two_profiles();
    wedged
        .harness
        .freeze(wedged.live(P_PANE).port)
        .expect("freeze");
    let down = two_profiles();
    down.harness.kill(down.live(P_PANE).port).expect("kill");
    for (rig, argv) in [
        (&mismatch, &["pane", "doctor"][..]),
        (&wedged, &["pane", "doctor"]),
        (&down, &["pane", "doctor"]),
        (&mismatch, &["pane", "doctor", "--profile", P]),
    ] {
        let (_, json) = both_formats(rig, argv);
        assert_eq!(json.code, 0, "{argv:?}: {json:?}");
    }
}

/// AC 17: a store that cannot be read fails the run, in one error envelope.
#[test]
fn doctor_store_failure_is_an_error() {
    let rig = two_profiles();
    let unavailable = PaneError::Unavailable {
        what: "the hub".to_owned(),
    };
    rig.panes.faults().set(Some(Fault::Fail(unavailable)));

    let (_, json) = both_formats(&rig, &["pane", "doctor"]);

    assert_eq!(json.code, 1, "{json:?}");
    let envelope = check_envelope(&json.out, json.code).expect("checked above");
    assert_eq!(envelope.data, Value::Null);
    assert_eq!(envelope.error.expect("an error").code, "unavailable");
}

// --- Output ---------------------------------------------------------------------------

/// AC 18: positions print as ROWCOL, and a pane's name is never read as one.
#[test]
fn finding_prints_rowcol() {
    let rig = Rig::new(&[Seed::new("demo-c1r2", 2, 1)]);
    rig.move_tui_to_new_session("demo-c1r2");

    let text = rig.verb(&["pane", "doctor"], Format::Text);
    let json = rig.verb(&["pane", "doctor"], Format::Json);

    assert_eq!(text.code, 0, "{text:?}");
    let prefix = "demo-c1r2 r2c1 shown-driven-mismatch: ";
    assert!(
        text.out.lines().any(|l| l.starts_with(prefix)),
        "{}",
        text.out
    );
    assert!(
        !text.out.replace("demo-c1r2", "").contains("c1r2"),
        "{}",
        text.out
    );
    let data = json_data(&json);
    let grid = json!({"row": 2, "col": 1, "pos": "r2c1"});
    assert_eq!(
        json_findings(&data, "shown-driven-mismatch")[0]["grid"],
        grid
    );
    assert_eq!(data["panes"][0]["grid"], grid, "{data}");
}

fn key_set(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("an object: {value}"))
        .keys()
        .map(String::as_str)
        .collect()
}

/// AC 19: the shape of `data`, with every absent value a `null`.
#[test]
fn json_data_shape_is_pinned() {
    let rig = Rig::new(&[Seed::new(P_PANE, 1, 1)]);
    rig.move_tui_to_new_session(P_PANE);

    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));

    let top = [
        "scope",
        "fix_requested",
        "herdr",
        "hosts",
        "panes",
        "findings",
    ];
    assert_eq!(key_set(&data), BTreeSet::from(top), "{data}");
    let finding = [
        "kind",
        "pane",
        "grid",
        "herdr_pane",
        "session",
        "ports",
        "message",
        "remedy",
        "fix",
        "fix_error",
    ];
    let findings = data["findings"].as_array().expect("findings");
    assert_eq!(findings.len(), 2, "{data}");
    for f in findings {
        assert_eq!(key_set(f), BTreeSet::from(finding), "{f}");
    }
}

/// AC 20: Herdr's version and the recorded hosts are shown; an unsupported Herdr is a
/// finding, and the pass goes on.
#[test]
fn herdr_version_shown_and_unsupported_reported() {
    let rig = Rig::new(&[
        Seed::new(P_PANE, 1, 1),
        Seed::new(Q_PANE, 1, 2),
        Seed::new("demo-c3r1", 1, 3),
    ]);
    rig.rewrite("demo-c3r1", |pane| {
        pane.host.name = "demo-host".to_owned();
        pane.host.herdr_api_version = Some("22".to_owned());
    });

    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));
    assert_eq!(data["herdr"]["version"], PROTOCOL_22_VERSION, "{data}");
    let hosts = json!([
        {"name": "demo-host", "herdr_api_version": "22"},
        {"name": "localhost", "herdr_api_version": null},
    ]);
    assert_eq!(data["hosts"], hosts, "each host once: {data}");

    rig.herdr.set_version(HerdrVersion::Unsupported);
    rig.move_tui_to_new_session(P_PANE);
    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));
    let unsupported = json_findings(&data, "herdr-version-unsupported");
    assert_eq!(unsupported.len(), 1, "{data}");
    assert_eq!(unsupported[0]["pane"], Value::Null);
    let message = unsupported[0]["message"].as_str().expect("a message");
    assert!(message.contains(SUPPORTED_VERSIONS), "{message:?}");
    assert_eq!(data["panes"].as_array().map(Vec::len), Some(3), "{data}");
    assert_eq!(
        json_findings(&data, "shown-driven-mismatch").len(),
        1,
        "{data}"
    );
}

/// AC 21: Herdr not answering is two findings, and nothing is guessed from it.
#[test]
fn observe_failure_is_a_finding_not_an_abort() {
    let rig = two_profiles();
    rig.move_tui_to_new_session(P_PANE);
    rig.unregistered_herdr_pane(3, 3);
    rig.herdr.vanish(&rig.pane_id(Q_PANE)).expect("vanish");
    rig.herdr.faults().set(Some(Fault::Wedged));

    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));

    let failed = json_findings(&data, "observe-failed");
    assert_eq!(failed.len(), 2, "{data}");
    for op in ["herdr.snapshot", "herdr.version"] {
        let named = failed
            .iter()
            .filter(|f| f["message"].as_str().is_some_and(|m| m.contains(op)));
        assert_eq!(named.count(), 1, "one observe-failed names {op}: {data}");
    }
    assert_eq!(data["herdr"]["version"], Value::Null);
    assert!(
        json_findings(&data, "herdr-pane-missing").is_empty(),
        "{data}"
    );
    assert!(
        json_findings(&data, "unregistered-herdr-pane").is_empty(),
        "{data}"
    );
    assert_eq!(
        json_findings(&data, "shown-driven-mismatch").len(),
        1,
        "{data}"
    );
    assert_eq!(data["panes"].as_array().map(Vec::len), Some(2), "{data}");
}

const SENTINEL: &str = "doctor-must-not-print-this-647";
const ENV_NAME: &str = "HLR_SENTINEL_647";

/// Every stream of `doctor` and `doctor --fix`, in both formats, over `rig` as it is now.
fn every_output(rig: &Rig) -> Vec<String> {
    let mut streams = Vec::new();
    for argv in [&["pane", "doctor"][..], &["pane", "doctor", "--fix"]] {
        for format in [Format::Text, Format::Json] {
            let run = rig.verb(argv, format);
            streams.extend([run.out, run.err]);
        }
    }
    streams
}

/// AC 22: the stored command (and the environment) never reach the output.
#[test]
fn doctor_output_holds_no_secret() {
    let rig = Rig::new(&[Seed::new(P_PANE, 1, 1)]);
    rig.rewrite(P_PANE, |pane| {
        let argv = ["prog", "--opt", SENTINEL].map(str::to_owned).to_vec();
        pane.command = Some(Argv::new(argv));
        pane.env = vec![EnvVarName::parse(ENV_NAME).expect("an env name")];
    });

    let mut streams = every_output(&rig);
    rig.move_tui_to_new_session(P_PANE);
    streams.extend(every_output(&rig));
    rig.harness.kill(rig.live(P_PANE).port).expect("kill");
    streams.extend(every_output(&rig));
    rig.herdr.faults().set(Some(Fault::Wedged));
    streams.extend(every_output(&rig));

    assert!(
        streams.iter().any(|s| s.contains(P_PANE)),
        "the runs reported the pane"
    );
    for stream in &streams {
        assert!(!stream.contains(SENTINEL), "the command leaked: {stream}");
        assert!(!stream.contains(ENV_NAME), "the env leaked: {stream}");
    }
}

/// AC 23: a control sequence in a record never reaches the terminal raw.
#[test]
fn text_output_escapes_control_characters() {
    let hostile = "demo\u{1b}[2J\nhost";
    let rig = Rig::new(&[Seed::new(P_PANE, 1, 1)]);
    rig.rewrite(P_PANE, |pane| pane.host.name = hostile.to_owned());

    let text = rig.verb(&["pane", "doctor"], Format::Text);

    assert_eq!(text.code, 0, "{text:?}");
    assert!(
        !text.out.contains('\u{1b}') && !text.err.contains('\u{1b}'),
        "{text:?}"
    );
    let lines: Vec<&str> = text.out.lines().filter(|l| l.contains("[2J")).collect();
    assert_eq!(lines.len(), 1, "the host is one line: {}", text.out);
    let after = lines[0].split("[2J").nth(1).unwrap_or_default();
    assert!(
        lines[0].contains("demo") && after.contains("host"),
        "{}",
        lines[0]
    );

    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));
    assert_eq!(
        data["hosts"][0]["name"], hostile,
        "JSON keeps the raw value: {data}"
    );
}

/// A `HarnessPort` over the rig's `FakeHarness` whose `health` holds each call until
/// `want` calls are in flight at once (at most two seconds), recording the most seen.
struct HealthGate<'a> {
    inner: &'a FakeHarness,
    want: usize,
    state: Mutex<(usize, usize)>,
    arrived: Condvar,
}

impl HealthGate<'_> {
    fn max_in_flight(&self) -> usize {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).1
    }
}

impl HarnessPort for HealthGate<'_> {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        self.inner.serve(name, port)
    }
    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.0 += 1;
        state.1 = state.1.max(state.0);
        self.arrived.notify_all();
        while state.1 < self.want {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            state = self
                .arrived
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        state.0 -= 1;
        drop(state);
        self.inner.health(port)
    }
    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        self.inner.create_session(port)
    }
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(port)
    }
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.abort(port, session)
    }
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.attach_tui(pane, port, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.inner.select_session(pane, session)
    }
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane)
    }
}

/// AC 24: the panes are observed concurrently (an invariant, not a duration).
#[test]
fn observation_runs_concurrently() {
    let rig = Rig::new(&[
        Seed::new("demo-c1r1", 1, 1),
        Seed::new("demo-c2r1", 1, 2),
        Seed::new("demo-c3r1", 1, 3),
        Seed::new("demo-c1r2", 2, 1),
    ]);
    let gate = HealthGate {
        inner: &rig.harness,
        want: 4,
        state: Mutex::new((0, 0)),
        arrived: Condvar::new(),
    };

    let result = reconcile(rig.ports_with(&gate), &whole(false));

    assert_eq!(
        gate.max_in_flight(),
        4,
        "four health checks in flight at once"
    );
    result.expect("the pass completes");
}

// --- Remedies -------------------------------------------------------------------------

/// AC 26: every `holler pane doctor ...` remedy is a command line that parses.
#[test]
fn doctor_remedies_parse() {
    let mismatch = Rig::new(&[
        Seed::new(P_PANE, 1, 1),
        Seed::new(Q_PANE, 1, 2).orchestrator(),
    ]);
    mismatch.move_tui_to_new_session(P_PANE);
    mismatch.move_tui_to_new_session(Q_PANE);
    let ps_failed = Rig::new(&[Seed::new(P_PANE, 1, 1)]);
    let unavailable = PaneError::Unavailable {
        what: "tmux".to_owned(),
    };
    ps_failed.host.faults().fail_next(HostOp::Ps, unavailable);
    let herdr_failed = Rig::new(&[Seed::new(P_PANE, 1, 1)]);
    herdr_failed.herdr.faults().set(Some(Fault::Wedged));

    let reports = [
        mismatch.run(&whole(false)),
        mismatch.run(&whole(true)),
        ps_failed.run(&whole(false)),
        herdr_failed.run(&whole(false)),
    ];

    let host_ps = one(&reports[2], K::ObserveFailed, P_PANE);
    assert!(host_ps.message.contains("host.ps"), "{host_ps:?}");
    let remedies: BTreeSet<String> = reports
        .iter()
        .flat_map(|r| r.findings.iter().filter_map(|f| f.remedy.clone()))
        .filter(|r| r.starts_with("holler pane doctor"))
        .collect();
    let expected = [
        "holler pane doctor",
        "holler pane doctor demo-c1r1",
        "holler pane doctor demo-c1r1 --fix",
        "holler pane doctor demo-c2r1 --fix",
    ];
    assert_eq!(remedies, expected.map(str::to_owned).into(), "{remedies:?}");
    for remedy in &remedies {
        let words: Vec<&str> = remedy.split(' ').skip(1).collect();
        try_parse(&words).unwrap_or_else(|e| panic!("{remedy:?} must parse: {e}"));
    }
}
