//! #838: the wizard asks which Herdr session to build in, epic #726.
//!
//! Text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`, plus one
//! run of `instance.sh` on a temp config. Nothing here touches a host or a live Herdr session.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #838

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text from `start` up to (not including) the next `end`, or to the end.
fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |i| i + start.len());
    rest[..to].to_owned()
}

fn skill() -> String {
    flat("agent-skills/herdr-setup/SKILL.md")
}

fn stage1() -> String {
    between(&skill(), "## Stage 1 ", "## Stage 2 ")
}

/// The Herdr-session question: from its marker to the workspace-name question.
fn question() -> String {
    let s1 = stage1();
    between(
        &s1,
        "the Herdr session question",
        "Suggest **two dictionary words",
    )
}

fn has_all(what: &str, text: &str, needles: &[&str]) {
    for n in needles {
        assert!(text.contains(n), "{what} lacks `{n}`");
    }
}

#[test]
fn the_session_question_sits_between_the_menu_and_the_workspace_name() {
    let s1 = stage1();
    let at = |m: &str| s1.find(m).unwrap_or_else(|| panic!("Stage 1 lacks `{m}`"));
    let menu = at("Present this as an actual choice");
    let ask = at("the Herdr session question");
    let name = at("Suggest **two dictionary words");
    assert!(menu < ask, "session question must follow the scenario menu");
    assert!(
        ask < name,
        "session question must precede the workspace name"
    );
}

#[test]
fn the_question_offers_the_default_and_a_new_named_session_only() {
    let q = question();
    has_all(
        "session question",
        &q,
        &[
            "The default, unnamed session",
            "A new named session",
            "writes no `[instance]` table",
            "an existing config is not re-asked",
            "default (unnamed)",
        ],
    );
    assert!(
        q.contains("herdr session list") && q.contains("running or stopped"),
        "the refusal must cover running and stopped sessions: {q}"
    );
}

#[test]
fn the_named_branch_states_the_pattern_the_refusal_and_the_proposed_values() {
    let q = question();
    has_all(
        "named branch",
        &q,
        &[
            "^[a-z][a-z0-9-]{0,23}$",
            "refuse",
            "another name or the default",
            "state_dir = \"~/.holler-<name>\"",
            "hub_port = 41808",
            "serve_https_port = 8443",
            "backend_port_base = 47101",
            "herdr_session",
            "after the bare top-level keys and before `[[orchestrator]]`",
            "instance.sh",
        ],
    );
}

#[test]
fn the_named_branch_states_the_outside_a_pane_rule_and_state_dir_env() {
    let q = question();
    has_all(
        "named branch",
        &q,
        &["outside any Herdr pane", "HOLLER_STATE_DIR=<state_dir>"],
    );
}

#[test]
fn stage1_verify_reports_the_targeted_session() {
    let v = between(&stage1(), "**Verify:**", "## Stage 2 ");
    has_all(
        "Stage 1 Verify",
        &v,
        &["Herdr session", "default (unnamed)"],
    );
}

#[test]
fn stage3_plan_names_the_herdr_session_in_both_cases() {
    let s3 = between(&skill(), "## Stage 3 ", "## Stage 4 ");
    has_all(
        "Stage 3",
        &s3,
        &[
            "Herdr session: default (unnamed)",
            "Herdr session: `<name>` (new, own state directory and ports)",
        ],
    );
}

#[test]
fn stage0_session_row_keeps_its_prefix_and_lists_every_session_with_status() {
    let s = skill();
    let row = between(&s, "| Herdr server state", " | ");
    assert!(row.starts_with("| Herdr server state"));
    let full = between(&s, "| Herdr server state", "| A Holler hub");
    has_all(
        "Stage 0 row",
        &full,
        &[
            "herdr session list",
            "every session",
            "its status (running or stopped)",
        ],
    );
}

#[test]
fn the_docs_document_the_question() {
    let docs = flat("docs/setup-wizard.md");
    let section = between(
        &docs,
        "## Running an instance beside another",
        "## Instance state and the ledger",
    );
    let sub = between(&section, "### Choosing the Herdr session", "\u{0}");
    has_all(
        "docs subsection",
        &sub,
        &[
            "The default, unnamed session",
            "A new named session",
            "~/.holler-<name>",
            "41808",
            "8443",
            "47101",
            "herdr session list",
            "outside any Herdr pane",
            "HOLLER_STATE_DIR=<state_dir>",
        ],
    );
}

#[test]
fn the_proposed_values_for_a_named_session_pass_instance_sh() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("sessions.toml");
    std::fs::write(
        &cfg,
        "layout = [[\"o1\"]]\n\n[instance]\nname = \"work\"\nhub_port = 41808\n\
         serve_https_port = 8443\nstate_dir = \"~/.holler-work\"\nherdr_session = \"work\"\n\
         backend_port_base = 47101\n\n[[orchestrator]]\nname = \"o1\"\ndir = \"~/Projects/holler\"\ncmd = \"claude\"\n",
    )
    .unwrap();
    let out = Command::new("bash")
        .arg(root().join("agent-skills/herdr-setup/lib/instance.sh"))
        .arg(&cfg)
        .env("HOME", "/home/test")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains(
            "instance: name=work prefix=work hub_port=41808 serve_https_port=8443 \
             state_dir=~/.holler-work herdr_session=work backend_port_base=47101"
        ),
        "{text}"
    );
}
