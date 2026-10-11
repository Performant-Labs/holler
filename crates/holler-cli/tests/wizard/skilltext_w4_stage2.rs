//! #805: Stage 2 text for local-only runs (zero sessions): skip the remote checks, epic #769.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #805

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    read(rel).split_whitespace().collect::<Vec<_>>().join(" ")
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

fn stage2() -> String {
    between(
        &flat("agent-skills/herdr-setup/SKILL.md"),
        "## Stage 2 ",
        "## Stage 3 ",
    )
}

/// The no-sessions paragraph: from its opening words to the first SSH reachability `Do`.
fn paragraph() -> String {
    between(
        &stage2(),
        "**With no sessions (N=0),",
        "**Do (SSH reachability",
    )
}

fn has_all(what: &str, text: &str, needles: &[&str]) {
    for n in needles {
        assert!(text.contains(n), "{what} lacks `{n}`");
    }
}

#[test]
fn the_paragraph_sits_before_the_first_check_of_do() {
    let s2 = stage2();
    let para = s2.find("**With no sessions (N=0),").unwrap();
    let first = s2.find("**Do (SSH reachability").unwrap();
    assert!(para < first, "the paragraph must come before the ssh loop");
}

#[test]
fn it_names_every_skipped_remote_check() {
    has_all(
        "no-sessions paragraph",
        &paragraph(),
        &[
            "skip the ssh reachability loop",
            "the remote `which holler`",
            "the model-endpoint checks",
            "the remote state-directory",
            "ledger fetch",
            "the `tailscale status` grep of remote names",
        ],
    );
}

#[test]
fn it_names_every_check_that_still_runs() {
    has_all(
        "no-sessions paragraph",
        &paragraph(),
        &[
            "orchestrator `dir`/`cmd` checks",
            "the local `which holler`",
            "scoped `holler hub status`",
            "local inventory",
            "`collide` inputs",
            "Herdr checks",
        ],
    );
}

#[test]
fn it_reports_the_skips_as_skipped_not_failed() {
    has_all(
        "no-sessions paragraph",
        &paragraph(),
        &["Report each skipped check as skipped", "never as failed"],
    );
}

#[test]
fn an_absent_hub_host_skips_the_tailscale_status_check_with_the_reason() {
    let s2 = stage2();
    has_all(
        "Stage 2",
        &s2,
        &[
            "When `hub_host` is absent",
            "skip the `tailscale status` check",
            "no `hub_host` and no remote host to look up",
        ],
    );
}

#[test]
fn verify_limits_the_remote_lines_to_one_or_more_sessions() {
    let s2 = stage2();
    let verify = between(&s2, "**Verify:**", "**If the provider block is missing");
    has_all(
        "Stage 2 Verify",
        &verify,
        &[
            "The remote lines below apply only when N>=1",
            "with N=0 only the orchestrator, local `holler` and Herdr lines apply",
        ],
    );
}

#[test]
fn gate_limits_the_remote_failures_to_one_or_more_sessions() {
    let s2 = stage2();
    let gate = between(&s2, "**Gate:**", "\n\n## ");
    has_all(
        "Stage 2 Gate",
        &gate,
        &[
            "remote host offline",
            "apply only when N>=1",
            "with N=0 none of the remote failures can occur",
        ],
    );
}

#[test]
fn the_guide_preflight_paragraph_says_local_only_runs_skip_the_remote_checks() {
    let docs = flat("docs/setup-wizard.md");
    assert!(
        docs.contains("With no sessions (N=0), preflight skips every remote check"),
        "docs/setup-wizard.md lacks the local-only preflight sentence"
    );
}
