//! #752: the skill text, the docs and the README read as one procedure (epic #726).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`, plus a check that every script command line the skill documents uses a verb and
//! flags the script itself knows. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #752

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(super) fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap()
}

pub(super) fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
}

/// The skill with every run of whitespace folded to one space, for phrases that wrap.
pub(super) fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn docs() -> String {
    read("docs/setup-wizard.md")
}

/// The README's wizard section (the rest of the README is the general quick start).
pub(super) fn readme() -> String {
    let text = read("README.md");
    let from = text
        .find("## Set up a Herdr workspace with an agent")
        .expect("wizard section");
    let rest = &text[from + 3..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())].to_owned()
}

/// The three files the story owns, as (name, text).
pub(super) fn all_texts() -> Vec<(&'static str, String)> {
    vec![
        ("SKILL.md", skill()),
        ("docs/setup-wizard.md", docs()),
        ("README.md", readme()),
    ]
}

/// Every line inside a fenced code block.
pub(super) fn code_lines(text: &str) -> Vec<String> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
        } else if inside {
            out.push(line.to_owned());
        }
    }
    out
}

/// Every inline `code span` on a line outside fenced blocks.
fn inline_spans(text: &str) -> Vec<String> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
        } else if !inside {
            out.extend(line.split('`').skip(1).step_by(2).map(str::to_owned));
        }
    }
    out
}

/// The text between a heading and the next heading of the same or higher level.
pub(super) fn section(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |i| i + start.len());
    rest[..to].to_owned()
}

fn scoped_to_instance(line: &str) -> bool {
    line.contains("HOLLER_STATE_DIR=") || line.contains("herdr.sh")
}

#[test]
fn wizard_lib_has_one_absolute_definition_and_no_relative_one() {
    let mut definitions = Vec::new();
    for (name, text) in all_texts() {
        for line in text.lines().filter(|l| l.contains("WIZARD_LIB=")) {
            assert!(
                !line.contains("WIZARD_LIB:-"),
                "{name}: default form: {line}"
            );
            definitions.push((name, line.trim().to_owned()));
        }
    }
    assert_eq!(definitions.len(), 1, "{definitions:?}");
    let (name, line) = &definitions[0];
    assert_eq!(*name, "SKILL.md");
    assert!(
        line.starts_with("WIZARD_LIB=/") || line.starts_with("WIZARD_LIB=<HOME>/"),
        "not absolute: {line}"
    );
    assert!(
        !line.contains("agent-skills/"),
        "relative to a checkout: {line}"
    );
}

#[test]
fn the_skill_says_shell_variables_do_not_persist_before_the_first_stage() {
    let text = skill();
    let sentence = "**Shell variables do not persist between commands.**";
    let at = text.find(sentence).expect("the shell-state sentence");
    assert!(
        at < text.find("## The config file").unwrap(),
        "too late in the file"
    );
}

#[test]
fn no_hub_token_command_lacks_the_state_directory() {
    for (name, text) in all_texts() {
        let mut found = code_lines(&text);
        found.extend(inline_spans(&text));
        for line in found.iter().filter(|l| l.contains("holler hub token")) {
            assert!(
                line.contains("HOLLER_STATE_DIR="),
                "{name}: unscoped: {line}"
            );
        }
    }
}

#[test]
fn holler_and_herdr_commands_in_code_blocks_are_scoped_to_the_instance() {
    let words = [
        "holler say",
        "holler roster",
        "holler interrupt",
        "holler hub status",
        "holler hub serve",
        "holler body join",
        "holler body run",
        "herdr pane",
        "herdr workspace",
        "herdr status",
        "herdr server",
        "herdr session",
        "holler pane",
        "holler profile",
    ];
    for (name, text) in all_texts() {
        // The AGENTS.md briefing is deliberately unscoped: it must not carry the state directory
        // (the orchestrator's pane environment sets it), see #761.
        let text = text.replace(&briefing(), "");
        for line in code_lines(&text) {
            let hit = words.iter().any(|w| line.contains(w));
            let marked = line.contains("default instance only");
            assert!(
                !hit || scoped_to_instance(&line) || marked,
                "{name}: unscoped: {line}"
            );
            // #803: a pane or profile verb goes through the one door, never HOLLER_STATE_DIR=.
            let pane_verb = line.contains("holler pane") || line.contains("holler profile");
            assert!(
                !pane_verb || line.contains("herdr.sh holler"),
                "{name}: not through `herdr.sh holler`: {line}"
            );
        }
    }
}

#[test]
fn no_code_block_writes_under_tmp_and_mktemp_names_a_directory() {
    for (name, text) in all_texts() {
        for line in code_lines(&text) {
            assert!(!line.contains("/tmp"), "{name}: {line}");
            assert!(
                !line.contains("mktemp") || line.contains("$HOME"),
                "{name}: {line}"
            );
        }
    }
}

#[test]
fn the_logs_rule_covers_stages_four_to_nine_and_stage_five_sets_the_log_dir() {
    let text = skill();
    assert!(
        flat(&text).contains("Every log Stages 4 to 9 create"),
        "the rule names Stages 4 to 9"
    );
    let stage5 = section(&text, "## Stage 5 ", "\n## Stage 6 ");
    assert!(stage5.contains("WIZARD_LOG_DIR=<logs_dir>"));
}

#[test]
fn the_install_command_fetches_the_whole_skill_directory() {
    for (name, text) in [("README.md", readme()), ("docs", docs())] {
        let lines = code_lines(&text);
        let install = lines
            .iter()
            .find(|l| l.contains("curl") && l.contains("tar"))
            .expect(name);
        assert!(install.contains("tar"), "{name}: {install}");
        assert!(
            install.contains("agent-skills/herdr-setup"),
            "{name}: {install}"
        );
        assert!(
            !install.contains("SKILL.md"),
            "{name}: single file: {install}"
        );
    }
}

#[test]
fn the_derived_body_config_names_what_to_drop_and_what_to_set() {
    let text = skill();
    let para = text
        .split("\n\n")
        .find(|p| p.contains("derived body config") && p.contains("drop"))
        .expect("a paragraph about the derived body config");
    // #803: the body's rows take their endpoint and session id from the registry (D13).
    for need in [
        "[instance]",
        "`backend_port`",
        "`[[pane]]`",
        "holler pane get",
    ] {
        assert!(flat(para).contains(need), "missing {need}: {para}");
    }
    assert!(!text.contains("can be passed to `holler body run --config` as-is"));
}

#[test]
fn stages_three_and_four_use_the_ports_panes_sh_printed() {
    let text = skill();
    let planning = section(&text, "## Stage 3 ", "\n## Stage 5 ");
    assert!(
        !planning.contains("backend_port_base +"),
        "a second port formula"
    );
    // #803: the OpenCode panes' ports come from `panes.sh grid` (D11).
    assert!(flat(&planning).contains("panes.sh grid"));
}

#[test]
fn background_commands_use_a_brace_group_that_prints_the_real_pid() {
    for line in code_lines(&skill())
        .iter()
        .filter(|l| l.contains("echo \\$!") || l.contains("echo $!"))
    {
        assert!(
            line.contains("{ ") && line.contains("; }"),
            "subshell pid risk: {line}"
        );
    }
    assert!(flat(&skill()).contains("starts with the program"));
}

#[test]
fn the_herdr_server_is_recorded_in_full_from_the_pid_server_start_prints() {
    let text = skill();
    assert!(text.contains(
        "HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role herdr --stage 5 --session <herdr_session>"
    ));
    assert!(flat(&text).contains("first line"));
}

#[test]
fn state_dir_is_resolved_once_per_host_to_an_absolute_path() {
    let text = skill();
    assert!(text.contains("printf %s ~/"));
    assert!(text.contains("$HOME/.holler"));
    assert!(
        !text.contains("<state_dir>` (empty = Holler's own default"),
        "stale empty default"
    );
}

#[test]
fn the_body_config_path_is_defined_once() {
    let text = skill();
    assert!(text.contains("<body_config>"));
    assert!(text.contains("<state_dir>/herdr/setup/<prefix>-sessions.toml"));
    assert!(!text.contains("scp <this-host-derived-sessions.toml> <remote_host>:~/sessions.toml"));
}

#[test]
fn reuse_means_a_live_ledger_row_and_status_calls_are_scoped() {
    let text = skill();
    assert!(!text.contains("every previously-running body process on every host"));
    assert!(!text.contains("(reused, not restarted)"));
    for line in code_lines(&text)
        .iter()
        .filter(|l| l.contains("hub status"))
    {
        assert!(line.contains("HOLLER_STATE_DIR="), "{line}");
    }
}

#[test]
fn teardown_names_the_named_herdr_session_and_never_resets_serve() {
    let text = skill();
    for line in text.lines().filter(|l| l.contains("serve reset")) {
        assert!(line.to_lowercase().contains("never"), "{line}");
    }
    assert!(flat(&text).contains("named Herdr session"));
}

#[test]
fn a_restart_reruns_the_stage_start_command() {
    let text = skill();
    assert!(text.contains("RESTART-CMD"));
    assert!(flat(&text).contains("only identifies"));
}

#[test]
fn the_token_label_folds_in_the_prefix_and_the_stop_path_for_herdr_is_named() {
    let text = flat(&skill());
    // #803: one local body, so the label is `<prefix>-local` (D13).
    assert!(text.contains("`<prefix>-local`"), "{text}");
    assert!(
        !text.contains("-<prefix>-<remote_host>"),
        "the remote-host label is gone"
    );
    assert!(text.contains("never `herdr.sh run server stop`"));
}

#[test]
fn the_connections_section_says_its_literals_are_defaults() {
    let text = skill();
    let part = section(&text, "## Connections this wizard", "\n## The config file");
    assert!(part.contains("defaults"), "{part}");
}

#[test]
fn the_docs_say_the_first_instance_has_no_ledger_and_is_never_fixed_by_hand() {
    let text = flat(&docs());
    assert!(text.contains("has no ledger"));
    assert!(text.contains("by hand"));
}

/// Words after `<script>.sh` on a documented command line.
fn script_words(line: &str, script: &str) -> Option<Vec<String>> {
    let at = line.find(&format!("{script}.sh"))?;
    let rest = &line[at + script.len() + 3..];
    let stop = rest.find(['|', '\\', '"', ')']).unwrap_or(rest.len());
    let words = rest[..stop].split_whitespace();
    Some(
        words
            .take_while(|w| !matches!(*w, ">" | "<" | "2>&1" | "#"))
            .map(str::to_owned)
            .collect(),
    )
}

fn script_source(script: &str) -> String {
    // #803: `panes.sh` is new; until it exists its verbs are unknown words.
    std::fs::read_to_string(repo_root().join(format!("agent-skills/herdr-setup/lib/{script}.sh")))
        .unwrap_or_default()
}

#[test]
fn documented_verbs_and_flags_exist_in_the_scripts() {
    let verbed = ["herdr", "ledger", "stop-owned", "panes"];
    for script in [
        "instance",
        "inventory",
        "collide",
        "ledger",
        "stop-owned",
        "herdr",
        "panes",
    ] {
        let source = script_source(script);
        for line in code_lines(&skill())
            .iter()
            .filter(|l| !l.contains("tar -C"))
        {
            let Some(words) = script_words(line, script) else {
                continue;
            };
            if let Some(verb) = words.first().filter(|_| verbed.contains(&script)) {
                assert!(
                    source.contains(verb.as_str()),
                    "{script}: verb {verb}: {line}"
                );
            }
            // `herdr.sh run` passes Herdr's own flags through; the rest are the script's.
            for flag in line.split_whitespace().filter(|w| w.starts_with("--")) {
                let known = script == "herdr" || source.contains(flag) || !line.contains("record");
                assert!(known, "{script}: flag {flag}: {line}");
            }
        }
    }
}

#[test]
fn documented_argument_counts_match_the_scripts_usage() {
    for line in code_lines(&skill()) {
        if let Some(w) = script_words(&line, "stop-owned").filter(|w| !w.is_empty()) {
            let min = if w[0] == "teardown" { 2 } else { 3 };
            assert!(
                w.len() >= min,
                "stop-owned needs <state_dir> and its argument: {line}"
            );
        }
        if let Some(w) =
            script_words(&line, "collide").filter(|w| !line.contains("tar -C") && !w.is_empty())
        {
            assert_eq!(
                w.len(),
                2,
                "collide.sh takes <host-label> <inventory-file>: {line}"
            );
        }
    }
}

// ---- #761: the text side of the verification findings (epic #726, Amendment 2) ----

/// The AGENTS.md briefing the skill proposes (the fenced block that opens with its heading).
fn briefing() -> String {
    let text = skill();
    let from = text.find("## Holler self-status").expect("the briefing");
    let rest = &text[from..];
    rest[..rest.find("```").expect("end of the briefing")].to_owned()
}

#[test]
fn the_briefing_carries_no_state_directory_path() {
    let block = briefing();
    assert!(!block.contains("HOLLER_STATE_DIR=<"), "{block}");
    assert!(!block.contains("<state_dir>"), "{block}");
    let flat_block = flat(&block);
    assert!(
        flat_block.contains("already sets `HOLLER_STATE_DIR`"),
        "{block}"
    );
    assert!(flat_block.contains("never change or unset it"), "{block}");
    // The instance-specific form and the stop-and-ask rule are in the skill and in the docs.
    for (name, text) in [("SKILL.md", flat(&skill())), ("docs", flat(&docs()))] {
        assert!(
            text.contains("## Holler self-status (instance <name>)"),
            "{name}"
        );
        assert!(
            text.contains("another instance's section already exists"),
            "{name}"
        );
    }
}

#[test]
fn stage_one_says_a_non_default_instance_runs_outside_any_herdr_pane_and_states_the_port_rule() {
    let text = skill();
    let stage1 = flat(&section(&text, "## Stage 1 ", "\n## Stage 2 "));
    assert!(stage1.contains("outside any Herdr pane"), "{stage1}");
    assert!(stage1.contains("hub_port` of 41807"), "{stage1}");
    assert!(stage1.contains("serve_https_port` of 443"), "{stage1}");
    assert!(stage1.contains("47001"), "{stage1}");
    assert!(flat(&docs()).contains("hub_port` of 41807"), "docs");
}

#[test]
fn stage_two_runs_the_herdr_checks_read_only() {
    let text = skill();
    let stage2 = section(&text, "## Stage 2 ", "\n## Stage 3 ");
    let flat2 = flat(&stage2);
    assert!(stage2.contains("herdr.sh check-pane"), "{stage2}");
    assert!(stage2.contains("herdr.sh check-session"), "{stage2}");
    assert!(flat2.contains("Stage 3 refusal"), "{flat2}");
    assert!(flat2.contains("read-only"), "{flat2}");
}

#[test]
fn the_herdr_inventory_flag_is_on_every_inventory_line_and_none_is_remote() {
    let lines = code_lines(&skill());
    let inventory: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("inventory.sh") && !l.contains("tar -C"))
        .collect();
    assert!(!inventory.is_empty());
    for line in &inventory {
        assert!(line.contains("WIZARD_INVENTORY_HERDR=1"), "{line}");
        assert!(
            !line.contains("ssh "),
            "a remote inventory (#803 D6): {line}"
        );
    }
}

#[test]
fn stage_three_names_the_herdr_refusals_and_session_delete() {
    let text = skill();
    let stage3 = section(&text, "## Stage 3 ", "\n## Stage 4 ");
    let flat3 = flat(&stage3);
    assert!(stage3.contains("session-delete"), "{flat3}");
    assert!(flat3.contains("with the user's yes"), "{flat3}");
    assert!(
        flat3.contains("this instance's port pair, no live hub"),
        "{flat3}"
    );
    assert!(flat3.contains("unnamed Herdr server"), "{flat3}");
    assert!(flat3.contains("build in it"), "{flat3}");
    let stage5 = flat(&section(&text, "## Stage 5 ", "\n## Stage 6 "));
    assert!(stage5.contains("unnamed Herdr server"), "{stage5}");
    assert!(stage5.contains("build in it"), "{stage5}");
    let teardown = flat(&section(
        &text,
        "- **Tearing the instance down:**",
        "\n- **A join fails",
    ));
    assert!(teardown.contains("session-delete"), "{teardown}");
}

#[test]
fn stages_one_and_two_change_nothing_but_the_scratch_directory_and_the_install_is_asked_first() {
    let text = skill();
    assert!(
        flat(&text).contains("read-only except the scratch directory"),
        "the hard-stop paragraph"
    );
    let stage0 = flat(&section(&text, "## Stage 0 ", "\n## Stage 1 "));
    assert!(
        stage0.contains("ask the user before installing"),
        "the install into ~/.claude/skills needs a yes: {stage0}"
    );
}

#[test]
fn no_runnable_block_carries_a_literal_example_label_and_label_is_defined() {
    for (name, text) in [("SKILL.md", skill()), ("docs", docs())] {
        for line in code_lines(&text) {
            assert!(!line.contains("hub1-"), "{name}: literal label: {line}");
            if line.contains("--label") {
                assert!(line.contains("--label <label>"), "{name}: {line}");
            }
        }
    }
    let text = skill();
    assert!(
        flat(&text).contains("`<label>` is the Stage 8 token label"),
        "undefined placeholder"
    );
}

#[test]
fn the_body_reuse_rule_compares_the_expanded_path() {
    let text = flat(&skill());
    assert!(text.contains("expanded path of `<body_config>`"), "{text}");
    assert!(!text.contains("its recorded `cmd` names `<body_config>`"));
}

#[test]
fn the_check_port_sentence_comes_before_the_stage_four_start_block() {
    let text = skill();
    let stage4 = section(&text, "## Stage 4 ", "\n## Stage 5 ");
    let check = stage4.find("check-port").expect("check-port in Stage 4");
    let start = stage4.find("holler hub serve").expect("the start block");
    assert!(check < start, "check-port after the start block");
}

#[test]
fn every_ledger_call_in_a_code_block_sets_the_state_directory() {
    for (name, text) in [("SKILL.md", skill()), ("docs", docs())] {
        for line in code_lines(&text)
            .iter()
            .filter(|l| l.contains("ledger.sh") && !l.contains("tar -C"))
        {
            assert!(line.contains("HOLLER_STATE_DIR="), "{name}: {line}");
        }
    }
}

#[test]
fn the_readme_says_a_pre_ledger_setup_is_refused_as_foreign_and_where_to_read_more() {
    let text = flat(&readme());
    assert!(text.contains("before ledgers existed"), "{text}");
    assert!(text.contains("foreign"), "{text}");
    assert!(text.contains("docs/setup-wizard.md#instance-state-and-the-ledger"));
}

#[test]
fn the_install_command_notes_that_no_release_tag_has_the_whole_directory() {
    for (name, text) in [("README.md", flat(&readme())), ("docs", flat(&docs()))] {
        assert!(
            text.contains("No release tag contains the whole skill directory yet"),
            "{name}"
        );
    }
    assert!(flat(&skill()).contains("No release tag contains the whole skill directory yet"));
}

#[test]
fn every_wizard_file_is_under_herdr_setup_and_the_old_paths_are_gone() {
    // The one place the old locations may be named is the migration note.
    for (name, text) in all_texts() {
        assert!(
            !text.contains(".config/setup-wizard"),
            "{name} names the old config path"
        );
        let new = text.matches("herdr/setup/wizard-ledger.toml").count();
        let all = text.matches("wizard-ledger.toml").count();
        let old = all - new;
        assert!(
            old <= usize::from(name != "README.md"),
            "{name}: {old} old ledger mentions"
        );
    }
    for (name, text) in all_texts() {
        if name != "README.md" {
            assert!(
                text.contains("<state_dir>/herdr/setup/wizard-ledger.toml"),
                "{name}"
            );
        }
    }
    assert!(skill().contains("~/.holler/herdr/setup/sessions.toml"));
    assert!(docs().contains("~/.holler/herdr/setup/sessions.toml"));
    assert!(readme().contains("~/.holler/herdr/setup/sessions.toml"));
    let d = flat(&docs());
    assert!(d.contains("not read"), "the migration note is missing");
    assert!(d.contains("foreign"));
}
