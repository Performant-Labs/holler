//! The setup wizard's `[instance]` table and its validator, `instance.sh` (holler#727, epic #726).
//!
//! Runs the real script with `std::process::Command` against fixture `sessions.toml` text in a
//! temp directory. Nothing here touches a real state directory, host or port.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #727

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap()
}

/// Write `toml` into a fresh temp directory and run `instance.sh` on it.
fn validate(toml: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("sessions.toml");
    std::fs::write(&cfg, toml).unwrap();
    Command::new("bash")
        .arg(repo_root().join("agent-skills/setup-wizard/lib/instance.sh"))
        .arg(&cfg)
        .output()
        .unwrap()
}

/// Like `validate`, with `HOME` set so the default state directory is known to the script.
fn validate_home(toml: &str, home: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("sessions.toml");
    std::fs::write(&cfg, toml).unwrap();
    Command::new("bash")
        .arg(repo_root().join("agent-skills/setup-wizard/lib/instance.sh"))
        .arg(&cfg)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

const HEAD: &str =
    "layout = [[\"o1\"], [\"alpha\", \"beta\"]]\nhub_host = \"hub.example.ts.net\"\n";

const TAIL: &str = r#"
[[orchestrator]]
name = "o1"
dir = "~/Projects/holler"
cmd = "claude"

[[session]]
name = "alpha"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47001"
remote_host = "remote-a"
remote_tailnet_host = "remote-a.example.ts.net"

[[session]]
name = "beta"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47002"
remote_host = "remote-a"
remote_tailnet_host = "remote-a.example.ts.net"
"#;

/// The recorded plan text for the config above with no `[instance]` table.
const RECORDED_PLAN: &str = "instance: name=default prefix=default hub_port=41807 \
serve_https_port=443 state_dir= herdr_session= backend_port_base=47001\n\
session alpha: backend_port=47001 endpoint=http://127.0.0.1:47001\n\
session beta: backend_port=47002 endpoint=http://127.0.0.1:47002\n";

fn config(instance: &str, tail: &str) -> String {
    format!("{HEAD}\n{instance}\n{tail}")
}

const FULL_INSTANCE: &str = r#"[instance]
name = "second"
hub_port = 41808
serve_https_port = 8443
state_dir = "/home/<user>/.holler-second"
herdr_session = "second"
backend_port_base = 47101
"#;

#[test]
fn no_instance_table_gives_the_recorded_plan() {
    let out = validate(&config("", TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), RECORDED_PLAN);
}

#[test]
fn explicit_defaults_give_the_same_plan_as_no_table() {
    let table = "[instance]\nname = \"default\"\nhub_port = 41807\nserve_https_port = 443\n\
                 backend_port_base = 47001\n";
    let out = validate(&config(table, TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), RECORDED_PLAN);
}

#[test]
fn a_full_non_default_instance_is_accepted_and_resolved() {
    let out = validate(&config(FULL_INSTANCE, TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "instance: name=second prefix=second hub_port=41808 serve_https_port=8443 \
             state_dir=/home/<user>/.holler-second herdr_session=second backend_port_base=47101"
        ),
        "{text}"
    );
    assert!(
        text.contains("session alpha: backend_port=47101 endpoint=http://127.0.0.1:47101"),
        "{text}"
    );
    assert!(
        text.contains("session beta: backend_port=47102 endpoint=http://127.0.0.1:47102"),
        "{text}"
    );
}

#[test]
fn a_session_backend_port_overrides_the_base() {
    let tail = TAIL.replacen(
        "name = \"beta\"\n",
        "name = \"beta\"\nbackend_port = 47500\n",
        1,
    );
    let out = validate(&config(FULL_INSTANCE, &tail));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("session beta: backend_port=47500 endpoint=http://127.0.0.1:47500"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_non_default_instance_missing_a_key_is_refused_naming_it() {
    for missing in ["name", "state_dir", "herdr_session"] {
        let table: String = FULL_INSTANCE
            .lines()
            .filter(|l| !l.starts_with(&format!("{missing} =")))
            .map(|l| format!("{l}\n"))
            .collect();
        let out = validate(&config(&table, TAIL));
        assert_eq!(out.status.code(), Some(1), "missing {missing}");
        let err = stderr(&out);
        assert!(
            err.contains(&format!("must set {missing}")),
            "missing {missing}: {err}"
        );
        assert!(
            stdout(&out).is_empty(),
            "no plan on refusal: {}",
            stdout(&out)
        );
    }
}

#[test]
fn a_changed_hub_port_alone_makes_the_instance_non_default() {
    let out = validate(&config("[instance]\nhub_port = 41900\n", TAIL));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    for key in ["name", "state_dir", "herdr_session"] {
        assert!(err.contains(&format!("must set {key}")), "{err}");
    }
}

#[test]
fn two_sessions_with_one_backend_port_are_refused_naming_both() {
    let tail = TAIL.replacen(
        "name = \"beta\"\n",
        "name = \"beta\"\nbackend_port = 47101\n",
        1,
    );
    let out = validate(&config(FULL_INSTANCE, &tail));
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("alpha") && err.contains("beta"), "{err}");
    assert!(err.contains("47101"), "{err}");
}

#[test]
fn the_same_port_on_two_different_hosts_is_not_a_collision() {
    let tail = TAIL
        .replacen(
            "name = \"beta\"\n",
            "name = \"beta\"\nbackend_port = 47101\n",
            1,
        )
        .replace(
            "remote_host = \"remote-a\"\nremote_tailnet_host = \"remote-a.example.ts.net\"\n\n",
            "remote_host = \"remote-b\"\nremote_tailnet_host = \"remote-b.example.ts.net\"\n\n",
        );
    let out = validate(&config(FULL_INSTANCE, &tail));
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn malformed_values_are_each_refused_naming_the_key() {
    let cases: &[(&str, &str)] = &[
        ("name = \"Second\"", "name"),
        ("name = \"2nd\"", "name"),
        ("name = \"abcdefghijklmnopqrstuvwxy\"", "name"),
        ("prefix = \"Bad_Prefix\"", "prefix"),
        ("hub_port = 80", "hub_port"),
        ("hub_port = 70000", "hub_port"),
        ("hub_port = \"41808\"", "hub_port"),
        ("serve_https_port = 0", "serve_https_port"),
        ("serve_https_port = 65536", "serve_https_port"),
        ("state_dir = \"relative/dir\"", "state_dir"),
        ("herdr_session = \"Bad Session\"", "herdr_session"),
        ("backend_port_base = 0", "backend_port_base"),
        ("backend_port_base = abc", "backend_port_base"),
        ("surprise = 1", "surprise"),
    ];
    for (line, key) in cases {
        let out = validate(&config(&format!("[instance]\n{line}\n"), TAIL));
        assert_eq!(out.status.code(), Some(1), "{line}");
        assert!(stderr(&out).contains(key), "{line}: {}", stderr(&out));
    }
}

#[test]
fn a_tilde_state_dir_is_accepted() {
    let table = FULL_INSTANCE.replace("/home/<user>/.holler-second", "~/.holler-second");
    let out = validate(&config(&table, TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn an_instance_table_after_a_session_table_is_refused() {
    let text = format!("{HEAD}{TAIL}\n{FULL_INSTANCE}");
    let out = validate(&text);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("must come before"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_top_level_key_placed_under_instance_is_refused_with_a_hint() {
    let text = format!("[instance]\nname = \"default\"\n{HEAD}{TAIL}");
    let out = validate(&text);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("layout"), "{}", stderr(&out));
}

#[test]
fn usage_errors_exit_2() {
    let out = Command::new("bash")
        .arg(repo_root().join("agent-skills/setup-wizard/lib/instance.sh"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// The documented defaults (docs table, SKILL.md example, the script's own output) must agree
/// with the values the stages use today.
#[test]
fn documented_defaults_match_the_skill_text() {
    let skill = read_repo("agent-skills/setup-wizard/SKILL.md");
    let docs = read_repo("docs/setup-wizard.md");

    // What the stages use today.
    assert!(skill.contains("holler hub serve --listen 127.0.0.1:<hub_port>"));
    assert!(skill.contains("tailscale serve --bg --https <serve_https_port> <hub_port>"));
    assert!(skill.contains("(defaults 41807 and 443)"));
    assert!(skill.contains("endpoint = \"http://127.0.0.1:47001\""));
    assert!(skill.contains("endpoint = \"http://127.0.0.1:47002\""));

    // What the script resolves with no table.
    let out = validate(&config("", TAIL));
    let plan = stdout(&out);
    assert!(plan.contains("hub_port=41807"), "{plan}");
    assert!(plan.contains("serve_https_port=443"), "{plan}");
    assert!(plan.contains("backend_port_base=47001"), "{plan}");

    // What the skill's commented example and the docs table document.
    for line in [
        "# hub_port = 41807",
        "# serve_https_port = 443",
        "# backend_port_base = 47001",
    ] {
        assert!(skill.contains(line), "SKILL.md lacks `{line}`");
    }
    assert!(docs.contains("## Running an instance beside another"));
    for row in [
        "| `hub_port` | integer | 41807 |",
        "| `serve_https_port` | integer | 443 |",
        "| `backend_port_base` | integer | 47001 |",
        "| `name` | string | `default` |",
    ] {
        assert!(docs.contains(row), "docs/setup-wizard.md lacks `{row}`");
    }
}

#[test]
fn stage_one_runs_the_validator() {
    let skill = read_repo("agent-skills/setup-wizard/SKILL.md");
    let stage1 = skill
        .split("## Stage 1")
        .nth(1)
        .and_then(|s| s.split("## Stage 2").next())
        .unwrap();
    assert!(
        stage1.contains("/instance.sh"),
        "Stage 1 must run instance.sh"
    );
}

#[test]
fn every_session_line_carries_its_endpoint() {
    let out = validate(&config("", TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
    let sessions: Vec<String> = stdout(&out)
        .lines()
        .filter(|l| l.starts_with("session "))
        .map(String::from)
        .collect();
    assert_eq!(sessions.len(), 2);
    assert!(sessions
        .iter()
        .all(|l| l.contains(" endpoint=http://127.0.0.1:")));
}

#[test]
fn a_session_endpoint_on_another_port_warns_naming_both_ports_and_exits_0() {
    let out = validate(&config(FULL_INSTANCE, TAIL));
    assert!(out.status.success(), "{}", stderr(&out));
    let warnings: Vec<String> = stderr(&out)
        .lines()
        .filter(|l| l.contains("alpha"))
        .map(String::from)
        .collect();
    assert_eq!(warnings.len(), 1, "{}", stderr(&out));
    assert!(
        warnings[0].contains("47001") && warnings[0].contains("47101"),
        "{warnings:?}"
    );
    assert!(stderr(&out).contains("beta"), "{}", stderr(&out));
}

#[test]
fn a_session_endpoint_matching_the_resolved_port_does_not_warn() {
    let tail = TAIL.replace("47001", "47101").replace("47002", "47102");
    let out = validate(&config(FULL_INSTANCE, &tail));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
}

#[test]
fn no_instance_table_never_warns_about_endpoints() {
    let out = validate(&config("", TAIL));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
}

fn with_state_dir(dir: &str) -> String {
    FULL_INSTANCE.replace("/home/<user>/.holler-second", dir)
}

fn assert_refused_naming_state_dir(out: &Output) {
    assert_eq!(out.status.code(), Some(1), "{}", stderr(out));
    assert!(stderr(out).contains("state_dir"), "{}", stderr(out));
    assert!(stdout(out).is_empty(), "{}", stdout(out));
}

#[test]
fn a_tilde_default_state_dir_is_refused_for_a_non_default_instance() {
    for dir in ["~/.holler", "~/.holler/"] {
        let out = validate_home(&config(&with_state_dir(dir), TAIL), "/home/<user>");
        assert_refused_naming_state_dir(&out);
    }
}

#[test]
fn an_absolute_default_state_dir_is_refused_for_a_non_default_instance() {
    for dir in ["/home/<user>/.holler", "/home/<user>/.holler/"] {
        let out = validate_home(&config(&with_state_dir(dir), TAIL), "/home/<user>");
        assert_refused_naming_state_dir(&out);
    }
}

#[test]
fn a_different_state_dir_is_accepted_for_a_non_default_instance() {
    let table = with_state_dir("/home/<user>/.holler-second");
    let out = validate_home(&config(&table, TAIL), "/home/<user>");
    assert!(out.status.success(), "{}", stderr(&out));
    let other = with_state_dir("~/.holler-other");
    let out = validate_home(&config(&other, TAIL), "/home/<user>");
    assert!(out.status.success(), "{}", stderr(&out));
}
