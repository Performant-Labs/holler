//! #803: the skill, the docs and the README after the wizard builds panes through `holler pane`
//! and `holler profile apply` (D2 to D22, the brief's Acceptance 10 to 24, A's W2 to W12).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md`, `docs/setup-wizard.md`, the
//! README's wizard section, ADR-0021, `docs/pane-control.md`, `docs/README.md` and the CHANGELOG,
//! reusing `skilltext.rs`'s readers and its fence and section parsers. Nothing here runs a script.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803

use super::skilltext::{all_texts, code_lines, docs, flat, read, readme, section, skill};

/// Stage `n`'s text, from its heading to the next stage's (Stage 11 runs to "If a stage fails").
fn stage(n: u32) -> String {
    let next = if n == 11 {
        "\n## If a stage fails".to_owned()
    } else {
        format!("\n## Stage {} ", n + 1)
    };
    section(&skill(), &format!("## Stage {n} "), &next)
}

fn lower(text: &str) -> String {
    flat(text).to_lowercase()
}

/// The "Tearing the instance down" entry of "If a stage fails".
fn teardown() -> String {
    flat(&section(
        &skill(),
        "- **Tearing the instance down:**",
        "\n- **",
    ))
}

#[test]
fn no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme() {
    let banned = [
        "pane split",
        "pane run",
        "send-keys",
        "pane rename",
        "pane read",
        "workspace create",
        "workspace rename",
        "herdr.sh label",
        "opencode attach",
        "--hostname 0.0.0.0",
        "POST /session",
        "tailscale serve --bg",
    ];
    for (name, text) in all_texts() {
        // Flattened, so a phrase wrapped across a line break is still seen.
        let flat_text = flat(&text);
        for b in banned {
            assert!(!contains_phrase(&flat_text, b), "{name} still has `{b}`");
        }
        for line in code_lines(&text) {
            let ssh = line.split_whitespace().any(|w| w == "ssh");
            assert!(!ssh, "{name}: an ssh command: {line}");
        }
        for (at, _) in flat_text.match_indices("herdr.sh run ") {
            // The one sanctioned mention of another word: naming the call
            // the wizard never makes (`skilltext.rs`'s stop-path pin).
            if flat_text[..at].ends_with("never `") {
                continue;
            }
            let rest = &flat_text[at + "herdr.sh run ".len()..];
            let ok = ["status", "session list", "workspace list"]
                .iter()
                .any(|w| rest.starts_with(w));
            assert!(ok, "{name}: herdr.sh run {}", &rest[..rest.len().min(30)]);
        }
    }
}

/// `phrase` occurs in `text` as whole words: the character after it is not a
/// letter or digit, so `pane run` matches `pane run` but not `pane runs`.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(at, _)| {
        text[at + phrase.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
    })
}

#[test]
fn the_twelve_stages_keep_their_slots_and_four_to_nine_follow_the_hub_first_order() {
    let text = skill();
    let mut last = 0;
    for n in 0..=11 {
        let heading = format!("## Stage {n} ");
        assert_eq!(text.matches(&heading).count(), 1, "{heading} once");
        let at = text.find(&heading).unwrap();
        assert!(at > last, "{heading} out of order");
        last = at;
    }
    let names = [
        (4, "Bring up the hub"),
        (5, "Start the Herdr server"),
        (6, "Create the profile"),
        (7, "Build the panes"),
        (8, "Join and run the body"),
        (9, "Check the registry"),
    ];
    let how = flat(&section(
        &text,
        "Keep a running status checklist",
        "## Stage 0",
    ));
    for (n, name) in names {
        let heading = format!("\n## Stage {n} \u{2014} {name}");
        assert!(text.contains(&heading), "no `{heading}`");
        let row = format!("| {n} | {name} |");
        assert!(how.contains(&row), "the checklist lacks `{row}`");
    }
    assert!(flat(&docs()).contains("a 12-stage wizard (Stage 0 through Stage 11)"));
}

#[test]
fn the_title_and_frontmatter_no_longer_promise_remote_sessions() {
    let text = skill();
    let h1 = text.lines().find(|l| l.starts_with("# ")).unwrap();
    assert!(!h1.to_lowercase().contains("remote"), "{h1}");
    let front = lower(&text[..text.find(h1).unwrap()]);
    assert!(
        !front.contains("remote") && !front.contains("attach-mode"),
        "{front}"
    );
    let docs_h1 = docs()
        .lines()
        .find(|l| l.starts_with("# "))
        .unwrap()
        .to_owned();
    assert!(!docs_h1.contains("attach-mode"), "{docs_h1}");
}

#[test]
fn every_pane_call_runs_through_the_door_with_the_instances_literals() {
    let one = lower(&skill());
    for lit in [
        "wizard_herdr_workspaces",
        "wizard_herdr_socket",
        "wizard_tmux_socket",
        "panes.sh env",
    ] {
        assert!(one.contains(lit), "the skill never names {lit}");
    }
    let door: Vec<String> = code_lines(&skill())
        .into_iter()
        .filter(|l| l.contains("herdr.sh holler"))
        .collect();
    assert!(door.len() >= 6, "{door:?}");
    for line in &door {
        assert!(
            line.contains("<herdr_env>"),
            "the literals are missing: {line}"
        );
    }
    for line in code_lines(&skill()).iter().filter(|l| l.contains("ls ")) {
        if line.contains("herdr.sh") {
            assert!(
                line.contains("panes.sh"),
                "Stage 0's file check (W6): {line}"
            );
        }
    }
}

#[test]
fn stage_zero_checks_tmux_opencode_claude_and_the_pane_verbs_and_nothing_remote() {
    let s = stage(0);
    for row in ["| `tmux`", "| `opencode`", "| `claude`"] {
        assert!(s.contains(row), "Stage 0 lacks the row `{row}`");
    }
    for cmd in [
        "command -v tmux",
        "command -v opencode",
        "command -v claude",
        "holler pane --help",
    ] {
        assert!(s.contains(cmd), "Stage 0 lacks `{cmd}`");
    }
    for gone in [
        "| `tailscale`",
        "| `ssh`",
        "tailscale status",
        "command -v ssh",
    ] {
        assert!(!s.contains(gone), "Stage 0 still has `{gone}`");
    }
    assert!(
        flat(&s).contains("not an alias"),
        "claude is an executable on PATH (E85)"
    );
}

#[test]
fn stage_one_asks_dirs_harnesses_the_orchestrator_and_the_model_once_and_refuses_old_configs() {
    let s = stage(1);
    let f = lower(&s);
    for need in [
        "harness",
        "orchestrator",
        "opencode.jsonc",
        "medium",
        "800000",
        "900000",
    ] {
        assert!(f.contains(need), "Stage 1 lacks `{need}`");
    }
    let code = code_lines(&s).join("\n");
    assert!(
        code.contains("panes.sh check"),
        "Stage 1's Verify runs panes.sh check"
    );
    assert!(
        code.contains("instance.sh"),
        "Stage 1's Verify runs instance.sh"
    );
    for need in [
        "[[orchestrator]]",
        "[[session]]",
        "conversion",
        "attach mode",
        "holler body",
    ] {
        assert!(
            f.contains(&need.to_lowercase()),
            "Stage 1 lacks `{need}` (D5, D6)"
        );
    }
    assert!(
        s.contains("<profile>"),
        "Stage 1 carries the profile literal (W2)"
    );
}

#[test]
fn stage_two_is_local_reads_the_local_model_and_checks_every_pane() {
    let s = stage(2);
    let f = lower(&s);
    for need in [
        "`dir`",
        "opencode.jsonc",
        "herdr.sh check-pane",
        "herdr.sh check-session",
    ] {
        assert!(f.contains(need), "Stage 2 lacks `{need}`");
    }
    assert!(code_lines(&s).iter().any(|l| l.contains("inventory.sh")));
    for gone in ["ssh", "tailscale"] {
        assert!(
            !code_lines(&s).iter().any(|l| l.contains(gone)),
            "Stage 2: {gone}"
        );
    }
}

#[test]
fn stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases() {
    let s = stage(3);
    let code = code_lines(&s);
    assert!(
        code.iter().any(|l| l.contains("panes.sh grid")),
        "the config-derived table"
    );
    let dry = code
        .iter()
        .find(|l| l.contains("profile apply"))
        .expect("Stage 3's dry run");
    assert!(
        dry.contains("--dry-run") && dry.contains("--format=json"),
        "{dry}"
    );
    let f = lower(&s);
    for case in ["fresh", "rerun", "outside", "previous wizard", "restored"] {
        assert!(f.contains(case), "Stage 3 does not name the case `{case}`");
    }
    assert!(
        f.contains("--from-current") && f.contains("--take-over"),
        "{f}"
    );
    assert!(f.contains("only if the user asks"), "{f}");
    assert!(
        f.contains("second instance"),
        "an old layout: a second instance or stop"
    );
    assert!(
        code.iter().any(|l| l.contains("holler pane list")),
        "the case-3 pre-check (W4)"
    );
    assert!(
        f.contains("body row") && f.contains("roster row"),
        "a Claude pane (D13)"
    );
    assert!(
        !skill().contains("pane-exists"),
        "apply never answers pane-exists (W4)"
    );
    for (name, text) in all_texts() {
        let import = code_lines(&text)
            .into_iter()
            .any(|l| l.contains("pane import"));
        assert!(!import, "{name}: the wizard never runs pane import");
    }
}

#[test]
fn stage_four_starts_the_hub_on_loopback_with_no_advertise_and_no_serve() {
    let s = stage(4);
    let code = code_lines(&s).join("\n");
    assert!(
        code.contains("holler hub serve --listen 127.0.0.1:<hub_port>"),
        "{code}"
    );
    assert!(code.contains("--role hub --stage 4"), "{code}");
    let all = skill();
    assert!(
        !all.contains("--advertise"),
        "the hub advertises nothing (F8)"
    );
    assert!(!code_lines(&all)
        .iter()
        .any(|l| l.contains("tailscale serve")));
    assert!(!all.contains("remote_host"), "D6");
    let unused = all
        .split("\n\n")
        .any(|p| p.contains("serve_https_port") && lower(p).contains("unused"));
    assert!(unused, "serve_https_port is marked unused");
}

#[test]
fn stage_five_starts_the_herdr_server_and_proves_the_adapter_reaches_it() {
    let code = code_lines(&stage(5)).join("\n");
    assert!(code.contains("herdr.sh server-start"), "{code}");
    assert!(code.contains("--role herdr --stage 5"), "{code}");
    assert!(
        code.contains("pane doctor"),
        "the adapter reaches this Herdr (D7)"
    );
    assert!(lower(&stage(5)).contains("herdr-version-unsupported"));
}

#[test]
fn stage_six_creates_the_profile_and_writes_only_spec_only_launches() {
    let code = code_lines(&stage(6)).join("\n");
    assert!(
        code.contains("panes.sh plan"),
        "the agent orders nothing (D9)"
    );
    for (name, text) in all_texts() {
        for line in code_lines(&text)
            .iter()
            .filter(|l| l.contains("holler pane launch"))
        {
            assert!(
                line.contains("--spec-only"),
                "{name}: a live launch: {line}"
            );
        }
    }
}

#[test]
fn stage_seven_dry_runs_then_applies_once_and_states_every_gate() {
    let s = stage(7);
    let code = code_lines(&s);
    let applies: Vec<(usize, &String)> = code
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("herdr.sh holler profile apply"))
        .collect();
    let dry = applies
        .iter()
        .find(|(_, l)| l.contains("--dry-run"))
        .expect("the dry run");
    let real: Vec<_> = applies
        .iter()
        .filter(|(_, l)| !l.contains("--dry-run"))
        .collect();
    assert_eq!(real.len(), 1, "one apply: {real:?}");
    assert!(dry.0 < real[0].0, "the dry run comes first");
    assert!(real[0].1.contains("--herdr-session"), "{}", real[0].1);
    let f = lower(&s);
    for need in [
        "exit 3",
        "exit 1",
        "reported",
        "refused",
        "grid-unreachable",
        "orchestrator-not-relaunched",
        "stage 3",
        "timeout",
    ] {
        assert!(f.contains(need), "Stage 7 does not state `{need}`");
    }
    assert!(
        !f.contains("column-first") && !f.contains("column 1 first"),
        "no workaround"
    );
}

#[test]
fn stage_eight_runs_one_local_body_from_the_registry() {
    let s = stage(8);
    let code = code_lines(&s).join("\n");
    assert!(code.contains("herdr.sh holler pane get") && code.contains("--format=json"));
    let f = flat(&s);
    assert!(f.contains("data.pane.harness.port") && f.contains("data.pane.session_of_record"));
    assert!(
        code.contains("--server ws://127.0.0.1:<hub_port>"),
        "{code}"
    );
    assert!(code.contains("--role body --stage 8"), "{code}");
    assert!(f.contains("`<prefix>-local`"), "{f}");
    assert!(
        lower(&s).contains("claude"),
        "Stage 8 says a Claude pane has no row"
    );
}

#[test]
fn stage_nine_reads_the_registry_and_gates_on_any_finding_but_one() {
    let s = stage(9);
    let code = code_lines(&s).join("\n");
    for need in [
        "pane doctor --format=json",
        "pane list --format=json",
        "profile show",
    ] {
        assert!(code.contains(need), "Stage 9 lacks `{need}`");
    }
    let f = flat(&s);
    assert!(f.contains("unregistered-herdr-pane"), "{f}");
    // REFRESH AFTER MERGE of #839: DRIVEN prints `-` until #839.
    assert!(f.contains("DRIVEN") && f.contains("#839"), "{f}");
}

#[test]
fn the_login_is_never_touched_and_its_findings_name_the_by_hand_remedy() {
    let text = skill();
    for line in code_lines(&text) {
        assert!(
            !line.contains("claude auth") && !line.contains("claude login"),
            "{line}"
        );
        assert!(
            !line.contains("--env"),
            "the wizard passes no --env: {line}"
        );
    }
    let f = flat(&text);
    for code in [
        "claude-login-missing",
        "harness-dialog-open",
        "process-in-use",
    ] {
        assert!(f.contains(code), "the skill does not name {code}");
    }
    assert!(f.contains("plain terminal"), "the by-hand remedy");
}

#[test]
fn teardown_closes_through_holler_then_stops_through_the_ledger() {
    let t = teardown();
    let close = t.find("pane close").expect("pane close");
    let delete = t.find("profile delete").expect("profile delete");
    let stop = t
        .find("stop-owned.sh teardown")
        .expect("stop-owned.sh teardown");
    assert!(close < delete && delete < stop, "{t}");
    assert!(
        t.contains("panes.sh teardown") && t.contains("--profile"),
        "{t}"
    );
    assert!(t.contains("hub is gone"), "what to do with no hub: {t}");
    assert!(
        t.to_lowercase().contains("tmux"),
        "what stays running (W9): {t}"
    );
}

#[test]
fn the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built() {
    let adr = flat(&read("docs/adr/ADR-0021.md"));
    let kept = "stays for sessions Holler does not own (a session with no pane record).";
    let added = "The setup wizard (#803) builds registry panes only";
    assert!(
        adr.contains(&format!("{kept} {added}")),
        "section 11's sentence (W11)"
    );
    // No closing backtick: `docs_cli_test` parses every inline `holler ...`
    // span under docs/, so the ADR may name the verb with its argument
    // (`holler profile apply NAME`) rather than the bare form.
    let decided =
        "**The setup wizard builds registry panes through `holler pane` and `holler profile apply";
    let taken = &adr[adr.find("## Decisions taken").unwrap()..];
    assert!(taken.contains(decided), "the Decisions taken item");
    assert!(!adr.contains("lands with #803, not here"), "item 16");
    assert!(adr.contains("As built (#803)"), "item 16");
    assert!(
        !adr.contains("the wizard's use of the flag (#803)"),
        "Not built here"
    );
}

#[test]
fn the_docs_readme_index_guide_and_changelog_carry_the_change() {
    let d = docs();
    for heading in [
        "\n## How the wizard builds panes",
        "\n## If you built a workspace with the previous wizard",
    ] {
        assert!(d.contains(heading), "docs lack `{heading}`");
    }
    let r = readme();
    assert!(!r.contains("(B)"), "the README's scenario B is gone");
    assert!(!r.contains("attach-mode"), "the README's wizard section");
    assert!(
        !read("docs/README.md").contains("attach-mode"),
        "the docs index"
    );
    let guide = read("docs/pane-control.md");
    let six = flat(&section(&guide, "## 6. Profiles", "\n## 7."));
    assert!(
        six.contains("setup wizard") && six.contains("--spec-only"),
        "the pointer"
    );
    let log = read("CHANGELOG.md");
    // `[Unreleased]` plus the newest released section: the release flow (docs/releasing.md)
    // moves these entries under `## [x.y.z]` and leaves `[Unreleased]` empty, so the check
    // holds both before and after the cut. Missing from both still fails.
    let from = &log[log.find("## [Unreleased]").expect("no `## [Unreleased]`")..];
    let unreleased = section(from, "## [Unreleased]", "\n## [");
    let newest = section(&from[unreleased.len()..], "\n## [", "\n## [");
    let unreleased = format!("{unreleased}{newest}");
    // Any Enhancements section: a non-empty `[Unreleased]` brings its own ahead of the released one.
    let enh_has_803 = unreleased
        .match_indices("### Enhancements")
        .any(|(at, _)| section(&unreleased[at..], "### Enhancements", "\n### ").contains("#803"));
    assert!(enh_has_803, "an Enhancements item for #803");
    let breaking = section(&unreleased, "### Breaking Changes", "\n### ");
    assert!(
        breaking.contains("#803") && breaking.contains("[[pane]]"),
        "{breaking}"
    );
    let (e, b) = (
        unreleased.find("### Enhancements"),
        unreleased.find("### Breaking Changes"),
    );
    assert!(e < b, "Breaking Changes after Enhancements");
}

/// Every `holler pane VERB` in `text` runs through the door: preceded by `herdr.sh `.
fn pane_verbs_outside_the_door(text: &str) -> Vec<String> {
    text.match_indices("holler pane ")
        .filter(|(at, _)| !text[..*at].ends_with("herdr.sh "))
        .map(|(at, _)| text[at..].chars().take(40).collect())
        .collect()
}

/// S's REWORK 1: a pane verb the user runs from their own shell goes through the door with the
/// instance's literals, never with `HOLLER_STATE_DIR` alone (a close would stop another
/// instance's tmux session of that name on tmux's default server).
#[test]
fn the_users_own_pane_verbs_go_through_the_door_with_the_literals_stage_ten_prints() {
    let note = flat(&section(
        &stage(11),
        "- **The wizard's panes are Holler's.**",
        "\n- **",
    ));
    let basics = flat(&section(&docs(), "\n## Basic Herdr commands", "\n## "));
    for (name, text) in [("Stage 11's first note", &note), ("docs' basics", &basics)] {
        for verb in [
            "herdr.sh holler pane close",
            "herdr.sh holler pane relaunch",
        ] {
            assert!(text.contains(verb), "{name} lacks `{verb}`: {text}");
        }
        let bare = pane_verbs_outside_the_door(text);
        assert!(
            bare.is_empty(),
            "{name}: a pane verb around the door: {bare:?}"
        );
    }
    let report = flat(&section(
        &stage(10),
        "**Every stage green or skipped = done.**",
        "\n1. ",
    ));
    assert!(
        report.contains("<herdr_env>") && report.contains("$WIZARD_LIB"),
        "Stage 10's report prints the door's literals: {report}"
    );
}

/// S's REWORK 3: the orchestrator's reset is refused without `--as-operator`
/// (`orchestrator-pane`), and it leaves the old OpenCode session as a `stray-session`.
#[test]
fn the_orchestrators_reset_is_as_operator_and_names_the_stray_session_it_leaves() {
    for (name, text) in [("Stage 10", flat(&stage(10))), ("docs", flat(&docs()))] {
        let resets: Vec<String> = text
            .match_indices("pane reset PANE")
            .map(|(at, _)| text[at..].chars().take(30).collect())
            .collect();
        assert!(!resets.is_empty(), "{name} never resets the orchestrator");
        for r in &resets {
            assert!(
                r.starts_with("pane reset PANE --as-operator"),
                "{name}: {r}"
            );
        }
        assert!(
            text.contains("stray-session"),
            "{name} omits the stray session"
        );
    }
}

/// S's REWORK 4 (Amendment (d)): the docs point to apply's creation order, not restate it.
#[test]
fn the_docs_point_to_applys_creation_order_and_do_not_restate_it() {
    let builds = flat(&section(
        &docs(),
        "\n## How the wizard builds panes",
        "\n## ",
    ));
    assert!(!builds.contains("column 1 by row"), "{builds}");
    assert!(
        builds.contains("creation order") && builds.contains("pane-control.md#6-profiles"),
        "{builds}"
    );
}
