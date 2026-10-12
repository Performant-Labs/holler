//! Unmanaged panes (story #832, epic #633 decision 9): a Herdr pane no record names is
//! listed with the state `unmanaged`, its id, label, directory and the time its screen
//! last changed; no verb that acts on a pane accepts one (`pane-unmanaged`, exit 3); and
//! `pane doctor` reports a long unchanged screen as `unmanaged-stalled`.
//!
//! Every case runs the verb in-process over the fakes of `holler-pane-testkit`. The
//! fixture is `seed_unmanaged_pane`: one fake pane that runs something no adapter knows,
//! with a screen that does not change.

use std::time::{SystemTime, UNIX_EPOCH};

use holler_cli::output::Format;
use holler_pane::{GridPos, PaneError};
use holler_pane_testkit::fixture::{sample_profile, seed_unmanaged_pane};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrOp};
use serde_json::Value;

use crate::list::{assert_fails, cells, member, ok_envelope, ok_text, pane, Rig};

const LABEL: &str = "seeding-pane";
const CWD: &str = "/work/seeding";
const HOUR_MS: i64 = 3_600_000;

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// A rig whose Herdr has a 3 by 3 workspace, holds `managed` records, and has one
/// unmanaged pane at `r1c1` whose screen last changed `hours` hours ago (`None`: Herdr
/// reports no time). Returns the rig and the unmanaged pane's id.
fn rig_with_unmanaged(managed: &[&str], hours: Option<i64>) -> (Rig, String) {
    rig_with_unmanaged_labelled(managed, hours, LABEL)
}

/// [`rig_with_unmanaged`] with the unmanaged pane labelled `label`, which Herdr allows to be
/// any text, not only a valid pane name.
fn rig_with_unmanaged_labelled(managed: &[&str], hours: Option<i64>, label: &str) -> (Rig, String) {
    let mut rig = Rig::new(managed.iter().map(|name| pane(name)), []).unwrap();
    rig.herdr = FakeHerdr::new("scratch")
        .with_workspace("scratch", 3, 3)
        .unwrap();
    let changed = hours.map(|h| now_ms() - h * HOUR_MS);
    let seeded =
        seed_unmanaged_pane(&rig.herdr, GridPos { row: 1, col: 1 }, label, CWD, changed).unwrap();
    (rig, seeded.pane_id.as_str().to_owned())
}

fn unmanaged_rows(data: &Value) -> &Vec<Value> {
    data["unmanaged"]
        .as_array()
        .unwrap_or_else(|| panic!("data has an `unmanaged` list: {data}"))
}

// --- list ---------------------------------------------------------------------------

#[test]
fn list_json_shows_an_unmanaged_pane_with_its_state_screen_change_and_idle_time() {
    let (rig, id) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    let envelope = ok_envelope(&rig.run(&["pane", "list"], Format::Json));
    let rows = unmanaged_rows(&envelope.data);
    assert_eq!(rows.len(), 1, "one unmanaged pane: {}", envelope.data);
    let row = &rows[0];
    assert_eq!(row["state"], "unmanaged");
    assert_eq!(row["pane_id"], id.as_str());
    assert_eq!(row["label"], LABEL);
    assert_eq!(row["cwd"], CWD);
    assert_eq!(row["pos"]["pos"], "r1c1");
    let changed = row["last_screen_change"].as_i64().expect("a time in ms");
    assert!(changed <= now_ms() - 4 * HOUR_MS + 5_000, "{row}");
    let idle = row["idle_seconds"].as_u64().expect("idle seconds");
    assert!((14_400..14_460).contains(&idle), "about four hours: {row}");
    // The managed pane is still listed as it was.
    assert_eq!(envelope.data["panes"][0]["name"], "demo-c2r1");
}

#[test]
fn list_text_shows_the_unmanaged_pane_in_a_table_of_its_own() {
    let (rig, id) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    let run = rig.run(&["pane", "list"], Format::Text);
    let out = ok_text(&run);
    let (managed, unmanaged) = out.split_once("\n\n").expect("a blank line between tables");
    assert!(managed.contains("demo-c2r1"), "{out}");
    let lines: Vec<&str> = unmanaged.lines().collect();
    assert_eq!(lines.len(), 2, "a header and one row: {out}");
    assert_eq!(
        cells(lines[0]),
        ["PANE-ID", "LABEL", "POS", "PROJECT", "STATE", "IDLE"]
    );
    let row = cells(lines[1]);
    assert_eq!(row[..5], [id.as_str(), LABEL, "r1c1", CWD, "unmanaged"]);
    assert!(row[5].starts_with("4h"), "idle time in hours: {out}");
    for line in out.lines() {
        assert!(!line.ends_with(' '), "no trailing space: {line:?}");
    }
}

#[test]
fn an_unmanaged_pane_whose_screen_change_herdr_does_not_report_has_no_time_and_no_idle() {
    let (rig, _) = rig_with_unmanaged(&[], None);
    let envelope = ok_envelope(&rig.run(&["pane", "list"], Format::Json));
    let row = &unmanaged_rows(&envelope.data)[0];
    assert!(row["last_screen_change"].is_null(), "never guessed: {row}");
    assert!(row["idle_seconds"].is_null(), "never guessed: {row}");
    assert_eq!(row["cwd"], CWD, "what Herdr does report is still shown");
    let text = rig.run(&["pane", "list"], Format::Text);
    let lines: Vec<&str> = ok_text(&text).lines().collect();
    assert_eq!(cells(lines.last().unwrap()).last(), Some(&"-"), "{text:?}");
}

#[test]
fn a_herdr_pane_with_a_record_is_not_listed_as_unmanaged() {
    let (mut rig, id) = rig_with_unmanaged(&[], Some(1));
    let mut record = pane("demo-c1r1");
    record.herdr.pane_id = holler_pane::PaneId::new(id);
    rig.panes = std::sync::Arc::new(
        holler_pane_testkit::pane_store::FakePaneStore::seeded([record]).unwrap(),
    );
    let envelope = ok_envelope(&rig.run(&["pane", "list"], Format::Json));
    assert!(
        envelope.data.get("unmanaged").is_none(),
        "the record names the pane: {}",
        envelope.data
    );
}

#[test]
fn a_pane_with_no_unmanaged_neighbour_prints_no_unmanaged_key_or_table() {
    let rig = Rig::new([pane("demo-c1r1")], []).unwrap();
    let envelope = ok_envelope(&rig.run(&["pane", "list"], Format::Json));
    assert!(
        envelope.data.get("unmanaged").is_none(),
        "{}",
        envelope.data
    );
    let text = rig.run(&["pane", "list"], Format::Text);
    assert!(!ok_text(&text).contains("\n\n"), "{text:?}");
}

#[test]
fn a_scoped_list_does_not_look_at_herdr_or_list_unmanaged_panes() {
    let (mut rig, _) = rig_with_unmanaged(&[], Some(4));
    rig.panes = std::sync::Arc::new(
        holler_pane_testkit::pane_store::FakePaneStore::seeded([member("demo-c1r1", "demo")])
            .unwrap(),
    );
    rig.profiles = std::sync::Arc::new(
        holler_pane_testkit::profile_store::FakeProfileStore::seeded(
            [sample_profile("demo", &["demo-c1r1"]).unwrap()],
            &holler_pane::Actor::parse("test").unwrap(),
        )
        .unwrap(),
    );
    rig.scope = holler_pane_testkit::profile_scope::FakeProfileScope::new(
        rig.profiles.clone(),
        rig.panes.clone(),
        holler_pane::Actor::parse("test").unwrap(),
    );
    let before = rig.herdr.faults().calls().len();
    for argv in [
        &["pane", "list", "demo-c1r1"][..],
        &["pane", "list", "--profile", "demo"][..],
    ] {
        let envelope = ok_envelope(&rig.run(argv, Format::Json));
        assert!(envelope.data.get("unmanaged").is_none(), "{argv:?}");
    }
    assert_eq!(rig.herdr.faults().calls().len(), before, "no Herdr call");
}

#[test]
fn when_herdr_cannot_be_read_the_managed_panes_are_still_listed_and_the_gap_is_said() {
    let (rig, _) = rig_with_unmanaged(&["demo-c1r1"], Some(4));
    rig.herdr.faults().fail_next(
        HerdrOp::Snapshot,
        PaneError::Unavailable {
            what: "herdr down".into(),
        },
    );
    let envelope = ok_envelope(&rig.run(&["pane", "list"], Format::Json));
    assert_eq!(envelope.data["panes"][0]["name"], "demo-c1r1");
    assert!(envelope.data.get("unmanaged").is_none());
    assert_eq!(
        envelope.data["unmanaged_unavailable"], "unavailable",
        "the code of the failed Herdr call: {}",
        envelope.data
    );

    rig.herdr.faults().fail_next(
        HerdrOp::Snapshot,
        PaneError::Unavailable {
            what: "herdr down".into(),
        },
    );
    let text = rig.run(&["pane", "list"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert!(text.out.contains("demo-c1r1"), "{text:?}");
    assert!(
        text.err.contains("unmanaged panes not listed") && text.err.contains("unavailable"),
        "{text:?}"
    );
}

// --- get ----------------------------------------------------------------------------

#[test]
fn get_shows_an_unmanaged_pane_by_its_label_or_its_herdr_id() {
    let (rig, id) = rig_with_unmanaged(&[], Some(4));
    for name in [LABEL, id.as_str()] {
        let text = rig.run(&["pane", "get", name], Format::Text);
        let out = ok_text(&text);
        for (key, want) in [
            ("state", "unmanaged"),
            ("pane-id", id.as_str()),
            ("label", LABEL),
            ("project", CWD),
        ] {
            assert_eq!(crate::list::field(out, key), Some(want), "{name}: {out}");
        }
        assert!(
            crate::list::field(out, "idle").is_some_and(|v| v.starts_with("4h")),
            "{name}: {out}"
        );
        assert!(
            crate::list::field(out, "last-screen-change").is_some_and(|v| v.ends_with("UTC")),
            "{name}: {out}"
        );

        let envelope = ok_envelope(&rig.run(&["pane", "get", name], Format::Json));
        assert_eq!(envelope.data["state"], "unmanaged", "{name}");
        assert_eq!(envelope.data["pane_id"], id.as_str());
        assert!(envelope.data["idle_seconds"].as_u64().is_some());
    }
}

#[test]
fn get_of_a_name_that_is_neither_a_record_nor_a_herdr_pane_is_still_pane_not_found() {
    let (rig, _) = rig_with_unmanaged(&[], Some(4));
    assert_fails(
        &rig,
        &["pane", "get", "demo-c9r9"],
        3,
        "pane-not-found",
        false,
    );
}

// --- the verbs that act on a pane refuse one -----------------------------------------

#[test]
fn every_verb_that_acts_on_a_pane_refuses_an_unmanaged_one_with_pane_unmanaged() {
    let (rig, id) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    for name in [LABEL, id.as_str()] {
        for verb in [
            &["pane", "relaunch", name][..],
            &["pane", "reset", name][..],
            &["pane", "switch", name, "ses_demo"][..],
            &["pane", "park", name, "--reason", "x", "--release-when", "y"][..],
            &["pane", "unpark", name][..],
            &["pane", "close", name][..],
        ] {
            assert_fails(&rig, verb, 3, "pane-unmanaged", false);
        }
    }
}

#[test]
fn a_named_record_is_never_taken_for_an_unmanaged_pane() {
    // A record called like the unmanaged pane's label: the record wins, and the guard
    // does not fire.
    let (rig, _) = rig_with_unmanaged(&[LABEL], Some(4));
    let run = rig.run(&["pane", "unpark", LABEL], Format::Json);
    assert!(
        !run.out.contains("pane-unmanaged"),
        "a record exists: {run:?}"
    );
}

#[test]
fn a_verb_given_no_pane_name_is_not_guarded() {
    let (rig, _) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    let run = rig.run(&["pane", "unpark"], Format::Json);
    assert!(!run.out.contains("pane-unmanaged"), "{run:?}");
}

// --- doctor -------------------------------------------------------------------------

fn finding_kinds(data: &Value) -> Vec<String> {
    data["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn doctor_reports_a_long_unchanged_screen_as_unmanaged_stalled() {
    let (rig, id) = rig_with_unmanaged(&[], Some(4));
    let run = rig.run(
        &["pane", "doctor", "--unmanaged-stalled-after", "1800"],
        Format::Json,
    );
    let envelope = ok_envelope(&run);
    let stalled: Vec<&Value> = envelope.data["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "unmanaged-stalled")
        .collect();
    assert_eq!(stalled.len(), 1, "{}", envelope.data);
    assert_eq!(stalled[0]["herdr_pane"], id.as_str());
    let message = stalled[0]["message"].as_str().unwrap();
    assert!(
        message.contains("4h") && message.contains("30m"),
        "idle time and threshold: {message}"
    );
    assert!(stalled[0]["remedy"].is_null(), "no verb repairs it");

    let text = rig.run(
        &["pane", "doctor", "--unmanaged-stalled-after", "1800"],
        Format::Text,
    );
    assert!(text.out.contains("unmanaged-stalled"), "{text:?}");
}

#[test]
fn doctor_is_silent_about_a_pane_below_the_threshold_without_a_threshold_or_with_no_time() {
    let (recent, _) = rig_with_unmanaged(&[], Some(0));
    let run = recent.run(
        &["pane", "doctor", "--unmanaged-stalled-after", "1800"],
        Format::Json,
    );
    assert!(!finding_kinds(&ok_envelope(&run).data).contains(&"unmanaged-stalled".to_owned()));

    let (old, _) = rig_with_unmanaged(&[], Some(4));
    let run = old.run(&["pane", "doctor"], Format::Json);
    let kinds = finding_kinds(&ok_envelope(&run).data);
    assert!(
        !kinds.contains(&"unmanaged-stalled".to_owned()),
        "no threshold, no look: {kinds:?}"
    );

    let (unknown, _) = rig_with_unmanaged(&[], None);
    let run = unknown.run(
        &["pane", "doctor", "--unmanaged-stalled-after", "1800"],
        Format::Json,
    );
    let kinds = finding_kinds(&ok_envelope(&run).data);
    assert!(
        !kinds.contains(&"unmanaged-stalled".to_owned()),
        "an unknown time is not stalled: {kinds:?}"
    );
}

#[test]
fn doctor_of_one_pane_does_not_look_for_unmanaged_panes() {
    let (rig, _) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    let run = rig.run(
        &[
            "pane",
            "doctor",
            "demo-c2r1",
            "--unmanaged-stalled-after",
            "1800",
        ],
        Format::Json,
    );
    let envelope = run_ok_or_findings(&run);
    assert!(!finding_kinds(&envelope).contains(&"unmanaged-stalled".to_owned()));
}

/// The data of a doctor run that exits 0 or reports findings.
fn run_ok_or_findings(run: &crate::verb_harness::Outcome) -> Value {
    holler_pane_testkit::envelope::check_envelope(&run.out, run.code)
        .unwrap_or_else(|f| panic!("an envelope: {f}: {run:?}"))
        .data
}

#[test]
fn get_of_an_unknown_herdr_pane_id_is_pane_not_found_not_usage() {
    let (rig, _) = rig_with_unmanaged(&[], Some(4));
    assert_fails(&rig, &["pane", "get", "w9:p9"], 3, "pane-not-found", false);
    // A bad name that does not look like a Herdr pane id is still a usage error.
    assert_fails(&rig, &["pane", "get", "a/b"], 2, "usage", false);
}

// --- review of #836 -------------------------------------------------------------------

/// A Herdr label is free text: these are not valid pane names (upper case, a space, 40
/// characters), and a hand-labelled pane is exactly such a pane. The verbs that act on a
/// pane refuse each as `pane-unmanaged`, exit 3, not as `usage`; `get` shows it.
#[test]
fn a_label_that_is_not_a_valid_pane_name_is_still_refused_by_every_acting_verb() {
    let long = "x".repeat(40);
    for label in ["Claude Code", "Seeding Pane", long.as_str()] {
        let (rig, _) = rig_with_unmanaged_labelled(&["demo-c2r1"], Some(4), label);
        for verb in [
            &["pane", "relaunch", label][..],
            &["pane", "reset", label][..],
            &["pane", "switch", label, "ses_demo"][..],
            &[
                "pane",
                "park",
                label,
                "--reason",
                "x",
                "--release-when",
                "y",
            ][..],
            &["pane", "unpark", label][..],
            &["pane", "close", label][..],
        ] {
            assert_fails(&rig, verb, 3, "pane-unmanaged", false);
        }
        let shown = ok_envelope(&rig.run(&["pane", "get", label], Format::Json));
        assert_eq!(shown.data["state"], "unmanaged", "{label:?}");
    }
}

/// A name that is not valid and that no unmanaged pane answers to is still a typo.
#[test]
fn an_invalid_name_that_no_pane_answers_to_is_still_usage() {
    let (rig, _) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    for verb in [
        &["pane", "reset", "Not A Pane"][..],
        &["pane", "close", "a/b"][..],
        &["pane", "relaunch", "x".repeat(40).as_str()][..],
    ] {
        assert_fails(&rig, verb, 2, "usage", false);
    }
}

/// A pane that has a record costs no read of Herdr: park and unpark of a recorded pane make
/// no Herdr call at all, even with an unmanaged pane next to it (the guard looks at Herdr
/// only on the missing-record path).
#[test]
fn a_recorded_pane_costs_no_read_of_herdr() {
    let (rig, _) = rig_with_unmanaged(&["demo-c2r1"], Some(4));
    let after_setup = rig.herdr.faults().calls();
    for argv in [
        &[
            "pane",
            "park",
            "demo-c2r1",
            "--reason",
            "x",
            "--release-when",
            "y",
        ][..],
        &["pane", "unpark", "demo-c2r1"][..],
    ] {
        let run = rig.run(argv, Format::Json);
        assert_eq!(run.code, 0, "{argv:?}: {run:?}");
    }
    assert_eq!(
        rig.herdr.faults().calls(),
        after_setup,
        "the verbs made no Herdr call (the calls so far are the fixture's own)"
    );
}

/// The rule "no record names this Herdr pane" is public, for the stories that decide whether
/// Holler drives a pane (#831, #835): `unregistered` lists such panes and `is_unmanaged`
/// answers for one.
#[test]
fn the_unmanaged_predicate_is_public_and_one_rule() {
    use holler_pane::unmanaged::{is_unmanaged, unregistered};
    use holler_pane::HerdrPort;
    let (rig, id) = rig_with_unmanaged(&[], Some(1));
    let snapshot = rig.herdr.snapshot().unwrap();
    let mut recorded = pane("demo-c1r1");
    recorded.herdr.pane_id = holler_pane::PaneId::new(id.clone());

    let listed: Vec<&str> = unregistered(&snapshot, &[])
        .map(|p| p.pane_id.as_str())
        .collect();
    assert_eq!(listed, [id.as_str()]);
    assert!(is_unmanaged(&snapshot.panes[0], &[]));
    assert!(!is_unmanaged(&snapshot.panes[0], &[recorded.clone()]));
    assert_eq!(unregistered(&snapshot, &[recorded]).count(), 0);
}
