//! #803: `herdr.sh` keeps the server and session guards, loses every pane verb, and gains
//! `holler`, the one door through which the wizard runs Holler's pane and profile verbs (D8, D16,
//! A's W1). Driven with the fake `herdr` of `herdr.rs`'s `Env` and a fake `holler` beside it that
//! records its argv (one word per line) and its whole environment. The fake finds its log files
//! from its own path: `herdr.sh holler` scrubs the environment, so no test variable reaches it.
//!
//! Unix-only: the script is bash.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

use super::herdr::{script, text, Env};

/// Variables of the agent's own shell that must never reach `holler` (and so no pane): login
/// and credential names, a made-up secret, the agent's own Herdr pane and tmux client, and the
/// `HOLLER_*` names the allow-list does not carry.
const SENTINELS: [(&str, &str); 11] = [
    ("ANTHROPIC_API_KEY", "sk-sentinel-anthropic"),
    ("CLAUDE_CODE_OAUTH_TOKEN", "sentinel-oauth"),
    ("MADE_UP_SECRET_803", "sentinel-made-up"),
    ("HERDR_SOCKET_PATH", "/sentinel/herdr.sock"),
    ("HERDR_PANE_ID", "p-sentinel"),
    ("HERDR_SESSION", "second"),
    ("TMUX", "/sentinel/tmux,1,0"),
    ("TMUX_PANE", "%9"),
    ("HOLLER_OPENCODE_BIN", "/sentinel/opencode"),
    ("HOLLER_CLAUDE_BIN", "/sentinel/claude"),
    ("HOLLER_STATE_DIR", "/sentinel/elsewhere"),
];

/// Puts the fake `holler` in the env's `bin`, beside the fake `herdr`.
fn with_holler(env: &Env) {
    let fake = env.path().join("bin/holler");
    fs::write(
        &fake,
        "#!/bin/sh\n\
         d=\"$(dirname \"$0\")/..\"\n\
         env > \"$d/holler.env\"\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\"; done > \"$d/holler.argv\"\n\
         exit 0\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The argv the fake `holler` last saw, or `None` when it never ran.
fn holler_argv(env: &Env) -> Option<Vec<String>> {
    let text = fs::read_to_string(env.path().join("holler.argv")).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

/// The environment the fake `holler` saw, as (name, value).
fn holler_env(env: &Env) -> Vec<(String, String)> {
    fs::read_to_string(env.path().join("holler.env"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn var(seen: &[(String, String)], name: &str) -> Option<String> {
    seen.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

/// `herdr.sh holler <args>` for a non-default instance `second` whose state is under the env,
/// with every sentinel exported; `tweak` adjusts the command before it runs.
fn second(env: &Env, args: &[&str], tweak: impl FnOnce(&mut Command)) -> Output {
    let state = env.path().join("state-second");
    let mut full = vec!["holler"];
    full.extend_from_slice(args);
    let mut c = env.base(Some("second"), "second", &full);
    c.env("WIZARD_STATE_DIR", &state)
        .env("WIZARD_HERDR_WORKSPACES", "my relay=2x2")
        .env("WIZARD_TMUX_SOCKET", state.join("tmux.sock"))
        .env("SSH_AUTH_SOCK", "/run/agent.sock")
        .env("LC_ALL", "C");
    for (k, v) in SENTINELS {
        c.env(k, v);
    }
    tweak(&mut c);
    c.output().unwrap()
}

/// The default instance: no Herdr session name, tmux's default server, the default socket.
fn default(env: &Env, tweak: impl FnOnce(&mut Command)) -> Output {
    let mut c = env.base(Some("default"), "", &["holler", "pane", "list"]);
    c.env("WIZARD_STATE_DIR", env.path().join(".holler"))
        .env("WIZARD_HERDR_WORKSPACES", "holler=2x4")
        .env(
            "WIZARD_HERDR_SOCKET",
            env.path().join(".config/herdr/herdr.sock"),
        );
    tweak(&mut c);
    c.output().unwrap()
}

#[test]
fn label_is_gone_and_is_an_ordinary_usage_error() {
    let env = Env::new(&[]);
    let out = env.run("second", "second", &["label", "w1:p3", "r1c2"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("usage"), "{}", text(&out));
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

#[test]
fn run_refuses_every_pane_and_layout_word_naming_holler_pane_and_calls_no_herdr() {
    let env = Env::new(&[]);
    let refused: &[&[&str]] = &[
        &["pane", "split", "--pane", "p1", "--direction", "right"],
        &["pane", "run", "p1", "cd /x && cmd"],
        &["pane", "send-keys", "p1", "enter"],
        &["pane", "read", "p1"],
        &["pane", "get", "p1"],
        &["pane", "list"],
        &["pane", "rename", "p1", "r1c1"],
        &["workspace", "create", "--label", "x"],
        &["workspace", "rename", "w1", "x"],
        &["workspace", "focus", "w1"],
        &["server", "stop"],
        &["session", "attach"],
    ];
    for (instance, session) in [("second", "second"), ("default", "")] {
        for words in refused {
            let mut args = vec!["run"];
            args.extend_from_slice(words);
            let out = env.run(instance, session, &args);
            assert_eq!(out.status.code(), Some(2), "{args:?}: {}", text(&out));
            assert!(
                text(&out).contains("holler pane"),
                "{args:?}: {}",
                text(&out)
            );
        }
    }
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

#[test]
fn the_header_names_the_holler_door_and_no_pane_verb() {
    let src = fs::read_to_string(script()).unwrap();
    let header: String = src
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for gone in ["label <", "pane rename", "pane split"] {
        assert!(
            !header.contains(gone),
            "`{gone}` still in the header:\n{header}"
        );
    }
    assert!(header.contains("holler pane"), "{header}");
    assert!(header.contains("holler profile"), "{header}");
}

#[test]
fn holler_runs_with_the_instances_literals_and_none_of_the_agents_variables() {
    let env = Env::new(&[]);
    with_holler(&env);
    let out = second(&env, &["pane", "list", "--format=json"], |_| {});
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        holler_argv(&env).unwrap(),
        ["pane", "list", "--format=json"]
    );
    let seen = holler_env(&env);
    let state = env.path().join("state-second");
    let want = [
        ("HOLLER_STATE_DIR", state.display().to_string()),
        ("HOLLER_HERDR_SESSION", "second".to_owned()),
        ("HOLLER_HERDR_WORKSPACES", "my relay=2x2".to_owned()),
        (
            "HOLLER_TMUX_SOCKET",
            state.join("tmux.sock").display().to_string(),
        ),
        ("SSH_AUTH_SOCK", "/run/agent.sock".to_owned()),
        ("LC_ALL", "C".to_owned()),
    ];
    for (k, v) in want {
        assert_eq!(var(&seen, k), Some(v), "{k}: {seen:?}");
    }
    for k in ["PATH", "HOME"] {
        assert!(var(&seen, k).is_some(), "{k} dropped: {seen:?}");
    }
    assert_eq!(var(&seen, "HOLLER_HERDR_SOCKET"), None, "{seen:?}");
    for (k, v) in SENTINELS.iter().filter(|(k, _)| *k != "HOLLER_STATE_DIR") {
        assert_eq!(var(&seen, k), None, "{k} leaked: {seen:?}");
        assert!(!text(&out).contains(v), "{k}'s value printed");
    }
    assert!(
        !seen.iter().any(|(k, _)| k.starts_with("HERDR_")),
        "{seen:?}"
    );
    assert!(
        env.recorded().is_empty(),
        "no Herdr call: {:?}",
        env.recorded()
    );
}

#[test]
fn holler_passes_its_arguments_through_unchanged() {
    let env = Env::new(&[]);
    with_holler(&env);
    let args = ["profile", "apply", "my relay", "--herdr-session", "second"];
    let out = second(&env, &args, |_| {});
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(holler_argv(&env).unwrap(), args);
}

#[test]
fn the_default_instance_names_its_session_default_and_sets_the_herdr_socket() {
    let env = Env::new(&[]);
    with_holler(&env);
    let out = default(&env, |_| {});
    assert!(out.status.success(), "{}", text(&out));
    let seen = holler_env(&env);
    assert_eq!(
        var(&seen, "HOLLER_HERDR_SESSION").as_deref(),
        Some("default")
    );
    let sock = env.path().join(".config/herdr/herdr.sock");
    assert_eq!(
        var(&seen, "HOLLER_HERDR_SOCKET"),
        Some(sock.display().to_string())
    );
    assert_eq!(
        var(&seen, "HOLLER_TMUX_SOCKET"),
        None,
        "tmux's default server"
    );
}

/// A refusal of `herdr.sh holler`: exit 2, one message naming `name`, and no `holler` run.
fn refused_naming(env: &Env, out: &Output, name: &str) {
    assert_eq!(out.status.code(), Some(2), "{}", text(out));
    assert!(
        text(out).contains(name),
        "does not name {name}: {}",
        text(out)
    );
    assert_eq!(holler_argv(env), None, "holler ran");
}

#[test]
fn holler_refuses_an_unset_or_empty_state_directory_and_runs_nothing() {
    let env = Env::new(&[]);
    with_holler(&env);
    let out = second(&env, &["pane", "list"], |c| {
        c.env_remove("WIZARD_STATE_DIR");
    });
    refused_naming(&env, &out, "WIZARD_STATE_DIR");
    let out = default(&env, |c| {
        c.env("WIZARD_STATE_DIR", "");
    });
    refused_naming(&env, &out, "WIZARD_STATE_DIR");
}

#[test]
fn holler_refuses_a_non_default_instance_without_an_absolute_tmux_socket() {
    let env = Env::new(&[]);
    with_holler(&env);
    let out = second(&env, &["pane", "list"], |c| {
        c.env_remove("WIZARD_TMUX_SOCKET");
    });
    refused_naming(&env, &out, "WIZARD_TMUX_SOCKET");
    for bad in ["", "state/tmux.sock"] {
        let out = second(&env, &["pane", "list"], |c| {
            c.env("WIZARD_TMUX_SOCKET", bad);
        });
        refused_naming(&env, &out, "WIZARD_TMUX_SOCKET");
    }
}

#[test]
fn holler_refuses_the_default_instance_without_a_herdr_socket() {
    let env = Env::new(&[]);
    with_holler(&env);
    let out = default(&env, |c| {
        c.env_remove("WIZARD_HERDR_SOCKET");
    });
    refused_naming(&env, &out, "WIZARD_HERDR_SOCKET");
}
