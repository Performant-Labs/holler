#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Docs conformance for the operator guide of `holler pane` (issue #652): `docs/pane-control.md`
//! must name what the code has, read from the code's own tables (the clap tree, `FindingKind::ALL`,
//! `ALL_CODES` with `class_of`, `COLUMNS`, `ImportFinding::ALL`), so a verb, finding, code or
//! column added without a docs line fails here. It also pins the rollback text of
//! `docs/orchestrating.md` and `scripts/pane-restart`, the `docs/README.md` index line and the
//! CHANGELOG entry.
//!
//! Every `holler ...` command the guide shows is parsed by `docs_cli_test`, which this file does
//! not repeat. A section is sliced from `\n## N. ` to the next `\n## `, as `docs_errors_test.rs`
//! slices its section. Every test collects its failures and asserts once, so one run lists every
//! missing item. A missing file reads as empty, so each test still reports its own list.

use std::path::{Path, PathBuf};

use clap::CommandFactory;
use holler_cli::pane::list::COLUMNS;
use holler_cli::roster_cmd::PANE_COLUMNS;
use holler_cli::Cli;
use holler_pane::error::{class_of, ErrorClass, ALL_CODES};
use holler_pane::findings::FindingKind;
use holler_pane::import::findings::ImportFinding;
use holler_pane::import::IMPORT_BLOCKED;
use holler_pane::unmanaged::PANE_UNMANAGED;
use holler_pane::GridPos;
use holler_pane_testkit::envelope::check_envelope;

/// What #665 built that the guide used to mark PROPOSED: three verbs and a doctor finding. No line
/// naming one of them may say PROPOSED any more.
const BUILT_BY_665: [&str; 4] = ["profile rename", "profile export", "profile import", "profile-drift"];

/// The section 3 headings of the verbs #665 built, as the clap tree computes them (`heading_forms`),
/// and `pane import`'s heading prefix: the two `import` verbs are two headings.
const PROFILE_IO_HEADINGS: [&str; 3] =
    ["### `holler profile rename OLD NEW`", "### `holler profile export NAME`", "### `holler profile import FILE`"];
const PANE_IMPORT_HEADING: &str = "### `holler pane import ";

/// The action and outcome words of a `profile apply` step, as ADR-0021 section 9's apply row
/// lists them. Hand-kept because `tx_apply`'s `Action` and `Outcome` have only `as_str` and no
/// `ALL` table (#652 acceptance 15).
const APPLY_OUTCOMES: [&str; 8] =
    ["created", "relaunched", "adopted", "reported", "refused", "failed", "skipped", "planned"];
const APPLY_ACTIONS: [&str; 6] = ["create", "adopt", "report", "relaunch", "refuse", "observe"];

/// The fence tags `docs_cli_test` scans (an untagged fence is `""`).
const SCANNED_TAGS: [&str; 6] = ["", "bash", "sh", "shell", "console", "text"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// A workspace file's text, or the empty string when it does not exist yet.
fn read(rel: &str) -> String {
    std::fs::read_to_string(workspace_root().join(rel)).unwrap_or_default()
}

fn guide() -> String {
    read("docs/pane-control.md")
}

/// The text of `## N. ` up to the next `## ` heading, or the empty string when the heading is
/// missing.
fn section(doc: &str, n: u32) -> String {
    let heading = format!("\n## {n}. ");
    let Some(start) = doc.find(&heading) else { return String::new() };
    let rest = &doc[start + 1..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// The text between `start` (a heading prefix) and the next `\n## ` after it (empty when
/// absent). The end search skips the first byte, so a `start` that itself begins with `\n## `
/// does not end at offset 0.
fn from_heading(doc: &str, start: &str) -> String {
    let Some(at) = doc.find(start) else { return String::new() };
    let rest = &doc[at..];
    let end = rest[1..].find("\n## ").map_or(rest.len(), |e| e + 1);
    rest[..end].to_string()
}

fn ticked(word: &str) -> String {
    format!("`{word}`")
}

/// Fenced blocks as `(tag, body lines)`. Only three-backtick fences open a block; a `~~~` line
/// is caught by `nothing_escapes_the_docs_test`.
fn fences(doc: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut open: Option<(String, Vec<String>)> = None;
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if let Some(tag) = trimmed.strip_prefix("```") {
            match open.take() {
                Some(block) => out.push(block),
                None => open = Some((tag.trim().to_string(), Vec::new())),
            }
        } else if let Some((_, body)) = open.as_mut() {
            body.push(line.to_string());
        }
    }
    out
}

fn assert_none(what: &str, missing: Vec<String>) {
    assert!(missing.is_empty(), "{what}:\n  {}", missing.join("\n  "));
}

/// The class word and exit code of `class_of(code)`, as the guide's table writes them.
fn class_cell(code: &str) -> String {
    let class = class_of(code);
    let word = match class {
        ErrorClass::Usage => "usage",
        ErrorClass::Refusal => "refusal",
        ErrorClass::Failure => "failure",
    };
    format!("{word} ({})", class.exit_code())
}

/// `(namespace, verb, heading form)` for every built `pane` and `profile` verb: `holler NS VERB`
/// plus, from the clap tree, each required positional's placeholder and each required flag with
/// its placeholder. The placeholder is the argument's first `value_name`, else its id upper-cased.
/// `pane import`'s `--herdr-session` is optional in clap and required at run time
/// (`pane/import.rs`), so it is added explicitly.
fn heading_forms() -> Vec<(String, String, String)> {
    let cli = Cli::command();
    let mut out = Vec::new();
    for ns in ["pane", "profile"] {
        let cmd = cli.find_subcommand(ns).unwrap_or_else(|| panic!("no `{ns}` subcommand"));
        for verb in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
            let name = verb.get_name();
            let placeholder = |a: &clap::Arg| {
                a.get_value_names()
                    .and_then(|v| v.first().map(|s| s.to_string()))
                    .unwrap_or_else(|| a.get_id().as_str().to_uppercase())
            };
            let mut form = format!("holler {ns} {name}");
            let required = |a: &&clap::Arg| a.is_required_set() && !a.is_global_set();
            for a in verb.get_arguments().filter(|a| a.is_positional()).filter(required) {
                form.push_str(&format!(" {}", placeholder(a)));
            }
            for a in verb.get_arguments().filter(|a| !a.is_positional()) {
                let long = a.get_long().unwrap_or_default();
                let runtime_required = ns == "pane" && name == "import" && long == "herdr-session";
                if (a.is_required_set() && !a.is_global_set()) || runtime_required {
                    form.push_str(&format!(" --{long} {}", placeholder(a)));
                }
            }
            out.push((ns.to_string(), name.to_string(), form));
        }
    }
    out
}

/// Acceptance 1: every verb is named in a span or heading.
#[test]
fn every_verb_is_named_in_a_span_or_heading() {
    let doc = guide();
    let mut missing = Vec::new();
    let cli = Cli::command();
    for ns in ["pane", "profile"] {
        let cmd = cli.find_subcommand(ns).unwrap();
        for verb in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
            let span = format!("`holler {ns} {}", verb.get_name());
            if ![" ", "`"].iter().any(|end| doc.contains(&format!("{span}{end}"))) {
                missing.push(format!("no span or heading starts {span}"));
            }
        }
    }
    assert_none("docs/pane-control.md, the verbs", missing);
}

/// Acceptance 2: section 5 names every doctor finding kind.
#[test]
fn section_5_names_every_finding_kind() {
    let s5 = section(&guide(), 5);
    let missing = FindingKind::ALL
        .iter()
        .map(|k| ticked(k.code()))
        .filter(|t| !s5.contains(t))
        .collect();
    assert_none("section 5 (doctor) does not name these finding kinds", missing);
}

/// Acceptance 3: section 9 has a row per closed error code with its class from `class_of`, an
/// exit-code row per class and for 0, and a code that is also a finding kind is in sections 5
/// and 9.
#[test]
fn section_9_has_a_classed_row_per_error_code() {
    let doc = guide();
    let (s5, s9) = (section(&doc, 5), section(&doc, 9));
    let mut missing = Vec::new();
    for code in ALL_CODES {
        let row = format!("| `{code}` |");
        let cell = class_cell(code);
        if !s9.lines().any(|l| l.starts_with(&row) && l.contains(&cell)) {
            missing.push(format!("no row starting {row} with `{cell}`"));
        }
    }
    let exits = [ErrorClass::Usage, ErrorClass::Refusal, ErrorClass::Failure].map(|c| c.exit_code());
    for exit in [0].into_iter().chain(exits) {
        if !s9.lines().any(|l| l.starts_with(&format!("| {exit} |"))) {
            missing.push(format!("no exit-code row starting `| {exit} |`"));
        }
    }
    let both: Vec<&str> =
        FindingKind::ALL.iter().map(|k| k.code()).filter(|c| ALL_CODES.contains(c)).collect();
    assert!(!both.is_empty(), "no code is both a finding kind and an error code; revisit this test");
    for code in both {
        for (n, text) in [(5, &s5), (9, &s9)] {
            if !text.contains(&ticked(code)) {
                missing.push(format!("`{code}` (a finding kind and an error code) is not in section {n}"));
            }
        }
    }
    assert_none("section 9 (JSON output reference)", missing);
}

/// Acceptance 4: section 4 names every column of `pane list`.
#[test]
fn section_4_names_every_column() {
    let s4 = section(&guide(), 4);
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let missing = COLUMNS
        .iter()
        .filter(|col| {
            !s4.match_indices(*col).any(|(i, _)| {
                let before = s4[..i].chars().next_back().is_none_or(|c| !is_word(c));
                let after = s4[i + col.len()..].chars().next().is_none_or(|c| !is_word(c));
                before && after
            })
        })
        .map(|c| c.to_string())
        .collect();
    assert_none("section 4 (Reading the table) does not name these columns", missing);
}

/// Acceptance 5: section 7's `text` block of grid examples is true of `GridPos::parse`.
#[test]
fn section_7_grid_examples_parse_as_stated() {
    let s7 = section(&guide(), 7);
    let lines: Vec<String> = fences(&s7)
        .into_iter()
        .filter(|(tag, _)| tag == "text")
        .flat_map(|(_, body)| body)
        .filter(|l| l.starts_with("ok ") || l.starts_with("refused "))
        .collect();
    let mut missing = Vec::new();
    let mut refused_codes = Vec::new();
    for line in &lines {
        if let Some(form) = line.strip_prefix("ok ") {
            if let Err(e) = GridPos::parse(form) {
                missing.push(format!("`{line}`: refused with {}", e.code()));
            }
        } else if let Some((code, form)) = line["refused ".len()..].split_once(' ') {
            match GridPos::parse(form) {
                Ok(p) => missing.push(format!("`{line}`: parsed to r{}c{}", p.row, p.col)),
                Err(e) if e.code() != code => missing.push(format!("`{line}`: refused {}", e.code())),
                Err(_) => refused_codes.push(code.to_string()),
            }
        }
    }
    for form in ["r2c1", "c1r2", "2,1", "R2C1"] {
        if !lines.iter().any(|l| l == &format!("ok {form}")) {
            missing.push(format!("no `ok {form}` line"));
        } else if GridPos::parse(form).ok().map(|p| (p.row, p.col)) != Some((2, 1)) {
            missing.push(format!("`{form}` does not parse to row 2, column 1"));
        }
    }
    if !lines.iter().any(|l| l == "refused grid-ambiguous hj-c1r2") {
        missing.push("no `refused grid-ambiguous hj-c1r2` line".into());
    }
    for code in ["grid-ambiguous", "grid-out-of-range"] {
        if !refused_codes.iter().any(|c| c == code) {
            missing.push(format!("no true `refused {code} ...` line"));
        }
    }
    assert_none("section 7 (Grid notation), the example block", missing);
}

/// Acceptance 7 of #652, as #648 changes it: section 4 documents the roster's pane columns
/// (each of `roster_cmd::PANE_COLUMNS` as a whole word) and `holler roster --profile`; no line
/// of the guide still says the roster extension is later, and section 12 (what is later) no
/// longer lists #648.
#[test]
fn section_4_documents_the_roster_columns() {
    let doc = guide();
    let s4 = section(&doc, 4);
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut missing: Vec<String> = PANE_COLUMNS
        .iter()
        .filter(|col| {
            !s4.match_indices(*col).any(|(i, _)| {
                let before = s4[..i].chars().next_back().is_none_or(|c| !is_word(c));
                let after = s4[i + col.len()..].chars().next().is_none_or(|c| !is_word(c));
                before && after
            })
        })
        .map(|c| format!("section 4 does not name the roster column {c}"))
        .collect();
    if !s4.lines().any(|l| l.contains("holler roster") && l.contains("--profile")) {
        missing.push("no line of section 4 names `holler roster` and `--profile`".into());
    }
    for l in doc.lines().filter(|l| l.contains("holler roster") && l.contains("#648") && l.contains("later")) {
        missing.push(format!("a line still says the roster extension is later: {l}"));
    }
    for l in section(&doc, 12).lines().filter(|l| l.contains("#648")) {
        missing.push(format!("section 12 still lists #648: {l}"));
    }
    assert_none("docs/pane-control.md, the roster (#648)", missing);
}

/// Acceptance 8: the fallback and the registry boundary, in the guide, `docs/orchestrating.md`
/// and the head of `scripts/pane-restart`.
#[test]
fn rollback_and_boundary_are_stated() {
    let doc = guide();
    let (s1, s11) = (section(&doc, 1), section(&doc, 11));
    let orch = read("docs/orchestrating.md");
    let script = read("scripts/pane-restart");
    let head: String = script.lines().take(10).collect::<Vec<_>>().join("\n");
    let mut missing = Vec::new();
    let mut need = |ok: bool, what: &str| {
        if !ok {
            missing.push(what.to_string());
        }
    };
    need(doc.contains("scripts/pane-restart"), "guide: `scripts/pane-restart`");
    need(doc.contains("until the 7-day sign-off"), "guide: `until the 7-day sign-off`");
    need(s11.contains("holler pane relaunch"), "section 11: `holler pane relaunch`");
    need(s11.contains("--fix"), "section 11: `--fix`");
    need(s1.contains("Claude Code"), "section 1: `Claude Code`");
    need(doc.contains("harness-not-opencode"), "guide: `harness-not-opencode`");
    need(
        doc.lines().any(|l| l.contains("Claude Code") && l.contains("scripts/pane-restart")),
        "guide: a line with both `Claude Code` and `scripts/pane-restart`",
    );
    for word in ["holler pane relaunch", "scripts/pane-restart", "Claude Code"] {
        need(orch.contains(word), &format!("docs/orchestrating.md: `{word}`"));
    }
    need(
        !orch.contains("removed when #644 and #653 have merged"),
        "docs/orchestrating.md still says `removed when #644 and #653 have merged`",
    );
    need(head.contains("until the 7-day sign-off"), "scripts/pane-restart head: `until the 7-day sign-off`");
    need(!head.contains("A stop-gap"), "scripts/pane-restart head still says `A stop-gap`");
    assert_none("the rollback and the registry boundary", missing);
}

/// Acceptance 9: section 10 names the files the registry replaces and the launcher table.
#[test]
fn section_10_names_the_replaced_files_and_the_launcher_table() {
    let s10 = section(&guide(), 10);
    let words = [
        "sessions.toml",
        "last-session",
        "owned-sessions",
        "pane.env",
        "launcher.toml",
        "--launcher",
        "source-missing",
        "ADR-0021.md",
    ];
    let missing = words.iter().filter(|w| !s10.contains(*w)).map(|w| w.to_string()).collect();
    assert_none("section 10 (Migration and cutover) does not name", missing);
}

/// Acceptance 10: `docs/README.md` links the guide under "Guidance".
#[test]
fn readme_indexes_the_guide_under_guidance() {
    let guidance = from_heading(&read("docs/README.md"), "\n## Guidance");
    assert!(!guidance.is_empty(), "docs/README.md has no `## Guidance` section");
    assert!(guidance.contains("](pane-control.md)"), "docs/README.md's Guidance does not link pane-control.md");
}

/// Acceptance 11: the twelve numbered `##` headings, in order, and no other.
#[test]
fn guide_has_the_twelve_numbered_headings_in_order() {
    let doc = guide();
    let headings: Vec<&str> = doc.lines().filter(|l| l.starts_with("## ")).collect();
    let numbers: Vec<Option<u32>> = headings
        .iter()
        .map(|h| h["## ".len()..].split_once(". ").and_then(|(n, _)| n.parse().ok()))
        .collect();
    let expected: Vec<Option<u32>> = (1..=12).map(Some).collect();
    assert_eq!(numbers, expected, "the guide's `## ` headings are {headings:?}");
}

/// Acceptance 12: no command hides from `docs_cli_test` in a `~~~` fence or an unscanned tag,
/// and the guide has commands for it to scan.
#[test]
fn nothing_escapes_the_docs_test() {
    let doc = guide();
    let mut missing: Vec<String> = doc
        .lines()
        .filter(|l| l.trim_start().starts_with("~~~"))
        .map(|l| format!("a `~~~` fence line: {l}"))
        .collect();
    let blocks = fences(&doc);
    for (tag, body) in &blocks {
        let has_command = body.iter().any(|l| l.trim_start().starts_with("holler "));
        if has_command && !SCANNED_TAGS.contains(&tag.as_str()) {
            missing.push(format!("a `{tag}` fence holds a `holler ` command docs_cli_test never parses"));
        }
    }
    let scanned_commands = blocks
        .iter()
        .filter(|(tag, _)| SCANNED_TAGS.contains(&tag.as_str()))
        .flat_map(|(_, body)| body)
        .filter(|l| l.trim_start().starts_with("holler "))
        .count();
    if scanned_commands == 0 {
        missing.push("no scanned fence holds a `holler ` command example".into());
    }
    assert_none("docs/pane-control.md, the fences", missing);
}

/// Acceptance 13: every envelope example in section 9 is a true envelope, and there is at least
/// one success and one failure.
#[test]
fn section_9_envelope_examples_are_true_envelopes() {
    let s9 = section(&guide(), 9);
    let mut missing = Vec::new();
    let (mut oks, mut fails) = (0, 0);
    for (_, body) in fences(&s9).into_iter().filter(|(tag, _)| tag == "json") {
        let block = body.join("\n");
        if !block.trim_start().starts_with("{\"schema_version\"") {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(&block) {
            Ok(v) => v,
            Err(e) => {
                missing.push(format!("not JSON ({e}): {block}"));
                continue;
            }
        };
        let exit = match value["error"]["code"].as_str() {
            Some(code) if value["ok"] == false => {
                fails += 1;
                class_of(code).exit_code()
            }
            _ => {
                oks += 1;
                0
            }
        };
        if let Err(fault) = check_envelope(&block, exit) {
            missing.push(format!("{fault} (exit {exit}): {block}"));
        }
    }
    if oks == 0 || fails == 0 {
        missing.push(format!("{oks} success and {fails} failure envelope examples; need at least one of each"));
    }
    assert_none("section 9, the envelope examples", missing);
}

/// Acceptance 14: every import finding and `import-blocked` is named in section 3 or 10.
#[test]
fn import_findings_are_named_in_section_3_or_10() {
    let doc = guide();
    let text = section(&doc, 3) + &section(&doc, 10);
    let missing = ImportFinding::ALL
        .iter()
        .map(|f| f.code())
        .chain([IMPORT_BLOCKED.as_str()])
        .map(ticked)
        .filter(|t| !text.contains(t))
        .collect();
    assert_none("sections 3 and 10 do not name these import codes", missing);
}

/// Acceptance 15: the `profile apply` reference (its subsection of section 3, and section 6)
/// names every outcome and action word, `detail` and apply's two open codes.
#[test]
fn profile_apply_reference_names_its_words() {
    let doc = guide();
    let reference = from_heading(&section(&doc, 3), "### `holler profile apply") + &section(&doc, 6);
    let words = APPLY_OUTCOMES
        .iter()
        .chain(APPLY_ACTIONS.iter())
        .chain(["detail", "orchestrator-not-relaunched", "spec-not-launchable"].iter());
    let missing = words.map(|w| ticked(w)).filter(|t| !reference.contains(t)).collect();
    assert_none("the `profile apply` reference does not name", missing);
}

/// Acceptance 16: an `## [Unreleased]` CHANGELOG line links #652 and names the guide.
#[test]
fn changelog_unreleased_names_the_guide() {
    let changelog = read("CHANGELOG.md");
    let unreleased = from_heading(&changelog, "## [Unreleased]");
    let unreleased = unreleased.split("\n## [").next().unwrap_or_default();
    assert!(
        unreleased.lines().any(|l| l.contains("/issues/652") && l.contains("pane-control.md")),
        "CHANGELOG.md's [Unreleased] has no line linking /issues/652 and naming pane-control.md"
    );
}

/// Acceptance 17: the content the audit found missing, and section 3's heading per built verb,
/// computed from the clap tree's required arguments. Sections 2 and 10 also say how `pane import`
/// answers a `--herdr-session` other than the adapter's (every pane `pane-unlabelled`, brief R2 as
/// re-cut after #649), and section 9 names #832's open code `pane-unmanaged` among the open codes.
#[test]
fn audit_content_and_section_3_headings_are_present() {
    let doc = guide();
    let (s2, s3, s5, s10) = (section(&doc, 2), section(&doc, 3), section(&doc, 5), section(&doc, 10));
    let s9 = section(&doc, 9);
    let mut missing = Vec::new();
    let unmanaged = ticked(PANE_UNMANAGED.as_str());
    let wanted: [(&str, &String, &[&str]); 4] = [
        ("2", &s2, &["agent-key-invalid", "agent-unusable", "--herdr-session", "`pane-unlabelled`"]),
        ("5", &s5, &["not-fixable"]),
        ("9", &s9, &[unmanaged.as_str()]),
        (
            "10",
            &s10,
            &["profile-exists", "already-imported", "--herdr-session", "launcher.toml", "source-missing", "`pane-unlabelled`"],
        ),
    ];
    for (n, text, words) in wanted {
        for w in words.iter().filter(|w| !text.contains(*w)) {
            missing.push(format!("section {n}: `{w}`"));
        }
    }
    for (_, _, form) in heading_forms() {
        let heading = format!("### `{form}`");
        if !s3.lines().any(|l| l.starts_with(&heading)) {
            missing.push(format!("section 3: no heading {heading}"));
        }
    }
    assert_none("the audit's content", missing);
}

/// #835 (handoff-A W4): section 3's `pane import` subsection names every long flag of the verb,
/// read from the clap tree, so a flag added to the import without a docs line (as
/// `--adopt-claude-code`) fails here. The subsection runs from its `### `holler pane import`
/// heading to the next heading; global flags (`--format`, ...) are documented once, not per verb.
/// Only this verb is pinned: the other verbs' subsections give a synopsis, not every flag.
#[test]
fn section_3_names_every_flag_of_pane_import() {
    let s3 = section(&guide(), 3);
    let sub = from_heading(&s3, "### `holler pane import");
    let end = sub[1..].find("\n#").map_or(sub.len(), |e| e + 1);
    let body = &sub[..end];
    assert!(!body.is_empty(), "section 3 has no `### `holler pane import` subsection");
    let cli = Cli::command();
    let verb = cli.find_subcommand("pane").and_then(|c| c.find_subcommand("import")).unwrap();
    let flags = verb.get_arguments().filter(|a| !a.is_positional() && !a.is_global_set() && !a.is_hide_set());
    let missing = flags
        .filter_map(|a| a.get_long().map(|l| format!("--{l}")))
        .filter(|flag| {
            !body.match_indices(flag.as_str()).any(|(i, _)| {
                body[i + flag.len()..].chars().next().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '-'))
            })
        })
        .collect();
    assert_none("section 3's `pane import` subsection does not name these flags", missing);
}

/// The lines of `doc` that break #665's rule: each of `BUILT_BY_665` is named on at least one line,
/// and no line naming one of them says PROPOSED.
fn built_lines_marked_proposed(doc: &str) -> Vec<String> {
    let mut missing = Vec::new();
    for word in BUILT_BY_665 {
        let lines: Vec<&str> = doc.lines().filter(|l| l.contains(word)).collect();
        if lines.is_empty() {
            missing.push(format!("no line names `{word}`"));
        }
        for l in lines.iter().filter(|l| l.contains("PROPOSED")) {
            missing.push(format!("a line names `{word}` and still says PROPOSED: {l}"));
        }
    }
    missing
}

/// #665 (D11): the three profile verbs and `profile-drift` are built, not proposed. Section 3 has their
/// headings and keeps `pane import` and `profile import` as two headings, section 5's `profile-drift`
/// row names no remedy, and section 12 no longer lists #665 as later work.
#[test]
fn profile_io_and_drift_are_built_not_proposed() {
    let doc = guide();
    let (s3, s5, s12) = (section(&doc, 3), section(&doc, 5), section(&doc, 12));
    let mut missing = built_lines_marked_proposed(&doc);
    for heading in PROFILE_IO_HEADINGS {
        if !s3.lines().any(|l| l.starts_with(heading)) {
            missing.push(format!("section 3: no heading {heading}"));
        }
    }
    if !s3.lines().any(|l| l.starts_with(PANE_IMPORT_HEADING)) {
        missing.push("section 3: `pane import` lost its own heading".into());
    }
    let row = s5.lines().find(|l| l.starts_with("| `profile-drift` |"));
    if !row.is_some_and(|l| l.trim_end().ends_with("none |")) {
        missing.push(format!("section 5: the `profile-drift` row must end `none |`: {row:?}"));
    }
    if s12.contains("#665") {
        missing.push("section 12 still lists #665".into());
    }
    assert_none("docs/pane-control.md, #665's verbs and finding", missing);
}

/// The negative control of the line rule above: a guide that still says PROPOSED on a line naming a built
/// verb, or that never names one, fails it.
#[test]
fn the_built_not_proposed_rule_fails_a_proposed_line() {
    let good = "`holler profile rename` x\n`holler profile export` x\n`holler profile import` x\n`profile-drift` x\n";
    assert_eq!(built_lines_marked_proposed(good), Vec::<String>::new());
    let proposed = good.replace("`holler profile export` x", "`holler profile export` (PROPOSED)");
    assert_eq!(built_lines_marked_proposed(&proposed).len(), 1, "a PROPOSED line fails");
    let unnamed = good.replace("`profile-drift` x", "");
    assert_eq!(built_lines_marked_proposed(&unnamed).len(), 1, "an unnamed item fails");
}

/// `text` as one line of words: each line's comment or quote marker (`//!`, `///`, `//`, `>`)
/// dropped, then every run of whitespace one space, so a sentence wrapped across lines is found.
fn normalised(text: &str) -> String {
    let lines = text.lines().map(|line| {
        let line = line.trim_start();
        ["//!", "///", "//", ">"].iter().fold(line, |l, marker| l.strip_prefix(marker).unwrap_or(l))
    });
    lines.collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// #839 Acceptance 17: the ADR, the guide, the CHANGELOG and the module docs carry none of the
/// sentences DRIVEN as built replaced (whitespace normalised; the guide's forms case-insensitive),
/// section 9 of the guide names the three new open codes, and the CHANGELOG has the entry linking
/// #839 and #828.
#[test]
fn the_sentences_driven_replaced_are_gone_and_the_entry_is_written() {
    let files = [
        "docs/adr/ADR-0021.md",
        "docs/pane-control.md",
        "CHANGELOG.md",
        "crates/holler-pane/src/tx_switch.rs",
        "crates/holler-pane/src/reconcile.rs",
        "crates/holler-pane/src/ports.rs",
        "crates/holler-cli/src/pane/reset.rs",
        "crates/holler-cli/src/say_cmd.rs",
        "crates/holler-cli/src/roster_cmd.rs",
        "crates/holler-adapter-opencode/src/prompt.rs",
        "crates/holler-proto/src/methods.rs",
    ];
    let exact = [
        "not use `reset --first` on a parked or held pane",
        "whether it moves to the hub's one prompt path is #828's",
        "until the cutover wires DRIVEN",
        "the closed 22-row catalog",
    ];
    let folded = [
        "not use `--first` on a parked or held pane",
        "port observes DRIVEN before #649",
        "`reset --first` is not used on a parked or held pane",
    ];
    let mut found = Vec::new();
    for file in files {
        let text = normalised(&read(file));
        assert!(!text.is_empty(), "{file} is read");
        let lower = text.to_lowercase();
        found.extend(exact.iter().filter(|s| text.contains(*s)).map(|s| format!("{file}: {s}")));
        let hits = folded.iter().filter(|s| lower.contains(&s.to_lowercase()));
        found.extend(hits.map(|s| format!("{file}: {s}")));
    }
    if normalised(&read("docs/adr/ADR-0021.md")).contains("stays in force, unchanged") {
        found.push("docs/adr/ADR-0021.md: stays in force, unchanged".to_owned());
    }
    assert_none("old sentences that #839 replaced", found);

    let errors = section(&guide(), 9);
    let missing = ["pane-not-driven", "session-held", "session-driven-elsewhere"]
        .into_iter()
        .filter(|code| !errors.contains(&ticked(code)))
        .map(str::to_owned)
        .collect();
    assert_none("section 9 lacks the open codes of #839", missing);

    let changelog = read("CHANGELOG.md");
    let unreleased = from_heading(&changelog, "## [Unreleased]");
    let unreleased = unreleased.split("\n## [").next().unwrap_or_default();
    assert!(
        unreleased.lines().any(|l| l.contains("/issues/839") && l.contains("/issues/828")),
        "CHANGELOG.md's [Unreleased] has no entry linking /issues/839 and /issues/828"
    );
}
