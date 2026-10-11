//! #811: the final step, "Basic Herdr commands" (epic #769). It follows Stage 10, lists the
//! handful of Herdr bindings a new user needs, names a restored workspace beside the new one,
//! warns about teardown, and changes nothing.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #811

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// Whitespace folded to one space, for phrases that wrap.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("section start");
    let rest = &text[from + 3..];
    rest[..rest.find(end).unwrap_or(rest.len())].to_owned()
}

fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
}

fn stage10() -> String {
    flat(&between(&skill(), "## Stage 10 ", "\n## Stage 11 "))
}

fn stage11() -> String {
    flat(&between(&skill(), "## Stage 11 ", "\n## If a stage fails"))
}

#[test]
fn the_final_step_follows_stage_10() {
    let s = skill();
    let ten = s.find("\n## Stage 10 ").unwrap();
    let eleven = s
        .find("\n## Stage 11 (final): Basic Herdr commands")
        .unwrap();
    let fails = s.find("\n## If a stage fails").unwrap();
    assert!(
        ten < eleven && eleven < fails,
        "Stage 11 sits between Stage 10 and the failure list"
    );
}

#[test]
fn every_command_in_the_table_is_there() {
    let s = stage11();
    for cmd in [
        "`herdr`",
        "`herdr --session <name>`",
        "`ctrl+b`",
        "`prefix+b`",
        "`»`",
        "`prefix+w`",
        "`prefix+shift+n`",
        "`prefix+shift+w`",
        "`prefix+shift+d`",
        "`herdr workspace close <id>`",
        "`prefix+v`",
        "`prefix+minus`",
        "`prefix+h/j/k/l`",
        "`prefix+z`",
        "`prefix+x`",
        "`prefix+q`",
        "`prefix+?`",
    ] {
        assert!(s.contains(cmd), "the table lists {cmd}");
    }
}

#[test]
fn the_step_says_to_confirm_a_differing_binding_with_the_help_screen() {
    let s = stage11();
    assert!(s.contains("differs"), "a differing binding is anticipated");
    assert!(s.contains("`prefix+?`"));
}

#[test]
fn the_checklist_has_the_extra_last_line() {
    let s = skill();
    let list = between(&s, "Keep a running status checklist", "## Stage 0");
    let last = list
        .lines()
        .rfind(|l| {
            l.split_once(' ')
                .is_some_and(|(_, r)| r.starts_with(|c: char| c.is_ascii_digit()))
        })
        .unwrap();
    assert!(
        last.contains("11. Basic Herdr commands"),
        "last checklist line: {last}"
    );
    assert!(
        list.contains("10. Verify end-to-end"),
        "Stage 10's line stays"
    );
}

#[test]
fn stage_10_points_at_the_final_step_instead_of_repeating_commands() {
    let s = stage10();
    assert!(s.contains("Stage 11"), "Stage 10 points at Stage 11");
    assert!(
        !s.contains("visible-in-TUI workspace switcher"),
        "the switcher hint moved"
    );
    assert!(
        !s.contains("herdr --session <herdr_session>"),
        "the attach variants moved"
    );
}

#[test]
fn a_restored_workspace_is_named_and_never_closed_for_the_user() {
    let s = stage11();
    assert!(
        s.contains("did **not** create"),
        "workspaces the wizard did not create"
    );
    assert!(s.contains("restored"), "a restored saved session");
    assert!(
        s.contains("id and pane count"),
        "named with its id and pane count"
    );
    assert!(
        s.to_lowercase().contains("never close it for them"),
        "the wizard does not close it"
    );
}

#[test]
fn the_teardown_command_is_named_with_its_restored_workspace_warning() {
    let s = stage11();
    assert!(s.contains("`stop-owned.sh teardown`"));
    assert!(s.contains("closes the restored workspace too"));
}

#[test]
fn the_step_changes_nothing() {
    let s = stage11();
    assert!(s.contains("changes nothing"));
    assert!(s.contains("output only"));
}

#[test]
fn the_docs_have_one_section_for_it() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(d.contains("## Basic Herdr commands"));
    assert!(d.contains("`prefix+?`"));
    assert!(d.contains("`prefix+shift+d`"));
}
