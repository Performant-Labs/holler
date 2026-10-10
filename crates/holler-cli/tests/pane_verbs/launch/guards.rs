//! `holler pane launch`: the refusals before anything live changes (brief AC 9, 10, 11a, 12,
//! 13b, 13c, 14, 15 and 16d). Each refusal before the act leaves every store unwritten and P
//! at its generation.

use holler_cli::output::Format;
use holler_pane::pane::PaneProbe;
use holler_pane::profile_snapshot::fixed_port_policy;
use holler_pane::tx_launch::{port_of_policy, PROBE_TIMEOUT};
use holler_pane::{GridPos, HerdrPane, Pane, PaneError, PaneId, ProbeResult};
use holler_pane_testkit::fixture::sample_pane;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::{HerdrOp, HerdrVersion, SUPPORTED_VERSIONS};
use holler_pane_testkit::prober::ProbeCall;
use holler_pane_testkit::profile_store::ProfileStoreOp;

use super::rig::{
    argv, assert_untouched, data_of, error_of, launch_replacing, launch_with, launch_without, Rig,
    LAUNCH, PANE, PORT, PROBE, PROBE_FLAGS, SESSION, WORKSPACE,
};

const R2C1: GridPos = GridPos { row: 2, col: 1 };

/// `demo-c2r1`'s record, naming the Herdr pane `herdr`.
fn other_record(herdr: HerdrPane) -> Pane {
    Pane {
        herdr,
        ..sample_pane("demo-c2r1").unwrap()
    }
}

/// AC 9a: a name that already has a record is `pane-exists`, and nothing is touched.
#[test]
fn launch_of_a_recorded_name_is_pane_exists() {
    let rig = Rig::new();
    rig.live(false);
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "pane-exists");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 9b: a cell another record's Herdr pane holds is `grid-occupied`; the message names the
/// pane and the record, which is joined by session and pane id, whatever its stored cell.
#[test]
fn launch_refuses_a_cell_another_record_holds() {
    for stored_grid in [R2C1, GridPos { row: 3, col: 1 }] {
        let rig = Rig::new();
        let occupant = rig.place_herdr_pane(2, 1);
        let stale = HerdrPane {
            grid: stored_grid,
            ..occupant.clone()
        };
        rig.fakes
            .panes
            .concurrent_put(&other_record(stale))
            .unwrap();
        let run = rig.run(LAUNCH, Format::Json);
        assert_eq!(run.code, 3, "stored at {stored_grid}: {run:?}");
        let (code, message) = error_of(&run);
        assert_eq!(code, "grid-occupied");
        assert!(message.contains(occupant.pane_id.as_str()), "{message}");
        assert!(message.contains("demo-c2r1"), "{message}");
        assert_eq!(run.calls.herdr, [HerdrOp::Version, HerdrOp::Snapshot]);
        assert_eq!((run.calls.host.len(), run.calls.harness.len()), (0, 0));
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// AC 9b: a record whose stored cell is the target but whose Herdr pane the snapshot does not
/// list refuses nothing: the cell is free by the snapshot.
#[test]
fn launch_ignores_a_stale_record_at_a_free_cell() {
    let rig = Rig::new();
    let gone = HerdrPane {
        session: SESSION.to_owned(),
        workspace: WORKSPACE.to_owned(),
        pane_id: PaneId::new("w1:p9"),
        grid: R2C1,
    };
    rig.fakes.panes.concurrent_put(&other_record(gone)).unwrap();
    let run = rig.run(LAUNCH, Format::Json);
    data_of(&run);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 9c: a Herdr pane no record names is never adopted: `grid-occupied`, and it stays.
#[test]
fn launch_never_adopts_an_unrecorded_pane() {
    let rig = Rig::new();
    let stranger = rig.place_herdr_pane(2, 1);
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "grid-occupied");
    assert!(message.contains(stranger.pane_id.as_str()), "{message}");
    assert!(message.contains("no record"), "{message}");
    assert_eq!(rig.herdr_ids(), vec![stranger.pane_id], "never closed");
    assert_eq!((run.calls.host.len(), run.calls.harness.len()), (0, 0));
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 9d: a server that already answers on the port is never adopted: `port-in-use`.
#[test]
fn launch_never_adopts_a_running_server() {
    let rig = Rig::new();
    rig.serve("demo-c2r1", PORT);
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "port-in-use");
    assert_eq!(run.calls.harness, [HarnessOp::Health]);
    assert_eq!(run.calls.host.len(), 0);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 10a and 16d: a failing probe refuses before any live step and before the profile write,
/// so P keeps its generation and its change log.
#[test]
fn a_failing_probe_refuses_before_any_step() {
    let rig = Rig::new();
    let missing = ProbeResult::Failed {
        missing: vec!["qwen38".to_owned()],
    };
    rig.fakes.prober.script(argv(&PROBE), missing);
    let run = rig.run(&launch_with(&PROBE_FLAGS), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "probe-failed");
    assert!(message.contains("qwen38"), "{message}");
    let live = (&run.calls.herdr, &run.calls.host, &run.calls.harness);
    assert_eq!(
        (live.0.len(), live.1.len(), live.2.len()),
        (0, 0, 0),
        "{live:?}"
    );
    assert!(!run.calls.profiles.contains(&ProfileStoreOp::CasPut), "16d");
    assert_eq!(rig.record(PANE), None);
    assert_eq!(
        rig.profile("demo").generation,
        1,
        "16d: P keeps its generation"
    );
    let log: Vec<u64> = rig
        .profile_log("demo")
        .iter()
        .map(|e| e.generation)
        .collect();
    assert_eq!(log, [1], "16d: P's change log is the seed's one entry");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 10b: a probe nobody scripted (the fake answers `Error`) refuses too.
#[test]
fn an_unscripted_probe_refuses() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&PROBE_FLAGS[2..]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "probe-failed");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 10c: a passing probe is recorded, and it ran once with the argv, expect and timeout.
#[test]
fn a_passing_probe_is_recorded() {
    let rig = Rig::new();
    rig.fakes.prober.script(argv(&PROBE), ProbeResult::Ok);
    let run = rig.run(&launch_with(&PROBE_FLAGS[2..]), Format::Json);
    data_of(&run);
    let probe = PaneProbe {
        check: Some(argv(&PROBE)),
        expect: vec!["qwen38".to_owned()],
        last: Some(ProbeResult::Ok),
    };
    assert_eq!(rig.record(PANE).unwrap().probe, probe);
    let call = ProbeCall {
        argv: argv(&PROBE),
        expect: vec!["qwen38".to_owned()],
        timeout: PROBE_TIMEOUT,
    };
    assert_eq!(run.calls.probes, [call]);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 11a: a command given as a JSON string is `command-not-argv`, before anything.
#[test]
fn a_command_string_is_command_not_argv() {
    let rig = Rig::new();
    let run = rig.run(
        &launch_with(&["--command-json", "\"opencode serve\""]),
        Format::Json,
    );
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "command-not-argv");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 12 (I7): an env entry with a value is refused, in both formats, and never echoed.
#[test]
fn an_env_value_is_refused_and_not_echoed() {
    for format in [Format::Text, Format::Json] {
        let rig = Rig::new();
        let run = rig.run(&launch_with(&["--env", "TOKEN=s3cr3t644"]), format);
        assert_eq!(run.code, 3, "{run:?}");
        assert!(!run.out.contains("s3cr3t644") && !run.err.contains("s3cr3t644"));
        if format == Format::Json {
            assert_eq!(error_of(&run).0, "profile-secret-refused");
        }
        assert_untouched(&run.calls);
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// AC 13b: an ambiguous grid is refused before any step.
#[test]
fn an_ambiguous_grid_is_refused_before_any_step() {
    let rig = Rig::new();
    let run = rig.run(&launch_replacing("--grid", "21"), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "grid-ambiguous");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 13c: a cell outside the workspace is `grid-out-of-range`, from `ensure_pane`.
#[test]
fn a_cell_outside_the_workspace_is_out_of_range() {
    let rig = Rig::new();
    let run = rig.run(&launch_replacing("--grid", "r9c1"), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "grid-out-of-range");
    assert!(
        run.calls.herdr.contains(&HerdrOp::EnsurePane),
        "from ensure_pane"
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 14 (B5): an unsupported Herdr is refused, naming the supported versions.
#[test]
fn an_unsupported_herdr_is_refused() {
    let rig = Rig::new();
    rig.fakes.herdr.set_version(HerdrVersion::Unsupported);
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "herdr-version-unsupported");
    assert!(message.contains(SUPPORTED_VERSIONS), "{message}");
    assert_eq!(run.calls.herdr, [HerdrOp::Version]);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 15: with no base spec, every required flag that is missing is named in one `usage` line.
#[test]
fn launch_names_every_missing_flag() {
    let rig = Rig::new();
    let run = rig.run(
        &["pane", "launch", PANE, "--herdr-session", "scratch"],
        Format::Json,
    );
    assert_eq!(run.code, 2, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "usage");
    for flag in [
        "--project",
        "--workspace",
        "--grid",
        "--model",
        "--effort",
        "--ctx-soft",
        "--ctx-hard",
        "--port-policy",
    ] {
        assert!(message.contains(flag), "names {flag}: {message}");
    }
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 15: a port policy that is not `fixed:<port>` in canonical form, and a soft ceiling above
/// the hard one, are `usage`.
#[test]
fn bad_policies_and_ceilings_are_usage() {
    let soft_above_hard = {
        let mut argv = launch_replacing("--ctx-soft", "2");
        let at = argv.iter().position(|a| *a == "--ctx-hard").unwrap();
        argv[at + 1] = "1";
        argv
    };
    for argv in [
        launch_replacing("--port-policy", "fixed"),
        launch_replacing("--port-policy", "fixed:048100"),
        soft_above_hard,
    ] {
        let rig = Rig::new();
        let run = rig.run(&argv, Format::Json);
        assert_eq!(run.code, 2, "{argv:?}: {run:?}");
        assert_eq!(error_of(&run).0, "usage", "{argv:?}");
        assert_untouched(&run.calls);
        rig.assert_no_keystroke();
    }
}

/// AC 15: a live launch needs `--herdr-session`.
#[test]
fn a_live_launch_needs_a_herdr_session() {
    let rig = Rig::new();
    let run = rig.run(&launch_without("--herdr-session"), Format::Json);
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(error_of(&run).0, "usage");
    assert_untouched(&run.calls);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 15 (decision 5): the policy parser inverts #662a's `fixed_port_policy`, and refuses a
/// port outside 1..=65535 or a non-canonical one.
#[test]
fn port_policy_round_trips_with_the_snapshot() {
    for port in [1, 80, 48100, 65535] {
        assert_eq!(port_of_policy(&fixed_port_policy(port)), Ok(port), "{port}");
    }
    for policy in ["fixed:0", "fixed:65536", "fixed", "fixed:048100", "auto"] {
        let refused = port_of_policy(policy);
        assert!(
            matches!(refused, Err(PaneError::Usage { .. })),
            "{policy}: {refused:?}"
        );
    }
}
