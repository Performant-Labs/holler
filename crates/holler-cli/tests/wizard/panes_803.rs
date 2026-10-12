//! #803: `lib/panes.sh`, the one helper the wizard's pane build adds (D1). It reads the `[[pane]]`
//! config and prints what the skill quotes: `check` (exit 1 naming every problem's key), `grid`
//! (the cells), `env` (the environment literals of D8), `plan` (the ordered `holler` arguments of
//! D9) and `teardown` (D15).
//!
//! The printed contract these tests pin (T's choice, recorded in `handoff-T-red.md`):
//! - `grid`: tab-separated lines `workspace <label>`, `extent <R>x<C>`, `profile <P>`, then one
//!   `cell <rRcC> <name> <harness> <role> <dir> <port or ->` per pane in layout order.
//! - `env`: shell assignments (`NAME=<shell-quoted value>`), one per line, safe to `eval`.
//! - `plan` and `teardown`: one line per call, `<tag> <holler arguments, shell-quoted>`;
//!   `plan CONFIG P --rerun` leaves out the `profile-create` line (A's W3).
//!
//! The profile name P is the workspace label (A's W2): `holler` when `workspace` is unset.
//! Nothing here runs Herdr, tmux or Holler: the script is a pure function of the file and the
//! environment. Unix-only: the script is bash.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(super) fn lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent-skills/herdr-setup/lib")
}

pub(super) fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub(super) fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The top-level defaults every pane inherits (D4, D12's proposals).
pub(super) const DEFAULTS: &str =
    "model = \"provider/model-a\"\neffort = \"medium\"\nctx_soft = 800000\nctx_hard = 900000\n";

/// A Claude Code pane's own lines (D4: it never inherits the top-level model).
const CLAUDE: &str = "harness = \"claude\"\nmodel = \"anthropic/claude-sonnet-5-5\"\n";

/// One `[[pane]]` table in `~/Projects/demo`, with extra lines.
pub(super) fn pane(name: &str, role: &str, extra: &str) -> String {
    format!("[[pane]]\nname = \"{name}\"\ndir = \"~/Projects/demo\"\nrole = \"{role}\"\n{extra}\n")
}

/// `layout = [["a", "b"], ["c"]]`
fn layout(cols: &[&[&str]]) -> String {
    let cols: Vec<String> = cols
        .iter()
        .map(|c| format!("[\"{}\"]", c.join("\", \"")))
        .collect();
    format!("layout = [{}]\n", cols.join(", "))
}

/// Top-level keys (the defaults, `top`, the layout), then `instance`, then the panes.
pub(super) fn config(top: &str, cols: &[&[&str]], instance: &str, panes: &str) -> String {
    format!("{DEFAULTS}{top}{}\n{instance}\n{panes}", layout(cols))
}

/// One OpenCode pane per layout name; `orch` names the orchestrators.
pub(super) fn panes_of(cols: &[&[&str]], orch: &[&str]) -> String {
    let role = |n: &&str| {
        if orch.contains(n) {
            "orchestrator"
        } else {
            "agent"
        }
    };
    cols.iter()
        .flat_map(|c| c.iter())
        .map(|n| pane(n, role(n), ""))
        .collect()
}

/// A non-default `[instance]` table whose state directory is `state`.
pub(super) fn second_instance(state: &Path, base: Option<u16>) -> String {
    let base = base.map_or(String::new(), |b| format!("backend_port_base = {b}\n"));
    format!(
        "[instance]\nname = \"second\"\nhub_port = 41900\nserve_https_port = 8443\n\
         state_dir = \"{}\"\nherdr_session = \"second\"\n{base}",
        state.display()
    )
}

/// A scratch HOME holding the config file.
pub(super) struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    pub(super) fn new() -> Scratch {
        Scratch {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    pub(super) fn home(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn write(&self, body: &str) -> PathBuf {
        let p = self.home().join("sessions.toml");
        fs::write(&p, body).unwrap();
        p
    }

    /// `bash panes.sh <verb> <config of body> <rest...>`, HOME the scratch, no XDG_CONFIG_HOME.
    pub(super) fn cmd(&self, verb: &str, body: &str, rest: &[&str]) -> Command {
        let mut c = Command::new("bash");
        c.arg(lib().join("panes.sh"))
            .arg(verb)
            .arg(self.write(body))
            .args(rest)
            .env("HOME", self.home())
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("WIZARD_LIB")
            .current_dir(self.home());
        c
    }

    pub(super) fn verb(&self, verb: &str, body: &str, rest: &[&str]) -> Output {
        self.cmd(verb, body, rest).output().unwrap()
    }

    /// The stdout of a run that must exit 0.
    pub(super) fn stdout(&self, verb: &str, body: &str, rest: &[&str]) -> String {
        let o = self.verb(verb, body, rest);
        assert_eq!(o.status.code(), Some(0), "{verb}: {}\n{body}", err(&o));
        out(&o)
    }

    fn passes(&self, body: &str) {
        self.stdout("check", body, &[]);
    }

    /// `check` on `body` exits 1 and its stderr names every word of `names`.
    fn refuses(&self, body: &str, names: &[&str]) {
        let o = self.verb("check", body, &[]);
        let e = err(&o);
        assert_eq!(o.status.code(), Some(1), "accepted:\n{body}\n{e}");
        for n in names {
            assert!(e.contains(n), "does not name `{n}`: {e}\n{body}");
        }
    }
}

/// The shell words of `text`, as bash reads them (quoting included).
pub(super) fn words(text: &str) -> Vec<String> {
    let o = Command::new("bash")
        .args(["-c", &format!("printf '%s\\0' {text}")])
        .output()
        .unwrap();
    assert!(o.status.success(), "not shell words: {text}");
    out(&o)
        .split('\0')
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `plan` or `teardown` output as (tag, holler argv) per line.
pub(super) fn calls(stdout: &str) -> Vec<(String, Vec<String>)> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (tag, rest) = l.split_once(' ').unwrap_or((l, ""));
            (tag.to_owned(), words(rest))
        })
        .collect()
}

/// The three `env` literals after `eval` in bash: (name, value) for each one that is set.
pub(super) fn env_literals(env_output: &str) -> Vec<(String, String)> {
    let script = "eval \"$1\" || exit 9; for v in WIZARD_HERDR_WORKSPACES WIZARD_HERDR_SOCKET \
                  WIZARD_TMUX_SOCKET; do [ -n \"${!v+set}\" ] && printf '%s=%s\\0' \"$v\" \
                  \"${!v}\"; done; exit 0";
    let o = Command::new("bash")
        .args(["-c", script, "_", env_output])
        .env_remove("WIZARD_HERDR_WORKSPACES")
        .env_remove("WIZARD_HERDR_SOCKET")
        .env_remove("WIZARD_TMUX_SOCKET")
        .output()
        .unwrap();
    assert!(o.status.success(), "env output does not eval: {env_output}");
    out(&o)
        .split('\0')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// The value after `flag` in `argv`.
pub(super) fn flag<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    let at = argv.iter().position(|w| w == flag)?;
    argv.get(at + 1).map(String::as_str)
}

/// `grid`'s `cell` rows as [pos, name, harness, role, dir, port].
fn cells(stdout: &str) -> Vec<Vec<String>> {
    stdout
        .lines()
        .filter_map(|l| l.strip_prefix("cell\t"))
        .map(|l| l.split('\t').map(str::to_owned).collect())
        .collect()
}

fn cell(stdout: &str, name: &str) -> Vec<String> {
    let found = cells(stdout).into_iter().find(|c| c[1] == name);
    found.unwrap_or_else(|| panic!("no cell {name}: {stdout}"))
}

fn keyed(stdout: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}\t");
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(&prefix).map(str::to_owned))
}

/// The first-run fixture of epic #769 (E67): eight orchestrators in four columns of two.
const FIRST_RUN: [&[&str]; 4] = [
    &["r1c1", "r2c1"],
    &["r1c2", "r2c2"],
    &["r1c3", "r2c3"],
    &["r1c4", "r2c4"],
];

pub(super) fn first_run() -> String {
    let all: Vec<&str> = FIRST_RUN.concat();
    config("", &FIRST_RUN, "", &panes_of(&FIRST_RUN, &all))
}

pub(super) const TWO_BY_TWO: [&[&str]; 2] = [&["r1c1", "r2c1"], &["r1c2", "r2c2"]];

pub(super) fn two_by_two(top: &str) -> String {
    config(top, &TWO_BY_TWO, "", &panes_of(&TWO_BY_TWO, &["r1c1"]))
}

/// Three panes in a staircase: `o1` (the orchestrator) over `a1`, and `b1` beside them.
const STAIR: [&[&str]; 2] = [&["o1", "a1"], &["b1"]];

fn stair(instance: &str, o1: &str, a1: &str, b1: &str) -> String {
    let panes =
        pane("o1", "orchestrator", o1) + &pane("a1", "agent", a1) + &pane("b1", "agent", b1);
    config("", &STAIR, instance, &panes)
}

fn port(n: u32) -> String {
    format!("backend_port = {n}\n")
}

/// A 4 x 4 grid: column 2 is Claude Code, `r1c3` runs the OpenCode agent `build`, the
/// orchestrator is at r4c4. Returns the config and the names in layout order.
pub(super) fn mixed() -> (String, Vec<String>) {
    let cols: Vec<Vec<String>> = (1..=4)
        .map(|c| (1..=4).map(|r| format!("r{r}c{c}")).collect())
        .collect();
    let order: Vec<String> = cols.concat();
    let mut body = String::new();
    for name in &order {
        let role = if name == "r4c4" {
            "orchestrator"
        } else {
            "agent"
        };
        let extra = match name.as_str() {
            n if n.ends_with("c2") => CLAUDE,
            "r1c3" => "agent = \"build\"\n",
            _ => "",
        };
        body.push_str(&pane(name, role, extra));
    }
    let refs: Vec<Vec<&str>> = cols
        .iter()
        .map(|c| c.iter().map(String::as_str).collect())
        .collect();
    let refs: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
    (config("", &refs, "", &body), order)
}

#[test]
fn the_first_run_grid_keeps_its_eight_names_at_their_cells_with_extent_2x4() {
    let s = Scratch::new();
    s.passes(&first_run());
    let g = s.stdout("grid", &first_run(), &[]);
    assert_eq!(keyed(&g, "extent").as_deref(), Some("2x4"), "{g}");
    let got: Vec<String> = cells(&g)
        .iter()
        .map(|c| format!("{}={}", c[0], c[1]))
        .collect();
    let want: Vec<String> = FIRST_RUN
        .concat()
        .iter()
        .map(|n| format!("{n}={n}"))
        .collect();
    assert_eq!(got, want, "{g}");
    let demo = s.home().join("Projects/demo").display().to_string();
    assert!(
        cells(&g).iter().all(|c| c[4] == demo),
        "~/ not expanded: {g}"
    );
}

#[test]
fn a_mixed_4x4_grid_has_sixteen_distinct_cells() {
    let s = Scratch::new();
    let (body, _) = mixed();
    s.passes(&body);
    let g = s.stdout("grid", &body, &[]);
    assert_eq!(keyed(&g, "extent").as_deref(), Some("4x4"), "{g}");
    let mut pos: Vec<String> = cells(&g).iter().map(|c| c[0].clone()).collect();
    pos.sort();
    pos.dedup();
    assert_eq!(pos.len(), 16, "{g}");
    assert_eq!(
        cell(&g, "r4c4")[..4],
        ["r4c4", "r4c4", "opencode", "orchestrator"]
    );
    let claude = cell(&g, "r1c2");
    assert_eq!(
        (claude[2].as_str(), claude[5].as_str()),
        ("claude", "-"),
        "{g}"
    );
}

#[test]
fn the_profile_is_named_for_the_workspace_label_which_defaults_to_holler() {
    let s = Scratch::new();
    let g = s.stdout("grid", &two_by_two(""), &[]);
    assert_eq!(keyed(&g, "workspace").as_deref(), Some("holler"), "{g}");
    assert_eq!(keyed(&g, "profile").as_deref(), Some("holler"), "{g}");
    let g = s.stdout("grid", &two_by_two("workspace = \"relay 2\"\n"), &[]);
    assert_eq!(keyed(&g, "workspace").as_deref(), Some("relay 2"), "{g}");
    assert_eq!(keyed(&g, "profile").as_deref(), Some("relay 2"), "{g}");
}

#[test]
fn a_claude_pane_with_an_agent_a_port_or_no_model_of_its_own_is_refused() {
    let s = Scratch::new();
    s.passes(&stair("", "", CLAUDE, ""));
    s.refuses(
        &stair("", "", &format!("{CLAUDE}agent = \"build\"\n"), ""),
        &["agent", "a1"],
    );
    s.refuses(
        &stair("", "", "harness = \"claude\"\n", ""),
        &["model", "a1"],
    );
    s.refuses(
        &stair("", "", &format!("{CLAUDE}{}", port(49001)), ""),
        &["backend_port"],
    );
}

#[test]
fn a_column_taller_than_the_one_to_its_left_is_refused_naming_layout_and_the_column() {
    let s = Scratch::new();
    let names =
        pane("o1", "orchestrator", "") + &pane("alpha", "agent", "") + &pane("beta", "agent", "");
    let tall = config("", &[&["o1"], &["alpha", "beta"]], "", &names);
    s.refuses(&tall, &["layout", "column 2"]);
    s.passes(&config("", &[&["alpha", "beta"], &["o1"]], "", &names));
}

#[test]
fn every_retired_table_and_key_is_refused_by_name_with_the_word_attach() {
    let s = Scratch::new();
    let with = |extra: &str, tail: &str| stair("", extra, "", "") + tail;
    let cases = [
        (
            with(
                "",
                "[[orchestrator]]\nname = \"x\"\ndir = \"/x\"\ncmd = \"claude\"\n",
            ),
            "orchestrator",
        ),
        (
            with(
                "",
                "[[session]]\nname = \"alpha\"\nharness = \"opencode\"\n",
            ),
            "session",
        ),
        (
            with("endpoint = \"http://127.0.0.1:47001\"\n", ""),
            "endpoint",
        ),
        (with("mode = \"attach\"\n", ""), "mode"),
        (with("session_id = \"ses_1\"\n", ""), "session_id"),
        (with("remote_host = \"remote-a\"\n", ""), "remote_host"),
        (
            with("remote_tailnet_host = \"remote-a.example.ts.net\"\n", ""),
            "remote_tailnet_host",
        ),
        (with("cmd = \"claude\"\n", ""), "cmd"),
    ];
    for (body, key) in &cases {
        s.refuses(body, &[key, "attach"]);
    }
    let hub_host = format!(
        "hub_host = \"hub.example.ts.net\"\n{}",
        stair("", "", "", "")
    );
    let o = s.verb("check", &hub_host, &[]);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(0), "{e}");
    let lines: Vec<&str> = e.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "one warning line: {lines:?}");
    assert!(lines[0].contains("hub_host"), "{lines:?}");
}

#[test]
fn every_other_validity_rule_is_refused_naming_its_key() {
    let s = Scratch::new();
    let good = stair("", "", "", "");
    s.passes(&good);
    for bad in ["Alpha", "my_pane", "-a", &"a".repeat(33)] {
        s.refuses(&good.replace("\"a1\"", &format!("\"{bad}\"")), &["name"]);
    }
    s.refuses(&(good.clone() + &pane("a1", "agent", "")), &["name", "a1"]);
    let panes = good
        .split_once("[[pane]]")
        .map(|(_, p)| format!("[[pane]]{p}"))
        .unwrap();
    s.refuses(&config("", &[&["o1", "a1"]], "", &panes), &["layout", "b1"]);
    s.refuses(
        &config("", &[&["o1", "a1"], &["b1", "zz"]], "", &panes),
        &["layout", "zz"],
    );
    s.refuses(
        &good.replacen("\"~/Projects/demo\"", "\"Projects/demo\"", 1),
        &["dir"],
    );
    s.refuses(&stair("", "", "harness = \"codex\"\n", ""), &["harness"]);
    s.refuses(
        &good.replacen("role = \"agent\"", "role = \"boss\"", 1),
        &["role"],
    );
    s.refuses(
        &good.replace("role = \"orchestrator\"", "role = \"agent\""),
        &["orchestrator"],
    );
    for key in ["model", "effort", "ctx_soft", "ctx_hard"] {
        let kept: String = DEFAULTS
            .lines()
            .filter(|l| !l.starts_with(&format!("{key} ")))
            .map(|l| format!("{l}\n"))
            .collect();
        s.refuses(&good.replacen(DEFAULTS, &kept, 1), &[key]);
    }
    s.refuses(&stair("", "ctx_soft = 950000\n", "", ""), &["ctx_soft"]);
    for label in ["a=b", "a,b"] {
        s.refuses(&format!("workspace = \"{label}\"\n{good}"), &["workspace"]);
    }
}

#[test]
fn opencode_ports_are_base_plus_index_or_their_own_and_bad_ports_are_refused() {
    let s = Scratch::new();
    let state = s.home().join("state-second");
    let (based, unbased) = (
        second_instance(&state, Some(48001)),
        second_instance(&state, None),
    );
    let g = s.stdout("grid", &stair(&based, &port(49100), "", ""), &[]);
    let ports: Vec<String> = cells(&g).iter().map(|c| c[5].clone()).collect();
    assert_eq!(ports, ["49100", "48002", "48003"], "{g}");
    // The first instance's default range is refused unless every pane sets its own port.
    s.refuses(&stair(&unbased, "", "", ""), &["backend_port_base"]);
    s.passes(&stair(&unbased, &port(49100), &port(49101), &port(49102)));
    s.refuses(
        &stair(&based, &port(49100), &port(49100), ""),
        &["backend_port", "49100"],
    );
    let high = second_instance(&state, Some(65534));
    s.refuses(&stair(&high, "", "", ""), &["65535"]);
    s.refuses(&stair(&based, &port(70000), "", ""), &["backend_port"]);
}

#[test]
fn the_default_range_panes_sh_refuses_starts_where_instance_sh_says() {
    let s = Scratch::new();
    let printed = Command::new("bash")
        .arg(lib().join("instance.sh"))
        .arg(s.write(&stair("", "", "", "")))
        .env("HOME", s.home())
        .output()
        .unwrap();
    let base = out(&printed)
        .split_whitespace()
        .find_map(|w| w.strip_prefix("backend_port_base=").map(str::to_owned))
        .expect("instance.sh prints backend_port_base");
    let inst = second_instance(&s.home().join("st"), None);
    s.refuses(&stair(&inst, "", "", ""), &["backend_port_base", &base]);
}

#[test]
fn no_wizard_script_uses_a_bash_4_feature() {
    let mut names: Vec<String> = fs::read_dir(lib())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".sh"))
        .collect();
    names.sort();
    assert!(
        names.iter().any(|n| n == "panes.sh"),
        "no lib/panes.sh: {names:?}"
    );
    for name in &names {
        let src = fs::read_to_string(lib().join(name)).unwrap();
        for (i, line) in src.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with('#') {
                continue;
            }
            for bad in [
                "declare -A",
                "local -A",
                "mapfile",
                "readarray",
                ",,}",
                "^^}",
                "|&",
            ] {
                assert!(
                    !code.contains(bad),
                    "{name}:{}: `{bad}` is not bash 3.2",
                    i + 1
                );
            }
        }
    }
}
