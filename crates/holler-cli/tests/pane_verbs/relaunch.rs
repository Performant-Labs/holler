//! `holler pane relaunch PANE` (story #644, brief AC 5b, 6a, 16b, 16g, 16i and 18-23): the same
//! transaction as launch after stopping only what the pane owns, keeping its cell (unless
//! `--grid` moves it) and its session of record. Every case runs over launch's rig
//! (`crate::launch::rig`) on a pane the rig launched first.

use holler_cli::output::Format;
use holler_pane::pane::PaneRole;
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::tx_launch::{self, LaunchRequest, RelaunchRequest, TxFailure, TxOptions};
use holler_pane::{GridPos, HarnessPort, Pane, PaneError, ProfileSpec};
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::harness::{HarnessOp, ServerState};
use holler_pane_testkit::herdr::HerdrOp;

use crate::launch::rig::{
    agent, argv, assert_untouched, data_of, error_of, launch_spec, name, occurrences, profile_name,
    Calls, Rig, Seed, PANE, PORT, STEP,
};
use crate::verb_harness::parse::assert_spec_flags_accepted;

const RELAUNCH: &[&str] = &["pane", "relaunch", PANE];
const R2C1: GridPos = GridPos { row: 2, col: 1 };
const R3C1: GridPos = GridPos { row: 3, col: 1 };

/// `RELAUNCH` followed by `extra`.
fn relaunch_with<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut argv = RELAUNCH.to_vec();
    argv.extend_from_slice(extra);
    argv
}

/// The relaunch's Herdr calls hold no `Close`.
fn assert_no_close(calls: &Calls) {
    assert!(!calls.herdr.contains(&HerdrOp::Close), "{:?}", calls.herdr);
}

/// AC 5b: a pane whose Herdr pane vanished is recreated at its own cell, and the new id is
/// recorded.
#[test]
fn relaunch_recreates_a_vanished_pane_at_its_cell() {
    let rig = Rig::new();
    let before = rig.live(false);
    rig.fakes.herdr.vanish(&before.herdr.pane_id).unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    let after = rig.record(PANE).unwrap();
    let at = rig.herdr_pane_at(R2C1).expect("a pane at r2c1");
    assert_ne!(after.herdr.pane_id, before.herdr.pane_id, "a new pane");
    assert_eq!(
        (after.herdr.pane_id.clone(), after.herdr.grid),
        (at.pane_id, R2C1)
    );
    assert_no_close(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 6a: another writer that moves the record during the act wins; the run fails loudly
/// with the unscoped reconcile step.
#[test]
fn relaunch_fails_on_a_stale_generation() {
    let rig = Rig::new();
    let theirs = Pane {
        role: PaneRole::Orchestrator,
        ..rig.live(false)
    };
    rig.after(HarnessOp::Serve, move |f, _| {
        f.panes.concurrent_put(&theirs).unwrap();
    });
    let run = rig.run(RELAUNCH, Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "generation-conflict");
    assert!(message.ends_with(&format!("; {STEP}")), "{message}");
    let stored = rig.record(PANE).unwrap();
    assert_eq!(
        (stored.generation, stored.role),
        (2, PaneRole::Orchestrator),
        "theirs"
    );
    rig.assert_no_keystroke();
}

/// AC 16b: a relaunch with `--profile` and a flag edits P's spec and the record together.
#[test]
fn relaunch_with_profile_and_model_updates_the_spec() {
    let rig = Rig::new();
    rig.live(true);
    let argv = relaunch_with(&["--profile", "demo", "--model", "demo-provider/other-model"]);
    let run = rig.run(&argv, Format::Json);
    data_of(&run);
    let profile = rig.profile("demo");
    assert_eq!(profile.generation, 3);
    assert_eq!(profile.panes.len(), 1);
    assert_eq!(profile.panes[0].model.model_id, "other-model");
    assert_eq!(rig.record(PANE).unwrap().model.model_id, "other-model");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16g: `relaunch --spec-only` gives P the spec and leaves the record and every fake as
/// they were.
#[test]
fn relaunch_spec_only_changes_the_profile_and_nothing_live() {
    let rig = Rig::new();
    let before = rig.live(true);
    let argv = relaunch_with(&[
        "--profile",
        "demo",
        "--spec-only",
        "--model",
        "demo-provider/m2",
    ]);
    let run = rig.run(&argv, Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.out.contains("nothing live changed"), "{run:?}");
    let profile = rig.profile("demo");
    assert_eq!(profile.generation, 3);
    assert_eq!(profile.panes[0].model.model_id, "m2");
    let live = (&run.calls.herdr, &run.calls.host, &run.calls.harness);
    assert_eq!(
        (live.0.len(), live.1.len(), live.2.len()),
        (0, 0, 0),
        "{live:?}"
    );
    assert_eq!(rig.record(PANE), Some(before), "the record is unchanged");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 16i: a pane of another profile is refused by the scope before anything is written.
#[test]
fn relaunch_refuses_a_pane_of_another_profile() {
    let profiles = ["demo", "other"].map(|p| sample_profile(p, &[]).unwrap());
    let rig = Rig::with(Vec::new(), profiles.to_vec());
    rig.seed_live(&Seed {
        profile: Some("other"),
        ..Seed::default()
    });
    let run = rig.run(&relaunch_with(&["--profile", "demo"]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "pane-in-other-profile");
    let reads_only = Calls {
        herdr: Vec::new(),
        ..run.calls.clone()
    };
    assert_untouched(&reads_only);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 18: without `--grid` a relaunch keeps the pane where it is.
#[test]
fn relaunch_without_grid_keeps_the_position() {
    let rig = Rig::new();
    let before = rig.live(false);
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert_eq!(rig.record(PANE).unwrap().herdr, before.herdr);
    assert_no_close(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 18b: a relaunch that fails never closes the record's own pane, and leaves the record.
#[test]
fn a_failed_relaunch_never_closes_the_records_pane() {
    let rig = Rig::new();
    let before = rig.live(false);
    let missing = PaneError::SessionNotFound {
        what: "ses_gone".into(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::AttachTui, missing);
    let run = rig.run(RELAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "session-not-found");
    assert_no_close(&run.calls);
    let at = rig.herdr_pane_at(R2C1).expect("the record's pane stays");
    assert_eq!(at.pane_id, before.herdr.pane_id);
    assert_eq!(rig.record(PANE), Some(before), "the record is unchanged");
    rig.assert_no_keystroke();
}

/// AC 19: `--grid` moves the pane: the new cell's pane is recorded and the old one closed.
#[test]
fn relaunch_with_grid_moves_the_pane() {
    let rig = Rig::new();
    let before = rig.live(false);
    let run = rig.run(&relaunch_with(&["--grid", "c1r3"]), Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.out.contains("r3c1"), "{run:?}");
    let after = rig.record(PANE).unwrap();
    let at = rig.herdr_pane_at(R3C1).expect("a pane at r3c1");
    assert_eq!(
        (after.herdr.grid, after.herdr.pane_id.clone()),
        (R3C1, at.pane_id)
    );
    assert!(
        !rig.herdr_ids().contains(&before.herdr.pane_id),
        "the old pane is gone"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 19: without `--grid`, a changed workspace or directory is `usage`, naming both whole
/// positions for the workspace.
#[test]
fn relaunch_refuses_a_move_without_grid_and_a_new_directory() {
    for (flag, value) in [("--workspace", "other"), ("--project", "/srv/other")] {
        let rig = Rig::new();
        rig.live(false);
        let run = rig.run(&relaunch_with(&[flag, value]), Format::Json);
        assert_eq!(run.code, 2, "{flag}: {run:?}");
        let (code, message) = error_of(&run);
        assert_eq!(code, "usage", "{flag}");
        if flag == "--workspace" {
            for needle in ["main", "other", "r2c1"] {
                assert!(message.contains(needle), "names {needle}: {message}");
            }
        }
        assert_untouched(&run.calls);
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// AC 19b: a relaunch request for the engine, as #664 would build it.
fn request(record: &Pane, spec: ProfileSpec, spec_only: bool) -> RelaunchRequest {
    RelaunchRequest {
        record: record.clone(),
        spec,
        grid_given: false,
        profile: spec_only.then(|| profile_name("demo")),
        spec_only,
    }
}

/// The engine's own `usage` refusal, before anything: `acted` is false.
fn is_plan_usage<T: std::fmt::Debug>(answer: &Result<T, TxFailure>) -> bool {
    matches!(
        answer,
        Err(TxFailure {
            error: PaneError::Usage { .. },
            acted: false
        })
    )
}

/// The three specs E0 refuses for a live relaunch of `record`, by test name.
fn drifted(record: &Pane) -> [(&'static str, ProfileSpec); 3] {
    let base = spec_from_pane(record);
    let mut cell = base.clone();
    cell.herdr.grid = R3C1;
    let mut cwd = base.clone();
    cwd.host.cwd = "/srv/other".to_owned();
    let mut other = base;
    other.pane = "demo-c2r1".to_owned();
    [
        ("relaunch_engine_refuses_a_cell_change_without_grid", cell),
        ("relaunch_engine_refuses_a_cwd_change", cwd),
        ("relaunch_engine_refuses_a_spec_for_another_pane", other),
    ]
}

/// AC 19b (decision 11): the engine holds every caller to the relaunch rules (E0); with
/// `--spec-only` only the spec's pane is checked, and P takes the drifted spec.
#[test]
fn relaunch_engine_enforces_its_rules() {
    for case in 0..3 {
        let rig = Rig::new();
        let record = rig.live(false);
        let (case, spec) = drifted(&record)[case].clone();
        let live = request(&record, spec.clone(), false);
        let opts = TxOptions::default();
        let (answer, calls) = rig.engine(|ports| tx_launch::relaunch(ports, &live, &opts));
        assert!(is_plan_usage(&answer), "{case}: {answer:?}");
        assert_untouched(&calls);
        let spec_only = request(&record, spec.clone(), true);
        let (answer, calls) = rig.engine(|ports| tx_launch::relaunch(ports, &spec_only, &opts));
        if spec.pane == PANE {
            assert!(
                answer.is_ok(),
                "{case}: spec-only accepts the drift: {answer:?}"
            );
            assert!(
                rig.profile("demo").panes.contains(&spec),
                "{case}: P holds it"
            );
            assert_eq!(
                (calls.herdr.len(), calls.host.len(), calls.harness.len()),
                (0, 0, 0)
            );
        } else {
            assert!(is_plan_usage(&answer), "{case}: spec-only: {answer:?}");
        }
        rig.assert_no_keystroke();
    }
}

/// AC 19b: `launch` refuses a spec for another pane, and both engines refuse `spec_only`
/// without a profile (which clap cannot express, but the request types can).
#[test]
fn launch_engine_refuses_a_spec_for_another_pane_and_spec_only_without_a_profile() {
    let rig = Rig::new();
    let opts = TxOptions::default();
    let launch = |spec: ProfileSpec, spec_only: bool| LaunchRequest {
        name: name(PANE),
        herdr_session: Some("scratch".to_owned()),
        spec,
        profile: None,
        spec_only,
    };
    let other = ProfileSpec {
        pane: "demo-c2r1".to_owned(),
        ..launch_spec()
    };
    for request in [launch(other, false), launch(launch_spec(), true)] {
        let (answer, calls) = rig.engine(|ports| tx_launch::launch(ports, &request, &opts));
        assert!(is_plan_usage(&answer), "{request:?}: {answer:?}");
        assert_untouched(&calls);
        assert!(
            calls.panes.is_empty(),
            "no pane-store call: {:?}",
            calls.panes
        );
    }
    let record = rig.live(false);
    let relaunch = RelaunchRequest {
        spec_only: true,
        profile: None,
        ..request(&record, spec_from_pane(&record), false)
    };
    let (answer, calls) = rig.engine(|ports| tx_launch::relaunch(ports, &relaunch, &opts));
    assert!(is_plan_usage(&answer), "{answer:?}");
    assert_untouched(&calls);
    assert!(
        calls.panes.is_empty(),
        "no pane-store call: {:?}",
        calls.panes
    );
    rig.assert_no_keystroke();
}

/// AC 19c (decision 23): a move is recorded before the old pane is closed, and a failed close
/// fails the run loudly without undoing the record or P's edit.
#[test]
fn relaunch_records_the_move_before_closing_the_old_pane() {
    let rig = Rig::new();
    let before = rig.live(true);
    let refused = PaneError::Unavailable {
        what: "the Herdr socket".into(),
    };
    rig.fakes.herdr.faults().fail_next(HerdrOp::Close, refused);
    let argv = relaunch_with(&["--profile", "demo", "--grid", "c1r3"]);
    let run = rig.run(&argv, Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "unavailable");
    assert!(message.contains("was not closed"), "{message}");
    assert!(message.contains(before.herdr.pane_id.as_str()), "{message}");
    assert_eq!(occurrences(&message, "to reconcile, run"), 1, "{message}");
    let after = rig.record(PANE).unwrap();
    let at = rig.herdr_pane_at(R3C1).expect("the new pane");
    assert_eq!(
        (after.generation, after.herdr.pane_id.clone()),
        (2, at.pane_id)
    );
    assert_eq!(
        rig.profile("demo").panes[0].herdr.grid,
        R3C1,
        "P keeps the edit"
    );
    assert!(
        rig.herdr_ids().contains(&before.herdr.pane_id),
        "the old pane stays"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 20 and decision 22: the record's session, its SHOWN and the TUI agree on one id.
fn assert_session_is(rig: &Rig, sid: &str) {
    let record = rig.record(PANE).unwrap();
    assert_eq!(record.session_of_record.as_deref(), Some(sid));
    assert_eq!(record.last_observed.shown.as_deref(), Some(sid));
    let shown = rig.fakes.harness.tui(&record.herdr.pane_id).unwrap().shown;
    assert_eq!(shown.as_deref(), Some(sid), "the TUI shows it");
}

/// AC 20 (decision 8): a relaunch keeps the session of record the restarted server still
/// lists, and creates none; DRIVEN stays as stored (never inferred).
#[test]
fn relaunch_keeps_the_session_of_record() {
    let rig = Rig::new();
    let sid = rig.live(false).session_of_record.unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert!(
        !run.calls.harness.contains(&HarnessOp::CreateSession),
        "{:?}",
        run.calls
    );
    assert_eq!(
        rig.fakes.harness.list_sessions(PORT).unwrap(),
        std::slice::from_ref(&sid)
    );
    assert_session_is(&rig, &sid);
    assert_eq!(rig.record(PANE).unwrap().last_observed.driven, None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 20: a session of record the server lost is replaced through the API.
#[test]
fn relaunch_replaces_a_deleted_session() {
    let rig = Rig::new();
    let old = rig.live(false).session_of_record.unwrap();
    rig.fakes.harness.delete_session(&old).unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert!(
        run.calls.harness.contains(&HarnessOp::CreateSession),
        "{:?}",
        run.calls
    );
    let sid = rig.record(PANE).unwrap().session_of_record.unwrap();
    assert_ne!(sid, old, "a new session");
    assert_session_is(&rig, &sid);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 20 (decision 22): the DRIVEN another writer stored is kept by a relaunch.
#[test]
fn relaunch_keeps_the_stored_driven() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.last_observed.driven = Some("ses_driven_by_hub".to_owned());
    rig.fakes.panes.concurrent_put(&theirs).unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    let driven = rig.record(PANE).unwrap().last_observed.driven;
    assert_eq!(driven.as_deref(), Some("ses_driven_by_hub"));
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 21: a relaunch stops only what its own pane owns.
#[test]
fn relaunch_leaves_other_panes_alone() {
    let rig = Rig::new();
    let command = Some(argv(&["sleep", "600"]));
    rig.seed_live(&Seed {
        command: command.clone(),
        ..Seed::default()
    });
    rig.seed_live(&Seed {
        pane: "demo-c2r1",
        grid: GridPos { row: 1, col: 2 },
        port: 48101,
        profile: None,
        command,
    });
    let ps = |pane| holler_pane::HostPort::ps(&rig.fakes.host, &name(pane)).unwrap();
    let (other_ps, other_server) = (ps("demo-c2r1"), rig.fakes.harness.server(48101).unwrap());
    let other_record = rig.record("demo-c2r1").unwrap();
    let old_pid = rig.fakes.harness.server(PORT).unwrap().pid;
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert_eq!(ps("demo-c2r1"), other_ps, "its processes run on");
    assert_eq!(rig.fakes.harness.server(48101).unwrap(), other_server);
    assert_eq!(other_server.state, ServerState::Running);
    assert_eq!(rig.record("demo-c2r1").unwrap(), other_record);
    assert_ne!(
        rig.fakes.harness.server(PORT).unwrap().pid,
        old_pid,
        "a new server"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_matches(&run, "demo-c2r1");
    rig.assert_no_keystroke();
}

/// AC 22: until #695, a server that survives the stop fails the relaunch loudly.
#[test]
fn relaunch_fails_when_the_old_server_survives() {
    let rig = Rig::new().unlinked();
    let before = rig.live(false);
    let run = rig.run(RELAUNCH, Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "unavailable");
    assert!(message.contains("still answers"), "{message}");
    assert_eq!(rig.record(PANE).unwrap().generation, before.generation);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 23: a relaunch of a pane with no record is `pane-not-found`, and nothing is touched.
#[test]
fn relaunch_of_a_missing_pane_is_refused() {
    let rig = Rig::new();
    let run = rig.run(RELAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "pane-not-found");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Story #670's AC 5, kept: every shared spec flag parses on `relaunch`.
#[test]
fn pane_relaunch_accepts_every_spec_flag() {
    assert_spec_flags_accepted("relaunch");
}

/// #700 AC 4: an invalid `--agent` key is refused before anything else runs — no port
/// call, no record write — with `agent-key-invalid`, which does not echo the key.
#[test]
fn relaunch_refuses_an_invalid_agent_key_before_anything() {
    let rig = Rig::new();
    rig.live(false);
    let run = rig.run(&relaunch_with(&["--agent", "a=b"]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "agent-key-invalid", "{message}");
    assert!(!message.contains("a=b"), "never echoes the key: {message}");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #700 AC 5 (A's DECISION 1, the relaunch landmine): without the flag a relaunch keeps
/// the stored key — its base spec is `spec_from_pane` of the record, which copies the
/// key, and the record literal writes the spec's key back rather than resetting it.
#[test]
fn relaunch_without_agent_keeps_the_stored_key() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();
    let run = rig.run(RELAUNCH, Format::Json);
    data_of(&run);
    assert_eq!(
        rig.record(PANE)
            .unwrap()
            .opencode_agent
            .as_ref()
            .map(|key| key.as_str()),
        Some("orchestrator")
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #700 AC 5: given, the flag replaces the stored key, like its siblings.
#[test]
fn relaunch_with_agent_replaces_the_stored_key() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();
    let run = rig.run(&relaunch_with(&["--agent", "feature-implementor"]), Format::Json);
    data_of(&run);
    assert_eq!(
        rig.record(PANE)
            .unwrap()
            .opencode_agent
            .as_ref()
            .map(|key| key.as_str()),
        Some("feature-implementor")
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}
