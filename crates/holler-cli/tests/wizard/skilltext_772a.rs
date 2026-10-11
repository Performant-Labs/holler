//! #772: the skill text for a local-only run (zero `[[session]]` tables), epic #769.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #772

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// The skill with every run of whitespace folded to one space, for phrases that wrap.
fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
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

fn stage(n: u32) -> String {
    let next = if n == 10 {
        "## If a stage fails".to_owned()
    } else {
        format!("## Stage {} ", n + 1)
    };
    between(&skill(), &format!("## Stage {n} "), &next)
}

#[test]
fn stage_one_accepts_zero_sessions_when_an_orchestrator_exists() {
    let s = stage(1);
    assert!(
        s.contains("zero or more `[[session]]` tables"),
        "Verify must accept zero sessions"
    );
    assert!(
        !s.contains("at least one `[[session]]`"),
        "Verify must no longer require a session"
    );
    assert!(s.contains("at least one `[[orchestrator]]`"));
    assert!(
        s.contains("N=0"),
        "the summary states N=0 plainly for a local-only config"
    );
    assert!(
        s.contains("no distinct remote hosts") || s.contains("0 distinct hosts"),
        "the distinct-host summary states none"
    );
    assert!(
        s.contains("every `[[orchestrator]]` name must appear in `layout` exactly once"),
        "the layout cross-check stays"
    );
    assert!(
        s.to_lowercase().contains("with no sessions") && s.contains("`instance.sh`"),
        "instance.sh with no sessions is described"
    );
}

#[test]
fn stage_one_gate_does_not_stop_on_an_empty_session_list() {
    let s = stage(1);
    assert!(
        s.contains("zero sessions is not a failure"),
        "the Gate says a local-only config passes"
    );
}

#[test]
fn stages_three_four_five_seven_and_nine_say_skipped_with_no_sessions() {
    for n in [3, 4, 5, 7, 9] {
        let s = stage(n);
        assert!(
            s.to_lowercase().contains("with no sessions"),
            "Stage {n} names the zero-session case"
        );
        assert!(
            s.contains("skipped"),
            "Stage {n} says the stage is reported as skipped"
        );
        assert!(
            s.contains("never as failed"),
            "Stage {n} says it is never reported as failed"
        );
    }
}

#[test]
fn stage_three_plan_says_which_stages_are_skipped() {
    let s = stage(3);
    assert!(
        s.contains("Stages 4, 5, 7 and 9 are skipped"),
        "the plan names the skipped stages"
    );
}

#[test]
fn the_checklist_shows_skipped_stages_with_a_distinct_symbol() {
    let text = skill();
    let list = between(&text, "Keep a running status checklist", "## Stage 0");
    assert!(
        list.contains("⏭️ skipped"),
        "the legend defines the skipped symbol"
    );
    for sym in ["✅", "🔄", "⬜", "❌"] {
        assert!(
            !"⏭️".contains(sym),
            "the skipped symbol differs from {sym}"
        );
    }
}

#[test]
fn stage_six_skips_tailscale_serve_with_no_remote_host_but_starts_the_hub() {
    let s = stage(6);
    assert!(
        s.contains("no remote host"),
        "Stage 6 names the no-remote-host case"
    );
    assert!(
        s.contains("skip the `tailscale serve` line"),
        "Stage 6 skips the serve line"
    );
    assert!(
        s.contains("the hub is still started"),
        "Stage 6 still starts the hub"
    );
    let plan = stage(3);
    assert!(
        plan.contains("`tailscale serve`") && plan.contains("no remote host"),
        "Stage 3 says the serve line is skipped"
    );
}

#[test]
fn stage_ten_verifies_the_hub_and_the_orchestrator_panes_not_sessions() {
    let s = stage(10);
    assert!(
        s.to_lowercase().contains("with no sessions") && s.contains("the hub answers"),
        "Stage 10 verifies the hub for a local-only run"
    );
    assert!(
        s.contains("does not look for sessions"),
        "Stage 10 does not look for sessions"
    );
    assert!(s.contains("every orchestrator pane is alive"));
}

#[test]
fn the_docs_and_readme_say_a_local_only_config_is_valid() {
    let docs = read("docs/setup-wizard.md");
    assert!(
        docs.contains("local-only") && docs.contains("zero `[[session]]`"),
        "docs name the local-only case"
    );
    let readme = read("README.md");
    let from = readme
        .find("## Set up a Herdr workspace with an agent")
        .unwrap();
    assert!(
        readme[from..].contains("local-only") || readme[from..].contains("orchestrators only"),
        "README wizard section names the local-only case"
    );
}
