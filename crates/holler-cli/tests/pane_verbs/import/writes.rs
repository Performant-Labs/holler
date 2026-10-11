//! The dry run and the real run's writes (#650 acceptance 2, 3, 9, 10, 14): a dry run reads
//! and writes nothing; a real run writes the profile `fleet` through `insert_profile`, then
//! each record, or writes nothing at all; a re-run is a no-op; a failed write is undone.

use std::collections::BTreeSet;

use holler_cli::output::Format;
use holler_pane::pane::PaneRole;
use holler_pane::profile_snapshot::profile_from_panes;
use holler_pane::{Actor, PaneError, ProfileName};
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::herdr::HerdrVersion;
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::Value;

use super::rig::{
    assert_absent, assert_no_store_write, assert_only_allowed_reads, data_of, error_of, findings,
    pane_row, ImportRig, FLEET, SECRET,
};
use super::seams::{NthCasPut, NthPut};
use crate::doctor::rig::Seed;
use crate::verb_harness::run_verb_with;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";

/// Two panes, `A` at `r1c1` and `B` at `r2c1`.
fn two_panes() -> ImportRig {
    ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)])
}

fn fleet_name() -> ProfileName {
    ProfileName::parse(FLEET).expect("a profile name")
}

/// The keys of a JSON object.
fn keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("an object: {value}"))
        .keys()
        .cloned()
        .collect()
}

fn set(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|k| (*k).to_owned()).collect()
}

// --- Acceptance 2: the dry run ------------------------------------------------------------

#[test]
fn a_dry_run_reads_only_and_reports_the_profile_it_would_create() {
    let rig = two_panes();
    rig.write("opencode-mo/opencode.json", "{}\n");
    let before = rig.files();
    let mark = rig.mark();
    let data = data_of(&rig.run(&["--dry-run"], Format::Json));
    let calls = rig.calls_since(&mark);
    assert_only_allowed_reads(&calls);
    assert_no_store_write(&calls);
    assert_eq!(
        rig.files(),
        before,
        "every file under the state root is unchanged"
    );

    assert_eq!(
        keys(&data),
        set(&[
            "dry_run",
            "state_root",
            "recorded",
            "profile",
            "panes",
            "findings"
        ])
    );
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["state_root"], rig.root_str().as_str());
    assert_eq!(data["recorded"], 0);
    assert_eq!(
        keys(pane_row(&data, A)),
        set(&["name", "action", "role", "pos", "session_of_record"])
    );
    let ignored = findings(&data, "ignored-entry");
    assert_eq!(ignored.len(), 1, "{data}");
    assert_eq!(keys(ignored[0]), set(&["pane", "level", "kind", "message"]));
    assert_eq!(ignored[0]["pane"], Value::Null);

    let would = profile_from_panes(
        &fleet_name(),
        &[
            rig.expected_record(A, PaneRole::Agent),
            rig.expected_record(B, PaneRole::Agent),
        ],
    );
    assert_eq!(data["profile"], serde_json::to_value(&would).expect("JSON"));
    assert!(rig.records().is_empty(), "no record");
    assert!(rig.fleet().is_none(), "no profile");
}

/// The text form of a dry run ends with the profile, as `profile show` prints a spec.
#[test]
fn a_dry_run_prints_the_profile_as_profile_show_prints_a_spec() {
    let rig = two_panes();
    let run = rig.run(&["--dry-run"], Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.out.lines().collect();
    for line in [
        "spec hj-c1r1",
        "spec hj-c1r2",
        "  herdr.grid: r2c1",
        "  model.model_id: demo/model-1",
        "  env: [\"OC_KEY_FILE\",\"DEMO_TOKEN\"]",
    ] {
        assert!(lines.contains(&line), "{line:?} in {}", run.out);
    }
    assert_absent(&run, SECRET);
}

// --- Acceptance 3: the real run -----------------------------------------------------------

#[test]
fn a_real_run_creates_fleet_and_every_member_matches_it() {
    let rig = two_panes();
    let before = rig.files();
    let dry = data_of(&rig.run(&["--dry-run"], Format::Json));
    rig.world.profiles.set_now(1_234);
    let run = rig.run(&[], Format::Json);
    let data = data_of(&run);
    assert_eq!(data["dry_run"], false);
    assert_eq!(data["recorded"], 2);

    let stored = rig.fleet().expect("fleet exists");
    assert_eq!(stored.generation, 1);
    assert_eq!(
        data["profile"],
        serde_json::to_value(&stored).expect("JSON")
    );
    let mut unstamped = data["profile"].clone();
    for (key, value) in [("generation", 0), ("created", 0), ("updated", 0)] {
        unstamped[key] = value.into();
    }
    assert_eq!(
        unstamped, dry["profile"],
        "the two runs differ only in the stamps"
    );
    assert_eq!(data["profile"]["created"], 1_234);

    for record in rig.records() {
        assert_eq!(record.profile, Some(fleet_name()), "{record:?}");
    }
    let show = run_verb_with(&["profile", "show", FLEET], Format::Json, rig.world.ports());
    let shown = data_of(&show);
    let statuses: Vec<&Value> = shown["comparison"]
        .as_array()
        .unwrap_or_else(|| panic!("a comparison: {shown}"))
        .iter()
        .map(|row| &row["status"])
        .collect();
    assert_eq!(statuses, [&Value::from("matches"), &Value::from("matches")]);

    assert_absent(&run, SECRET);
    let stored_text = serde_json::to_string(&(rig.records(), &stored)).expect("JSON");
    assert!(!stored_text.contains(SECRET), "no record holds a value");
    assert_eq!(rig.files(), before, "no source file changed");
}

// --- Acceptance 9: re-runs and an existing fleet ----------------------------------------

#[test]
fn a_second_real_run_is_a_no_op() {
    let rig = two_panes();
    data_of(&rig.run(&[], Format::Json));
    let mark = rig.mark();
    let data = data_of(&rig.run(&[], Format::Json));
    assert_no_store_write(&rig.calls_since(&mark));
    assert_eq!(data["recorded"], 0);
    for name in [A, B] {
        assert_eq!(pane_row(&data, name)["action"], "already-imported");
        let found: Vec<&Value> = findings(&data, "already-imported")
            .into_iter()
            .filter(|f| f["pane"] == name)
            .collect();
        assert_eq!(found.len(), 1, "{data}");
        assert_eq!(found[0]["level"], "info");
    }
    assert_eq!(rig.fleet().expect("fleet").generation, 1, "fleet unchanged");
}

#[test]
fn an_existing_fleet_with_a_pane_to_record_is_profile_exists() {
    let rig = two_panes();
    let actor = Actor::parse("someone-else").expect("an actor");
    let existing = sample_profile(FLEET, &[]).expect("a profile");
    rig.world
        .profiles
        .concurrent_put(&existing, &actor)
        .expect("seed fleet");
    let before = rig.fleet();
    let mark = rig.mark();
    let run = rig.run(&[], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "profile-exists");
    assert_no_store_write(&rig.calls_since(&mark));
    assert!(rig.records().is_empty());
    assert_eq!(rig.fleet(), before, "fleet unchanged");
}

#[test]
fn a_state_root_with_no_pane_writes_nothing() {
    let rig = ImportRig::new(&[]);
    let mark = rig.mark();
    let data = data_of(&rig.run(&[], Format::Json));
    assert_eq!(data["recorded"], 0);
    assert_eq!(data["panes"], Value::Array(vec![]));
    assert_no_store_write(&rig.calls_since(&mark));
    assert!(rig.fleet().is_none(), "no profile");
}

// --- Acceptance 10: a failed write records nothing --------------------------------------

/// The real run of `rig` with the pane store's second `cas_put` acted on as `step`.
fn run_failing_write_2(rig: &ImportRig, step: NthPut) -> (i32, String, String) {
    let nth = NthCasPut::new(&rig.world.panes, vec![(2, step)]);
    let run = rig.run_with_store(&nth, &rig.args(&[]), Format::Json);
    let (code, message) = error_of(&run);
    assert!(
        !message.contains("pane close"),
        "never pane close: {message}"
    );
    (run.code, code, message)
}

/// The tail of the pane store's log since `mark`.
fn pane_ops_tail(rig: &ImportRig, mark: &crate::doctor::rig::Calls, n: usize) -> Vec<PaneStoreOp> {
    let ops = rig.calls_since(mark).panes;
    ops[ops.len().saturating_sub(n)..].to_vec()
}

/// (a) Write 2 fails and writes nothing: the undo finds no second record, deletes the first,
/// then the profile; the verb answers the write's own code.
#[test]
fn a_failed_write_is_undone_and_answers_its_own_code() {
    let rig = two_panes();
    let mark = rig.mark();
    let (exit, code, _) = run_failing_write_2(&rig, NthPut::Fail(PaneError::Conflict));
    // Read the log before `records()`, whose own `List` lands in it.
    let tail = pane_ops_tail(&rig, &mark, 3);
    assert_eq!((exit, code.as_str()), (1, "generation-conflict"));
    assert!(rig.records().is_empty(), "the pane store is empty");
    assert!(rig.fleet().is_none(), "fleet is deleted");
    assert_eq!(
        tail[..],
        [PaneStoreOp::CasPut, PaneStoreOp::Get, PaneStoreOp::Delete],
        "write 1, the undo's get of B, the delete of A"
    );
}

/// (b) Write 2 lands and answers `timeout`: the undo finds it and deletes it too.
#[test]
fn a_write_that_landed_and_timed_out_is_undone_too() {
    let rig = two_panes();
    let mark = rig.mark();
    let timeout = PaneError::Timeout {
        op: "pane_store.cas_put".to_owned(),
    };
    let (exit, code, _) = run_failing_write_2(&rig, NthPut::ApplyThenFail(timeout));
    // Read the log before `records()`, whose own `List` lands in it.
    let tail = pane_ops_tail(&rig, &mark, 3);
    assert_eq!((exit, code.as_str()), (1, "timeout"));
    assert!(rig.records().is_empty(), "the pane store is empty");
    assert!(rig.fleet().is_none(), "fleet is deleted");
    assert_eq!(
        tail[..],
        [PaneStoreOp::Get, PaneStoreOp::Delete, PaneStoreOp::Delete],
        "the undo's get of B, the delete of B, then of A"
    );
}

/// (c) The undo's profile delete fails: `profile-conflict`, naming the remedy for it.
#[test]
fn a_failed_profile_delete_is_profile_conflict_with_its_remedy() {
    let rig = two_panes();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    rig.world
        .profiles
        .faults()
        .fail_next(ProfileStoreOp::Delete, unavailable);
    let (exit, code, message) = run_failing_write_2(&rig, NthPut::Fail(PaneError::Conflict));
    assert_eq!((exit, code.as_str()), (1, "profile-conflict"));
    assert!(rig.records().is_empty(), "the pane store is empty");
    assert!(rig.fleet().is_some(), "fleet remains");
    assert!(
        message.contains("holler profile delete 'fleet'"),
        "the remedy: {message}"
    );
}

/// (d) The undo's delete of the first record fails: it stops there, and the message names
/// the remedy for records kept.
#[test]
fn a_failed_record_delete_stops_the_undo_with_its_remedy() {
    let rig = two_panes();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    rig.world
        .panes
        .faults()
        .fail_next(PaneStoreOp::Delete, unavailable);
    let (exit, code, message) = run_failing_write_2(&rig, NthPut::Fail(PaneError::Conflict));
    assert_eq!((exit, code.as_str()), (1, "profile-conflict"));
    let kept = rig.record(A).expect("the first record remains");
    assert_eq!(kept.profile, Some(fleet_name()));
    assert!(rig.record(B).is_none());
    assert!(rig.fleet().is_some(), "fleet remains");
    for step in [
        "holler profile delete 'fleet' --keep-panes",
        "holler profile create 'fleet' --from-current",
    ] {
        assert!(message.contains(step), "{step:?} in {message}");
    }
}

// --- Acceptance 14: an unsupported Herdr -----------------------------------------------

#[test]
fn an_unsupported_herdr_ends_the_import_with_its_error() {
    let rig = two_panes();
    rig.world.herdr.set_version(HerdrVersion::Unsupported);
    let mark = rig.mark();
    for extra in [&["--dry-run"][..], &[]] {
        let run = rig.run(extra, Format::Json);
        assert_eq!(run.code, 3, "{run:?}");
        assert_eq!(error_of(&run).0, "herdr-version-unsupported");
        assert!(!run.out.contains("findings"), "no report: {run:?}");
        let text = rig.run(extra, Format::Text);
        assert_eq!(text.code, 3, "{text:?}");
        assert!(text.out.is_empty(), "no report: {text:?}");
    }
    assert_no_store_write(&rig.calls_since(&mark));
    assert!(rig.records().is_empty());
}
