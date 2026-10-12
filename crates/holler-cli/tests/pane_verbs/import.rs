//! `holler pane import` (#650): read the old launcher's per-pane files, join each pane to
//! what Herdr, tmux and the harness show, and record the fleet as the profile `fleet`.
//!
//! Every case runs the verb in-process over the test kit's fakes (`import/rig.rs`), with
//! the files in a temporary `<state-root>`. The join and the observations are
//! `import/join.rs`; the readers of the files and their secrets are `import/sources.rs`; the
//! dry run, the real run's writes and their undo are `import/writes.rs`. The finding
//! vocabulary's code table is `crates/holler-pane/tests/import_test.rs`. A pane whose tmux
//! session runs Claude Code, and the `--adopt-claude-code` switch (#835), are
//! `import/harness.rs`.

mod harness;
mod join;
pub(crate) mod rig;
mod seams;
mod sources;
mod writes;

use holler_cli::output::Format;
use holler_pane_testkit::pane_store::PaneStoreOp;

use crate::doctor::rig::Seed;
use rig::{assert_no_store_write, error_of, ImportRig};

const PANE: &str = "hj-c1r1";

fn workspace_file(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Acceptance 11: `--herdr-session` is required, as on `launch`: without it the verb answers
/// `usage` (exit 2) before any adapter call or store write.
#[test]
fn an_import_needs_a_herdr_session() {
    let rig = ImportRig::new(&[Seed::new(PANE, 1, 1)]);
    let argv: Vec<String> = ["pane", "import", "--from", &rig.root_str()]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let mark = rig.mark();
    let run = rig.run_argv(&argv, Format::Json);
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(error_of(&run).0, "usage");
    let calls = rig.calls_since(&mark);
    assert_eq!(calls.herdr, vec![], "no Herdr call");
    assert_eq!(calls.host, vec![], "no host call");
    assert_eq!(calls.harness, vec![], "no harness call");
    assert_no_store_write(&calls);
}

/// Acceptance 11: a `--orchestrator` name that matches no pane directory is `usage` (exit 2),
/// before any adapter is read.
#[test]
fn an_unknown_orchestrator_is_a_usage_error() {
    let rig = ImportRig::new(&[Seed::new(PANE, 1, 1)]);
    let mark = rig.mark();
    for format in [Format::Json, Format::Text] {
        let run = rig.run(&["--orchestrator", "hj-c9r9", "--dry-run"], format);
        assert_eq!(run.code, 2, "{run:?}");
        if format == Format::Json {
            assert_eq!(error_of(&run).0, "usage");
        } else {
            assert!(run.out.is_empty(), "{run:?}");
            assert_eq!(run.err.lines().count(), 1, "one line: {run:?}");
        }
    }
    let calls = rig.calls_since(&mark);
    assert_eq!(calls.herdr, vec![], "no Herdr call");
    assert_eq!(calls.host, vec![], "no host call");
    assert_eq!(calls.harness, vec![], "no harness call");
    assert!(
        !calls.panes.contains(&PaneStoreOp::CasPut),
        "no write: {:?}",
        calls.panes
    );
}

/// Acceptance 12: ADR-0021 section 9's import row names every error code the verb answers,
/// and the open-codes row lists `import-blocked`.
#[test]
fn adr_0021_lists_the_import_codes() {
    let adr = workspace_file("docs/adr/ADR-0021.md");
    let rows: Vec<&str> = adr
        .lines()
        .filter(|l| l.starts_with("| `pane import`"))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "one `pane import` row in section 9: {rows:?}"
    );
    for code in [
        "profile-exists",
        "profile-conflict",
        "generation-conflict",
        "herdr-version-unsupported",
        "import-blocked",
        "usage",
    ] {
        assert!(
            rows[0].contains(&format!("`{code}`")),
            "the import row names `{code}`: {}",
            rows[0]
        );
    }
    let open_row = adr
        .lines()
        .find(|l| l.contains("any other well-formed code (open, raised as `Refused`)"))
        .unwrap_or_else(|| panic!("ADR-0021 has the open-codes row"));
    assert!(
        open_row.contains("`import-blocked`"),
        "the open-codes row lists `import-blocked`: {open_row}"
    );
}

/// #835 (brief section 4, R2-2): the switch is in ADR-0021 section 9's import row and in ADR
/// 0003's line for the verb, and ADR-0021 records the decision and the two new codes in a
/// "Decided (#835)" paragraph.
#[test]
fn the_adrs_name_the_switch_and_the_two_new_codes() {
    let adr = workspace_file("docs/adr/ADR-0021.md");
    let row = adr
        .lines()
        .find(|l| l.starts_with("| `pane import`"))
        .unwrap_or_else(|| panic!("ADR-0021 has the `pane import` row"));
    assert!(
        row.contains("--adopt-claude-code"),
        "the import row names the switch: {row}"
    );
    assert!(
        adr.contains("Decided (#835)"),
        "ADR-0021 records #835's decision"
    );
    for code in ["harness-claude-code", "harness-unreadable"] {
        assert!(adr.contains(&format!("`{code}`")), "ADR-0021 names `{code}`");
    }
    let adr_0003 = workspace_file("docs/adr/ADR-0003.md");
    let verb = adr_0003
        .lines()
        .find(|l| l.starts_with("holler pane import "))
        .unwrap_or_else(|| panic!("ADR 0003 has a line for `holler pane import`"));
    assert!(
        verb.contains("[--adopt-claude-code]"),
        "ADR 0003's line names the switch: {verb}"
    );
}
