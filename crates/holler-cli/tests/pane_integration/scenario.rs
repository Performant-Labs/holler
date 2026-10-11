//! The scripted scenario (Scope 2, D10 to D15, AC 4 to AC 12): the real `holler` binary over
//! the real wiring, against a scratch Herdr, a private tmux server and OpenCode with the
//! dead-end provider only, all started by the test (`scratch`). Every test is `#[ignore]`
//! and runs only opted in:
//!
//! ```text
//! HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1
//! ```
//!
//! No step types into a TUI (I4): every verb acts through the ports, and the test reads
//! Herdr's snapshot, tmux's listings and OpenCode's HTTP API, never a screen. No model can be
//! reached: after every server a verb starts, the model guard runs before the next step.
//! `reset --first` needs #642 part 3's `send_prompt` (D19).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use holler_adapter_opencode::{OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig};
use holler_pane::tx_launch::{wait_healthy, BOOT_POLL};
use holler_pane::{GridPos, HarnessPort, HostPort, PaneError, PaneName, ServeSpec};
use holler_pane_testkit::envelope::check_envelope;
use serde_json::{json, Value};

use crate::scratch::rig::{free_port, no_model_can_be_reached, raw, refused, signal_group};
use crate::scratch::Scratch;
use crate::support::wait_for;

const C1: &str = "hlr649-c1r1";
const C2: &str = "hlr649-c2r1";

/// `pane get <pane>`'s record.
fn record(scratch: &Scratch, pane: &str) -> Value {
    scratch.json(&["pane", "get", pane], 0).data()["pane"].clone()
}

/// The session ids OpenCode's server on `port` lists.
fn sessions_on(port: u16) -> Vec<String> {
    let reply = raw(port, "GET", "/session", None);
    let listed: Value = serde_json::from_slice(&reply.body).unwrap_or(Value::Null);
    listed
        .as_array()
        .map(|all| all.iter().filter_map(|s| s["id"].as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// Delete every session on `port` but `keep`: a reset or a switch leaves the previous one on
/// the server as a stray, which doctor reports (ADR-0021; no port deletes a session).
fn delete_strays(port: u16, keep: &str) {
    for session in sessions_on(port).into_iter().filter(|s| s != keep) {
        raw(port, "DELETE", &format!("/session/{session}"), None);
    }
}

/// `pane doctor`'s findings that name `pane`.
fn findings_for(scratch: &Scratch, pane: &str) -> Vec<Value> {
    let data = scratch.json(&["pane", "doctor"], 0).data();
    data["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f["pane"] == pane)
        .collect()
}

/// The private tmux server's environment is the rig's isolated pairs and nothing else (bar
/// the `PWD` tmux adds, the scratch directory), so no credential and nothing of the
/// operator's reached any OpenCode the scenario started (W-1, AC 12).
fn assert_tmux_env_is_scratch(scratch: &Scratch) {
    let allowed: BTreeSet<String> = crate::scratch::tmux_server_env(
        &scratch.settings.opencode,
        &scratch.settings.oc_root,
    )
    .into_iter()
    .map(|(name, _)| name)
    .collect();
    let base = scratch.settings.base.display().to_string();
    for line in scratch.tmux_global_env() {
        let (name, value) = line.trim_start_matches('-').split_once('=').unwrap_or((&line, ""));
        if name == "PWD" {
            assert_eq!(value, base, "tmux's own PWD is the scratch directory");
            continue;
        }
        assert!(allowed.contains(name), "the tmux server carries {name}, which the test never set");
    }
}

/// AC 5, 6, 9 and 12: two launches; a relaunch of one; a reset of one with a first message;
/// a switch of one; a park of one; a killed server found by `doctor`; a session deleted under
/// a TUI reported; a hub restart after which `pane list` equals what it was; then, once the
/// findings are remedied, the registry equals what Herdr and OpenCode observe.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_scenario_runs_in_order_on_scratch_instances() {
    let Some(mut scratch) = Scratch::start() else {
        return;
    };
    let (p1, p2) = (free_port(), free_port());
    assert_tmux_env_is_scratch(&scratch);

    // 1. Two launches (W-14: the first is timed, the budget is 10 s, D9).
    let begun = Instant::now();
    scratch.launch(C1, "hlr649", "r1c1", p1, &[]);
    eprintln!("the first launch took {:?}", begun.elapsed());
    scratch.launch(C2, "hlr649", "r2c1", p2, &[]);

    // 2. A relaunch of one: the same port, its session of record kept.
    let kept = record(&scratch, C1)["session_of_record"].clone();
    scratch.json(&["pane", "relaunch", C1], 0);
    no_model_can_be_reached(p1);
    assert_eq!(record(&scratch, C1)["session_of_record"], kept, "relaunch keeps the session");

    // 3. A reset of one, with a first message (through `send_prompt`; no model is reached).
    let before = record(&scratch, C2)["session_of_record"].clone();
    scratch.json(&["pane", "reset", C2, "--first", "hlr649 first message"], 0);
    let after = record(&scratch, C2);
    assert_ne!(after["session_of_record"], before, "a fresh session");
    assert_eq!(after["last_observed"]["shown"], after["session_of_record"], "SHOWN follows");

    // 4. A switch of one, to a session its server already has.
    let created = raw(p1, "POST", "/session", Some(&json!({})));
    let target: Value = serde_json::from_slice(&created.body).unwrap();
    let target = target["id"].as_str().expect("a session id").to_owned();
    scratch.json(&["pane", "switch", C1, &target], 0);
    let switched = record(&scratch, C1);
    assert_eq!(switched["session_of_record"], target.as_str());
    assert_eq!(switched["last_observed"]["shown"], target.as_str(), "SHOWN against the record");

    // 5. A park of one.
    scratch.json(&["pane", "park", C2, "--reason", "hlr649", "--release-when", "manual"], 0);

    // 6. A killed server, found by doctor.
    let pids = scratch.owned_pids(C2);
    assert!(!pids.is_empty(), "the server of {C2} runs under its tmux session");
    for pid in pids {
        signal_group(pid, "KILL");
    }
    assert!(wait_for(Duration::from_secs(10), || refused(p2).then_some(())).is_some());
    assert!(!findings_for(&scratch, C2).is_empty(), "doctor finds the killed server");

    // 7. A session deleted under a TUI, reported.
    raw(p1, "DELETE", &format!("/session/{target}"), None);
    assert!(!findings_for(&scratch, C1).is_empty(), "doctor reports the deleted session");

    // 8. A hub restart, after which `pane list` is what it was.
    let listed = scratch.json(&["pane", "list"], 0).data();
    scratch.restart_hub();
    assert_eq!(scratch.json(&["pane", "list"], 0).data(), listed, "the registry is intact");

    // The remedies doctor names, then the strays the reset and the switch left.
    scratch.json(&["pane", "reset", C1], 0);
    scratch.json(&["pane", "unpark", C2], 0);
    scratch.json(&["pane", "relaunch", C2], 0);
    no_model_can_be_reached(p2);
    for (pane, port) in [(C1, p1), (C2, p2)] {
        let sor = record(&scratch, pane)["session_of_record"].as_str().unwrap().to_owned();
        delete_strays(port, &sor);
    }

    // AC 6: the registry equals the observed state.
    let report = scratch.json(&["pane", "doctor"], 0).data();
    assert_eq!(report["findings"], json!([]), "doctor reports no finding: {report}");
    let observed: BTreeSet<(String, String)> = scratch
        .herdr_snapshot()
        .into_iter()
        .filter(|p| p.workspace == "hlr649")
        .map(|p| (p.pane_id.as_str().to_owned(), json!(p.grid).to_string()))
        .collect();
    let recorded: BTreeSet<(String, String)> = [C1, C2]
        .into_iter()
        .map(|pane| {
            let herdr = record(&scratch, pane)["herdr"].clone();
            (herdr["pane_id"].as_str().unwrap().to_owned(), herdr["grid"].to_string())
        })
        .collect();
    assert_eq!(recorded, observed, "pane list equals Herdr's snapshot for the scratch panes");

    // AC 9: each `watch` line is an envelope too.
    let watch = scratch.holler(&["pane", "watch", "--until-idle", "--format=json"]);
    assert_eq!(watch.code, 0, "{watch:?}");
    for line in watch.stdout.lines() {
        check_envelope(&format!("{line}\n"), 0)
            .unwrap_or_else(|fault| panic!("a watch line is not an envelope: {fault}: {line}"));
    }
    assert_tmux_env_is_scratch(&scratch);
}

/// AC 7: the profile scenario. Create from current; edit through `launch --profile`; `close
/// --profile`; `show` and `list`; `delete` refused while its panes are live, then
/// `--keep-panes`, after which the panes keep running with no profile.
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
        ["hlr649-c1r1", "hlr649-c2r1", "hlr649-p1"]
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
}

/// AC 8: one launch with `--grid c1r2` and one with `--grid 2,1`, each into an otherwise
/// empty scratch workspace, each at row 2, column 1 in Herdr's own snapshot.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    scratch.launch("hlr649-g1", "hlr649g", "c1r2", free_port(), &[]);
    scratch.launch("hlr649-g2", "hlr649h", "2,1", free_port(), &[]);
    let snapshot = scratch.herdr_snapshot();
    for (pane, workspace) in [("hlr649-g1", "hlr649g"), ("hlr649-g2", "hlr649h")] {
        let id = record(&scratch, pane)["herdr"]["pane_id"].as_str().unwrap().to_owned();
        let seen = snapshot
            .iter()
            .find(|p| p.pane_id.as_str() == id)
            .unwrap_or_else(|| panic!("{pane} ({id}) is in Herdr's snapshot: {snapshot:?}"));
        assert_eq!(seen.workspace, workspace);
        assert_eq!(seen.grid, GridPos { row: 2, col: 1 }, "{pane} as Herdr counts it");
    }
}

/// D15 (#802's follow-up) and AC 11: the server argv `serve_command` builds runs under a real
/// tmux pty through `TmuxHost::run`; the engine's health wait sees it, the model guard runs
/// first, `verify_serving` holds, and `stop_owned` leaves the port refusing and no process of
/// the test running.
#[test]
#[ignore = "opt-in: scratch tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
fn the_server_argv_runs_under_a_real_tmux_pty_and_stops() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    let name = PaneName::parse("hlr649-serve").unwrap();
    let port = free_port();
    let harness = OpenCodeHarness::new(OpenCodeConfig {
        opencode_bin: scratch.settings.opencode.clone(),
        serve_args: vec!["--pure".to_owned()],
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: scratch.tmux_bin.clone(),
            socket: holler_adapter_opencode::TmuxSocket::Path(scratch.tmux.sock()),
        },
        tui_session: std::sync::Arc::new(|pane: &holler_pane::PaneId| {
            Err(PaneError::PaneNotFound {
                what: pane.as_str().to_owned(),
            })
        }),
        timeouts: Timeouts::default(),
    });
    let project = scratch.settings.oc_root.join("proj").display().to_string();
    let argv = harness
        .serve_command(&name, port, &ServeSpec::default())
        .expect("serve_command");
    scratch.host.ensure_session(&name, &project).expect("ensure_session");
    let begun = Instant::now();
    scratch.host.run(&name, &project, &argv).expect("run under tmux");
    let healthy = wait_healthy(&harness, port, BOOT_POLL, Instant::now() + Duration::from_secs(30));
    eprintln!("a cold server under tmux answered its health in {:?}", begun.elapsed());
    assert_eq!(healthy, Ok(true), "the server under tmux answers its health");
    no_model_can_be_reached(port);
    assert_eq!(harness.verify_serving(port, &ServeSpec::default()), Ok(()));

    let owned = scratch.owned_pids(name.as_str());
    assert!(!owned.is_empty(), "run tagged the server's window");
    assert_eq!(scratch.host.stop_owned(&name), Ok(()));
    assert!(
        wait_for(Duration::from_secs(10), || refused(port).then_some(())).is_some(),
        "the port refuses once the server is stopped"
    );
    for pid in owned {
        let alive = std::process::Command::new("kill")
            .args(["-s", "0", "--", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        assert!(!alive, "no server process of the test is left ({pid})");
    }
}

#[test]
#[ignore = "throwaway probe"]
fn zz_probe_attach_after_run() {
    let Some(scratch) = Scratch::start() else { return; };
    let name = PaneName::parse("hlr649-c1r1").unwrap();
    let port = free_port();
    let n2 = name.clone();
    let harness = OpenCodeHarness::new(OpenCodeConfig {
        opencode_bin: scratch.settings.opencode.clone(),
        serve_args: vec!["--pure".to_owned()],
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig { tmux_bin: scratch.tmux_bin.clone(), socket: holler_adapter_opencode::TmuxSocket::Path(scratch.tmux.sock()) },
        tui_session: std::sync::Arc::new(move |_: &holler_pane::PaneId| Ok(n2.clone())),
        timeouts: Timeouts::default(),
    });
    let project = scratch.settings.oc_root.join("proj").display().to_string();
    let argv = harness.serve_command(&name, port, &ServeSpec::default()).unwrap();
    scratch.host.ensure_session(&name, &project).unwrap();
    eprintln!("before run: {:?}", scratch.tmux.lines(&["list-panes", "-s", "-t", "=hlr649-c1r1", "-F", "#{window_index}.#{pane_index} #{window_active} #{pane_current_command} #{pane_dead}"]));
    scratch.host.run(&name, &project, &argv).unwrap();
    eprintln!("after run: {:?}", scratch.tmux.lines(&["list-panes", "-s", "-t", "=hlr649-c1r1", "-F", "#{window_index}.#{pane_index} #{window_active} #{pane_current_command} #{pane_dead}"]));
    assert_eq!(wait_healthy(&harness, port, BOOT_POLL, Instant::now() + Duration::from_secs(30)), Ok(true));
    let sid = harness.create_session(port).unwrap();
    let r = harness.attach_tui(&holler_pane::PaneId::new("w1:p1"), port, &sid);
    eprintln!("attach: {r:?}");
    eprintln!("after attach: {:?}", scratch.tmux.tmux(&["list-panes", "-s", "-t", "=hlr649-c1r1", "-F", "#{window_index}.#{pane_index} #{window_active} #{pane_current_command} #{pane_dead} #{pane_dead_status}"]));
    eprintln!("sessions: {:?}", scratch.tmux.tmux(&["list-sessions"]));
}
