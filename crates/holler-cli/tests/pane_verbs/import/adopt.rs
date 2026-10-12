//! `holler pane import --adopt-claude-code` (#831, ADR-0021 section 15 item 17): the flag adopts
//! the panes whose tmux session runs Claude Code into the registry, with the Claude Code
//! harness, acting on nothing live. This file is the adoption itself (the record it writes, the
//! dry run, the reads it makes, what it takes from `pane.env`, a mixed grid); the cases that
//! stop an adoption are `adopt_problems.rs`, the existing `fleet` and idempotence are
//! `adopt_fleet.rs`, and the undo is `adopt_undo.rs`. What the import does *without* the flag is
//! `harness.rs`.
//!
//! Every case runs over `AdoptRig` (`adopt_rig.rs`): the import rig's world, a `FakeClaude`, and
//! one router that reaches both harness fakes. A Claude Code pane is a tmux session in which a
//! process named `claude` runs, as the fleet's panes do: a person typed it into the pane's shell,
//! so the session shows `bash` (a pid only the host knows) and then the Claude process, whose pid
//! the host and `FakeClaude` share.

use holler_cli::output::Format;
use holler_cli::pane::list::optional_text;
use holler_pane::pane::{HarnessKind, PaneRole};
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use serde_json::Value;

use super::adopt_rig::{
    findings_about, spec_names, spec_of, AdoptRig, ADOPT, ANTHROPIC_MODEL, CLAUDE_SESSION,
};
use super::rig::{
    assert_absent, assert_no_store_write, data_of, findings, message, one_finding, pane_row,
    text_line, ImportRig, FLEET, SECRET,
};
use crate::doctor::rig::Seed;

/// The OpenCode pane and the Claude Code pane of the grid; each name spells the cell it sits in.
const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";

/// The Claude Code pane alone, in the fleet's real shape (`bash`, then a typed `claude`), with
/// an Anthropic model in its `pane.env`. Returns the rig and the Claude process's pid.
fn claude_pane() -> (AdoptRig, u32) {
    let rig = AdoptRig::new(&[Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    let pid = rig.typed_claude(B, CLAUDE_SESSION);
    (rig, pid)
}

/// `A` as an OpenCode pane beside `B` as a Claude Code pane.
fn mixed_grid() -> (AdoptRig, u32) {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    let pid = rig.typed_claude(B, CLAUDE_SESSION);
    (rig, pid)
}

// --- the record --------------------------------------------------------------------------

/// AC4: the record is the Claude Code harness: `kind: claude`, no port, the pid `agents --json`
/// lists (found through `foreground_pids` and matched by `shown_session`, skipping the shell's
/// pid), health unknown, the session that process holds (never the old `last-session`), no
/// command, no agent, `hold: none`, in `fleet`, and the pane's spec in `fleet` is a Claude spec
/// with the `none` port policy. The row is `record` and no source file changes.
#[test]
fn a_typed_in_claude_pane_is_adopted_with_the_process_and_session_agents_json_lists() {
    let (rig, pid) = claude_pane();
    let live = rig.world.live(B).clone();
    let before = rig.files();
    let run = rig.run_routed(&[ADOPT], Format::Json);
    let data = data_of(&run);

    let want = rig.expected_claude(
        B,
        PaneRole::Agent,
        pid,
        CLAUDE_SESSION,
        AdoptRig::anthropic(),
    );
    assert_eq!(rig.record(B), Some(want), "the record an adoption writes");
    assert_ne!(
        rig.record(B).and_then(|r| r.session_of_record),
        Some(live.session),
        "the session is the process's, not last-session's"
    );
    assert_eq!(data["recorded"], 1, "{data}");
    let row = pane_row(&data, B);
    assert_eq!(row["action"], "record", "{row}");
    assert_eq!(row["role"], "agent", "{row}");
    assert_eq!(row["session_of_record"], CLAUDE_SESSION, "{row}");

    let spec = spec_of(&data, B);
    assert_eq!(spec["harness"]["kind"], "claude", "{spec}");
    assert_eq!(spec["harness"]["port_policy"], "none", "{spec}");
    assert_eq!(rig.fleet().expect("fleet").panes.len(), 1);
    assert_absent(&run, SECRET);
    assert_eq!(rig.files(), before, "no source file changed");
}

/// AC8, AC4: the report of an adoption has the pane's one info finding and no other, and it ends
/// with the next step, `holler pane relaunch NAME` (an Anthropic model and a clean `pane.env`
/// give no warning); `harness-claude-code` is not reported for a pane that was adopted.
#[test]
fn an_adopted_pane_has_one_info_finding_naming_relaunch() {
    let (rig, _) = claude_pane();
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let adopted = one_finding(&data, "claude-code-adopted", B, "info");
    assert!(
        message(adopted).contains(&format!("holler pane relaunch {B}")),
        "the report ends with the next step: {adopted}"
    );
    assert_eq!(
        findings_about(&data, B),
        vec![adopted],
        "no other finding: {data}"
    );
    assert!(findings(&data, "harness-claude-code").is_empty(), "{data}");
}

/// AC4, AC9: the chain is the tmux reads of any pane (`ps`, `foreground_commands`), then
/// `foreground_pids`, then one `shown_session` per pid in the order `foreground_pids` gave them
/// (the shell's pid, which the harness does not know, and then `claude`'s), and nothing of an
/// OpenCode server: none of the three OpenCode helpers' reads.
#[test]
fn the_chain_of_an_adopted_pane_reads_tmux_then_one_session_per_pid() {
    let (rig, _) = claude_pane();
    let mark = rig.mark_all();
    data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let since = rig.since(&mark);
    assert_eq!(
        since.world.host,
        [
            HostOp::Ps,
            HostOp::ForegroundCommands,
            HostOp::ForegroundPids
        ],
        "the chain begins as for any pane, then reads the pids"
    );
    assert_eq!(
        since.claude,
        [ClaudeOp::ShownSession, ClaudeOp::ShownSession],
        "one shown_session per pid: the shell's (no such process) and then claude's"
    );
    assert_eq!(
        since.world.harness,
        vec![],
        "no OpenCode server call for it"
    );
}

/// AC4: a pane whose `claude` is the first and only process (nothing typed into a shell) is
/// adopted the same way.
#[test]
fn a_claude_pane_with_no_shell_beside_it_is_adopted_too() {
    let rig = AdoptRig::new(&[Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    let pid = rig.claude_process(B, CLAUDE_SESSION);
    data_of(&rig.run_routed(&[ADOPT], Format::Json));
    let want = rig.expected_claude(
        B,
        PaneRole::Agent,
        pid,
        CLAUDE_SESSION,
        AdoptRig::anthropic(),
    );
    assert_eq!(rig.record(B), Some(want));
}

/// The orchestrator role works as for any pane.
#[test]
fn an_adopted_pane_named_by_orchestrator_takes_that_role() {
    let (rig, pid) = claude_pane();
    data_of(&rig.run_routed(&[ADOPT, "--orchestrator", B], Format::Json));
    let want = rig.expected_claude(
        B,
        PaneRole::Orchestrator,
        pid,
        CLAUDE_SESSION,
        AdoptRig::anthropic(),
    );
    assert_eq!(rig.record(B), Some(want));
}

// --- the text form and the dry run --------------------------------------------------------

/// AC11 and AC8: a dry run with the flag exits 0 and writes nothing; each adopted pane has its
/// row (`record`, role, position, session) and its `claude-code-adopted` line in the text form;
/// the profile is the one a real run would create ("to create"); the summary counts the adopted
/// panes; and the real run's report equals the dry run's apart from the stamps.
#[test]
fn a_dry_run_shows_the_adoption_and_a_real_run_does_what_it_showed() {
    let (rig, _) = claude_pane();
    let before = rig.files();
    let mark = rig.mark_all();
    let dry = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    assert_no_store_write(&rig.since(&mark).world);
    assert_eq!(rig.files(), before);
    assert!(
        rig.records().is_empty() && rig.fleet().is_none(),
        "nothing written"
    );
    assert_eq!(dry["recorded"], 0);
    assert_eq!(pane_row(&dry, B)["action"], "record", "{dry}");
    assert_eq!(spec_names(&dry), [B]);

    let text = rig.run_routed(&[ADOPT, "--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert_eq!(
        text_line(&text.out, B),
        [
            B,
            "record",
            "agent",
            "r2c1",
            &optional_text(Some(CLAUDE_SESSION))
        ]
    );
    assert!(
        text.out
            .lines()
            .any(|l| l.starts_with("  info claude-code-adopted: ")),
        "{}",
        text.out
    );
    assert!(
        text.out.contains("profile fleet (fleet) to create: 1 spec"),
        "{}",
        text.out
    );
    let summary = text.out.lines().last().expect("a summary line");
    assert!(
        summary.contains("adopted"),
        "the summary counts them: {summary}"
    );

    let real = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(real["panes"], dry["panes"], "the plan a dry run shows");
    assert_eq!(real["findings"], dry["findings"]);
    assert_eq!(real["profile"]["panes"], dry["profile"]["panes"]);
}

/// AC8: the text summary of a real run counts the adopted panes; without any it does not say
/// `adopted`.
#[test]
fn the_text_summary_counts_the_adopted_panes_only_when_there_are_some() {
    let (rig, _) = mixed_grid();
    let adopting = rig.run_routed(&[ADOPT], Format::Text);
    assert_eq!(adopting.code, 0, "{adopting:?}");
    let summary = adopting.out.lines().last().expect("a summary line");
    assert!(summary.contains("adopted"), "{summary}");

    let plain = ImportRig::new(&[Seed::new(A, 1, 1)]);
    let plain = plain.run(&[], Format::Text);
    let summary = plain.out.lines().last().expect("a summary line");
    assert!(!summary.contains("adopted"), "{summary}");
}

// --- acts on nothing live -----------------------------------------------------------------

/// AC7, dry run and real run both: the calls are reads (Herdr's snapshot and version, the
/// host's `ps`, `foreground_commands` and `foreground_pids`, the harness's `shown_session`),
/// the stores are written only by a real run, no OpenCode server is asked about the Claude
/// pane, and nothing is run, stopped, typed, sent or created.
#[test]
fn the_adoption_reads_and_acts_on_nothing_live() {
    let (rig, _) = claude_pane();
    for extra in [&[ADOPT, "--dry-run"][..], &[ADOPT]] {
        let mark = rig.mark_all();
        let data = data_of(&rig.run_routed(extra, Format::Json));
        let calls = rig.since(&mark);
        let live = &calls.world;
        assert_eq!(
            pane_row(&data, B)["action"],
            "record",
            "{extra:?} adopts: {data}"
        );
        assert!(
            live.herdr
                .iter()
                .all(|op| matches!(op, HerdrOp::Snapshot | HerdrOp::Version)),
            "{extra:?}: Herdr: {:?}",
            live.herdr
        );
        assert!(
            live.host.iter().all(|op| matches!(
                op,
                HostOp::Ps | HostOp::ForegroundCommands | HostOp::ForegroundPids
            )),
            "{extra:?}: host: {:?}",
            live.host
        );
        assert!(
            calls.claude.iter().all(|op| *op == ClaudeOp::ShownSession),
            "{extra:?}: Claude harness: {:?}",
            calls.claude
        );
        assert_eq!(live.harness, vec![], "{extra:?}: no OpenCode harness call");
        assert_eq!(live.probes, 0, "{extra:?}: no probe");
        let dry = extra.contains(&"--dry-run");
        if dry {
            assert_no_store_write(live);
        } else {
            assert!(rig.record(B).is_some(), "the real run adopted the pane");
        }
        assert!(
            live.panes.iter().all(|op| matches!(
                op,
                PaneStoreOp::Get | PaneStoreOp::List | PaneStoreOp::CasPut
            )),
            "{extra:?}: pane store: {:?}",
            live.panes
        );
    }
}

// --- what the adoption takes from pane.env (D3) ----------------------------------------

/// D3: the file's agent is ignored without a finding (`relaunch` refuses any agent for Claude
/// Code), so the record has none, the pane is not left with a difference from its own spec, and
/// `profile show fleet` compares it `matches`.
#[test]
fn a_default_agent_in_pane_env_is_ignored_and_leaves_no_difference() {
    let rig = AdoptRig::new(&[Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &["export OC_DEFAULT_AGENT=builder"]);
    rig.typed_claude(B, CLAUDE_SESSION);
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert_eq!(rig.record(B).expect("a record").opencode_agent, None);
    one_finding(&data, "claude-code-adopted", B, "info");
    assert_eq!(
        findings_about(&data, B).len(),
        1,
        "no finding for the agent: {data}"
    );

    let shown = data_of(&rig.verb(&["profile", "show", FLEET], Format::Json));
    assert_eq!(shown["comparison"][0]["status"], "matches", "{shown}");

    let apply = data_of(&rig.verb(
        &[
            "profile",
            "apply",
            FLEET,
            "--herdr-session",
            "scratch",
            "--dry-run",
        ],
        Format::Json,
    ));
    let step = &apply["steps"][0];
    assert_eq!(step["action"], "adopt", "{apply}");
    assert_eq!(
        step["diff"],
        Value::Null,
        "plans no `differs` for the pane: {apply}"
    );
}

/// D3: a provider that is not `anthropic` is a warning that names the pane and the variable
/// `OC_MODEL` and never its value; the pane is still adopted, with the model as the file
/// declares it, because the operator wants it adopted and the report saying so.
#[test]
fn a_model_that_is_not_anthropics_is_a_warning_and_the_pane_is_still_adopted() {
    let rig = AdoptRig::new(&[Seed::new(B, 2, 1)]);
    rig.write_env(B, "vllm/some-model", &[]);
    let pid = rig.typed_claude(B, CLAUDE_SESSION);
    let run = rig.run_routed(&[ADOPT], Format::Json);
    let data = data_of(&run);
    let warning = one_finding(&data, "claude-model-not-anthropic", B, "warning");
    assert!(message(warning).contains("OC_MODEL"), "{warning}");
    assert!(
        !message(warning).contains("vllm"),
        "never a value: {warning}"
    );
    assert!(
        !message(warning).contains("some-model"),
        "never a value: {warning}"
    );
    one_finding(&data, "claude-code-adopted", B, "info");
    let want = rig.expected_claude(
        B,
        PaneRole::Agent,
        pid,
        CLAUDE_SESSION,
        AdoptRig::model("vllm", "some-model"),
    );
    assert_eq!(rig.record(B), Some(want), "adopted, the model as read");
}

/// D3: the provider is compared case-insensitively, and `anthropic/...` gets neither a warning
/// nor any other finding than the adoption's own.
#[test]
fn an_anthropic_model_in_any_case_gets_no_warning() {
    for model in [
        ANTHROPIC_MODEL,
        "Anthropic/claude-opus-4",
        "ANTHROPIC/claude-opus-4",
    ] {
        let rig = AdoptRig::new(&[Seed::new(B, 2, 1)]);
        rig.write_env(B, model, &[]);
        rig.typed_claude(B, CLAUDE_SESSION);
        let data = data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
        assert!(
            findings(&data, "claude-model-not-anthropic").is_empty(),
            "{model}: {data}"
        );
        assert_eq!(pane_row(&data, B)["action"], "record", "{model}: {data}");
    }
}

/// AC4: the adoption calls none of the three OpenCode helpers: a launcher `command` that is a
/// shell string adds no `command-not-argv`, and a `last-session` that disagrees with
/// `sessions.toml` adds no `sources-disagree` (the session is the process's); the record has no
/// command; and no OpenCode server is asked.
#[test]
fn the_opencode_only_files_checks_do_not_apply_to_an_adopted_pane() {
    let (rig, pid) = claude_pane();
    let port = rig.world.live(B).port;
    rig.write_launcher_with(
        B,
        &format!("port = {port}\ncwd = \"/srv/demo/{B}\"\ncommand = \"claude --resume\""),
    );
    rig.write(&format!("{B}/last-session"), "some-other-session\n");
    let mark = rig.mark_all();
    let data = data_of(&rig.run_routed(&[ADOPT], Format::Json));
    assert!(findings(&data, "command-not-argv").is_empty(), "{data}");
    assert!(findings(&data, "sources-disagree").is_empty(), "{data}");
    assert!(findings(&data, "no-session-recorded").is_empty(), "{data}");
    let want = rig.expected_claude(
        B,
        PaneRole::Agent,
        pid,
        CLAUDE_SESSION,
        AdoptRig::anthropic(),
    );
    assert_eq!(rig.record(B), Some(want));
    assert_eq!(rig.since(&mark).world.harness, vec![], "no OpenCode call");
}

// --- one fleet of both kinds (AC12) ---------------------------------------------------------

/// AC12: one run adopts the Claude Code pane and records the OpenCode pane into a new `fleet`;
/// the specs have the two kinds; `profile show fleet` compares both `matches`; and a later
/// `profile apply --dry-run fleet` plans `adopt` for both with no difference (a round trip).
#[test]
fn one_run_adopts_a_claude_pane_and_records_an_opencode_pane_into_one_fleet() {
    let (rig, pid) = mixed_grid();
    let run = rig.run_routed(&[ADOPT], Format::Json);
    let data = data_of(&run);
    assert_eq!(data["recorded"], 2, "{data}");
    assert_eq!(spec_names(&data), [A, B]);
    assert_eq!(spec_of(&data, A)["harness"]["kind"], "opencode", "{data}");
    assert_eq!(spec_of(&data, B)["harness"]["kind"], "claude", "{data}");
    assert_eq!(
        rig.record(A),
        Some(rig.expected_record(A, PaneRole::Agent)),
        "the OpenCode pane is recorded as it is without the flag"
    );
    let adopted = rig.record(B).expect("the Claude pane is adopted");
    assert_eq!(adopted.harness.kind, HarnessKind::Claude);
    assert_eq!(adopted.harness.pid, Some(pid));
    one_finding(&data, "claude-code-adopted", B, "info");

    let show = rig.verb(&["profile", "show", FLEET], Format::Text);
    assert_eq!(show.code, 0, "{show:?}");
    for line in [
        "  harness.kind: opencode",
        "  harness.kind: claude",
        &format!("pane {A}: matches"),
        &format!("pane {B}: matches"),
    ] {
        assert!(
            show.out.lines().any(|l| l == line),
            "{line:?} in {}",
            show.out
        );
    }

    let apply = data_of(&rig.verb(
        &[
            "profile",
            "apply",
            FLEET,
            "--herdr-session",
            "scratch",
            "--dry-run",
        ],
        Format::Json,
    ));
    let steps = apply["steps"].as_array().expect("steps");
    assert_eq!(steps.len(), 2, "{apply}");
    for step in steps {
        assert_eq!(step["action"], "adopt", "{step}");
        assert_eq!(
            step["diff"],
            Value::Null,
            "no difference from the round trip: {step}"
        );
    }
}

/// AC4: in a mixed run the OpenCode pane makes its health and list calls and the Claude pane
/// makes none, but reads its pids: one `foreground_pids` call, the Claude pane's.
#[test]
fn in_a_mixed_run_only_the_opencode_pane_asks_a_server() {
    let (rig, _) = mixed_grid();
    let mark = rig.mark_all();
    data_of(&rig.run_routed(&[ADOPT, "--dry-run"], Format::Json));
    let calls = rig.since(&mark);
    assert_eq!(
        calls.world.harness,
        [HarnessOp::Health, HarnessOp::ListSessions],
        "one server, the OpenCode pane's"
    );
    assert!(
        calls.world.host.contains(&HostOp::ForegroundPids),
        "{:?}",
        calls.world.host
    );
    let foreground = calls
        .world
        .host
        .iter()
        .filter(|op| **op == HostOp::ForegroundPids)
        .count();
    assert_eq!(foreground, 1, "only the Claude pane's pids are read");
}
