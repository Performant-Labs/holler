//! What stops an adoption (#831, AC9, AC4): a Claude Code pane is never recorded on a guess.
//! No pid found, more than one, no session held, `agents --json` or the pid read failing, a
//! `pane.env` variable that changes Claude Code's login, no unique Herdr label, and a launcher
//! entry that cannot give the pane's directory are each a `problem` with no record, so a real
//! run is `import-blocked` and writes nothing (not even the other pane's record). The pane's
//! name and a variable's name only, never a value (I7).
//!
//! The chain's bound is pinned here too: the pids are tried in the order `foreground_pids`
//! gives them, the walk stops at the second match, and a pane with no unique label is blocked
//! before any pid is read.

use holler_cli::output::Format;
use holler_pane::pane::HarnessKind;
use holler_pane::PaneError;
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::host::HostOp;
use serde_json::Value;

use super::adopt_rig::{blocked, findings_about, AdoptRig, ADOPT, ANTHROPIC_MODEL, CLAUDE_SESSION};
use super::rig::{
    assert_absent, data_of, error_of, findings, message, one_finding, pane_row, sessions_toml,
    SECRET,
};
use crate::doctor::rig::Seed;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";

fn unavailable() -> PaneError {
    PaneError::Unavailable {
        what: "made-up".to_owned(),
    }
}

/// The OpenCode pane `A` and the Claude Code pane `B`, with an Anthropic model for `B`; no
/// Claude process yet.
fn grid() -> AdoptRig {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    rig
}

/// The real run is refused as `import-blocked` for `n` panes, and nothing is written: neither
/// `B`'s record nor `A`'s, nor the profile.
fn assert_real_run_blocked(rig: &AdoptRig, n: usize) {
    let run = rig.run_routed(&[ADOPT], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(n)));
    assert!(
        rig.records().is_empty(),
        "no record, not even the other pane's"
    );
    assert!(rig.fleet().is_none(), "no profile");
}

/// The dry run of `rig`: `B` has the one `problem` finding `code`, its row is `blocked`, and no
/// adoption is reported for it; then the real run is refused. Returns the dry run's data.
fn blocked_by(rig: &AdoptRig, code: &str) -> Value {
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    one_finding(&data, code, B, "problem");
    assert_eq!(pane_row(&data, B)["action"], "blocked", "{data}");
    assert!(findings(&data, "claude-code-adopted").is_empty(), "{data}");
    assert_eq!(
        pane_row(&data, A)["action"],
        "record",
        "the other pane is still planned: {data}"
    );
    assert_real_run_blocked(rig, 1);
    data
}

// --- the process cannot be named (AC9) -------------------------------------------------

/// A session that shows a process named `claude` that `agents --json` does not list: zero pids
/// matched.
#[test]
fn a_process_agents_json_does_not_list_is_unmatched_and_blocks() {
    let rig = grid();
    rig.unlisted_claude(B);
    let mark = rig.mark_all();
    blocked_by(&rig, "claude-process-unmatched");
    let calls = rig.since(&mark);
    assert!(
        calls.claude.iter().all(|op| *op == ClaudeOp::ShownSession),
        "{:?}",
        calls.claude
    );
}

/// More than one match is never a guess, and the walk stops at the second: three Claude
/// processes in the pane's session, and `shown_session` is asked twice, not three times.
#[test]
fn two_matching_processes_block_and_the_walk_stops_at_the_second() {
    let rig = grid();
    for session in ["session-one", "session-two", "session-three"] {
        rig.claude_process(B, session);
    }
    let mark = rig.mark_all();
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    let asked = rig.since(&mark).claude;
    assert_eq!(
        asked,
        [ClaudeOp::ShownSession, ClaudeOp::ShownSession],
        "the outcome is already `more than one` at the second match"
    );
    let text = message(one_finding(&data, "claude-process-unmatched", B, "problem"));
    for session in ["session-one", "session-two", "session-three"] {
        assert!(!text.contains(session), "never a session id: {text}");
    }
    assert_real_run_blocked(&rig, 1);
}

/// The one match holds an empty session: unmatched, since a record without a session cannot be
/// relaunched on it.
#[test]
fn a_matching_process_with_no_session_is_unmatched_and_blocks() {
    let rig = grid();
    rig.claude_process(B, "");
    blocked_by(&rig, "claude-process-unmatched");
}

/// The pid read failing is `harness-unreadable`, naming the call, and `shown_session` is never
/// asked (no partial list is walked).
#[test]
fn a_failed_foreground_pids_is_harness_unreadable_and_asks_no_session() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    let arm = || {
        rig.world
            .host
            .faults()
            .fail_next(HostOp::ForegroundPids, unavailable());
    };
    let mark = rig.mark_all();
    let data = blocked_by_unreadable(&rig, &arm);
    assert!(
        findings_about(&data, B)
            .iter()
            .any(|f| message(f).contains("host.foreground_pids")),
        "the report names the call that failed: {data}"
    );
    assert_eq!(
        rig.since(&mark).claude,
        vec![],
        "no shown_session for a failed pid read"
    );
}

/// `agents --json` unreadable (`shown_session` failing) is `harness-unreadable` too.
#[test]
fn a_failed_shown_session_is_harness_unreadable() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    let arm = || {
        rig.claude
            .faults()
            .fail_next(ClaudeOp::ShownSession, unavailable());
    };
    let data = blocked_by_unreadable(&rig, &arm);
    assert!(
        findings_about(&data, B)
            .iter()
            .any(|f| message(f).contains("harness.shown_session")),
        "the report names the call that failed: {data}"
    );
}

/// `arm` queues one failure (a queued fault fails the next call only). The dry run reports `B`
/// as `harness-unreadable` and not as `claude-process-unmatched`; armed again, the real run is
/// refused and writes nothing.
fn blocked_by_unreadable(rig: &AdoptRig, arm: &dyn Fn()) -> Value {
    arm();
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    one_finding(&data, "harness-unreadable", B, "problem");
    assert_eq!(pane_row(&data, B)["action"], "blocked", "{data}");
    assert!(
        findings(&data, "claude-process-unmatched").is_empty(),
        "{data}"
    );
    arm();
    assert_real_run_blocked(rig, 1);
    data
}

// --- a login variable (AC9) ----------------------------------------------------------------

/// Each variable that changes Claude Code's login (`HarnessKind::login_variables()`, the one
/// list; not retyped here) in `pane.env` blocks the pane: `claude-login-env`, naming the pane
/// and the variable, never its value, and a real run is refused.
#[test]
fn a_login_variable_in_pane_env_blocks_the_pane_without_its_value() {
    let variables = HarnessKind::Claude.login_variables();
    assert!(!variables.is_empty(), "claude names login variables");
    for variable in variables {
        let rig = grid();
        rig.write_env(
            B,
            ANTHROPIC_MODEL,
            &[&format!("export {variable}={SECRET}")],
        );
        rig.typed_claude(B, CLAUDE_SESSION);
        let dry = rig.run_routed(&[ADOPT, "--dry-run"], Format::Json);
        let data = data_of(&dry);
        let finding = one_finding(&data, "claude-login-env", B, "problem");
        assert!(message(finding).contains(variable), "{variable}: {finding}");
        assert_eq!(
            pane_row(&data, B)["action"],
            "blocked",
            "{variable}: {data}"
        );
        assert_absent(&dry, SECRET);
        let real = rig.run_routed(&[ADOPT], Format::Json);
        assert_eq!(real.code, 3, "{variable}: {real:?}");
        assert_absent(&real, SECRET);
        assert!(rig.records().is_empty(), "{variable}: no record");
    }
}

// --- the Herdr label (AC4: the three states of Target.adopt) ---------------------------------

/// A Claude pane with no Herdr pane labelled with its name: blocked as `pane-unlabelled`, with
/// no record and no placeholder, and neither `foreground_pids` nor `shown_session` is called.
#[test]
fn an_unlabelled_claude_pane_reads_no_pid_and_no_session() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    rig.label(B, "someone-else");
    let mark = rig.mark_all();
    blocked_by(&rig, "pane-unlabelled");
    let calls = rig.since(&mark);
    assert!(
        !calls.world.host.contains(&HostOp::ForegroundPids),
        "{:?}",
        calls.world.host
    );
    assert_eq!(calls.claude, vec![], "no shown_session");
}

/// Two Herdr panes with the label: `pane-label-ambiguous`, and the same two silences.
#[test]
fn an_ambiguously_labelled_claude_pane_reads_no_pid_and_no_session() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    let other = rig.world.unregistered_herdr_pane(3, 3);
    rig.world.herdr.rename(&other.pane_id, B).expect("label");
    let mark = rig.mark_all();
    blocked_by(&rig, "pane-label-ambiguous");
    let calls = rig.since(&mark);
    assert!(
        !calls.world.host.contains(&HostOp::ForegroundPids),
        "{:?}",
        calls.world.host
    );
    assert_eq!(calls.claude, vec![], "no shown_session");
}

// --- the files (AC4 "What the flag-on Claude path does with files.problems") ----------------

/// The pane's cwd comes from the launcher entry, whose `port` a Claude pane may omit: with a cwd
/// and no port it is adopted, with that cwd in `host.cwd`.
#[test]
fn a_launcher_entry_with_a_cwd_and_no_port_is_enough_to_adopt() {
    let rig = grid();
    let pid = rig.typed_claude(B, CLAUDE_SESSION);
    rig.write_launcher_with(B, &format!("cwd = \"/srv/demo/{B}\""));
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(data["recorded"], 2, "{data}");
    let record = rig.record(B).expect("adopted");
    assert_eq!(record.host.cwd, format!("/srv/demo/{B}"));
    assert_eq!((record.harness.pid, record.harness.port), (Some(pid), None));
}

/// The launcher entries that cannot give the directory block the adopted pane, with the codes
/// the tree gives them: no entry, no cwd, and no port-and-no-cwd are `source-missing`; port 0
/// and a relative cwd (port-less or not) are `source-unreadable`.
#[test]
fn a_launcher_entry_that_cannot_give_the_directory_blocks_the_adopted_pane() {
    let cwd = format!("cwd = \"/srv/demo/{B}\"");
    let cases = [
        ("no entry", None, "source-missing"),
        ("no cwd", Some("port = 48101".to_owned()), "source-missing"),
        ("nothing", Some(String::new()), "source-missing"),
        (
            "port 0",
            Some(format!("port = 0\n{cwd}")),
            "source-unreadable",
        ),
        (
            "a relative cwd",
            Some("port = 48101\ncwd = \"relative/dir\"".to_owned()),
            "source-unreadable",
        ),
        (
            "no port and a relative cwd",
            Some("cwd = \"relative/dir\"".to_owned()),
            "source-unreadable",
        ),
    ];
    for (case, body, code) in cases {
        let rig = grid();
        rig.typed_claude(B, CLAUDE_SESSION);
        match body {
            Some(body) => rig.write_launcher_with(B, &body),
            None => rig.write(
                "launcher.toml",
                &format!(
                    "[{A}]\nport = 48100\ncwd = \"/srv/demo/{A}\"\ncommand = [\"opencode\"]\n"
                ),
            ),
        }
        let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
        one_finding(&data, code, B, "problem");
        assert_eq!(pane_row(&data, B)["action"], "blocked", "{case}: {data}");
        assert_real_run_blocked(&rig, 1);
    }
}

/// An unreadable or missing `pane.env` blocks an adopted pane as it blocks an OpenCode pane: the
/// adoption reads the file (the model, the ceilings), and a pane that cannot be read whole is
/// never recorded.
#[test]
fn a_missing_pane_env_blocks_the_adopted_pane() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    rig.remove(&format!("{B}/pane.env"));
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    assert_eq!(pane_row(&data, B)["action"], "blocked", "{data}");
    assert!(
        findings_about(&data, B)
            .iter()
            .any(|f| f["level"] == "problem"),
        "{data}"
    );
    assert_real_run_blocked(&rig, 1);
}

/// The twin for OpenCode, with the switch: a port-less entry is still the `source-missing`
/// problem with its message as `checked` words it, and the real run is `import-blocked`. The
/// switch changes nothing for a pane that runs OpenCode.
#[test]
fn a_port_less_entry_for_an_opencode_pane_is_source_missing_with_the_switch_too() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    rig.write_launcher_with(
        A,
        &format!("cwd = \"/srv/demo/{A}\"\ncommand = [\"opencode\"]"),
    );
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    let finding = one_finding(&data, "source-missing", A, "problem");
    let text = message(finding);
    assert!(
        text.starts_with(&format!("the entry for {A} in the launcher table ")),
        "{text}"
    );
    assert!(text.ends_with(" has no port"), "{text}");
    assert_eq!(pane_row(&data, A)["action"], "blocked", "{data}");
    let run = rig.run_routed(&[ADOPT], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(1)));
    assert!(rig.records().is_empty());
}

// --- what the switch does not adopt ------------------------------------------------------------

/// A pane whose `sessions.toml` names `claude` is `harness-not-opencode` (`skipped`), flag or no
/// flag: the rule that drops it is earlier than the chain, and no pid is read for it.
#[test]
fn a_pane_whose_sessions_toml_names_claude_is_still_skipped_with_the_switch() {
    let rig = grid();
    rig.typed_claude(B, CLAUDE_SESSION);
    let port = rig.world.live(B).port;
    // A Claude Code row has no endpoint (#850): a valid row, so the skip is the rule's.
    let toml = sessions_toml(B, port, CLAUDE_SESSION)
        .replace("\"opencode\"", "\"claude\"")
        .replace(&format!("endpoint = \"http://127.0.0.1:{port}\"\n"), "");
    rig.write(&format!("{B}/sessions.toml"), &toml);
    let mark = rig.mark_all();
    let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    one_finding(&data, "harness-not-opencode", B, "skipped");
    assert_eq!(pane_row(&data, B)["action"], "skipped", "{data}");
    let calls = rig.since(&mark);
    assert!(
        !calls.world.host.contains(&HostOp::ForegroundPids),
        "{:?}",
        calls.world.host
    );
    assert_eq!(calls.claude, vec![], "no shown_session");
}
