//! The scenario's scratch set (D10, D11, D12 and A's W-1): one fresh `/tmp/hlr649-*`
//! directory holding a hub's state dir, a scratch Herdr, a private tmux server and
//! OpenCode's whole `HOME`/`XDG_*`, every process started by the test itself, and five
//! guards, each a pure function the default run tests (`guards.rs`).
//!
//! **Reuse, never a copy (D11).** The Herdr guards and server come from #640's scratch module,
//! the dead-end config, the isolated environment, the port picker and the model guard from
//! #642's OpenCode rig, and the private tmux server from #641's tests, each included below with
//! `#[path]` and generalised in place where it had to be (A's W-3).
//!
//! **The guards.**
//!
//! 1. *Opt-in* ([`GATE_VAR`]): only `HOLLER_TEST_PANE_INTEGRATION=1` runs a scenario; opted in,
//!    a missing `herdr`, `tmux` or `opencode` is a failure, not a skip.
//! 2. *Unique names* ([`check_session`], [`check_pane_name`]): the Herdr session is
//!    `holler649-<8 hex>` (`default` refused by name) and every pane name is `hlr649-...`.
//! 3. *One directory* ([`check_inside`]): the state dir, the Herdr socket, the tmux socket and
//!    OpenCode's root all lie inside it; the guard removes it after killing what it started.
//! 4. *Refusal of real names* ([`binary_env`]): every name and path is checked before a verb
//!    runs, and the binary runs with exactly the environment built here, nothing inherited (no
//!    `HERDR_*`, no `TMUX`, no `TMUX_PANE`, no `HOLLER_*` the test did not set). A verb's
//!    arguments may name no Herdr session but the scratch one ([`check_herdr_session_args`],
//!    #667).
//! 5. *OpenCode's environment* ([`tmux_server_env`], W-1): the test starts the private tmux
//!    server itself, with exactly the rig's isolated pairs, before any verb runs. Under D7's
//!    `ProcessEnv::Inherit` every OpenCode process inherits that server's environment, so no
//!    OpenCode the scenario starts can read the operator's config, auth or session store. The
//!    model guard (D12) then runs on every new server right after its verb returns.
//! 6. *Killing what it started* (D10 (6)): the servers and TUIs are started by the private
//!    tmux server and Herdr, not by the test, so [`Scratch`]'s drop finds them by
//!    `tmux list-panes -a` on the scratch socket and `HostPort::ps` per scratch session, sends
//!    `TERM` then `KILL` to their process groups, kills the tmux server, stops Herdr, and only
//!    then removes the directory.
//!
//! **No credential leaks (D28).** [`Scratch::start`] plants a sentinel in this process's
//! environment under the names a provider credential goes by; [`Scratch::assert_no_credential_leaked`]
//! greps every verb's stdout and stderr, every file of the scratch directory and the argv of
//! every process (`ps`) for it.

#[path = "../../../../holler-adapter-herdr/tests/scratch_herdr/mod.rs"]
pub mod scratch_herdr;

#[path = "../../../../holler-adapter-opencode/tests/real_opencode/rig.rs"]
pub mod rig;

#[path = "../../../../holler-adapter-host/tests/real_tmux/mod.rs"]
pub mod real_tmux;

mod verbs;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_herdr::plan::Extent;
use holler_adapter_host::TmuxHost;
use holler_adapter_opencode::{OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig};
use holler_pane::{HerdrPane, HerdrPort, HostPort, PaneError, PaneId, PaneName};
use holler_pane_testkit::envelope::check_envelope;
use tempfile::TempDir;

use crate::support::{holler_bin, wait_for, Hub, StateDir};
use crate::{output_of, Out};
use scratch_herdr::{check_name_as, check_socket, Flavor, ScratchHerdr};

/// Guard 1: the variable that opts in; only the exact value "1" runs a scenario.
pub const GATE_VAR: &str = "HOLLER_TEST_PANE_INTEGRATION";

/// Guard 2: this story's flavor of #640's scratch Herdr.
pub const FLAVOR: Flavor = Flavor {
    gate_var: GATE_VAR,
    name_prefix: "holler649-",
    root_prefix: "h649.",
};

/// Guard 2: every pane (and tmux session) the scenario names starts with this.
pub const PANE_PREFIX: &str = "hlr649-";

/// The tmux session that keeps the private server running; never a pane's.
pub const ANCHOR: &str = "hlr649-anchor";

/// The names a provider credential goes by; [`Scratch::start`] sets each to the sentinel in
/// this process's environment (D28).
pub const SENTINEL_VARS: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENROUTER_API_KEY"];

/// Whether `pid` answers `kill -0` (D10 (6), D15).
pub fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", "--", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The workspaces the wiring is told about (`label=RxC`): four of 2 rows by 1 column, and
/// `hlr649sq`, 2 rows by 2 columns, for the whole grid `profile apply` builds (#853).
pub const WORKSPACES: &str = "hlr649=2x1,hlr649b=2x1,hlr649g=2x1,hlr649h=2x1,hlr649sq=2x2";

/// Each workspace of [`WORKSPACES`] with its extent.
pub fn workspace_extents() -> Vec<(&'static str, Extent)> {
    WORKSPACES
        .split(',')
        .map(|entry| {
            let (label, extent) = entry.split_once('=').expect("label=RxC");
            let (rows, cols) = extent.split_once('x').expect("RxC");
            let rows = rows.parse().expect("a row count");
            let cols = cols.parse().expect("a column count");
            (label, Extent { rows, cols })
        })
        .collect()
}

/// Guard 2: the Herdr session must be `holler649-<8 hex>`; `default` is refused by name.
pub fn check_session(name: &str) -> Result<(), String> {
    check_name_as(FLAVOR.name_prefix, name)
}

/// Guard 2: a pane name the scenario may use: `hlr649-...` and a valid pane name.
pub fn check_pane_name(name: &str) -> Result<(), String> {
    if !name.starts_with(PANE_PREFIX) || name.len() == PANE_PREFIX.len() {
        return Err(format!("refusing the pane name {name:?}: a scratch pane is {PANE_PREFIX}..."));
    }
    PaneName::parse(name)
        .map(drop)
        .map_err(|e| format!("refusing the pane name {name:?}: {e}"))
}

/// Guard 3: `path` is a plain absolute path inside `base` (and short enough for a socket).
pub fn check_inside(base: &Path, path: &Path) -> Result<(), String> {
    check_socket(base, path)
}

/// The one flag by which a verb's argument vector can name a Herdr session.
pub const HERDR_SESSION_FLAG: &str = "--herdr-session";

/// Guard 4, for an argument vector (#667, A's W-2): every `--herdr-session` in `args`, in
/// either spelling clap accepts (`--herdr-session X` and `--herdr-session=X`), names exactly
/// `session`, the scratch set's own. A flag with no value, an empty value or another flag in
/// the value position is refused too. A vector with no such flag passes.
pub fn check_herdr_session_args(args: &[&str], session: &str) -> Result<(), String> {
    let refuse = |value: Option<&str>| {
        Err(format!(
            "refusing {HERDR_SESSION_FLAG} {value:?} in {args:?}: a scratch verb names only the \
             scratch session {session:?}"
        ))
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let value = if *arg == HERDR_SESSION_FLAG {
            rest.next().copied()
        } else if let Some(value) = arg
            .strip_prefix(HERDR_SESSION_FLAG)
            .and_then(|tail| tail.strip_prefix('='))
        {
            Some(value)
        } else {
            continue;
        };
        if value != Some(session) {
            return refuse(value);
        }
    }
    Ok(())
}

/// The argument vector of a `pane launch` of `pane` into the Herdr session `session`, in
/// `workspace` at `grid` on the fixed `port`, in the directory `project`, with the dead-end
/// model and the scratch ceilings, then `extra`. Pure (#667, A's W-5): [`Scratch::launch`] and
/// a spec-only launch build their flags here, and `guards.rs` pins the list.
pub fn launch_args(
    session: &str,
    project: &str,
    pane: &str,
    workspace: &str,
    grid: &str,
    port: u16,
    extra: &[&str],
) -> Vec<String> {
    let policy = format!("fixed:{port}");
    let mut argv: Vec<String> = [
        "pane",
        "launch",
        pane,
        HERDR_SESSION_FLAG,
        session,
        "--project",
        project,
        "--workspace",
        workspace,
        "--grid",
        grid,
        "--model",
        "deadend/none",
        "--effort",
        "medium",
        "--ctx-soft",
        "100000",
        "--ctx-hard",
        "150000",
        "--port-policy",
        &policy,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    argv.extend(extra.iter().map(|&arg| arg.to_owned()));
    argv
}

/// What the binary is pointed at. Every path lies inside `base`.
#[derive(Debug, Clone)]
pub struct Settings {
    pub base: PathBuf,
    pub state_dir: PathBuf,
    pub session: String,
    pub herdr_socket: PathBuf,
    pub tmux_socket: PathBuf,
    pub opencode: PathBuf,
    /// OpenCode's scratch root, laid out by the rig (`home`, `data`, `config`, ...).
    pub oc_root: PathBuf,
    /// The binary's `PATH`: where `tmux`, `kill` and `opencode` are found.
    pub path: String,
}

/// Guard 5: the private tmux server's whole environment, exactly the rig's isolated pairs.
pub fn tmux_server_env(opencode: &Path, oc_root: &Path) -> Vec<(String, String)> {
    rig::isolated_vars(opencode, oc_root)
}

/// Guard 4: the binary's whole environment, built from nothing inherited, after every name
/// and path is checked: OpenCode's scratch `HOME`/`XDG_*` (the rig's), the given `PATH`,
/// deterministic logging, and the wiring's settings. Fails closed on any refusal.
pub fn binary_env(settings: &Settings) -> Result<Vec<(String, String)>, String> {
    check_session(&settings.session)?;
    for path in [
        &settings.state_dir,
        &settings.herdr_socket,
        &settings.tmux_socket,
        &settings.oc_root,
    ] {
        check_inside(&settings.base, path)?;
    }
    let mut env: Vec<(String, String)> = tmux_server_env(&settings.opencode, &settings.oc_root)
        .into_iter()
        .filter(|(name, _)| name == "HOME" || name.starts_with("XDG_"))
        .collect();
    let text = |path: &Path| path.display().to_string();
    env.extend(
        [
            ("PATH", settings.path.clone()),
            ("HOLLER_STATE_DIR", text(&settings.state_dir)),
            ("HOLLER_DEBUG", "quiet".to_owned()),
            ("HOLLER_LOG_FORMAT", "json".to_owned()),
            ("HOLLER_HERDR_SESSION", settings.session.clone()),
            ("HOLLER_HERDR_SOCKET", text(&settings.herdr_socket)),
            ("HOLLER_HERDR_WORKSPACES", WORKSPACES.to_owned()),
            ("HOLLER_TMUX_SOCKET", text(&settings.tmux_socket)),
            ("HOLLER_OPENCODE_BIN", text(&settings.opencode)),
            ("HOLLER_OPENCODE_SERVE_ARGS", r#"["--pure"]"#.to_owned()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v)),
    );
    Ok(env)
}

/// The parent directory of `bin`, as text.
fn dir_of(bin: &Path) -> String {
    bin.parent().map_or_else(String::new, |d| d.display().to_string())
}

/// One scenario's scratch set. Its drop first signals every process the private tmux server
/// runs (guard 6); then the fields drop in order: the hub, then the tmux server (killed by
/// its socket), then Herdr (`server stop`), then the state dir, then the directory itself.
pub struct Scratch {
    pub hub: Option<Hub>,
    pub host: TmuxHost,
    pub tmux: real_tmux::Server,
    pub herdr: ScratchHerdr,
    pub state: StateDir,
    pub settings: Settings,
    pub tmux_bin: PathBuf,
    /// What every verb run printed, stdout then stderr (D28).
    outputs: Mutex<Vec<String>>,
    /// The planted credential (D28).
    sentinel: String,
    /// The sentinel variables as they were before the run, put back on drop (NV-6).
    saved_sentinels: Vec<(&'static str, Option<std::ffi::OsString>)>,
    base: TempDir,
}

impl Scratch {
    /// Guard 1, then the whole set; `None` when the run did not opt in.
    pub fn start() -> Option<Scratch> {
        let herdr_bin = scratch_herdr::opt_in_as(GATE_VAR)?;
        let (opencode, tmux_bin) = rig::binaries();
        assert!(
            rig::refused(9),
            "127.0.0.1:9 accepts connections; the dead-end provider would not be dead. Refusing."
        );
        let dir = tempfile::Builder::new()
            .prefix(PANE_PREFIX)
            .tempdir_in("/tmp")
            .unwrap();
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let oc_root = base.join("oc");
        rig::lay_out(&oc_root);
        let state = StateDir::at(base.join("state"));
        let herdr = ScratchHerdr::start_in(&herdr_bin, FLAVOR, &base);
        let tmux = real_tmux::isolated(&base, &tmux_server_env(&opencode, &oc_root), ANCHOR);
        let host = TmuxHost::new(holler_adapter_host::TmuxSocket::Path(tmux.sock()))
            .with_config(tmux.config());
        let settings = Settings {
            base: base.clone(),
            state_dir: state.path().to_path_buf(),
            session: herdr.session().to_owned(),
            herdr_socket: herdr.socket().to_path_buf(),
            tmux_socket: tmux.sock(),
            path: format!("{}:{}:/usr/bin:/bin", dir_of(&tmux_bin), dir_of(&opencode)),
            opencode,
            oc_root,
        };
        binary_env(&settings).unwrap_or_else(|refusal| panic!("{refusal}"));
        let hub = Hub::start(&state);
        let sentinel = scratch_herdr::new_name_as("hlr649-sentinel-");
        let saved_sentinels = SENTINEL_VARS
            .iter()
            .map(|&name| (name, std::env::var_os(name)))
            .collect();
        for name in SENTINEL_VARS {
            std::env::set_var(name, &sentinel);
        }
        Some(Scratch {
            hub: Some(hub),
            host,
            tmux,
            herdr,
            state,
            settings,
            tmux_bin,
            outputs: Mutex::new(Vec::new()),
            sentinel,
            saved_sentinels,
            base: dir,
        })
    }

    /// `holler <args>` with exactly [`binary_env`], in the scratch directory. The pane a
    /// `pane` verb names (its first positional argument) is checked first (guard 2), and so is
    /// every `--herdr-session` the arguments carry ([`check_herdr_session_args`]).
    pub fn holler(&self, args: &[&str]) -> Out {
        if let ["pane", _, pane, ..] = args {
            if !pane.starts_with("--") {
                check_pane_name(pane).unwrap_or_else(|refusal| panic!("{refusal}"));
            }
        }
        check_herdr_session_args(args, &self.settings.session)
            .unwrap_or_else(|refusal| panic!("{refusal}"));
        let env = binary_env(&self.settings).unwrap_or_else(|refusal| panic!("{refusal}"));
        let mut command = Command::new(holler_bin());
        command
            .env_clear()
            .envs(env)
            .current_dir(&self.settings.base)
            .args(args);
        let out = output_of(command);
        let mut outputs = self.outputs.lock().unwrap_or_else(PoisonError::into_inner);
        outputs.push(out.stdout.clone());
        outputs.push(out.stderr.clone());
        out
    }

    /// The test's own `HarnessPort` on the scratch tmux server, for what the scenario reads
    /// or makes through a port (W-G): sessions, activity, the server argv of D15. It never
    /// attaches a TUI, so its `tui_session` knows no pane.
    pub fn harness(&self) -> OpenCodeHarness {
        OpenCodeHarness::new(OpenCodeConfig {
            opencode_bin: self.settings.opencode.clone(),
            serve_args: vec!["--pure".to_owned()],
            env: ProcessEnv::Inherit,
            tmux: TmuxConfig {
                tmux_bin: self.tmux_bin.clone(),
                socket: holler_adapter_opencode::TmuxSocket::Path(self.tmux.sock()),
            },
            tui_session: Arc::new(|pane: &PaneId| {
                Err(PaneError::PaneNotFound {
                    what: pane.as_str().to_owned(),
                })
            }),
            timeouts: Timeouts::default(),
        })
    }

    /// D28: the planted sentinel is in no verb's output, no file of the scratch directory and
    /// no process's argv.
    pub fn assert_no_credential_leaked(&self) {
        let mut leaks = Vec::new();
        let outputs = self.outputs.lock().unwrap_or_else(PoisonError::into_inner);
        if outputs.iter().any(|text| text.contains(&self.sentinel)) {
            leaks.push("a verb's stdout or stderr".to_owned());
        }
        drop(outputs);
        let mut dirs = vec![self.settings.base.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(kind) = entry.file_type() else { continue };
                if kind.is_dir() {
                    dirs.push(entry.path());
                } else if kind.is_file() {
                    let bytes = std::fs::read(entry.path()).unwrap_or_default();
                    let needle = self.sentinel.as_bytes();
                    if bytes.windows(needle.len()).any(|w| w == needle) {
                        leaks.push(format!("the file {}", entry.path().display()));
                    }
                }
            }
        }
        let ps = Command::new("ps").args(["-eww", "-o", "pid=,args="]).output().expect("ps");
        for line in String::from_utf8_lossy(&ps.stdout).lines() {
            if line.contains(&self.sentinel) {
                leaks.push(format!("a process's argv: {line}"));
            }
        }
        assert!(leaks.is_empty(), "a credential leaked into: {leaks:#?}");
    }

    /// Every process the private tmux server runs: each pane's pid on the scratch socket, and
    /// `HostPort::ps` of each of its sessions (guard 6).
    fn started_pids(&self) -> Vec<u32> {
        let parse = |out: std::process::Output| -> Vec<String> {
            String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_owned()).collect()
        };
        let mut pids: Vec<u32> = parse(self.tmux.tmux(&["list-panes", "-a", "-F", "#{pane_pid}"]))
            .iter()
            .filter_map(|line| line.parse().ok())
            .collect();
        for session in parse(self.tmux.tmux(&["list-sessions", "-F", "#{session_name}"])) {
            if let Ok(name) = PaneName::parse(&session) {
                pids.extend(self.host.ps(&name).unwrap_or_default());
            }
        }
        pids.sort_unstable();
        pids.dedup();
        pids.retain(|&pid| pid > 1);
        pids
    }

    /// [`Scratch::holler`] with `--format=json`, its stdout checked against the envelope
    /// (AC 9), and the exit code `code`.
    pub fn json(&self, args: &[&str], code: i32) -> Out {
        let mut argv = args.to_vec();
        argv.push("--format=json");
        let out = self.holler(&argv);
        check_envelope(&out.stdout, out.code)
            .unwrap_or_else(|fault| panic!("holler {argv:?}: not one envelope: {fault}: {out:?}"));
        assert_eq!(out.code, code, "holler {argv:?}: {out:?}");
        out
    }

    /// `pane launch` of `pane` in `workspace` at `grid` on `port`, with the dead-end model,
    /// then the model guard on its server (D12, W-1 (c)).
    pub fn launch(&self, pane: &str, workspace: &str, grid: &str, port: u16, extra: &[&str]) -> Out {
        let argv = self.launch_argv(pane, workspace, grid, port, extra);
        let out = self.json(&argv.iter().map(String::as_str).collect::<Vec<_>>(), 0);
        rig::no_model_can_be_reached(port);
        out
    }

    /// [`launch_args`] for this scratch set: its Herdr session and its project directory.
    pub fn launch_argv(
        &self,
        pane: &str,
        workspace: &str,
        grid: &str,
        port: u16,
        extra: &[&str],
    ) -> Vec<String> {
        let project = self.settings.oc_root.join("proj").display().to_string();
        launch_args(self.herdr.session(), &project, pane, workspace, grid, port, extra)
    }

    /// Stop the hub and start a new one on the same state dir.
    pub fn restart_hub(&mut self) {
        if let Some(hub) = self.hub.take() {
            hub.stop(Duration::from_secs(5));
        }
        self.hub = Some(Hub::start(&self.state));
    }

    /// Herdr's own snapshot, read by the test through the adapter on the scratch socket.
    pub fn herdr_snapshot(&self) -> Vec<HerdrPane> {
        let config = workspace_extents().into_iter().fold(
            HerdrConfig::new(self.herdr.session(), self.herdr.socket()),
            |config, (label, extent)| config.with_workspace(label, extent),
        );
        let adapter = HerdrAdapter::connect(config).expect(
            "connect to the scratch Herdr (the first `herdr` on PATH must speak protocol 22, \
             Herdr 0.9.1)",
        );
        adapter.snapshot().expect("Herdr's snapshot").panes
    }

    /// The private tmux server's global environment, as `NAME=value` lines.
    pub fn tmux_global_env(&self) -> Vec<String> {
        self.tmux.lines(&["show-environment", "-g"])
    }

    /// The pids `run` tagged in the tmux session `pane` (its owned processes, #641): what
    /// `tmux list-panes` lists for the session's tagged windows.
    pub fn owned_pids(&self, pane: &str) -> Vec<u32> {
        let target = format!("={pane}");
        let out = self.tmux.tmux(&["list-panes", "-s", "-t", &target, "-F", "#{@holler-pid}"]);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }
}

impl Drop for Scratch {
    /// Guard 6: `TERM` then `KILL` to the process group of every process the private tmux
    /// server runs; the fields' own drops then kill the tmux server, stop Herdr and remove the
    /// directory, in that order.
    fn drop(&mut self) {
        let pids = self.started_pids();
        for &pid in &pids {
            let _ = rig::signal_group(pid, "TERM");
        }
        let all_gone = || pids.iter().all(|&pid| !alive(pid)).then_some(());
        if wait_for(Duration::from_secs(2), all_gone).is_none() {
            for &pid in pids.iter().filter(|&&pid| alive(pid)) {
                let _ = rig::signal_group(pid, "KILL");
            }
        }
        let left: Vec<u32> = pids.into_iter().filter(|&pid| alive(pid)).collect();
        if !left.is_empty() {
            eprintln!("scratch: processes still answer kill -0 after KILL: {left:?}");
        }
        for (name, value) in &self.saved_sentinels {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}
