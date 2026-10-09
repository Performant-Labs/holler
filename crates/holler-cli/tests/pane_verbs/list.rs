//! `holler pane list` (story #643), the rig every read-verb test runs over, and the cases
//! that span the three read verbs (positions, `--profile` refusals, bad names, store
//! failures, nothing observed).
//!
//! Every case runs the verb in-process through `verb_harness::run_verb_with` over the
//! fakes of `holler-pane-testkit`, and every JSON check goes through its envelope helper.

use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::pane::{Health, LastObserved};
use holler_pane::{Actor, GridPos, Pane, PaneError, Ports, Profile, ProfileName};
use holler_pane_testkit::envelope::{check_envelope, check_ndjson, Envelope};
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;
use serde_json::{json, Value};

use crate::verb_harness::{run_verb_with, Outcome};

// The consumer of `holler-pane-testkit` (a dev-dependency declared by #670 so that
// #638 and #643-#647 add no manifest line). The manifest comment names this case, so it
// stays; the real uses of the fakes are the cases below.
use holler_pane_testkit as _;

#[test]
fn testkit_links() {}

// --- the rig ---------------------------------------------------------------------

/// The fakes a read verb runs over: the two stores, a scope over them, and the adapter
/// and probe fakes, whose call logs prove the verbs observe nothing.
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
    /// A rig whose pane store holds `panes` and whose profile store holds `profiles`,
    /// each seeded in order (so the n-th pane's put has cursor n).
    pub fn new(
        panes: impl IntoIterator<Item = Pane>,
        profiles: impl IntoIterator<Item = Profile>,
    ) -> Result<Self, PaneError> {
        let actor = Actor::parse("test")?;
        let panes = Arc::new(FakePaneStore::seeded(panes)?);
        let profiles = Arc::new(FakeProfileStore::seeded(profiles, &actor)?);
        let scope = FakeProfileScope::new(profiles.clone(), panes.clone(), actor);
        Ok(Self {
            panes,
            profiles,
            scope,
            herdr: FakeHerdr::new("scratch"),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
        })
    }

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

    /// Run `holler <argv...>` over this rig.
    pub fn run(&self, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(argv, format, self.ports())
    }

    /// No Herdr, host, harness or probe call was made through this rig.
    pub fn assert_nothing_observed(&self) {
        assert_eq!(self.herdr.faults().calls(), vec![], "no Herdr call");
        assert_eq!(self.host.faults().calls(), vec![], "no host call");
        assert_eq!(self.harness.faults().calls(), vec![], "no harness call");
        assert_eq!(self.prober.calls(), vec![], "no probe run");
    }
}

// --- shared helpers ----------------------------------------------------------------

/// The sample pane `name` (a valid name is a test's own constant).
pub(crate) fn pane(name: &str) -> Pane {
    sample_pane(name).unwrap()
}

/// The sample pane `name`, in the profile `profile`.
pub(crate) fn member(name: &str, profile: &str) -> Pane {
    Pane {
        profile: Some(ProfileName::parse(profile).unwrap()),
        ..pane(name)
    }
}

/// The sample pane `name` with reconcile's last observation `shown`/`driven`.
pub(crate) fn observed(name: &str, shown: Option<&str>, driven: Option<&str>) -> Pane {
    Pane {
        last_observed: LastObserved {
            shown: shown.map(str::to_owned),
            driven: driven.map(str::to_owned),
            at: 0,
        },
        ..pane(name)
    }
}

/// The scoping store of AC 6: c1 and c2 in `demo`; c3 in `other` while `demo` also holds
/// a (detached) spec for it; c4 in no profile.
pub(crate) fn scoped_rig() -> Rig {
    Rig::new(
        [
            member("demo-c1r1", "demo"),
            member("demo-c2r1", "demo"),
            member("demo-c3r1", "other"),
            pane("demo-c4r1"),
        ],
        [
            sample_profile("demo", &["demo-c1r1", "demo-c2r1", "demo-c3r1"]).unwrap(),
            sample_profile("other", &["demo-c3r1"]).unwrap(),
        ],
    )
    .unwrap()
}

/// The one success envelope of a JSON run that must exit 0, with nothing on `err`.
pub(crate) fn ok_envelope(run: &Outcome) -> Envelope {
    assert_eq!(run.code, 0, "exit 0: {run:?}");
    assert!(run.err.is_empty(), "nothing on err in JSON mode: {run:?}");
    check_envelope(&run.out, 0).unwrap_or_else(|f| panic!("one valid envelope: {f}: {run:?}"))
}

/// The NDJSON envelopes of a JSON `watch` run that must exit 0 and print at least one line.
pub(crate) fn ok_stream(run: &Outcome) -> Vec<Envelope> {
    assert_eq!(run.code, 0, "exit 0: {run:?}");
    assert!(run.err.is_empty(), "nothing on err in JSON mode: {run:?}");
    check_ndjson(&run.out, 0).unwrap_or_else(|f| panic!("a valid NDJSON stream: {f}: {run:?}"))
}

/// A text run that must exit 0: its `out`, with nothing on `err`.
pub(crate) fn ok_text(run: &Outcome) -> &str {
    assert_eq!(run.code, 0, "exit 0: {run:?}");
    assert!(run.err.is_empty(), "nothing on err: {run:?}");
    &run.out
}

/// `argv` fails with `code` at exit `exit` in both formats: text writes one `error: ...`
/// line to `err` and nothing to `out`; JSON writes the one failure envelope (checked as
/// an NDJSON stream when `stream`) to `out` and nothing to `err`.
pub(crate) fn assert_fails(rig: &Rig, argv: &[&str], exit: i32, code: &str, stream: bool) {
    let text = rig.run(argv, Format::Text);
    assert_eq!(text.code, exit, "{argv:?} text: exit {exit}: {text:?}");
    assert!(
        text.out.is_empty(),
        "{argv:?} text: nothing on out: {text:?}"
    );
    assert_eq!(
        text.err.lines().count(),
        1,
        "{argv:?} text: one err line: {text:?}"
    );
    assert!(text.err.starts_with("error: "), "{argv:?} text: {text:?}");

    let json = rig.run(argv, Format::Json);
    assert_eq!(
        json.code, text.code,
        "{argv:?}: the same exit in both formats: {json:?}"
    );
    assert!(
        json.err.is_empty(),
        "{argv:?} json: nothing on err: {json:?}"
    );
    let envelope = if stream {
        check_ndjson(&json.out, exit).map(|mut all| all.remove(all.len() - 1))
    } else {
        check_envelope(&json.out, exit)
    }
    .unwrap_or_else(|f| panic!("{argv:?} json: a valid failure envelope: {f}: {json:?}"));
    let got = envelope.error.map(|e| e.code);
    assert_eq!(got.as_deref(), Some(code), "{argv:?} json: {json:?}");
}

/// The whitespace-separated cells of a table line.
pub(crate) fn cells(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

/// The value of the `key: value` line of `get`'s text output, if there is one.
pub(crate) fn field<'a>(out: &'a str, key: &str) -> Option<&'a str> {
    out.lines()
        .find_map(|l| l.strip_prefix(&format!("{key}: ")))
}

/// The value of `key=` in a `watch` text line, if there is one.
pub(crate) fn kv<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(' ')
        .find_map(|w| w.strip_prefix(&format!("{key}=")))
}

/// The `list` row of `name` in a JSON `data`.
fn json_row<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["panes"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["name"] == name))
        .unwrap_or_else(|| panic!("no row {name:?} in {data}"))
}

const HEADER: [&str; 9] = [
    "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
];

// --- list -------------------------------------------------------------------------

#[test]
fn list_prints_a_header_and_one_row_per_pane_sorted_by_name() {
    let rig = Rig::new([pane("demo-c2r1"), pane("demo-c1r1")], []).unwrap();
    let run = rig.run(&["pane", "list"], Format::Text);
    let lines: Vec<&str> = ok_text(&run).lines().collect();
    assert_eq!(lines.len(), 3, "a header and two rows: {run:?}");
    assert_eq!(cells(lines[0]), HEADER, "{run:?}");
    for (line, name) in lines[1..].iter().zip(["demo-c1r1", "demo-c2r1"]) {
        let want = [
            name,
            "r1c1",
            "-",
            "/srv/demo",
            "unknown",
            "-",
            "-",
            "-",
            "none",
        ];
        assert_eq!(cells(line), want, "rows sorted by name: {run:?}");
    }
    for line in &lines {
        assert!(!line.ends_with(' '), "no trailing space: {line:?}");
    }
}

#[test]
fn list_json_is_one_envelope_with_a_row_per_pane() {
    let rig = Rig::new([pane("demo-c2r1"), pane("demo-c1r1")], []).unwrap();
    let data = ok_envelope(&rig.run(&["pane", "list"], Format::Json)).data;
    let rows = data["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.panes: {data}"));
    let names: Vec<&Value> = rows.iter().map(|r| &r["name"]).collect();
    assert_eq!(names, [&json!("demo-c1r1"), &json!("demo-c2r1")], "{data}");
    let mut want = [
        "driven", "health", "hold", "name", "pos", "profile", "project", "shown", "sync",
    ];
    want.sort_unstable();
    for row in rows {
        let mut keys: Vec<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, want, "{row}");
        assert_eq!(row["profile"], Value::Null, "{row}");
        assert_eq!(row["shown"], Value::Null, "{row}");
        assert_eq!(row["driven"], Value::Null, "{row}");
        assert_eq!(row["health"], "unknown", "{row}");
        assert_eq!(row["hold"], "none", "{row}");
        assert_eq!(row["sync"], "unobserved", "{row}");
    }
}

/// The three SHOWN/DRIVEN cases of AC 3: differ, agree, one side unobserved.
pub(crate) fn sync_rig() -> Rig {
    Rig::new(
        [
            observed("demo-c1r1", Some("ses-a"), Some("ses-b")),
            observed("demo-c2r1", Some("ses-a"), Some("ses-a")),
            observed("demo-c3r1", None, Some("ses-a")),
        ],
        [],
    )
    .unwrap()
}

#[test]
fn list_flags_a_pane_whose_shown_and_driven_differ() {
    let rig = sync_rig();
    let run = rig.run(&["pane", "list"], Format::Text);
    let rows: Vec<Vec<&str>> = ok_text(&run).lines().skip(1).map(cells).collect();
    let want = [
        ["ses-a", "ses-b", "MISMATCH"],
        ["ses-a", "ses-a", "ok"],
        ["-", "ses-a", "-"],
    ];
    for (row, want) in rows.iter().zip(want) {
        assert_eq!(&row[5..8], want, "SHOWN DRIVEN SYNC: {run:?}");
    }
    assert_eq!(rows.len(), 3, "{run:?}");

    let data = ok_envelope(&rig.run(&["pane", "list"], Format::Json)).data;
    let want = [
        ("demo-c1r1", json!("ses-a"), json!("ses-b"), "mismatch"),
        ("demo-c2r1", json!("ses-a"), json!("ses-a"), "ok"),
        ("demo-c3r1", Value::Null, json!("ses-a"), "unobserved"),
    ];
    for (name, shown, driven, sync) in want {
        let row = json_row(&data, name);
        assert_eq!((&row["shown"], &row["driven"]), (&shown, &driven), "{row}");
        assert_eq!(row["sync"], sync, "{row}");
    }
}

#[test]
fn list_shows_an_unhealthy_server() {
    let mut sick = pane("demo-c1r1");
    sick.harness.health = Health::Unhealthy("server wedged".into());
    let rig = Rig::new([sick], []).unwrap();
    let run = rig.run(&["pane", "list"], Format::Text);
    let row = ok_text(&run).lines().nth(1).map(cells).unwrap_or_default();
    assert_eq!(row.get(4), Some(&"unhealthy"), "HEALTH: {run:?}");
    let data = ok_envelope(&rig.run(&["pane", "list"], Format::Json)).data;
    assert_eq!(
        json_row(&data, "demo-c1r1")["health"],
        json!({"unhealthy": "server wedged"})
    );

    let run = rig.run(&["pane", "get", "demo-c1r1"], Format::Text);
    assert_eq!(
        field(ok_text(&run), "health"),
        Some("unhealthy \"server wedged\""),
        "{run:?}"
    );
    let data = ok_envelope(&rig.run(&["pane", "get", "demo-c1r1"], Format::Json)).data;
    assert_eq!(
        data["pane"]["harness"]["health"],
        json!({"unhealthy": "server wedged"})
    );
}

#[test]
fn every_verb_prints_positions_row_first() {
    let mut cell = pane("demo-c1r2");
    cell.herdr.grid = GridPos { row: 2, col: 1 };
    let rig = Rig::new([cell], []).unwrap();
    let text = |argv: &[&str]| ok_text(&rig.run(argv, Format::Text)).to_owned();

    let list = text(&["pane", "list"]);
    let list_pos = list
        .lines()
        .nth(1)
        .map(cells)
        .and_then(|c| c.get(1).copied());
    let get = text(&["pane", "get", "demo-c1r2"]);
    let watch = text(&["pane", "watch", "--until-idle"]);
    let watch_pos = watch.lines().next().and_then(|l| kv(l, "pos"));
    let shown = [list_pos, field(&get, "pos"), watch_pos];
    assert_eq!(
        shown,
        [Some("r2c1"); 3],
        "list {list:?}, get {get:?}, watch {watch:?}"
    );
    assert!(!shown.contains(&Some("c1r2")), "never column first");

    let raw = |argv: &[&str]| rig.run(argv, Format::Json).out;
    let pos = r#""pos":{"row":2,"col":1,"pos":"r2c1"}"#;
    let list = raw(&["pane", "list"]);
    assert!(list.contains(pos), "list JSON: {list}");
    let get = raw(&["pane", "get", "demo-c1r2"]);
    assert!(
        get.contains(r#""grid":{"row":2,"col":1,"pos":"r2c1"}"#),
        "get JSON: {get}"
    );
    let watch = raw(&["pane", "watch", "--until-idle"]);
    assert!(watch.contains(pos), "watch JSON: {watch}");
}

#[test]
fn list_profile_lists_only_the_profile_s_members() {
    let rig = scoped_rig();
    let run = rig.run(&["pane", "list", "--profile", "demo"], Format::Text);
    let rows: Vec<Vec<&str>> = ok_text(&run).lines().skip(1).map(cells).collect();
    let names: Vec<&str> = rows.iter().map(|r| r[0]).collect();
    assert_eq!(
        names,
        ["demo-c1r1", "demo-c2r1"],
        "only the members: {run:?}"
    );
    assert!(rows.iter().all(|r| r[2] == "demo"), "PROFILE: {run:?}");

    let data = ok_envelope(&rig.run(&["pane", "list", "--profile", "demo"], Format::Json)).data;
    let rows = data["panes"].as_array().unwrap_or_else(|| panic!("{data}"));
    let names: Vec<&Value> = rows.iter().map(|r| &r["name"]).collect();
    assert_eq!(names, [&json!("demo-c1r1"), &json!("demo-c2r1")], "{data}");
    assert!(rows.iter().all(|r| r["profile"] == "demo"), "{data}");
}

#[test]
fn profile_refusals_exit_3_in_both_formats() {
    let rig = scoped_rig();
    let not_found: [&[&str]; 3] = [
        &["pane", "list", "--profile", "nope"],
        &["pane", "get", "demo-c1r1", "--profile", "nope"],
        &["pane", "watch", "--profile", "nope", "--until-idle"],
    ];
    for argv in not_found {
        assert_fails(&rig, argv, 3, "profile-not-found", false);
    }
    let not_in: [&[&str]; 4] = [
        &["pane", "list", "demo-c3r1", "--profile", "demo"],
        &["pane", "get", "demo-c3r1", "--profile", "demo"],
        &["pane", "get", "demo-c4r1", "--profile", "demo"],
        &[
            "pane",
            "watch",
            "demo-c4r1",
            "--profile",
            "demo",
            "--until-idle",
        ],
    ];
    for argv in not_in {
        assert_fails(&rig, argv, 3, "pane-not-in-profile", false);
    }
}

#[test]
fn bad_names_are_usage_in_both_formats() {
    let rig = Rig::new([pane("demo-c1r1")], []).unwrap();
    let bad: [&[&str]; 3] = [
        &["pane", "list", "--profile", ""],
        &["pane", "get", "NOT_A_PANE!"],
        &["pane", "watch", "--profile", "  ", "--until-idle"],
    ];
    for argv in bad {
        assert_fails(&rig, argv, 2, "usage", false);
    }
}

#[test]
fn store_failures_exit_1_in_both_formats() {
    let rig = Rig::new([pane("demo-c1r1")], []).unwrap();
    let faults = [
        (
            Fault::Fail(PaneError::Unavailable { what: "hub".into() }),
            "unavailable",
        ),
        (Fault::Wedged, "timeout"),
    ];
    for (fault, code) in faults {
        rig.panes.faults().set(Some(fault));
        assert_fails(&rig, &["pane", "list"], 1, code, false);
        assert_fails(&rig, &["pane", "get", "demo-c1r1"], 1, code, false);
        assert_fails(&rig, &["pane", "watch", "--until-idle"], 1, code, true);
    }
}

#[test]
fn read_verbs_call_no_adapter_or_probe() {
    let rig = scoped_rig();
    let runs: [&[&str]; 5] = [
        &["pane", "list"],
        &["pane", "get", "demo-c1r1"],
        &["pane", "list", "--profile", "demo"],
        &["pane", "get", "demo-c1r1", "--profile", "demo"],
        &["pane", "watch", "--until-idle"],
    ];
    for argv in runs {
        for format in [Format::Text, Format::Json] {
            let run = rig.run(argv, format);
            assert_eq!(run.code, 0, "{argv:?} {format:?} succeeds: {run:?}");
        }
    }
    let calls = rig.panes.faults().calls();
    assert!(!calls.is_empty(), "the verbs read the pane store");
    let writes = calls
        .iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete));
    assert_eq!(writes.count(), 0, "the verbs write nothing: {calls:?}");
    rig.assert_nothing_observed();
}

#[test]
fn list_help_documents_the_columns_and_the_json_shape() {
    let help = crate::get::help(&["pane", "list"]);
    for needle in [
        "POS",
        "PROFILE",
        "PROJECT",
        "HEALTH",
        "SHOWN",
        "DRIVEN",
        "SYNC",
        "MISMATCH",
        "HOLD",
        "--format=json",
        "\"panes\"",
        "r2c1",
    ] {
        assert!(
            help.contains(needle),
            "`pane list --help` names {needle:?}:\n{help}"
        );
    }
}
