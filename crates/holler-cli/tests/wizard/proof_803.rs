//! #803: the two-instance proof gains pane verbs. Instance B's panes are built "as the skill
//! tells": `panes.sh` prints the plan and the environment literals, and every line runs through
//! `herdr.sh holler` against the fake `holler` (`proof_fakes/holler`), on the host of `proof.rs`
//! with A already up. Then a rerun converges, the teardown lines empty the registry, and A's
//! snapshot never changes.
//!
//! The fake's `profile apply` creates in `cell_key`'s order (column 1 by row, then the rest by row
//! and column) and its planner refuses a cell Herdr cannot split to (E40). Those are a test of the
//! fake that keeps the scenario honest, not of the adapter: #853 owns the Rust test of the real
//! order. Unix-only: the scripts are bash.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use super::panes_803::{calls, config, env_literals, lib, panes_of, TWO_BY_TWO};
use super::proof::{
    first_up, holler_calls, ok, record, snapshot, start_detached, stdout, text, Host, Inst,
};

/// How the skill reaches an instance: its `WIZARD_*` values and the literals `panes.sh env` printed.
struct Door {
    vars: Vec<(String, String)>,
    config: PathBuf,
    home: PathBuf,
}

impl Door {
    /// B, the second instance of `proof.rs`, with `cols` as its layout.
    fn second(host: &Host, b: &Inst, cols: &[&[&str]]) -> Door {
        let inst = format!(
            "[instance]\nname = \"{}\"\nhub_port = {}\nserve_https_port = {}\nstate_dir = \"{}\"\n\
             herdr_session = \"{}\"\nbackend_port_base = 52001\n",
            b.name,
            b.hub_port,
            b.serve_port,
            b.state.display(),
            b.herdr_session
        );
        let vars = vec![
            ("WIZARD_INSTANCE_NAME".to_owned(), b.name.clone()),
            ("WIZARD_HERDR_SESSION".to_owned(), b.herdr_session.clone()),
            ("WIZARD_STATE_DIR".to_owned(), b.state.display().to_string()),
        ];
        Door::with(
            host,
            &format!("{}-sessions.toml", b.name),
            &inst,
            cols,
            vars,
        )
    }

    /// The default instance, whose state directory is `state`.
    fn default(host: &Host, state: &Path) -> Door {
        let vars = vec![
            ("WIZARD_INSTANCE_NAME".to_owned(), "default".to_owned()),
            ("WIZARD_HERDR_SESSION".to_owned(), String::new()),
            ("WIZARD_STATE_DIR".to_owned(), state.display().to_string()),
        ];
        Door::with(host, "default-sessions.toml", "", &TWO_BY_TWO, vars)
    }

    fn with(
        host: &Host,
        file: &str,
        inst: &str,
        cols: &[&[&str]],
        mut vars: Vec<(String, String)>,
    ) -> Door {
        let config = host.path("scratch").join(file);
        fs::write(&config, config_of(inst, cols)).unwrap();
        let home = host.path("home");
        let door = Door {
            vars: Vec::new(),
            config,
            home,
        };
        door.panes(&["check"]);
        vars.extend(env_literals(&door.panes(&["env"])));
        Door { vars, ..door }
    }

    /// Rewrites the config with another layout (a pane added on a rerun).
    fn relayout(&self, inst: &str, cols: &[&[&str]]) {
        fs::write(&self.config, config_of(inst, cols)).unwrap();
    }

    /// `panes.sh <verb> <config> <rest...>`'s stdout; it must exit 0.
    fn panes(&self, args: &[&str]) -> String {
        let mut c = std::process::Command::new("bash");
        c.arg(lib().join("panes.sh"))
            .arg(args[0])
            .arg(&self.config)
            .args(&args[1..])
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME");
        stdout(&ok(c.output().unwrap()))
    }

    /// The profile name `grid` prints.
    fn profile(&self) -> String {
        let grid = self.panes(&["grid"]);
        let p = grid.lines().find_map(|l| l.strip_prefix("profile\t"));
        p.expect("grid prints the profile").to_owned()
    }

    /// `herdr.sh holler <argv>` as the skill types it: `<herdr_env> bash $WIZARD_LIB/herdr.sh holler`.
    fn holler(&self, host: &Host, argv: &[String]) -> Output {
        let mut c = host.script("herdr.sh");
        c.envs(self.vars.iter().map(|(k, v)| (k, v)))
            .arg("holler")
            .args(argv);
        c.output().unwrap()
    }

    /// Runs every line of `panes.sh <verb> ...`, each of which must exit 0; returns the outputs.
    fn run_all(&self, host: &Host, args: &[&str]) -> Vec<(String, Output)> {
        let lines = calls(&self.panes(args));
        assert!(!lines.is_empty(), "{args:?} printed nothing");
        lines
            .into_iter()
            .map(|(tag, argv)| {
                let out = self.holler(host, &argv);
                assert!(out.status.success(), "{tag} {argv:?}: {}", text(&out));
                (tag, out)
            })
            .collect()
    }

    /// The `data` of a JSON call through the door.
    fn data(&self, host: &Host, args: &[&str]) -> Value {
        let argv: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let out = ok(self.holler(host, &argv));
        let v: Value = serde_json::from_str(stdout(&out).trim()).unwrap();
        assert_eq!(v["ok"], Value::Bool(true), "{v}");
        v["data"].clone()
    }

    /// (name, pos, profile) of every registry pane.
    fn registry(&self, host: &Host) -> Vec<(String, String, String)> {
        let data = self.data(host, &["pane", "list", "--format=json"]);
        let rows = data["panes"].as_array().unwrap().iter();
        rows.map(|p| (s(&p["name"]), s(&p["pos"]["pos"]), s(&p["profile"])))
            .collect()
    }
}

/// A config whose panes are `cols`, `r1c1` the orchestrator, after the `[instance]` table `inst`.
fn config_of(inst: &str, cols: &[&[&str]]) -> String {
    config("", cols, inst, &panes_of(cols, &["r1c1"]))
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_owned()
}

/// Stage 4: the instance's hub on its own port and state directory, recorded `--stage 4`.
fn stage4_hub(host: &Host, i: &Inst) {
    let listen = format!("127.0.0.1:{}", i.hub_port);
    fs::create_dir_all(i.logs()).unwrap();
    let log = i.logs().join(format!("{}-hub.log", i.name));
    let pid = start_detached(
        host,
        Some(&i.state),
        &log,
        &["holler", "hub", "serve", "--listen", &listen],
    );
    record(host, i, pid, "hub", 4, "");
}

/// The dry run's steps as "pane:action".
fn steps(dry_run: &Output) -> Vec<String> {
    let v: Value = serde_json::from_str(stdout(dry_run).trim()).unwrap();
    let steps = v["data"]["steps"].as_array().cloned().unwrap_or_default();
    steps
        .iter()
        .map(|st| format!("{}:{}", s(&st["pane"]), s(&st["action"])))
        .collect()
}

fn tagged<'a>(outs: &'a [(String, Output)], tag: &str) -> &'a Output {
    &outs
        .iter()
        .find(|(t, _)| t == tag)
        .unwrap_or_else(|| panic!("no {tag} line"))
        .1
}

/// A up, then B's hub and B's panes built through the plan.
fn b_built(host: &Host) -> (Inst, Inst, Door, String, super::proof::Snap) {
    let (a, before) = first_up(host);
    let b = Inst::second(host);
    let door = Door::second(host, &b, &TWO_BY_TWO);
    stage4_hub(host, &b);
    let p = door.profile();
    let outs = door.run_all(host, &["plan", &p]);
    let want = ["r1c1:create", "r2c1:create", "r1c2:create", "r2c2:create"];
    assert_eq!(
        steps(tagged(&outs, "dry-run")),
        want,
        "a fresh build: every pane created"
    );
    assert_eq!(before, snapshot(host, &a), "A changed during B's build");
    (a, b, door, p, before)
}

#[test]
fn instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched() {
    let host = Host::new();
    let (a, b, door, p, before) = b_built(&host);
    let want: Vec<(String, String, String)> = ["r1c1", "r1c2", "r2c1", "r2c2"]
        .iter()
        .map(|n| ((*n).to_owned(), (*n).to_owned(), p.clone()))
        .collect();
    assert_eq!(door.registry(&host), want);
    let findings = door.data(&host, &["pane", "doctor", "--format=json"])["findings"].clone();
    assert_eq!(findings, Value::Array(vec![]), "{findings}");
    let b_dir = b.state.display().to_string();
    let pane_calls: Vec<_> = holler_calls(&host)
        .into_iter()
        .filter(|(_, args)| args.starts_with("pane ") || args.starts_with("profile "))
        .collect();
    assert!(pane_calls.len() >= 7, "{pane_calls:?}");
    for (state, args) in &pane_calls {
        assert_eq!(
            *state, b_dir,
            "`holler {args}` went without B's state directory"
        );
    }
    assert_eq!(before, snapshot(&host, &a));
    assert!(
        !host.path("home/.holler").exists(),
        "something wrote $HOME/.holler"
    );
}

#[test]
fn a_rerun_of_the_same_sequence_converges_with_no_create_and_every_pane_adopted() {
    let host = Host::new();
    let (_, _, door, p, _) = b_built(&host);
    let registry = door.registry(&host);
    let outs = door.run_all(&host, &["plan", &p, "--rerun"]);
    assert!(
        outs.iter().all(|(t, _)| t != "profile-create"),
        "a rerun creates no profile"
    );
    let want = ["r1c1:adopt", "r2c1:adopt", "r1c2:adopt", "r2c2:adopt"];
    assert_eq!(steps(tagged(&outs, "dry-run")), want);
    assert_eq!(door.registry(&host), registry);
}

#[test]
fn the_teardown_lines_leave_an_empty_registry_and_no_profile() {
    let host = Host::new();
    let (a, _, door, p, before) = b_built(&host);
    door.run_all(&host, &["teardown", &p]);
    assert_eq!(door.registry(&host), vec![]);
    let profiles = door.data(&host, &["profile", "list", "--format=json"]);
    assert_eq!(profiles, Value::Array(vec![]), "{profiles}");
    assert_eq!(before, snapshot(&host, &a));
}

/// The text-mode apply of `p`: the call whose exit code and lines D9's Gate reads.
fn apply_text(door: &Door, host: &Host, p: &str, session: &str) -> Output {
    let argv: Vec<String> = ["profile", "apply", p, "--herdr-session", session]
        .iter()
        .map(|a| (*a).to_owned())
        .collect();
    door.holler(host, &argv)
}

/// Exit 3 with nothing on stdout and one `error:` line naming `code` (ADR-0021 decision 33).
fn assert_refused_step(out: &Output, code: &str) {
    assert_eq!(out.status.code(), Some(3), "{}", text(out));
    assert_eq!(stdout(out), "", "a refused run prints no steps");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "{err}");
    assert!(
        lines[0].starts_with("error: apply refused") && lines[0].contains(code),
        "{err}"
    );
}

#[test]
fn a_pane_of_another_profile_is_a_refused_step_exit_3_and_an_empty_stdout() {
    let host = Host::new();
    let b = Inst::second(&host);
    let door = Door::second(&host, &b, &TWO_BY_TWO);
    stage4_hub(&host, &b);
    let p = door.profile();
    // Someone else's profile already holds a pane named r1c1, at another workspace's r1c1.
    let other: Vec<Vec<String>> = vec![
        vec!["profile", "create", "other"],
        vec![
            "pane",
            "launch",
            "r1c1",
            "--profile",
            "other",
            "--spec-only",
            "--workspace",
            "elsewhere",
            "--grid",
            "r1c1",
            "--project",
            "/srv/x",
            "--model",
            "p/m",
            "--effort",
            "low",
            "--role",
            "orchestrator",
            "--ctx-soft",
            "1",
            "--ctx-hard",
            "2",
            "--port-policy",
            "fixed:53001",
        ],
        vec![
            "profile",
            "apply",
            "other",
            "--herdr-session",
            &b.herdr_session,
        ],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(str::to_owned).collect())
    .collect();
    for argv in &other {
        ok(door.holler(&host, argv));
    }
    let lines = calls(&door.panes(&["plan", &p]));
    for (tag, argv) in lines.iter().filter(|(t, _)| t != "apply") {
        let out = door.holler(&host, argv);
        assert!(out.status.success(), "{tag}: {}", text(&out));
    }
    assert_refused_step(
        &apply_text(&door, &host, &p, &b.herdr_session),
        "pane-in-other-profile",
    );
}

#[test]
fn a_new_row_below_a_live_row_of_two_panes_is_a_refused_grid_unreachable_step() {
    let host = Host::new();
    let b = Inst::second(&host);
    let one_row: [&[&str]; 2] = [&["r1c1"], &["r1c2"]];
    let door = Door::second(&host, &b, &one_row);
    stage4_hub(&host, &b);
    let p = door.profile();
    door.run_all(&host, &["plan", &p]);
    let inst = fs::read_to_string(&door.config).unwrap();
    let inst = &inst[inst.find("[instance]").unwrap()..inst.find("[[pane]]").unwrap()];
    door.relayout(inst, &TWO_BY_TWO);
    let lines = calls(&door.panes(&["plan", &p, "--rerun"]));
    for (tag, argv) in lines.iter().filter(|(t, _)| t != "apply") {
        let out = door.holler(&host, argv);
        assert!(out.status.success(), "{tag}: {}", text(&out));
    }
    assert_refused_step(
        &apply_text(&door, &host, &p, &b.herdr_session),
        "grid-unreachable",
    );
    let at: Vec<String> = door
        .registry(&host)
        .into_iter()
        .map(|(_, pos, _)| pos)
        .collect();
    assert_eq!(
        at,
        ["r1c1", "r1c2"],
        "nothing moved, nothing created below row 1"
    );
}

#[test]
fn the_default_instances_apply_agrees_with_its_environment_on_the_session_default() {
    let host = Host::new();
    let state = host.path("home/.holler");
    let door = Door::default(&host, &state);
    let listen = "127.0.0.1:0";
    let log = host.path("scratch/default-hub.log");
    start_detached(
        &host,
        Some(&state),
        &log,
        &["holler", "hub", "serve", "--listen", listen],
    );
    let p = door.profile();
    assert_eq!(p, "holler");
    door.run_all(&host, &["plan", &p]);
    assert_eq!(door.registry(&host).len(), 4);
}
