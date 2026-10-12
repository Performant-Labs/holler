//! The profile `fleet` under the switch (#831, AC5, AC6, AC11, decision D1): with
//! `--adopt-claude-code` an existing `fleet` is extended by one compare-and-swap instead of
//! being refused `profile-exists`, whatever the kind of the panes recorded; a re-run changes
//! nothing; and a dry run says what a real run would do. Without the switch an existing `fleet`
//! is still `profile-exists` (`writes.rs`, unchanged).

use std::collections::BTreeSet;

use holler_cli::output::Format;
use holler_pane::Profile;
use holler_pane_testkit::fixture::sample_spec;
use holler_pane_testkit::profile_store::ProfileStoreOp;

use super::adopt_rig::{
    findings_about, spec_names, AdoptRig, Mark, ADOPT, ANTHROPIC_MODEL, CLAUDE_SESSION,
};
use super::rig::{
    assert_no_store_write, data_of, error_of, findings, message, one_finding, pane_row, FLEET,
};
use crate::doctor::rig::Seed;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";

/// `A` as an OpenCode pane and `B` as a Claude Code pane.
fn grid() -> AdoptRig {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    rig.typed_claude(B, CLAUDE_SESSION);
    rig
}

fn profile_ops(rig: &AdoptRig, mark: &Mark) -> Vec<ProfileStoreOp> {
    rig.since(mark).world.profiles
}

fn names(profile: &Profile) -> BTreeSet<String> {
    profile.panes.iter().map(|spec| spec.pane.clone()).collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

// --- an existing fleet is extended (AC5, D1) ------------------------------------------------

/// AC5: the records are written, then their specs are added to `fleet` by one `cas_put` at the
/// generation just read, with the change log's actor `holler pane import`, instead of
/// `profile-exists`; no profile is created, deleted or renamed; every record is in `fleet`.
#[test]
fn an_existing_fleet_is_extended_by_one_cas_put() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    let mark = rig.mark_all();
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let ops = profile_ops(&rig, &mark);
    assert_eq!(
        ops.iter()
            .filter(|op| **op == ProfileStoreOp::CasPut)
            .count(),
        1,
        "one cas_put of fleet: {ops:?}"
    );
    assert!(
        !ops.iter()
            .any(|op| matches!(op, ProfileStoreOp::Delete | ProfileStoreOp::Rename)),
        "{ops:?}"
    );

    assert_eq!(data["recorded"], 2, "{data}");
    let stored = rig.fleet_now();
    assert_eq!(stored.generation, before.generation + 1, "one write");
    assert_eq!(names(&stored), set(&[A, B]));
    assert_eq!(
        data["profile"],
        serde_json::to_value(&stored).expect("JSON")
    );
    for record in rig.records() {
        assert_eq!(record.profile.as_ref().map(|p| p.as_str()), Some(FLEET));
    }
    assert_eq!(
        rig.fleet_actors().last().map(String::as_str),
        Some("holler pane import"),
        "the change log names the import"
    );
}

/// D1 = A: the switch's meaning grows from "adopt Claude Code panes" to "extend the existing
/// `fleet`", so a run with no Claude Code pane at all extends it too with its OpenCode panes.
#[test]
fn with_the_switch_an_existing_fleet_takes_opencode_panes_too() {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    let before = rig.seed_fleet(&[]);
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(data["recorded"], 2, "{data}");
    let stored = rig.fleet_now();
    assert_eq!(stored.generation, before.generation + 1);
    assert_eq!(names(&stored), set(&[A, B]));
}

/// AC5: when every spec of a recorded pane exists in `fleet` already (a closed pane: it has a
/// spec and no record) the records are written and `fleet`'s generation does not move, with no
/// `cas_put` of it. An existing spec is left as it is, and a spec of the same kind says nothing.
#[test]
fn an_extension_with_no_new_spec_writes_the_record_and_leaves_fleet_alone() {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1)]);
    let before = rig.seed_fleet(&[A]);
    let mark = rig.mark_all();
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(data["recorded"], 1, "{data}");
    assert!(rig.record(A).is_some(), "the record is written");
    assert!(
        !profile_ops(&rig, &mark).contains(&ProfileStoreOp::CasPut),
        "no cas_put of fleet"
    );
    assert_eq!(rig.fleet_now(), before, "fleet is exactly as it was");
    assert!(findings(&data, "spec-kind-differs").is_empty(), "{data}");
}

/// AC5: an existing spec for a recorded pane's name is left unchanged even when its harness kind
/// differs from the new record's, and the pane gets the warning `spec-kind-differs`, naming the
/// pane and the two kinds, so the permanent `differs` of a later `profile apply --dry-run` is
/// explained. The pane is still recorded, and the specs of the other panes are added.
#[test]
fn an_existing_spec_of_another_kind_is_kept_and_warned_about() {
    let rig = grid();
    let before = rig.seed_fleet(&[B]);
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let warning = one_finding(&data, "spec-kind-differs", B, "warning");
    let text = message(warning);
    assert!(
        text.contains("opencode") && text.contains("claude"),
        "{text}"
    );
    one_finding(&data, "claude-code-adopted", B, "info");
    assert_eq!(pane_row(&data, B)["action"], "record", "{data}");
    assert!(findings_about(&data, A)
        .iter()
        .all(|f| f["kind"] != "spec-kind-differs"));

    let stored = rig.fleet_now();
    assert_eq!(
        stored.generation,
        before.generation + 1,
        "A's spec was added"
    );
    assert_eq!(names(&stored), set(&[A, B]));
    let kept = stored.panes.iter().find(|s| s.pane == B).expect("B's spec");
    assert_eq!(*kept, sample_spec(B), "the existing spec is unchanged");
    assert!(rig.record(B).is_some());
}

// --- the dry run of an extension (AC11) ----------------------------------------------------------

/// AC11: a dry run against an existing `fleet` exits 0, writes nothing, and says it would extend:
/// `profile fleet (fleet) to extend: N specs added, M total`, then the added specs as `profile
/// show` prints a spec. In JSON `profile` keeps its keys and carries the profile a real run would
/// store, at the generation it was read at. A dry run's report is the real run's.
#[test]
fn a_dry_run_against_an_existing_fleet_says_to_extend_and_equals_the_real_plan() {
    let rig = grid();
    let before = rig.seed_fleet(&["demo-existing"]);
    let mark = rig.mark_all();
    let dry = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    assert_no_store_write(&rig.since(&mark).world);
    assert_eq!(
        dry.as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        set(&[
            "dry_run",
            "state_root",
            "recorded",
            "profile",
            "panes",
            "findings"
        ]),
        "no key is added"
    );
    assert_eq!(
        dry["profile"]["generation"], before.generation,
        "the generation read"
    );
    let planned: BTreeSet<String> = spec_names(&dry).into_iter().collect();
    assert_eq!(planned, set(&["demo-existing", A, B]));
    assert!(rig.records().is_empty(), "no record");
    assert_eq!(rig.fleet_now(), before, "fleet unchanged");

    let text = rig.run_routed(&[ADOPT, "--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    let lines: Vec<&str> = text.out.lines().collect();
    assert!(
        lines.contains(&"profile fleet (fleet) to extend: 2 specs added, 3 total"),
        "{}",
        text.out
    );
    for line in [format!("spec {A}"), format!("spec {B}")] {
        assert!(lines.contains(&line.as_str()), "{line:?} in {}", text.out);
    }
    assert!(
        !text.out.contains("to create"),
        "an existing fleet is extended, not created: {}",
        text.out
    );

    let real = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(real["panes"], dry["panes"]);
    assert_eq!(real["findings"], dry["findings"]);
    assert_eq!(real["profile"]["panes"], dry["profile"]["panes"]);
    assert_eq!(real["profile"]["generation"], before.generation + 1);
}

// --- idempotence (AC6) ---------------------------------------------------------------------------

/// AC6: a re-run after an adoption finds every pane with a record: `already-imported` (info),
/// exit 0, nothing recorded, no store write, `fleet`'s generation unchanged; with or without the
/// switch. Nothing is read from the panes' tmux sessions or from the harness, and with the
/// switch and nothing to record, `fleet` is not even read.
#[test]
fn a_second_run_after_an_adoption_is_a_no_op() {
    let rig = grid();
    data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let generation = rig.fleet_generation();
    for extra in [&[ADOPT][..], &[]] {
        let mark = rig.mark_all();
        let data = data_of(&rig.run_routed(extra, Format::Json));
        let calls = rig.since(&mark);
        assert_no_store_write(&calls.world);
        assert_eq!(data["recorded"], 0, "{extra:?}: {data}");
        for pane in [A, B] {
            assert_eq!(
                pane_row(&data, pane)["action"],
                "already-imported",
                "{data}"
            );
            one_finding(&data, "already-imported", pane, "info");
        }
        assert_eq!(calls.world.host, vec![], "{extra:?}: no tmux read");
        assert_eq!(calls.claude, vec![], "{extra:?}: no Claude read");
        assert_eq!(calls.world.profiles, vec![], "{extra:?}: fleet is not read");
        assert_eq!(
            rig.fleet_generation(),
            generation,
            "{extra:?}: fleet unchanged"
        );
    }
}

/// The case the switch exists for: a pane added since the first run. Without the switch the
/// second run is refused `profile-exists`, as today; with it the new pane is recorded and its
/// spec added to `fleet`, and the adopted pane keeps its record exactly.
#[test]
fn a_pane_added_since_is_recorded_into_fleet_beside_the_adopted_ones() {
    let c = "hj-c2r1";
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1), Seed::new(c, 1, 2)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    rig.typed_claude(B, CLAUDE_SESSION);
    std::fs::remove_dir_all(rig.path(c)).expect("the new pane's directory is not there yet");
    let first = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(first["recorded"], 2, "{first}");
    let adopted = rig.record(B);
    let generation = rig.fleet_generation();

    rig.write_agreeing_files(c);
    let refused = rig.run_routed(&[], Format::Json);
    assert_eq!(refused.code, 3, "{refused:?}");
    assert_eq!(error_of(&refused).0, "profile-exists");
    assert!(
        rig.record(c).is_none(),
        "nothing written without the switch"
    );

    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(data["recorded"], 1, "{data}");
    assert_eq!(pane_row(&data, c)["action"], "record", "{data}");
    for pane in [A, B] {
        assert_eq!(
            pane_row(&data, pane)["action"],
            "already-imported",
            "{data}"
        );
    }
    assert_eq!(
        rig.record(B),
        adopted,
        "the adopted pane's record is untouched"
    );
    let stored = rig.fleet_now();
    assert_eq!(stored.generation, generation + 1);
    assert_eq!(names(&stored), set(&[A, B, c]));
}
