//! `holler pane doctor` without `--fix` (#647, ACs 12 and 13): a pass makes read calls
//! only, and writes a record only when what it observed changed.

use holler_pane::findings::FindingKind as K;
use holler_pane::PaneError;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::rig::{keys, kinds, one, whole, Rig, Seed};
use super::{one_pane, PANE};

/// Every incident at once, on one fleet.
fn every_incident_rig() -> Rig {
    let rig = Rig::new(&[
        Seed::new("demo-mismatch", 1, 1),
        Seed::new("demo-wedged", 1, 2),
        Seed::new("demo-down", 1, 3),
        Seed::new("demo-notmux", 2, 1),
        Seed::new("demo-noherdr", 2, 2),
        Seed::new("demo-deleted", 2, 3),
        Seed::new("demo-orch", 3, 1).orchestrator(),
    ]);
    rig.move_tui_to_new_session("demo-mismatch");
    rig.harness
        .freeze(rig.live("demo-wedged").port)
        .expect("freeze");
    rig.harness.kill(rig.live("demo-down").port).expect("kill");
    rig.host
        .end_session(&rig.live("demo-notmux").name)
        .expect("end");
    rig.herdr
        .vanish(&rig.pane_id("demo-noherdr"))
        .expect("vanish");
    rig.harness
        .delete_session(&rig.live("demo-deleted").session)
        .expect("delete");
    rig.attach_foreign_tui("demo-orch");
    rig.harness.seed_session(rig.live("demo-mismatch").port);
    rig.unregistered_herdr_pane(3, 3);
    rig
}

/// AC 12.
#[test]
fn doctor_without_fix_makes_only_read_calls() {
    let rig = every_incident_rig();
    let mark = rig.mark();

    let report = rig.run(&whole(false));

    let calls = rig.calls_since(&mark);
    for kind in [
        "shown-driven-mismatch",
        "server-wedged",
        "server-down",
        "tmux-session-missing",
        "herdr-pane-missing",
        "session-of-record-missing",
        "tui-foreign-session",
        "stray-session",
        "unregistered-herdr-pane",
    ] {
        assert!(
            kinds(&report).contains(&kind),
            "{kind} in one pass: {:?}",
            keys(&report)
        );
    }
    let reads = [
        HarnessOp::Health,
        HarnessOp::ListSessions,
        HarnessOp::ShownSession,
    ];
    assert!(
        calls.harness.iter().all(|op| reads.contains(op)),
        "{:?}",
        calls.harness
    );
    let reads = [HerdrOp::Snapshot, HerdrOp::Version];
    assert!(
        calls.herdr.iter().all(|op| reads.contains(op)),
        "{:?}",
        calls.herdr
    );
    assert!(
        calls.host.iter().all(|op| *op == HostOp::Ps),
        "{:?}",
        calls.host
    );
    let reads = [PaneStoreOp::List, PaneStoreOp::Get, PaneStoreOp::CasPut];
    assert!(
        calls.panes.iter().all(|op| reads.contains(op)),
        "{:?}",
        calls.panes
    );
    assert!(calls.profiles.is_empty(), "{:?}", calls.profiles);
    assert_eq!(calls.probes, 0);
}

/// AC 13: an observation that did not change writes nothing (`panes/mod.rs:27-28`).
#[test]
fn unchanged_observation_writes_nothing() {
    let rig = Rig::new(&[Seed::new(PANE, 1, 1), Seed::new("demo-c2r1", 1, 2)]);
    rig.run(&whole(false));
    let generations = |rig: &Rig| {
        [
            rig.record(PANE).generation,
            rig.record("demo-c2r1").generation,
        ]
    };
    let before = generations(&rig);
    let mark = rig.mark();

    rig.run(&whole(false));

    let calls = rig.calls_since(&mark);
    assert!(
        !calls.panes.contains(&PaneStoreOp::CasPut),
        "{:?}",
        calls.panes
    );
    assert_eq!(generations(&rig), before);
}

/// Decision 6: a verb that wrote the record meanwhile wins; the conflict is reported once
/// and not retried.
#[test]
fn record_write_conflict_is_reported_not_retried() {
    let rig = one_pane();
    rig.panes
        .faults()
        .fail_next(PaneStoreOp::CasPut, PaneError::Conflict);
    let mark = rig.mark();

    let report = rig.run(&whole(false));

    let calls = rig.calls_since(&mark);
    let failed = one(&report, K::ObserveFailed, PANE);
    assert_eq!(
        failed.remedy.as_deref(),
        Some("holler pane doctor demo-c1r1")
    );
    let writes = calls.panes.iter().filter(|op| **op == PaneStoreOp::CasPut);
    assert_eq!(writes.count(), 1, "not retried: {:?}", calls.panes);
}
