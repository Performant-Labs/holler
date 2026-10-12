//! A pane whose tmux session runs Claude Code (#835). The import reads the command name of
//! the foreground process of each tmux pane in the pane's session
//! (`HostPort::foreground_commands`):
//!
//! - a session with a pane whose command is exactly `claude` is left alone by default: one
//!   `info` finding `harness-claude-code`, the row's action `skipped`, no record, no harness
//!   call for it, and it blocks nothing;
//! - every other answer (`bash`, which the old launcher's OpenCode panes show, `opencode`, a
//!   session with no process) is imported as OpenCode, as before;
//! - `--adopt-claude-code` is refused as `usage` (exit 2), naming #831,
//!   until #831's adoption step lands, before any file is read or any port is called;
//! - a pane whose harness could not be read is never recorded on a guess: `harness-unreadable`,
//!   a problem, and a real run is refused (`import-blocked`).
//!
//! The fixture is the import rig's world (no process in any tmux session) with processes
//! started through the test kit's `FakeHost::run`, whose command name is the basename of the
//! argv's first element, as tmux reports it.

use holler_cli::output::{not_implemented_message, Format};
use holler_pane::pane::PaneRole;
use holler_pane::{Argv, HostPort, PaneError, PaneName};
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::host::HostOp;
use serde_json::Value;

use super::rig::{
    assert_no_store_write, data_of, error_of, message, one_finding, pane_row, text_line,
    ImportRig,
};
use crate::doctor::rig::Seed;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";
const C: &str = "hj-c2r1";

/// The pinned refusal of a real run with problems.
fn blocked(n: usize) -> String {
    format!("import blocked: {n} pane(s) have problems; run with --dry-run for the report")
}

/// Start `argv` in the tmux session of the pane `pane`, as something outside Holler did.
fn start(rig: &ImportRig, pane: &str, argv: &[&str]) {
    let name = PaneName::parse(pane).expect("a pane name");
    let argv = Argv::new(argv.iter().map(|part| (*part).to_owned()).collect());
    rig.world
        .host
        .run(&name, "/srv/demo", &argv)
        .expect("start a process in the pane's session");
}

/// The panes `A` and `B`, where `B`'s session runs a shell and Claude Code beside it (the old
/// launcher types `claude` into the pane's shell).
fn claude_in_b() -> ImportRig {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    start(&rig, B, &["bash"]);
    start(&rig, B, &["claude", "--resume"]);
    rig
}

/// Every finding of `data` about `pane`.
fn findings_about<'v>(data: &'v Value, pane: &str) -> Vec<&'v Value> {
    data["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("data.findings is an array: {data}"))
        .iter()
        .filter(|f| f["pane"] == pane)
        .collect()
}

/// The pane names of the specs of `data.profile` (`ProfileSpec.pane`).
fn spec_names(data: &Value) -> Vec<String> {
    data["profile"]["panes"]
        .as_array()
        .unwrap_or_else(|| panic!("data.profile.panes is an array: {data}"))
        .iter()
        .map(|spec| spec["pane"].as_str().expect("a spec's pane name").to_owned())
        .collect()
}

/// `pane`'s row is a pane left alone: `skipped`, with no role, position or session.
fn assert_left_alone_row(data: &Value, pane: &str) {
    let row = pane_row(data, pane);
    assert_eq!(row["action"], "skipped", "{row}");
    assert_eq!(row["role"], Value::Null, "{row}");
    assert_eq!(row["pos"], Value::Null, "{row}");
    assert_eq!(row["session_of_record"], Value::Null, "{row}");
}

// --- the default: a Claude Code pane is left alone -----------------------------------------

/// Issue Work item 1, brief section 2: `claude` is left alone; `bash` and a session with no
/// process are imported as OpenCode, exactly as before.
#[test]
fn a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode() {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1), Seed::new(C, 1, 2)]);
    start(&rig, A, &["bash", "-c", "oc-holler"]);
    start(&rig, B, &["bash"]);
    start(&rig, B, &["claude", "--resume"]);
    let mark = rig.mark();
    let data = data_of(&rig.run(&[], Format::Json));
    let calls = rig.calls_since(&mark);

    let finding = one_finding(&data, "harness-claude-code", B, "info");
    assert_eq!(
        findings_about(&data, B),
        vec![finding],
        "one finding and no others for a pane left alone: {data}"
    );
    assert_left_alone_row(&data, B);
    assert_eq!(data["recorded"], 2, "{data}");
    assert!(rig.record(B).is_none(), "no record for the pane left alone");
    assert_eq!(
        rig.record(A),
        Some(rig.expected_record(A, PaneRole::Agent)),
        "a pane whose session shows bash is imported as OpenCode"
    );
    assert_eq!(
        rig.record(C),
        Some(rig.expected_record(C, PaneRole::Agent)),
        "a session with no process is imported as OpenCode"
    );
    assert_eq!(spec_names(&data), [A, C], "fleet holds only the recorded panes");

    let health = calls
        .harness
        .iter()
        .filter(|op| **op == HarnessOp::Health)
        .count();
    let lists = calls
        .harness
        .iter()
        .filter(|op| **op == HarnessOp::ListSessions)
        .count();
    assert_eq!(
        (health, lists),
        (2, 2),
        "no harness call for the pane left alone: {:?}",
        calls.harness
    );
}

/// Issue Work item 1: "a dry run shows the same finding", and nothing is written.
#[test]
fn a_dry_run_reports_the_pane_left_alone_and_writes_nothing() {
    let rig = claude_in_b();
    let before = rig.files();
    let mark = rig.mark();
    let data = data_of(&rig.run(&["--dry-run"], Format::Json));
    assert_no_store_write(&rig.calls_since(&mark));
    assert_eq!(rig.files(), before, "no file under the state root changed");
    one_finding(&data, "harness-claude-code", B, "info");
    assert_left_alone_row(&data, B);
    assert_eq!(pane_row(&data, A)["action"], "record", "{data}");
    assert_eq!(spec_names(&data), [A], "the profile a real run would create");
    assert!(rig.records().is_empty(), "no record");
    assert!(rig.fleet().is_none(), "no profile");

    let text = rig.run(&["--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert_eq!(text_line(&text.out, B), [B, "skipped", "-", "-", "-"]);
    assert!(
        text.out
            .lines()
            .any(|l| l.starts_with("  info harness-claude-code: ")),
        "{}",
        text.out
    );
}

/// Brief section 3: one finding and no others, so a pane left alone blocks nothing even when
/// it would have had a problem as an OpenCode pane (here, no Herdr pane carries its label).
#[test]
fn a_pane_left_alone_blocks_nothing_even_unlabelled() {
    let rig = claude_in_b();
    rig.label(B, "someone-else");
    let data = data_of(&rig.run(&[], Format::Json));
    assert_eq!(
        findings_about(&data, B).len(),
        1,
        "only harness-claude-code: {data}"
    );
    one_finding(&data, "harness-claude-code", B, "info");
    assert_eq!(data["recorded"], 1, "the real run is not refused: {data}");
    assert!(rig.record(A).is_some(), "the other pane imports");
    assert!(rig.record(B).is_none());
}

// --- the switch -------------------------------------------------------------------------

/// Issue Work item 2, brief section 4: until #831 the switch is `usage` (exit 2), with the
/// one wording of a story not landed (`not_implemented_message(831)`), refused before any
/// file is read or any port is called, so a dry run is refused too and nothing is written.
#[test]
fn adopt_claude_code_is_usage_naming_831_and_writes_nothing() {
    let rig = claude_in_b();
    let before = rig.files();
    let mark = rig.mark();
    let want = not_implemented_message(831);
    for extra in [&["--adopt-claude-code"][..], &["--adopt-claude-code", "--dry-run"]] {
        let run = rig.run(extra, Format::Json);
        assert_eq!(run.code, 2, "{extra:?}: {run:?}");
        let (code, text) = error_of(&run);
        assert_eq!(code, "usage", "{extra:?}");
        assert!(text.contains(&want), "{extra:?}: {text:?} names {want:?}");

        let text = rig.run(extra, Format::Text);
        assert_eq!(text.code, 2, "{extra:?}: {text:?}");
        assert!(text.out.is_empty(), "nothing on stdout: {text:?}");
        assert_eq!(text.err.lines().count(), 1, "one line: {text:?}");
        assert!(text.err.contains(&want), "{text:?}");
    }
    let calls = rig.calls_since(&mark);
    assert_eq!(calls.herdr, vec![], "no Herdr call");
    assert_eq!(calls.host, vec![], "no host call");
    assert_eq!(calls.harness, vec![], "no harness call");
    assert_no_store_write(&calls);
    assert_eq!(rig.files(), before, "no file under the state root changed");
    assert!(rig.records().is_empty(), "no record");
    assert!(rig.fleet().is_none(), "no profile");
}

// --- an unreadable harness ----------------------------------------------------------------

/// Brief R2-6 (and A's W4, which reverses #650's `a_failed_ps_is_observe_failed_and_the_pane_imports`):
/// a failed `ps` that is not `pane-not-found` leaves the harness unknown, so the pane is a
/// problem, never recorded as OpenCode on a guess, and a real run is refused.
#[test]
fn a_failed_ps_is_harness_unreadable_and_blocks() {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1)]);
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    rig.world.host.faults().fail_next(HostOp::Ps, unavailable);
    let data = data_of(&rig.run(&["--dry-run"], Format::Json));
    one_finding(&data, "harness-unreadable", A, "problem");
    assert_eq!(pane_row(&data, A)["action"], "blocked", "{data}");
    assert!(
        findings_about(&data, A)
            .iter()
            .any(|f| message(f).contains("host.ps")),
        "the report names the call that failed: {data}"
    );

    rig.world.host.faults().fail_next(
        HostOp::Ps,
        PaneError::Unavailable {
            what: "made-up".to_owned(),
        },
    );
    let run = rig.run(&[], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(1)));
    assert!(rig.records().is_empty(), "no record");
    assert!(rig.fleet().is_none(), "no profile");
}

/// Brief section 2 and A's W4: an error from the read of the command names is never a guess.
/// `pane-not-found` (the session ended between `ps` and the read) is `tmux-session-missing`,
/// as for `ps`; any other error is `harness-unreadable`, naming the call. Either is a
/// problem: no record, and a real run is refused.
#[test]
fn a_failed_command_name_read_is_a_problem_and_blocks() {
    let cases = [
        (
            PaneError::Unavailable {
                what: "made-up".to_owned(),
            },
            "harness-unreadable",
        ),
        (
            PaneError::PaneNotFound {
                what: A.to_owned(),
            },
            "tmux-session-missing",
        ),
    ];
    for (error, code) in cases {
        let rig = ImportRig::new(&[Seed::new(A, 1, 1)]);
        let faults = rig.world.host.faults();
        faults.fail_next(HostOp::ForegroundCommands, error.clone());
        let data = data_of(&rig.run(&["--dry-run"], Format::Json));
        one_finding(&data, code, A, "problem");
        assert_eq!(pane_row(&data, A)["action"], "blocked", "{code}: {data}");
        if code == "harness-unreadable" {
            assert!(
                findings_about(&data, A)
                    .iter()
                    .any(|f| message(f).contains("host.foreground_commands")),
                "the report names the call that failed: {data}"
            );
        }

        faults.fail_next(HostOp::ForegroundCommands, error);
        let run = rig.run(&[], Format::Json);
        assert_eq!(run.code, 3, "{code}: {run:?}");
        assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(1)));
        assert!(rig.records().is_empty(), "{code}: no record");
    }
}

/// A host whose `ps` panics, so the pane's chain is lost (its thread panics); every other
/// call goes to the rig's fake host.
struct LostChain<'a> {
    host: &'a holler_pane_testkit::host::FakeHost,
}

impl HostPort for LostChain<'_> {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.host.ensure_session(name, cwd)
    }
    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.host.run(name, cwd, argv)
    }
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.host.stop_owned(name)
    }
    fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
        panic!("a made-up crash in the pane's chain (#835 test)")
    }
    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.host.foreground_commands(name)
    }
    fn attached_clients(&self, name: &PaneName) -> Result<u32, PaneError> {
        self.host.attached_clients(name)
    }
}

/// Brief R2-6: a chain that did not finish read nothing, so the pane is `harness-unreadable`
/// (it was an `observe-failed` warning, and the pane imported, under #650).
#[test]
fn a_lost_chain_is_harness_unreadable_and_blocks() {
    let rig = ImportRig::new(&[Seed::new(A, 1, 1)]);
    let lost = LostChain {
        host: &rig.world.host,
    };
    let data = data_of(&rig.run_with_host(&lost, &rig.args(&["--dry-run"]), Format::Json));
    one_finding(&data, "harness-unreadable", A, "problem");
    assert_eq!(pane_row(&data, A)["action"], "blocked", "{data}");

    let run = rig.run_with_host(&lost, &rig.args(&[]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run), ("import-blocked".to_owned(), blocked(1)));
    assert!(rig.records().is_empty(), "no record");
}
