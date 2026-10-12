//! #804: Stage 3/8 and teardown text. The base pane and the count belong to the new workspace,
//! one yes covers creating it, the briefing is name-free and not offered with no sessions, and
//! teardown warns about the restored workspace (epic #769).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #804

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// Whitespace folded to one space, for phrases that wrap.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text from `start` to the next `end` marker (or the end of the text).
fn section(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("section start");
    let rest = &text[from + 3..];
    rest[..rest.find(end).unwrap_or(rest.len())].to_owned()
}

fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
}

fn docs() -> String {
    flat(&read("docs/setup-wizard.md"))
}

fn stage3() -> String {
    flat(&section(&skill(), "## Stage 3 ", "\n## Stage 4 "))
}

/// #803: the panes are built in Stage 7.
fn stage7() -> String {
    flat(&section(&skill(), "## Stage 7 ", "\n## Stage 8 "))
}

fn teardown() -> String {
    flat(&section(
        &skill(),
        "- **Tearing the instance down:**",
        "\n- **A join fails",
    ))
}

#[test]
fn stage7_ask_is_covered_by_the_plan_yes() {
    let s = stage7();
    assert!(
        s.contains("the \"Proceed with this plan?\" yes covers creating it"),
        "{s}"
    );
    assert!(s.contains("a \"no\" means stop"), "{s}");
}

#[test]
fn stage3_plan_names_the_new_workspace_and_lists_restored_panes() {
    let s = stage3();
    assert!(
        s.contains("name the new workspace in this plan and list the restored panes beside it"),
        "{s}"
    );
    assert!(
        s.contains("the \"Proceed with this plan?\" yes covers creating it"),
        "{s}"
    );
}

#[test]
fn briefing_is_written_in_stage_7_and_is_name_free() {
    let text = skill();
    let from = text.find("## Holler self-status\n").expect("the briefing");
    let stage7 = text.find("## Stage 7 ").expect("stage 7");
    let stage8 = text.find("## Stage 8 ").expect("stage 8");
    assert!(
        stage7 < from && from < stage8,
        "#803 D10: the write moves to Stage 7"
    );
    let block = flat(&text[from..from + text[from..].find("```").unwrap()]);
    assert!(
        block.contains("You are an orchestrator in a Holler-driven"),
        "{block}"
    );
    assert!(!block.contains("<name>"), "{block}");
    assert!(block.contains("already sets `HOLLER_STATE_DIR`"), "{block}");
    assert!(!flat(&text).contains("Substitute that orchestrator's real `name`"));
    assert!(
        flat(&text).contains("orchestrators that share a `dir` share one file"),
        "why the text is name-free"
    );
}

#[test]
fn teardown_warns_about_the_restored_workspace_and_offers_to_leave_the_server() {
    for (name, t) in [("teardown entry", teardown()), ("Stage 3", stage3())] {
        assert!(
            t.contains("restored workspace"),
            "{name} names the restored workspace: {t}"
        );
        assert!(t.contains("auto-resumed agent"), "{name}: {t}");
        assert!(
            t.contains("leave the server running and stop only the hub"),
            "{name}: {t}"
        );
    }
    let t = teardown();
    assert!(t.contains("list those panes"), "{t}");
    assert!(t.contains("ask before"), "{t}");
}

#[test]
fn docs_carry_the_same_rules() {
    let d = docs();
    assert!(
        d.contains("the \"Proceed with this plan?\" yes covers creating it"),
        "one yes"
    );
    assert!(
        d.contains("leave the server running and stop only the hub"),
        "teardown"
    );
}
