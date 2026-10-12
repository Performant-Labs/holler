//! The scripted scenario (Scope 2, D10 to D15, D21, D23, D28): the real `holler` binary over
//! the real wiring, against a scratch Herdr, a private tmux server and OpenCode with the
//! dead-end provider only, all started by the test (`scratch`). Every test is `#[ignore]`
//! and runs only opted in:
//!
//! ```text
//! HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1
//! ```
//!
//! No step types into a TUI (I4): every verb acts through the ports, and the test reads
//! Herdr's snapshot, tmux's listings and OpenCode through a `HarnessPort` it builds itself
//! ([`Scratch::harness`]), never a screen. Raw HTTP is used only where no port can (A's W-G):
//! deleting a session, and the model guard's read of the providers. No model can be reached:
//! after every server a verb starts, the model guard runs before the next step.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use holler_pane::tx_launch::{wait_healthy, BOOT_POLL};
use holler_pane::{AgentKey, GridPos, HarnessEndpoint, HarnessPort, HostPort, PaneName, ServeSpec};
use holler_pane_testkit::envelope::check_envelope;
use serde_json::Value;

use crate::scratch::rig::{free_port, no_model_can_be_reached, raw, refused};
use crate::scratch::{alive, Scratch};
use crate::support::wait_for;

const C1: &str = "hlr649-c1r1";
const C2: &str = "hlr649-c2r1";

/// Delete every session on `port` that is no pane's session of record: a reset or a switch
/// leaves the previous one on the server as a stray, which doctor reports. The sessions are
/// listed through the port; no port deletes one, so the delete is raw HTTP (W-G).
fn delete_strays(scratch: &Scratch, port: u16, keep: &BTreeSet<String>) {
    let listed = scratch.harness().list_sessions(&HarnessEndpoint::Port(port)).expect("list_sessions");
    for session in listed.into_iter().filter(|s| !keep.contains(s)) {
        raw(port, "DELETE", &format!("/session/{session}"), None);
    }
}

/// Scenario steps 1 to 8 in the order the acceptance lists them, then the repairs of D21 and
/// the end state: two launches; a relaunch of one; a reset of one with a first message; a
/// switch of one; a park of one; a killed server found by `doctor`; a session deleted under a
/// TUI reported; a hub restart after which `pane list` equals what it was; then `doctor`
/// reports no finding and `pane list` equals Herdr's snapshot. No credential leaked (D28).
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_scenario_runs_in_order_on_scratch_instances() {
    let Some(mut scratch) = Scratch::start() else {
        return;
    };
    let (p1, p2) = (free_port(), free_port());
    scratch.assert_tmux_env_is_scratch();

    // 1. Two launches. Each resolves `attach_tui`'s pane before its record exists (D8): the
    // launch exits 0 only if the resolver answered.
    let begun = Instant::now();
    scratch.launch(C1, "hlr649", "r1c1", p1, &[]);
    eprintln!("the first launch took {:?}", begun.elapsed());
    scratch.launch(C2, "hlr649", "r2c1", p2, &[]);
    scratch.assert_no_credential_leaked();

    // 2. A relaunch of one: the same port, its session of record kept.
    let kept = scratch.session_of_record(C1);
    scratch.json(&["pane", "relaunch", C1], 0);
    no_model_can_be_reached(p1);
    assert_eq!(scratch.session_of_record(C1), kept, "relaunch keeps the session");

    // 3. A reset of one, with a first message.
    reset_with_a_first_message(&scratch, p2);

    // 4. A switch of one, to a session its server already has, made through the port.
    let target = scratch.harness().create_session(&HarnessEndpoint::Port(p1)).expect("create_session");
    scratch.json(&["pane", "switch", C1, &target], 0);
    let switched = scratch.record(C1);
    assert_eq!(switched["session_of_record"], target.as_str());
    assert_eq!(switched["last_observed"]["shown"], target.as_str(), "SHOWN against the record");

    // 5. A park of one.
    scratch.json(&["pane", "park", C2, "--reason", "hlr649", "--release-when", "manual"], 0);

    // 6 and 7. C2's server killed and C1's session deleted; doctor finds both.
    break_both(&scratch, p1, p2, &target);

    // 8. A hub restart, after which `pane list` is what it was.
    let listed = scratch.json(&["pane", "list"], 0).data();
    scratch.restart_hub();
    assert_eq!(scratch.json(&["pane", "list"], 0).data(), listed, "the registry is intact");

    // The repairs (D21), then the strays the reset and the switch left.
    scratch.json(&["pane", "reset", C1], 0);
    scratch.json(&["pane", "unpark", C2], 0);
    scratch.json(&["pane", "relaunch", C2], 0);
    no_model_can_be_reached(p2);
    let keep: BTreeSet<String> = [C1, C2].into_iter().map(|p| scratch.session_of_record(p)).collect();
    for port in [p1, p2] {
        delete_strays(&scratch, port, &keep);
    }

    scratch.assert_end_state(&[C1, C2], &["hlr649"]);

    // Each `watch` line is an envelope too.
    let watch = scratch.holler(&["pane", "watch", "--until-idle", "--format=json"]);
    assert_eq!(watch.code, 0, "{watch:?}");
    for line in watch.stdout.lines() {
        check_envelope(&format!("{line}\n"), 0)
            .unwrap_or_else(|fault| panic!("a watch line is not an envelope: {fault}: {line}"));
    }
    scratch.assert_tmux_env_is_scratch();
    scratch.assert_no_credential_leaked();
}

/// Step 3 (#839, D9): a reset of C2 with a first message. The scenario starts a hub and no
/// body, so no body drives C2 and `reset --first` is refused `pane-not-driven` in the plan,
/// before anything moves: no session is created and the record is byte-for-byte unchanged.
/// Then `reset C2` without `--first` makes the fresh session the later steps use. That a first
/// message reaches a real session (it reads `Busy`) is the default-run end-to-end test's
/// (`driven_e2e_test.rs`), which has a real body.
fn reset_with_a_first_message(scratch: &Scratch, p2: u16) {
    let before = scratch.record(C2);
    let listed = scratch.harness().list_sessions(&HarnessEndpoint::Port(p2)).expect("list_sessions");
    let refused = scratch.json(&["pane", "reset", C2, "--first", "hlr649 first message"], 3);
    assert_eq!(refused.error_code().as_deref(), Some("pane-not-driven"), "{refused:?}");
    assert_eq!(scratch.record(C2), before, "the record is unchanged, its generation included");
    let after = scratch.harness().list_sessions(&HarnessEndpoint::Port(p2)).expect("list_sessions");
    assert_eq!(after, listed, "no session was created");
    scratch.json(&["pane", "reset", C2], 0);
    let after = scratch.record(C2);
    let fresh = after["session_of_record"].as_str().unwrap().to_owned();
    assert_ne!(Some(fresh.as_str()), before["session_of_record"].as_str(), "a fresh session");
    assert_eq!(after["last_observed"]["shown"], fresh.as_str(), "SHOWN follows");
    no_model_can_be_reached(p2);
}

/// Steps 6 and 7 (D21). C2's server killed: SIGKILL to the process group of each pid tmux
/// lists as one `run` tagged in C2's session that `HostPort::ps` reports live there. Then C1's
/// session of record after the switch, `target`, deleted under its TUI (raw HTTP: no port
/// deletes a session). `doctor` finds both.
fn break_both(scratch: &Scratch, p1: u16, p2: u16, target: &str) {
    scratch.kill_server(C2, p2);

    raw(p1, "DELETE", &format!("/session/{target}"), None);

    let found = scratch.findings();
    assert!(
        found.contains(&(C2.to_owned(), "server-down".to_owned())),
        "doctor finds C2's server down: {found:?}"
    );
    assert!(
        found.iter().any(|(pane, _)| pane == C1),
        "doctor reports C1's deleted session: {found:?}"
    );
}

/// The profile scenario. Create from current; edit through `launch --profile`; `close
/// --profile` closes the one named pane and removes its spec; `show` and `list`; `delete`
/// refused while other panes of the profile are live, then `--keep-panes`, after which the
/// panes keep running with no profile.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_profile_scenario_runs_on_scratch_instances() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    let (p1, p2, p3) = (free_port(), free_port(), free_port());
    scratch.launch(C1, "hlr649", "r1c1", p1, &[]);
    scratch.launch(C2, "hlr649", "r2c1", p2, &[]);

    scratch.json(&["profile", "create", "hlr649p", "--from-current"], 0);
    let members = |scratch: &Scratch| -> Vec<String> {
        let shown = scratch.json(&["profile", "show", "hlr649p"], 0).data().to_string();
        [C1, C2, "hlr649-p1"]
            .into_iter()
            .filter(|pane| shown.contains(pane))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(members(&scratch), [C1, C2]);

    scratch.launch("hlr649-p1", "hlr649b", "r1c1", p3, &["--profile", "hlr649p"]);
    assert_eq!(members(&scratch), [C1, C2, "hlr649-p1"], "launch --profile adds its spec");

    scratch.json(&["pane", "close", "hlr649-p1", "--profile", "hlr649p"], 0);
    assert_eq!(members(&scratch), [C1, C2], "close --profile removes it");
    assert!(wait_for(Duration::from_secs(10), || refused(p3).then_some(())).is_some());

    let listed = scratch.json(&["profile", "list"], 0).data().to_string();
    assert!(listed.contains("hlr649p"), "{listed}");

    let refusal = scratch.json(&["profile", "delete", "hlr649p"], 3);
    assert_eq!(refusal.error_code().as_deref(), Some("profile-has-live-panes"));

    scratch.json(&["profile", "delete", "hlr649p", "--keep-panes"], 0);
    let panes = scratch.json(&["pane", "list"], 0).data();
    let rows = panes["panes"].as_array().expect("rows");
    assert_eq!(rows.len(), 2, "{panes}");
    for row in rows {
        assert_eq!(row["profile"], Value::Null, "no profile: {row}");
    }
    for port in [p1, p2] {
        assert!(!refused(port), "the server on {port} keeps running");
    }
    assert!(!scratch.json(&["profile", "list"], 0).data().to_string().contains("hlr649p"));
    scratch.assert_no_credential_leaked();
}

/// D23: one launch with `--grid c1r2` and one with `--grid 2,1`, each into a workspace of its
/// own whose r1c1 a Holler launch made first (the adapter refuses a first pane anywhere but
/// r1c1 as `grid-unreachable`); each sits at row 2, column 1 in Herdr's own snapshot.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    scratch.launch("hlr649-g0", "hlr649g", "r1c1", free_port(), &[]);
    scratch.launch("hlr649-g1", "hlr649g", "c1r2", free_port(), &[]);
    scratch.launch("hlr649-h0", "hlr649h", "r1c1", free_port(), &[]);
    scratch.launch("hlr649-h1", "hlr649h", "2,1", free_port(), &[]);
    let snapshot = scratch.herdr_snapshot();
    for (pane, workspace) in [("hlr649-g1", "hlr649g"), ("hlr649-h1", "hlr649h")] {
        let id = scratch.record(pane)["herdr"]["pane_id"].as_str().unwrap().to_owned();
        let seen = snapshot
            .iter()
            .find(|p| p.pane_id.as_str() == id)
            .unwrap_or_else(|| panic!("{pane} ({id}) is in Herdr's snapshot: {snapshot:?}"));
        assert_eq!(seen.workspace, workspace);
        assert_eq!(seen.grid, GridPos { row: 2, col: 1 }, "{pane} as Herdr counts it");
    }
}

/// D15 (#802's follow-up): the server argv `serve_command` builds runs under a real tmux pty
/// through `TmuxHost::run`, for the default spec and for one carrying an agent (the
/// `OPENCODE_CONFIG_CONTENT` of #642 part 3 through a real `new-window`); the engine's health
/// wait sees it, the model guard runs, `verify_serving` accepts the spec, and `stop_owned`
/// leaves the port refusing and no process of the test running.
#[test]
#[ignore = "opt-in: scratch tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_server_argv_runs_under_a_real_tmux_pty_and_stops() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    let harness = scratch.harness();
    let project = scratch.settings.oc_root.join("proj").display().to_string();
    let specs = [
        ("hlr649-serve", ServeSpec::default()),
        ("hlr649-agent", ServeSpec::default().with_agent(AgentKey::parse("plan").unwrap())),
    ];
    for (pane, spec) in specs {
        let name = PaneName::parse(pane).unwrap();
        let port = free_port();
        let argv = harness.serve_command(&name, port, &spec).expect("serve_command");
        scratch.host.ensure_session(&name, &project).expect("ensure_session");
        let begun = Instant::now();
        scratch.host.run(&name, &project, &argv).expect("run under tmux");
        let healthy = wait_healthy(&harness, &HarnessEndpoint::Port(port), BOOT_POLL, Instant::now() + Duration::from_secs(30));
        eprintln!("{pane}: a cold server under tmux answered its health in {:?}", begun.elapsed());
        assert_eq!(healthy, Ok(true), "{pane}: the server under tmux answers its health");
        no_model_can_be_reached(port);
        assert_eq!(harness.verify_serving(&HarnessEndpoint::Port(port), &spec), Ok(()), "{pane}: {spec:?}");

        let owned = scratch.owned_pids(pane);
        assert!(!owned.is_empty(), "{pane}: run tagged the server's window");
        assert_eq!(scratch.host.stop_owned(&name), Ok(()));
        assert!(
            wait_for(Duration::from_secs(10), || refused(port).then_some(())).is_some(),
            "{pane}: the port refuses once the server is stopped"
        );
        for pid in owned {
            assert!(!alive(pid), "{pane}: no server process of the test is left ({pid})");
        }
    }
}
