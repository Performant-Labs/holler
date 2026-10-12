//! `holler pane compact` on a Claude Code pane and on an OpenCode pane next to it (#878,
//! brief AC 3, second half; ADR-0021 "Compaction"), over the router of the two fakes.
//!
//! Claude Code stays `compact-unsupported` in Part A: whether it accepts `/compact` as a prompt
//! and where its token counts can be read are the spike's questions (Part B), so the verb
//! must say so, and must say so *after* the engine's checks, not instead of them: a busy,
//! waiting or stopped Claude pane is told what it is doing, not `compact-unsupported`. The verb
//! never types: the fake Herdr and the host see no call that acts (I4).

use holler_cli::output::Format;
use holler_pane::ports::CompactModel;
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::harness::HarnessOp;

use crate::rig::{cell, data_of, error_of, Calls, Rig, CLAUDE, OPENCODE, PORT};

/// A rig with one live Claude pane, seeded idle and healthy, whose session has a conversation.
fn idle_claude_pane() -> Rig {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    rig
}

/// The run changed nothing and typed nothing: no registry write, and no host or Herdr call that
/// acts.
fn assert_inert(calls: &Calls) {
    assert!(calls.wrote_no_record(), "no registry write: {calls:?}");
    assert!(
        calls.acted_on_nothing_live(),
        "nothing live acted (I4): {calls:?}"
    );
}

/// AC 3: an idle, healthy Claude pane is refused `compact-unsupported`, exit 3, in both
/// formats, after the engine's health and idle reads and with nothing else done to the process.
#[test]
fn an_idle_claude_pane_is_compact_unsupported() {
    let rig = idle_claude_pane();
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.expect("a pid");

    let (run, calls) = rig.run_logged(&["pane", "compact", CLAUDE], Format::Json);
    assert_eq!(run.code, 3, "a refusal: {run:?}");
    assert_eq!(error_of(&run).0, "compact-unsupported");
    assert_eq!(
        calls.claude,
        [ClaudeOp::Health, ClaudeOp::SessionActivity],
        "the engine asked health, then idle, and the default refused the act"
    );
    assert_inert(&calls);
    assert_eq!(rig.record(CLAUDE), before, "the record is unchanged");
    assert_eq!(
        rig.process_of(CLAUDE).map(|p| p.pid),
        Some(pid),
        "the same process runs"
    );

    let (text, _) = rig.run_logged(&["pane", "compact", CLAUDE], Format::Text);
    assert_eq!(text.code, 3, "{text:?}");
    assert!(text.out.is_empty(), "text: nothing on out: {text:?}");
    assert!(
        text.err.starts_with("error: ") && text.err.lines().count() == 1,
        "text: one error line: {text:?}"
    );
}

/// A Claude pane that is running a turn, holds a question or waits on a permission is told so,
/// not `compact-unsupported`: the engine's idle check comes first, so the refusal a person
/// reads is the one they can act on.
#[test]
fn a_busy_or_waiting_claude_pane_is_told_what_it_is_doing_first() {
    type Make = fn(&Rig, u32);
    let makes: [(&str, Make); 3] = [
        ("session-busy", |rig, pid| rig.claude().stall(pid).unwrap()),
        ("session-holds-question", |rig, pid| {
            rig.claude().hold_question(pid).unwrap()
        }),
        ("session-holds-permission", |rig, pid| {
            rig.claude().hold_permission(pid).unwrap()
        }),
    ];
    for (want, make) in makes {
        let rig = idle_claude_pane();
        make(&rig, rig.process_of(CLAUDE).unwrap().pid);
        let (run, calls) = rig.run_logged(&["pane", "compact", CLAUDE], Format::Json);
        assert_eq!(run.code, 3, "{want}: {run:?}");
        assert_eq!(error_of(&run).0, want);
        assert_inert(&calls);
    }
}

/// A Claude pane whose process is gone is `server-unhealthy` first, the engine's health check,
/// with the remedy; it is not `compact-unsupported` either.
#[test]
fn a_stopped_claude_process_is_server_unhealthy_first() {
    let rig = idle_claude_pane();
    let pid = rig.process_of(CLAUDE).unwrap().pid;
    rig.claude().sigint(pid).unwrap();

    let (run, calls) = rig.run_logged(&["pane", "compact", CLAUDE], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "server-unhealthy");
    assert_eq!(calls.claude, [ClaudeOp::Health]);
    assert_inert(&calls);
}

/// The verb reaches an OpenCode pane through the router of the two fakes (the wiring's shape):
/// it compacts on that pane's server, on its pin, and the Claude fake is not asked to act. A
/// Claude pane beside it is unaffected.
#[test]
fn an_opencode_pane_compacts_through_the_router_beside_a_claude_pane() {
    let rig = Rig::new();
    let opencode = rig.seed_opencode(OPENCODE, cell(1, 1), PORT);
    rig.seed_claude(CLAUDE, cell(1, 2));
    let session = opencode
        .session_of_record
        .clone()
        .expect("a session of record");
    rig.opencode().seed_context(PORT, &session, 64_000, 8_000);
    let pin = CompactModel {
        provider: "pinned-provider".to_owned(),
        model_id: "pinned-model".to_owned(),
    };
    rig.opencode().set_compaction_pin(PORT, Some(pin.clone()));

    let (run, calls) = rig.run_logged(&["pane", "compact", OPENCODE], Format::Json);
    let data = data_of(&run);
    assert_eq!(data["tokens_before"], 64_000);
    assert_eq!(data["tokens_after"], 8_000);
    assert_eq!(data["session"], session.as_str());
    assert_eq!(calls.opencode, [HarnessOp::Health, HarnessOp::Compact]);
    assert!(
        calls.claude.is_empty(),
        "the Claude fake was not asked: {calls:?}"
    );
    let runs = rig.opencode().compact_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!((runs[0].asked.clone(), runs[0].ran_on.clone()), (None, pin));
    assert_inert(&calls);
}
