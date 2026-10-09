//! `holler pane doctor` and the reconcile engine (#647): the incidents of 2026-10-07 on the
//! test kit's fakes, `--fix`, and the read-only pass.
//!
//! The engine is called directly (`holler_pane::reconcile::reconcile`, with a fixed clock)
//! where a test pins what it found or wrote; the verb is run through the in-process harness
//! where a test pins exit codes or output. The rig is `doctor/rig.rs`; scope, output and
//! remedy cases are `doctor/surface.rs`; the read-only pass is `doctor/read_only.rs`.

mod read_only;
mod rig;
mod surface;

use holler_cli::output::Format;
use holler_pane::findings::{FindingKind as K, FixState};
use holler_pane::pane::Health;
use holler_pane::reconcile::ReconcileRequest;
use holler_pane::{PaneError, PaneName};
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use rig::{json_data, json_findings, keys, kinds, of_kind, one, whole, Rig, Seed, NOW};

const PANE: &str = "demo-c1r1";

fn one_pane() -> Rig {
    Rig::new(&[Seed::new(PANE, 1, 1)])
}

/// A healthy fleet has nothing to report, and the first pass records what it saw.
#[test]
fn healthy_fleet_reports_nothing_and_records_the_observation() {
    let rig = Rig::new(&[Seed::new(PANE, 1, 1), Seed::new("demo-c2r1", 1, 2)]);
    let report = rig.run(&whole(false));
    assert!(report.findings.is_empty(), "{:?}", keys(&report));
    assert_eq!(report.panes.len(), 2, "{report:?}");
    let stored = rig.record(PANE);
    assert_eq!(stored.harness.health, Health::Healthy);
    assert_eq!(
        stored.last_observed.shown,
        Some(rig.live(PANE).session.clone())
    );
    assert_eq!(stored.last_observed.at, NOW);
}

// --- Incidents ------------------------------------------------------------------------

/// AC 1: the TUI was moved to a fresh session while the record (and so the hub) names S1.
#[test]
fn incident_tui_on_new_empty_session_while_hub_drives_the_old_one() {
    let rig = one_pane();
    let s1 = rig.live(PANE).session.clone();
    let s2 = rig.move_tui_to_new_session(PANE);

    let report = rig.run(&whole(false));

    let mismatch = one(&report, K::ShownDrivenMismatch, PANE);
    assert_eq!(
        mismatch.session.as_deref(),
        Some(s2.as_str()),
        "{mismatch:?}"
    );
    assert_eq!(mismatch.fix, FixState::Fixable, "{mismatch:?}");
    assert_eq!(
        mismatch.remedy.as_deref(),
        Some("holler pane doctor demo-c1r1 --fix")
    );
    let strays = of_kind(&report, K::StraySession);
    assert_eq!(strays.len(), 1, "{:?}", keys(&report));
    assert_eq!(strays[0].session.as_deref(), Some(s2.as_str()));
    assert_eq!(strays[0].pane, None, "a stray belongs to no pane");
    assert_eq!(report.findings.len(), 2, "{:?}", keys(&report));

    let stored = rig.record(PANE);
    assert_eq!(stored.last_observed.shown, Some(s2));
    assert_eq!(stored.last_observed.at, NOW);
    assert_eq!(
        stored.session_of_record,
        Some(s1),
        "never changed by doctor"
    );
}

/// AC 2: a frozen server (accepts, never answers).
#[test]
fn incident_three_day_wedged_server() {
    let rig = one_pane();
    rig.harness.freeze(rig.live(PANE).port).expect("freeze");

    let report = rig.run(&whole(false));

    assert_eq!(kinds(&report), ["server-wedged"], "{:?}", keys(&report));
    let wedged = one(&report, K::ServerWedged, PANE);
    assert_eq!(
        wedged.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
    assert_eq!(wedged.fix, FixState::NotFixable);
    assert_eq!(
        rig.record(PANE).harness.health,
        Health::Unhealthy("server-wedged".to_owned())
    );
}

/// AC 3: the orchestrator's TUI is attached to a bare server no record names.
#[test]
fn incident_bare_unregistered_harness_in_the_orchestrators_pane() {
    let rig = Rig::new(&[Seed::new(PANE, 1, 1).orchestrator()]);
    let foreign = rig.attach_foreign_tui(PANE);

    let report = rig.run(&whole(false));

    assert_eq!(
        kinds(&report),
        ["tui-foreign-session"],
        "{:?}",
        keys(&report)
    );
    let finding = one(&report, K::TuiForeignSession, PANE);
    assert_eq!(finding.session.as_deref(), Some(foreign.as_str()));
    assert_eq!(
        finding.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
    assert_eq!(finding.fix, FixState::NotFixable);
}

/// AC 3: a Herdr pane that no record names ("a process with no record"), whole fleet only.
#[test]
fn unregistered_herdr_pane_is_reported() {
    let rig = one_pane();
    let extra = rig.unregistered_herdr_pane(3, 3);

    let report = rig.run(&whole(false));

    assert_eq!(
        kinds(&report),
        ["unregistered-herdr-pane"],
        "{:?}",
        keys(&report)
    );
    let finding = &report.findings[0];
    assert_eq!(finding.pane, None);
    assert_eq!(finding.grid, Some(extra.grid));
    assert_eq!(finding.herdr_pane.as_ref(), Some(&extra.pane_id));
    assert_eq!(finding.remedy, None, "no holler verb adopts a pane");

    let name = PaneName::parse(PANE).expect("a pane name");
    let named = rig.run(&ReconcileRequest {
        pane: Some(&name),
        ..whole(false)
    });
    assert!(
        of_kind(&named, K::UnregisteredHerdrPane).is_empty(),
        "a named-pane run reports no unregistered pane: {:?}",
        keys(&named)
    );
}

/// AC 4: the registered pane's server died.
#[test]
fn incident_registered_pane_whose_process_died() {
    let rig = one_pane();
    rig.harness.kill(rig.live(PANE).port).expect("kill");

    let report = rig.run(&whole(false));

    assert_eq!(kinds(&report), ["server-down"], "{:?}", keys(&report));
    let down = one(&report, K::ServerDown, PANE);
    assert_eq!(
        down.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
    assert_eq!(
        rig.record(PANE).harness.health,
        Health::Unhealthy("server-down".to_owned())
    );
}

/// AC 4: the pane's tmux session ended outside Holler.
#[test]
fn tmux_session_gone_is_reported() {
    let rig = one_pane();
    rig.host.end_session(&rig.live(PANE).name).expect("end it");

    let report = rig.run(&whole(false));

    assert_eq!(
        kinds(&report),
        ["tmux-session-missing"],
        "{:?}",
        keys(&report)
    );
    let gone = one(&report, K::TmuxSessionMissing, PANE);
    assert_eq!(
        gone.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
}

/// AC 4: the pane's Herdr pane exited.
#[test]
fn herdr_pane_gone_is_reported() {
    let rig = one_pane();
    rig.herdr.vanish(&rig.pane_id(PANE)).expect("vanish");

    let report = rig.run(&whole(false));

    assert_eq!(
        kinds(&report),
        ["herdr-pane-missing"],
        "{:?}",
        keys(&report)
    );
    let gone = one(&report, K::HerdrPaneMissing, PANE);
    assert_eq!(
        gone.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
}

/// AC 5: a "ping" session no record names; on a shared data directory it is one finding.
#[test]
fn incident_stray_ping_session() {
    let rig = one_pane();
    let ping = rig.harness.seed_session(rig.live(PANE).port);
    let report = rig.run(&whole(false));
    assert_eq!(kinds(&report), ["stray-session"], "{:?}", keys(&report));
    let stray = &report.findings[0];
    assert_eq!(stray.session.as_deref(), Some(ping.as_str()));
    assert_eq!(stray.pane, None);
    assert_eq!(stray.ports, [48100]);
    assert_eq!(
        stray.remedy, None,
        "no holler verb deletes a harness session"
    );

    let two = Rig::new(&[Seed::new(PANE, 1, 1), Seed::new("demo-c2r1", 1, 2)]);
    let ping = two.harness.seed_session(two.live("demo-c2r1").port);
    let report = two.run(&whole(false));
    let strays = of_kind(&report, K::StraySession);
    assert_eq!(
        strays.len(),
        1,
        "one stray, seen on two servers: {:?}",
        keys(&report)
    );
    assert_eq!(strays[0].session.as_deref(), Some(ping.as_str()));
    assert_eq!(
        strays[0].ports,
        [48100, 48101],
        "every server it was seen on, sorted"
    );
}

/// The rig of AC 6: the session of record was deleted under the TUI (which goes home).
fn deleted_session_rig() -> Rig {
    let rig = one_pane();
    rig.harness
        .delete_session(&rig.live(PANE).session)
        .expect("delete S1");
    rig
}

/// AC 6.
#[test]
fn incident_deleted_session_under_a_tui() {
    let rig = deleted_session_rig();
    let report = rig.run(&whole(false));

    let missing = one(&report, K::SessionOfRecordMissing, PANE);
    assert_eq!(
        missing.remedy.as_deref(),
        Some("holler pane reset demo-c1r1")
    );
    let mismatch = one(&report, K::ShownDrivenMismatch, PANE);
    assert_eq!(mismatch.fix, FixState::NotFixable, "the server lacks S1");
    assert_eq!(
        mismatch.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1")
    );
    assert_eq!(report.findings.len(), 2, "{:?}", keys(&report));
}

// --- --fix ----------------------------------------------------------------------------

/// AC 7.
#[test]
fn fix_selects_the_session_of_record_and_never_changes_it() {
    let rig = one_pane();
    let s1 = rig.live(PANE).session.clone();
    rig.move_tui_to_new_session(PANE);
    let before = rig.record(PANE);
    let mark = rig.mark();

    let report = rig.run(&whole(true));

    let calls = rig.calls_since(&mark);
    let mismatch = one(&report, K::ShownDrivenMismatch, PANE);
    assert_eq!(mismatch.fix, FixState::Fixed, "{mismatch:?}");
    assert_eq!(mismatch.remedy, None);
    let tui = rig.harness.tui(&rig.pane_id(PANE)).expect("the TUI");
    assert_eq!(tui.shown.as_deref(), Some(s1.as_str()));

    let mut after = rig.record(PANE);
    assert_eq!(after.last_observed.shown.as_deref(), Some(s1.as_str()));
    assert_eq!(after.session_of_record.as_deref(), Some(s1.as_str()));
    after.generation = before.generation;
    after.last_observed = before.last_observed.clone();
    after.harness.health = before.harness.health.clone();
    assert_eq!(after, before, "no other field of the record changes");

    let selects = calls
        .harness
        .iter()
        .filter(|op| **op == HarnessOp::SelectSession);
    assert_eq!(selects.count(), 1, "{:?}", calls.harness);
    for writer in [
        HarnessOp::CreateSession,
        HarnessOp::Serve,
        HarnessOp::AttachTui,
        HarnessOp::Abort,
    ] {
        assert!(
            !calls.harness.contains(&writer),
            "{writer:?}: {:?}",
            calls.harness
        );
    }
    assert!(
        !calls.herdr.contains(&HerdrOp::SendText) && !calls.herdr.contains(&HerdrOp::SendKeys),
        "no keystroke (I4): {:?}",
        calls.herdr
    );
    assert!(
        calls.host.iter().all(|op| *op == HostOp::Ps),
        "{:?}",
        calls.host
    );
}

/// AC 8: after a fix, a second `--fix` finds nothing new and writes nothing.
#[test]
fn second_fix_run_reports_nothing_new() {
    let rig = one_pane();
    rig.move_tui_to_new_session(PANE);
    let first = rig.run(&whole(true));
    let mark = rig.mark();

    let second = rig.run(&whole(true));

    let calls = rig.calls_since(&mark);
    let first_keys = keys(&first);
    for key in keys(&second) {
        assert!(
            first_keys.contains(&key),
            "{key:?} is new; first run: {first_keys:?}"
        );
    }
    assert!(
        of_kind(&second, K::ShownDrivenMismatch).is_empty(),
        "{:?}",
        keys(&second)
    );
    assert!(
        !calls.panes.contains(&PaneStoreOp::CasPut),
        "{:?}",
        calls.panes
    );
}

/// AC 9: with no session of record, or with it gone, `--fix` picks no session.
#[test]
fn fix_never_guesses_a_session() {
    let rig = one_pane();
    rig.rewrite(PANE, |pane| pane.session_of_record = None);
    rig.harness.seed_session(rig.live(PANE).port);
    let mark = rig.mark();

    let report = rig.run(&whole(true));

    let calls = rig.calls_since(&mark);
    assert!(
        !calls.harness.contains(&HarnessOp::SelectSession),
        "{:?}",
        calls.harness
    );
    assert_eq!(
        rig.record(PANE).session_of_record,
        None,
        "doctor writes no session"
    );
    let finding = one(&report, K::NoSessionOfRecord, PANE);
    assert_eq!(
        finding.remedy.as_deref(),
        Some("holler pane reset demo-c1r1")
    );

    let deleted = deleted_session_rig();
    let mark = deleted.mark();
    deleted.run(&whole(true));
    let calls = deleted.calls_since(&mark);
    assert!(
        !calls.harness.contains(&HarnessOp::SelectSession),
        "{:?}",
        calls.harness
    );
}

/// AC 10: the orchestrator's TUI is switched only by a run that names it.
#[test]
fn fix_skips_the_orchestrator_unless_named() {
    let rig = Rig::new(&[Seed::new(PANE, 1, 1).orchestrator()]);
    rig.move_tui_to_new_session(PANE);
    let mark = rig.mark();

    let report = rig.run(&whole(true));

    let calls = rig.calls_since(&mark);
    let skipped = one(&report, K::ShownDrivenMismatch, PANE);
    assert_eq!(skipped.fix, FixState::Skipped, "{skipped:?}");
    assert_eq!(
        skipped.remedy.as_deref(),
        Some("holler pane doctor demo-c1r1 --fix")
    );
    assert!(
        !calls.harness.contains(&HarnessOp::SelectSession),
        "{:?}",
        calls.harness
    );

    let name = PaneName::parse(PANE).expect("a pane name");
    let named = rig.run(&ReconcileRequest {
        pane: Some(&name),
        ..whole(true)
    });
    assert_eq!(
        one(&named, K::ShownDrivenMismatch, PANE).fix,
        FixState::Fixed
    );
}

/// AC 11: a fix that fails is reported, and the pass still succeeds.
#[test]
fn fix_failure_is_reported() {
    let rig = one_pane();
    rig.move_tui_to_new_session(PANE);
    let unavailable = PaneError::Unavailable {
        what: "no TUI in pane".to_owned(),
    };
    rig.harness
        .faults()
        .fail_next(HarnessOp::SelectSession, unavailable);

    let run = rig.verb(&["pane", "doctor", "--fix"], Format::Json);

    let data = json_data(&run);
    let mismatches = json_findings(&data, "shown-driven-mismatch");
    assert_eq!(mismatches.len(), 1, "{data}");
    assert_eq!(mismatches[0]["fix"], "failed", "{data}");
    assert_eq!(mismatches[0]["fix_error"]["code"], "unavailable", "{data}");
    assert_eq!(
        mismatches[0]["remedy"], "holler pane relaunch demo-c1r1",
        "{data}"
    );
}
