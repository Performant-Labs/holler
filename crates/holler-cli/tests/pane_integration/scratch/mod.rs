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
//!    `HERDR_*`, no `TMUX`, no `TMUX_PANE`, no `HOLLER_*` the test did not set).
//! 5. *OpenCode's environment* ([`tmux_server_env`], W-1): the test starts the private tmux
//!    server itself, with exactly the rig's isolated pairs, before any verb runs. Under D7's
//!    `ProcessEnv::Inherit` every OpenCode process inherits that server's environment, so no
//!    OpenCode the scenario starts can read the operator's config, auth or session store. The
//!    model guard (D12) then runs on every new server right after its verb returns.

#[path = "../../../../holler-adapter-herdr/tests/scratch_herdr/mod.rs"]
pub mod scratch_herdr;

#[path = "../../../../holler-adapter-opencode/tests/real_opencode/rig.rs"]
pub mod rig;

#[path = "../../../../holler-adapter-host/tests/real_tmux/mod.rs"]
pub mod real_tmux;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_herdr::plan::Extent;
use holler_adapter_host::TmuxHost;
use holler_pane::{HerdrPane, HerdrPort, PaneName};
use holler_pane_testkit::envelope::check_envelope;
use tempfile::TempDir;

use crate::support::{holler_bin, Hub, StateDir};
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

/// The workspaces the wiring is told about, each 2 rows by 1 column (`label=RxC`).
pub const WORKSPACES: &str = "hlr649=2x1,hlr649b=2x1,hlr649g=2x1,hlr649h=2x1";

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

/// One scenario's scratch set. Dropped in field order: the hub, then Herdr (stopped), then
/// the tmux server (killed by its socket, which ends every process under it: the OpenCode
/// servers and TUIs the verbs started), then the state dir, then the directory itself.
pub struct Scratch {
    pub hub: Option<Hub>,
    pub herdr: ScratchHerdr,
    pub host: TmuxHost,
    pub tmux: real_tmux::Server,
    pub state: StateDir,
    pub settings: Settings,
    pub tmux_bin: PathBuf,
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
        let (host, tmux) =
            real_tmux::isolated(&base, &tmux_server_env(&opencode, &oc_root), ANCHOR);
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
        Some(Scratch {
            hub: Some(hub),
            herdr,
            host,
            tmux,
            state,
            settings,
            tmux_bin,
            base: dir,
        })
    }

    /// `holler <args>` with exactly [`binary_env`], in the scratch directory. The pane a
    /// `pane` verb names (its first positional argument) is checked first (guard 2).
    pub fn holler(&self, args: &[&str]) -> Out {
        if let ["pane", _, pane, ..] = args {
            if !pane.starts_with("--") {
                check_pane_name(pane).unwrap_or_else(|refusal| panic!("{refusal}"));
            }
        }
        let env = binary_env(&self.settings).unwrap_or_else(|refusal| panic!("{refusal}"));
        let mut command = Command::new(holler_bin());
        command
            .env_clear()
            .envs(env)
            .current_dir(&self.settings.base)
            .args(args);
        output_of(command)
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
        let project = self.settings.oc_root.join("proj").display().to_string();
        let policy = format!("fixed:{port}");
        let mut argv = vec![
            "pane",
            "launch",
            pane,
            "--herdr-session",
            self.herdr.session(),
            "--project",
            &project,
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
        ];
        argv.extend_from_slice(extra);
        let out = self.json(&argv, 0);
        rig::no_model_can_be_reached(port);
        out
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
        let extent = Extent { rows: 2, cols: 1 };
        let config = ["hlr649", "hlr649b", "hlr649g", "hlr649h"].into_iter().fold(
            HerdrConfig::new(self.herdr.session(), self.herdr.socket()),
            |config, label| config.with_workspace(label, extent),
        );
        let adapter = HerdrAdapter::connect(config).expect("connect to the scratch Herdr");
        adapter.snapshot().expect("Herdr's snapshot").panes
    }

    /// The private tmux server's global environment, as `NAME=value` lines.
    pub fn tmux_global_env(&self) -> Vec<String> {
        self.tmux.lines(&["show-environment", "-g"])
    }

    /// The pids `run` tagged in the tmux session `pane` (its owned processes, #641).
    pub fn owned_pids(&self, pane: &str) -> Vec<u32> {
        let target = format!("={pane}");
        let out = self.tmux.tmux(&["list-panes", "-s", "-t", &target, "-F", "#{@holler-pid}"]);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }
}
