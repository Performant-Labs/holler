//! A grid that mixes OpenCode and Claude Code panes, and #831's acceptance scenario on the test
//! kit's fakes (ADR-0021 section 15 items 4, 12 and 18).
//!
//! The scenario: one profile launches one OpenCode pane and one Claude Code pane; `pane list`
//! shows both with SHOWN and DRIVEN as their records give them; a message is delivered to the
//! Claude pane at `HarnessPort::send_prompt` (W12); one pane is interrupted (the Claude pane: a
//! relaunch on its session of record, the session unchanged), one is parked, and `doctor` is
//! clean. The step "deliver a message to each through `holler say`" needs #839 and #850 and is
//! written down below as an ignored test that names both (item 18), not left out.

use holler_cli::interrupt_cmd::{interrupt_pane, PaneInterrupt};
use holler_cli::output::Format;
use holler_cli::say_cmd::resolve_pane_target;
use holler_pane::pane::{HarnessKind, Hold};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::{HarnessPort, ProfileSpec};
use holler_pane_testkit::fixture::sample_spec;
use holler_pane_testkit::harness::ServerState;

use crate::rig::{
    cell, data_of, findings, kinds, name, Rig, CLAUDE, CWD, OPENCODE, PORT, PROFILE, SESSION,
    WORKSPACE,
};

/// A spec of the profile: `pane` at `r1c<col>` in the rig's workspace, on `kind` with `policy`.
fn spec(pane: &str, col: u16, kind: HarnessKind, policy: &str) -> ProfileSpec {
    ProfileSpec {
        herdr: SpecHerdr {
            workspace: WORKSPACE.to_owned(),
            grid: cell(1, col),
        },
        host: SpecHost {
            cwd: CWD.to_owned(),
        },
        harness: SpecHarness {
            kind,
            port_policy: policy.to_owned(),
        },
        ..sample_spec(pane)
    }
}

/// The profile `demo` with an OpenCode pane at r1c1 and a Claude pane at r1c2, applied.
fn applied() -> Rig {
    let rig = Rig::new();
    rig.set_specs(vec![
        spec(OPENCODE, 1, HarnessKind::Opencode, &format!("fixed:{PORT}")),
        spec(CLAUDE, 2, HarnessKind::Claude, "none"),
    ]);
    let run = rig.run(
        &["profile", "apply", PROFILE, "--herdr-session", SESSION],
        Format::Json,
    );
    assert_eq!(run.code, 0, "the mixed grid applies: {run:?}");
    // The Herdr pane of each shows its tmux session through a client (#647 part 2): the apply
    // made the sessions, and no Holler verb attaches a client, so the world gets one here.
    for pane in [OPENCODE, CLAUDE] {
        rig.host.attach_client(&name(pane)).unwrap();
    }
    rig
}

#[test]
fn one_profile_launches_an_opencode_pane_and_a_claude_pane() {
    let rig = applied();

    let opencode = rig.record(OPENCODE);
    assert_eq!(opencode.harness.kind, HarnessKind::Opencode);
    assert_eq!(opencode.harness.port, Some(PORT));
    assert_eq!(
        rig.opencode().server(PORT).map(|s| s.state),
        Some(ServerState::Running)
    );

    let claude = rig.record(CLAUDE);
    assert_eq!(claude.harness.kind, HarnessKind::Claude);
    assert_eq!(claude.harness.port, None);
    let process = rig.process_of(CLAUDE).expect("the Claude process runs");
    assert_eq!(claude.harness.pid, Some(process.pid));
    assert_eq!(
        claude.session_of_record.as_deref(),
        Some(process.session_id.as_str())
    );
    for record in [&opencode, &claude] {
        assert_eq!(record.profile.as_ref().map(|p| p.as_str()), Some(PROFILE));
    }
}

#[test]
fn a_second_apply_of_the_mixed_grid_changes_nothing() {
    let rig = applied();
    let before = (rig.record(OPENCODE), rig.record(CLAUDE));
    let (run, calls) = rig.run_logged(
        &["profile", "apply", PROFILE, "--herdr-session", SESSION],
        Format::Json,
    );
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        (rig.record(OPENCODE), rig.record(CLAUDE)),
        before,
        "nothing relaunched"
    );
    assert!(calls.wrote_no_record(), "{calls:?}");
}

/// `profile apply` never rewrites a live pane's harness: a spec that names another harness
/// than the live pane's is refused for that pane, and the pane keeps running as it was.
#[test]
fn apply_never_changes_a_live_panes_harness() {
    let rig = applied();
    let before = rig.record(OPENCODE);
    rig.set_specs(vec![
        spec(OPENCODE, 1, HarnessKind::Claude, "none"),
        spec(CLAUDE, 2, HarnessKind::Claude, "none"),
    ]);
    let run = rig.run(
        &["profile", "apply", PROFILE, "--herdr-session", SESSION],
        Format::Json,
    );
    assert!(
        run.out.contains("harness-change-refused"),
        "the pane's step names the refusal: {run:?}"
    );
    let after = rig.record(OPENCODE);
    assert_eq!(
        after.harness, before.harness,
        "still the OpenCode pane it was"
    );
    assert_eq!(
        rig.opencode().server(PORT).map(|s| s.state),
        Some(ServerState::Running)
    );
    assert!(
        rig.process_of(OPENCODE).is_none(),
        "no Claude process for it"
    );
}

#[test]
fn profile_show_compares_the_harness_kind() {
    let rig = applied();
    rig.set_specs(vec![
        spec(OPENCODE, 1, HarnessKind::Claude, "none"),
        spec(CLAUDE, 2, HarnessKind::Claude, "none"),
    ]);
    let run = rig.run(&["profile", "show", PROFILE], Format::Json);
    let data = data_of(&run);
    assert!(
        data.to_string().contains("harness.kind"),
        "the drift names harness.kind: {data}"
    );
}

/// The acceptance scenario on the fakes, every line but `holler say` (item 18).
#[test]
fn the_acceptance_scenario_on_the_fakes() {
    let rig = applied();

    // Both panes in `pane list`, with SHOWN and DRIVEN as their records give them.
    let listed = data_of(&rig.run(&["pane", "list"], Format::Json));
    let rows = listed["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.panes is the list's rows: {listed}"))
        .clone();
    for pane in [OPENCODE, CLAUDE] {
        let record = rig.record(pane);
        let row = rows
            .iter()
            .find(|r| r["name"] == pane)
            .unwrap_or_else(|| panic!("{pane} is listed: {listed}"));
        assert_eq!(
            row["shown"].as_str(),
            record.last_observed.shown.as_deref(),
            "{row}"
        );
        assert_eq!(
            row["driven"].as_str(),
            record.last_observed.driven.as_deref(),
            "{row}"
        );
        assert_eq!(row["sync"], "ok", "{row}");
    }

    // Delivery to the Claude pane, at the port (W12).
    let claude = rig.record(CLAUDE);
    let session = claude.session_of_record.clone().unwrap();
    rig.harnesses
        .send_prompt(
            &rig.endpoint_of(CLAUDE),
            &session,
            "hello from the scenario",
        )
        .unwrap();
    assert_eq!(
        rig.process_of(CLAUDE).unwrap().inbox,
        vec!["hello from the scenario"]
    );

    // Interrupt the Claude pane: a relaunch on its session of record, the session unchanged.
    let interrupted = rig.with_ports(|ports| interrupt_pane(ports, CLAUDE, None, None));
    assert_eq!(
        interrupted.unwrap(),
        PaneInterrupt::Relaunched {
            session: session.clone()
        }
    );
    assert_ne!(
        rig.record(CLAUDE).harness.pid,
        claude.harness.pid,
        "a new process"
    );
    assert_eq!(
        rig.record(CLAUDE).session_of_record.as_deref(),
        Some(session.as_str())
    );
    assert_eq!(rig.process_of(CLAUDE).unwrap().session_id, session);

    // Park the OpenCode pane.
    let parked = rig.run(
        &[
            "pane",
            "park",
            OPENCODE,
            "--reason",
            "scenario",
            "--release-when",
            "the end",
        ],
        Format::Json,
    );
    assert_eq!(parked.code, 0, "{parked:?}");
    assert!(matches!(rig.record(OPENCODE).hold, Hold::Parked { .. }));

    // `doctor` is clean.
    let found = findings(&data_of(&rig.run(&["pane", "doctor"], Format::Json)));
    assert_eq!(kinds(&found), Vec::<String>::new(), "{found:?}");
}

/// The scenario's `holler say` step (#831's acceptance, ADR-0021 section 15 item 18). It cannot
/// pass until the hub can reach both panes: the hub delivers only to a connected body addressed by
/// its advertised name, and `say --pane` resolves a pane to its session of record, which matches
/// no body's name until #839 joins a body's `harness_session_id` to `Pane.session_of_record`; a
/// Claude Code pane also has no body at all until #850. Run it with `--ignored` once both land.
#[test]
#[ignore = "blocked by #839 (the hub joins a body's harness session to Pane.session_of_record) and #850 (a body driver for a Claude Code pane): holler say --pane reaches no pane of either harness until both land"]
fn say_delivers_a_message_to_each_pane_of_the_scenario() {
    let rig = applied();
    let mut addresses = Vec::new();
    for pane in [OPENCODE, CLAUDE] {
        let address = rig
            .with_ports(|ports| resolve_pane_target(ports, pane, None))
            .unwrap_or_else(|e| panic!("{pane} resolves: {e}"));
        assert_eq!(
            Some(address.as_str()),
            rig.record(pane).session_of_record.as_deref()
        );
        addresses.push((pane, address));
    }
    panic!(
        "`holler say --pane` would send to the hub addresses {addresses:?}, which no body \
         advertises until #839 (and, for the Claude pane, #850): replace this line with the \
         real delivery through the hub, then assert the message reached each pane"
    );
}
