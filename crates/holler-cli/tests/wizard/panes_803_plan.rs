//! #803: `lib/panes.sh`'s `plan`, `env` and `teardown`, the lines the skill runs (D8, D9, D15),
//! split from `panes_803` (which holds the fixtures and the `check` and `grid` tests) to keep each
//! file under the 600-line warning. The printed contract is the one `panes_803` states.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803

use std::path::Path;

use super::panes_803::{
    calls, config, env_literals, err, first_run, flag, mixed, out, panes_of, second_instance,
    two_by_two, Scratch, TWO_BY_TWO,
};

fn literal(lits: &[(String, String)], name: &str) -> Option<String> {
    lits.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

/// A `spec` line of an OpenCode pane: a spec-only launch with every flag a new spec needs.
fn assert_spec(argv: &[String], name: &str, role: &str, port: u16, demo: &str) {
    assert_eq!(argv[..3], ["pane", "launch", name], "{argv:?}");
    assert!(argv.iter().any(|w| w == "--spec-only"), "{argv:?}");
    assert!(
        !argv.iter().any(|w| w.contains("--herdr-session")),
        "{argv:?}"
    );
    assert!(
        flag(argv, "--harness").is_none(),
        "an OpenCode pane: {argv:?}"
    );
    let fixed = format!("fixed:{port}");
    for (f, want) in [
        ("--profile", "holler"),
        ("--workspace", "holler"),
        ("--grid", name),
        ("--project", demo),
        ("--model", "provider/model-a"),
        ("--effort", "medium"),
        ("--role", role),
        ("--ctx-soft", "800000"),
        ("--ctx-hard", "900000"),
        ("--port-policy", fixed.as_str()),
    ] {
        assert_eq!(flag(argv, f), Some(want), "{f}: {argv:?}");
    }
}

#[test]
fn the_plan_creates_the_profile_writes_every_spec_in_layout_order_then_dry_runs_then_applies() {
    let s = Scratch::new();
    let c = calls(&s.stdout("plan", &two_by_two(""), &["holler"]));
    let tags: Vec<&str> = c.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        tags.join(" "),
        "profile-create spec spec spec spec dry-run apply"
    );
    assert_eq!(c[0].1, ["profile", "create", "holler"]);
    let demo = s.home().join("Projects/demo").display().to_string();
    let want = [
        ("r1c1", 47001),
        ("r2c1", 47002),
        ("r1c2", 47003),
        ("r2c2", 47004),
    ];
    for ((_, argv), (name, port)) in c[1..5].iter().zip(want) {
        let role = if name == "r1c1" {
            "orchestrator"
        } else {
            "agent"
        };
        assert_spec(argv, name, role, port, &demo);
    }
    let (dry, apply) = (&c[5].1, &c[6].1);
    assert_eq!(dry[..3], ["profile", "apply", "holler"]);
    assert!(dry.iter().any(|w| w == "--dry-run"), "{dry:?}");
    assert!(dry.iter().any(|w| w == "--format=json"), "{dry:?}");
    assert_eq!(flag(dry, "--herdr-session"), Some("default"), "{dry:?}");
    assert_eq!(apply[..3], ["profile", "apply", "holler"]);
    assert!(!apply.iter().any(|w| w == "--dry-run"), "{apply:?}");
    assert_eq!(flag(apply, "--herdr-session"), Some("default"), "{apply:?}");
}

#[test]
fn a_rerun_plan_leaves_out_the_profile_create_and_keeps_the_rest() {
    let s = Scratch::new();
    let first = calls(&s.stdout("plan", &two_by_two(""), &["holler"]));
    let rerun = calls(&s.stdout("plan", &two_by_two(""), &["holler", "--rerun"]));
    assert_eq!(first[0].0, "profile-create");
    assert_eq!(rerun, first[1..].to_vec());
}

#[test]
fn a_workspace_label_with_a_space_is_one_argument_in_every_plan_line() {
    let s = Scratch::new();
    let body = two_by_two("workspace = \"my relay\"\n");
    let c = calls(&s.stdout("plan", &body, &["my relay"]));
    assert_eq!(c.len(), 7, "{c:?}");
    assert_eq!(c[0].1, ["profile", "create", "my relay"]);
    for (tag, argv) in &c[1..5] {
        assert_eq!(tag, "spec");
        assert_eq!(flag(argv, "--workspace"), Some("my relay"), "{argv:?}");
        assert_eq!(flag(argv, "--profile"), Some("my relay"), "{argv:?}");
    }
    assert_eq!(c[6].1[2], "my relay");
}

#[test]
fn a_non_default_instance_plans_with_its_own_herdr_session_and_port_base() {
    let s = Scratch::new();
    let inst = second_instance(&s.home().join("state-second"), Some(48001));
    let body = config("", &TWO_BY_TWO, &inst, &panes_of(&TWO_BY_TWO, &["r1c1"]));
    let c = calls(&s.stdout("plan", &body, &["holler"]));
    assert_eq!(flag(&c[1].1, "--port-policy"), Some("fixed:48001"), "{c:?}");
    assert_eq!(flag(&c[4].1, "--port-policy"), Some("fixed:48004"), "{c:?}");
    assert_eq!(flag(&c[5].1, "--herdr-session"), Some("second"));
    assert_eq!(flag(&c[6].1, "--herdr-session"), Some("second"));
}

#[test]
fn a_claude_pane_gets_its_harness_and_no_port_and_ports_count_opencode_panes_only() {
    let s = Scratch::new();
    let (body, order) = mixed();
    let specs: Vec<Vec<String>> = calls(&s.stdout("plan", &body, &["holler"]))
        .into_iter()
        .filter_map(|(t, a)| (t == "spec").then_some(a))
        .collect();
    assert_eq!(specs.len(), 16);
    let mut next = 47001;
    for (argv, name) in specs.iter().zip(&order) {
        assert_eq!(&argv[2], name, "layout order: {argv:?}");
        if name.ends_with("c2") {
            assert_eq!(flag(argv, "--harness"), Some("claude"), "{argv:?}");
            assert_eq!(flag(argv, "--port-policy"), Some("none"), "{argv:?}");
            assert_eq!(flag(argv, "--model"), Some("anthropic/claude-sonnet-5-5"));
            assert!(flag(argv, "--agent").is_none(), "{argv:?}");
        } else {
            let want = format!("fixed:{next}");
            assert!(flag(argv, "--harness").is_none(), "{argv:?}");
            assert_eq!(flag(argv, "--port-policy"), Some(want.as_str()), "{argv:?}");
            next += 1;
        }
    }
    let r1c3 = specs.iter().find(|a| a[2] == "r1c3").unwrap();
    assert_eq!(flag(r1c3, "--agent"), Some("build"));
}

#[test]
fn env_prints_the_workspace_extent_and_the_default_instances_herdr_socket() {
    let s = Scratch::new();
    let xdg = s.home().join("xdg");
    let run = |xdg_value: &Path| {
        let o = s
            .cmd("env", &first_run(), &[])
            .env("XDG_CONFIG_HOME", xdg_value)
            .output();
        let o = o.unwrap();
        assert_eq!(o.status.code(), Some(0), "{}", err(&o));
        env_literals(&out(&o))
    };
    let lits = run(&xdg);
    assert_eq!(
        literal(&lits, "WIZARD_HERDR_WORKSPACES").as_deref(),
        Some("holler=2x4")
    );
    let sock = xdg.join("herdr/herdr.sock").display().to_string();
    assert_eq!(literal(&lits, "WIZARD_HERDR_SOCKET"), Some(sock));
    assert_eq!(
        literal(&lits, "WIZARD_TMUX_SOCKET"),
        None,
        "the default keeps tmux's server"
    );
    // The XDG rule (env.rs): an empty or relative XDG_CONFIG_HOME means $HOME/.config.
    let home_sock = s
        .home()
        .join(".config/herdr/herdr.sock")
        .display()
        .to_string();
    for bad in ["", "relative/dir"] {
        let lits = run(Path::new(bad));
        assert_eq!(
            literal(&lits, "WIZARD_HERDR_SOCKET").as_ref(),
            Some(&home_sock),
            "{bad:?}"
        );
    }
}

#[test]
fn env_gives_a_non_default_instance_its_own_tmux_socket_and_no_herdr_socket() {
    let s = Scratch::new();
    let state = s.home().join("state-second");
    let cols: [&[&str]; 2] = [&["o1", "a1", "a2"], &["b1"]];
    let inst = second_instance(&state, Some(48001));
    let body = config(
        "workspace = \"my relay\"\n",
        &cols,
        &inst,
        &panes_of(&cols, &["o1"]),
    );
    let lits = env_literals(&s.stdout("env", &body, &[]));
    let extent = literal(&lits, "WIZARD_HERDR_WORKSPACES");
    assert_eq!(
        extent.as_deref(),
        Some("my relay=3x2"),
        "rows are column 1's height"
    );
    let tmux = state.join("tmux.sock").display().to_string();
    assert_eq!(literal(&lits, "WIZARD_TMUX_SOCKET"), Some(tmux));
    assert_eq!(literal(&lits, "WIZARD_HERDR_SOCKET"), None);
}

#[test]
fn teardown_closes_every_pane_in_reverse_layout_order_then_deletes_the_profile() {
    let s = Scratch::new();
    let lines = calls(&s.stdout("teardown", &two_by_two(""), &["holler"]));
    let got: Vec<String> = lines.iter().map(|(_, a)| a.join(" ")).collect();
    let mut want: Vec<String> = ["r2c2", "r1c2", "r2c1", "r1c1"]
        .iter()
        .map(|n| format!("pane close {n} --profile holler"))
        .collect();
    want.push("profile delete holler".to_owned());
    assert_eq!(got, want);
}
