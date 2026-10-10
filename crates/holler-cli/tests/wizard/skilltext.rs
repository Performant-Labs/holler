//! #752: the skill text, the docs and the README read as one procedure (epic #726).
//!
//! Plain text assertions over `agent-skills/setup-wizard/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`, plus a check that every script command line the skill documents uses a verb and
//! flags the script itself knows. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #752

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap()
}

fn skill() -> String {
    read("agent-skills/setup-wizard/SKILL.md")
}

/// The skill with every run of whitespace folded to one space, for phrases that wrap.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn docs() -> String {
    read("docs/setup-wizard.md")
}

/// The README's wizard section (the rest of the README is the general quick start).
fn readme() -> String {
    let text = read("README.md");
    let from = text
        .find("## Set up a Herdr workspace with an agent")
        .expect("wizard section");
    let rest = &text[from + 3..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())].to_owned()
}

/// The three files the story owns, as (name, text).
fn all_texts() -> Vec<(&'static str, String)> {
    vec![
        ("SKILL.md", skill()),
        ("docs/setup-wizard.md", docs()),
        ("README.md", readme()),
    ]
}

/// Every line inside a fenced code block.
fn code_lines(text: &str) -> Vec<String> {
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
fn section(text: &str, start: &str, end: &str) -> String {
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
    ];
    for (name, text) in all_texts() {
        for line in code_lines(&text) {
            let hit = words.iter().any(|w| line.contains(w));
            let marked = line.contains("default instance only");
            assert!(
                !hit || scoped_to_instance(&line) || marked,
                "{name}: unscoped: {line}"
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
fn the_logs_rule_covers_stages_four_to_nine_and_stage_eight_sets_the_log_dir() {
    let text = skill();
    assert!(
        flat(&text).contains("Every log Stages 4 to 9 create"),
        "the rule names Stages 4 to 9"
    );
    let stage8 = section(&text, "## Stage 8 ", "\n## Stage 9 ");
    assert!(stage8.contains("WIZARD_LOG_DIR=<logs_dir>"));
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
            install.contains("agent-skills/setup-wizard"),
            "{name}: {install}"
        );
        assert!(
            !install.contains("SKILL.md"),
            "{name}: single file: {install}"
        );
    }
}

#[test]
fn the_join_url_and_advertise_carry_the_serve_port() {
    let text = skill();
    assert!(text.contains("--advertise <hub_host>:<serve_https_port>"));
    assert!(text.contains("--server wss://<hub_host>:<serve_https_port>"));
    assert!(!text.contains("--advertise <hub_host> "));
    assert!(!text.contains("wss://<hub_host> "));
}

#[test]
fn the_derived_body_config_names_what_to_drop_and_what_to_set() {
    let text = skill();
    let para = text
        .split("\n\n")
        .find(|p| p.contains("derived body config") && p.contains("drop"))
        .expect("a paragraph about the derived body config");
    for need in [
        "[instance]",
        "`backend_port`",
        "`endpoint`",
        "`instance.sh` printed",
    ] {
        assert!(flat(para).contains(need), "missing {need}: {para}");
    }
    assert!(!text.contains("can be passed to `holler body run --config` as-is"));
}

#[test]
fn stages_three_and_four_use_the_ports_instance_sh_printed() {
    let text = skill();
    let planning = section(&text, "## Stage 3 ", "\n## Stage 5 ");
    assert!(
        !planning.contains("backend_port_base +"),
        "a second port formula"
    );
    assert!(flat(&planning).contains("`instance.sh` printed"));
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
        "HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role herdr --stage 8 --session <herdr_session>"
    ));
    assert!(flat(&text).contains("first line"));
}

#[test]
fn stage_two_fetches_each_remote_ledger_and_passes_it_to_collide() {
    let text = skill();
    assert!(text.contains("cat <state_dir>/wizard-ledger.toml"));
    assert!(text.contains("WIZARD_LEDGER=<scratch>/ledger-<host>.toml"));
    assert!(text.contains("wizard-scratch"));
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
    assert!(text.contains("<state_dir>/<prefix>-sessions.toml"));
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
fn teardown_runs_on_every_host_and_turns_off_only_its_own_serve_entry() {
    let text = skill();
    assert!(text.contains("tailscale serve --https=<serve_https_port> off"));
    for line in text.lines().filter(|l| l.contains("serve reset")) {
        assert!(line.to_lowercase().contains("never"), "{line}");
    }
    assert!(flat(&text).contains("every host the run touched"));
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
    let text = skill();
    assert!(text.contains("--label <hub_host's short name>-<prefix>-<remote_host>"));
    assert!(flat(&text).contains("never `herdr.sh run server stop`"));
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
    read(&format!("agent-skills/setup-wizard/lib/{script}.sh"))
}

#[test]
fn documented_verbs_and_flags_exist_in_the_scripts() {
    let verbed = ["herdr", "ledger", "stop-owned"];
    for script in [
        "instance",
        "inventory",
        "collide",
        "ledger",
        "stop-owned",
        "herdr",
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
