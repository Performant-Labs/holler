//! The ports a `holler pane` or `holler profile` verb runs against (epic #633).
//!
//! [`Wiring`] owns them: [`Wiring::connect`] builds them, and [`Wiring::ports`] lends them as the
//! [`Ports`] bundle a verb holds. `main.rs` calls `connect` once per run, and `say_cmd.rs` once for
//! `say`, `interrupt` and `answer` with `--pane`; each reports its error the way a verb reports one,
//! so `pane::run` and `profile::run` never see it. The in-process verb harness and the verb tests
//! build their `Ports` from fakes, or from [`Unwired`], and never call `connect`.
//!
//! **What `connect` builds (#649, D1).** The verbs run here, CLI-side; the hub is the store only
//! and constructs no adapter (ADR-0021 section 5). So `connect` builds:
//!
//! - the two registries' clients ([`store`]), over the hub's control socket;
//! - the Herdr adapter, built on its first use ([`bind::LazyHerdr`]): it pings Herdr when it is
//!   built;
//! - the tmux host (`TmuxHost`) and the harness, on one tmux server: `HarnessRouter` over the
//!   OpenCode harness (`OpenCodeHarness`) and the Claude Code harness (`ClaudeHarness`, #831), so a
//!   grid that mixes the two needs one `harness` in `Ports`; building any of them does no I/O;
//! - `SystemProber`, and `StoreScope` over the two clients as the `ProfileScope`. The scope logs
//!   every profile write as `holler pane`: what a `--profile` edit of a pane verb writes, and
//!   through it what `profile apply`'s creates and relaunches write (one actor; per-verb actors are
//!   #825);
//! - the hub port (#839, [`hub`]): a client of the hub's control socket, as the registries'
//!   clients are, through which switch, reset and doctor read the hub's live attach bodies,
//!   re-point one, and ask the hub's one prompt path. It needs only the hub.
//!
//! **What `connect` needs: the hub, and nothing else (D2).** It makes one `control/status`
//! exchange, and any failure of it (no hub, an error, no answer within 5 s) is `unavailable`
//! naming the hub, so with no hub every verb exits 1 and touches no adapter. It touches no Herdr,
//! tmux or OpenCode, and a configuration variable that is missing or malformed never fails it
//! (D26): the error is kept, and the port it configures answers it at its first use. So `pane list`,
//! `pane get`, `pane watch`, `profile list` and `profile show` work with nothing but the hub.
//!
//! **The environment (D5 to D7).** Read once, by `connect`, through [`mod@env`]:
//!
//! | Variable | What it sets | When it is unset |
//! |---|---|---|
//! | `HOLLER_HERDR_SESSION` | the one Herdr session the panes live in (Herdr's `HERDR_SESSION` is not read) | each Herdr call answers `usage` naming it |
//! | `HOLLER_HERDR_SOCKET` | that session's socket, an absolute path | `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`$HOME/.config` for `XDG_CONFIG_HOME`) |
//! | `HOLLER_HERDR_WORKSPACES` | each workspace's rows by columns, `label=RxC[,label=RxC]...` (a launch into an unlisted workspace is `unavailable`) | no workspace |
//! | `HOLLER_TMUX_SOCKET` | the tmux server of the host and the harness, an absolute path (`tmux` and `kill` come from `PATH`) | tmux's default socket |
//! | `HOLLER_OPENCODE_BIN` | the `opencode` binary, an absolute path | `opencode` found on `PATH` (none: each harness call answers `unavailable` naming the variable) |
//! | `HOLLER_OPENCODE_SERVE_ARGS` | a JSON array of strings, appended to `opencode serve --port P --hostname 127.0.0.1` | none |
//! | `HOLLER_CLAUDE_BIN` | the `claude` binary, an absolute path (#831) | `claude` found on `PATH`, never a shell alias (none: each Claude harness call answers `unavailable` naming the variable) |
//!
//! The Claude Code harness also reads `HOME`, for Claude Code's configuration directory
//! (`$HOME/.claude`, where it checks only whether a session's transcript exists), and keeps the
//! socket files its launch hook writes under the Holler state directory, in `claude-inbox/`
//! (`HOLLER_STATE_DIR`, else `$HOME/.holler`; the hook makes it `0700`, each file `0600`).
//!
//! A malformed value (a relative or empty path, a workspace list that does not parse, serve
//! arguments that are not a JSON array of strings) is `usage` naming its variable, at the first
//! use of the port it configures; a bad `HOLLER_TMUX_SOCKET` fails the host and the harness both,
//! and never falls back to the default tmux server. A variable of the table whose value is not
//! UTF-8 is read as the empty value, never as unset, so it takes no fallback: the empty value is
//! `usage` naming the variable, but for `HOLLER_HERDR_WORKSPACES`, where it is no workspace, so
//! the Herdr adapter refuses every launch as it refuses one into an unlisted workspace
//! ([`env::Lookup`]). The OpenCode processes run with `ProcessEnv::Inherit`: they inherit the
//! tmux server's environment, never the CLI's, so a provider credential reaches a harness server
//! only through that tmux server's environment. No variable here carries a credential, and
//! nothing of this module puts one on an argv, in a file or in a message.
//!
//! **The `tui_session` lookup (D8)** answers from the bindings of a launch in progress, then the
//! pane registry, then Herdr's labels ([`bind::tui_session`]).
//!
//! **Bounds (D9).** Each adapter and each registry request keeps its own 10 s bound (I5), and the
//! verbs keep their own budgets; there is no flag or variable for a bound.
//!
//! [`Unwired`] stays: it is the one not-implemented port set, which tests build `Ports` from.

pub mod bind;
pub mod env;
pub mod hub;
pub mod store;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use holler_adapter_claude::{ClaudeConfig, ClaudeHarness};
use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_host::{TmuxHost, TmuxSocket as HostSocket};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Resolver, Timeouts, TmuxConfig,
    TmuxSocket as TuiSocket,
};
use holler_pane::hub::{Advertised, HubPort, PromptMode};
use holler_pane::ports::{LoginStatus, SessionChoice};
use holler_pane::{
    Actor, AgentView, Argv, Cursor, HarnessEndpoint, HarnessPort, HarnessRouter, HerdrPane,
    HerdrPort, HerdrSnapshot, HerdrSpec, HostPort, Key, Pane, PaneError, PaneEvent, PaneId,
    PaneName, PaneObservation, PaneStore, Ports, ProbeResult, Prober, Profile, ProfileEvent,
    ProfileLogEntry, ProfileName, ProfileScope, ProfileStore, ResolvedScope, ServeSpec, SpecEdit,
    SystemProber, Watch,
};

use self::bind::{
    tui_session, BindingHerdr, BindingHost, Bindings, LazyHerdr, WiredHarness, WiredHost,
};
use self::env::Lookup;
use crate::pane::profile_scope::StoreScope;

/// Who the profile scope logs every profile write of a wired run as (D1).
const ACTOR: &str = "holler pane";

/// The directory, under the Holler state directory, of the socket files the Claude Code
/// harness's launch hook writes (#831).
const CLAUDE_INBOX_DIR: &str = "claude-inbox";

/// The wired harness: the router of the OpenCode harness and the Claude Code harness (#831),
/// each wired, so a misconfigured one fails closed at its first use and the other still works.
type RoutedHarness = HarnessRouter<WiredHarness, WiredHarness>;

/// The ports a verb runs against, owned (D1).
pub struct Wiring {
    panes: Arc<dyn PaneStore>,
    profiles: Arc<dyn ProfileStore>,
    herdr: BindingHerdr,
    host: BindingHost,
    harness: RoutedHarness,
    scope: StoreScope,
    prober: SystemProber,
    hub: hub::HubClient,
}

impl Wiring {
    /// Build the ports of a run from this process's environment, once the hub has answered.
    /// Fails `unavailable`, naming the hub, only when the hub cannot be reached (D2).
    ///
    /// A variable whose value is not UTF-8 is read as the empty value, never as unset
    /// ([`env::Lookup`]): unset, `HOLLER_TMUX_SOCKET` would be tmux's default server, the
    /// operator's live one (D6).
    pub fn connect() -> Result<Self, PaneError> {
        store::reach_hub()?;
        let actor = Actor::parse(ACTOR)?;
        let lookup = |name: &str| {
            std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
        };
        Ok(Self::from_env(&lookup, actor))
    }

    /// The ports, lent for one verb.
    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: self.panes.as_ref(),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &self.scope,
            prober: &self.prober,
            hub: &self.hub,
        }
    }

    /// The ports `env` configures, each configuration error kept for its port's first use. No
    /// I/O but the search of `PATH` for `opencode`.
    fn from_env(env: Lookup<'_>, actor: Actor) -> Self {
        let (panes, profiles) = store::stores();
        let herdr_config = env::herdr_config(env);
        let session = herdr_config
            .as_ref()
            .map(|config| config.session.clone())
            .unwrap_or_default();
        // `move`: the closure owns `herdr_config` from here on, so it is `'static`, as
        // `LazyHerdr::new` requires, and each build borrows the config from the closure.
        let lazy: Arc<dyn HerdrPort> =
            Arc::new(LazyHerdr::new(move || connect_herdr(&herdr_config)));
        let bindings = Bindings::new();
        let tmux = env::tmux_socket(env);
        let lookup = tui_session(
            &session,
            bindings.clone(),
            Arc::clone(&panes),
            Arc::clone(&lazy),
        );
        Self {
            herdr: BindingHerdr::new(lazy, bindings.clone()),
            host: BindingHost::new(Arc::new(tmux_host(&tmux)), bindings),
            harness: HarnessRouter::new(
                WiredHarness::new(opencode_harness(env, &tmux, lookup)),
                WiredHarness::new(claude_harness(env)),
            ),
            scope: StoreScope::new(Arc::clone(&profiles), Arc::clone(&panes), actor),
            panes,
            profiles,
            prober: SystemProber,
            hub: hub::HubClient::new(),
        }
    }
}

/// The Herdr adapter of `config` (D5), connected now: [`LazyHerdr`] calls this on its first use.
fn connect_herdr(config: &Result<HerdrConfig, PaneError>) -> Result<Arc<dyn HerdrPort>, PaneError> {
    let adapter = HerdrAdapter::connect(config.clone()?)?;
    Ok(Arc::new(adapter))
}

/// The tmux host on `socket` (D6), or the socket's configuration error.
fn tmux_host(socket: &Result<HostSocket, PaneError>) -> WiredHost {
    match socket {
        Ok(socket) => WiredHost::new(Ok(Arc::new(TmuxHost::new(socket.clone())))),
        Err(error) => WiredHost::new(Err(error.clone())),
    }
}

/// The OpenCode harness (D7) on the tmux server of `socket` (D6), with `tui_session` as its
/// lookup (D8), or the first configuration error of the socket, the binary and the serve
/// arguments, in that order.
fn opencode_harness(
    env: Lookup<'_>,
    socket: &Result<HostSocket, PaneError>,
    tui_session: Resolver<PaneId, PaneName>,
) -> Result<Arc<dyn HarnessPort>, PaneError> {
    // The two adapters' socket types mirror each other (an adapter may not depend on another).
    let socket = match socket.clone()? {
        HostSocket::Default => TuiSocket::Default,
        HostSocket::Name(name) => TuiSocket::Name(name),
        HostSocket::Path(path) => TuiSocket::Path(path),
    };
    let config = OpenCodeConfig {
        opencode_bin: env::opencode_bin(env)?,
        serve_args: env::serve_args(env)?,
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: PathBuf::from("tmux"),
            socket,
        },
        tui_session,
        timeouts: Timeouts::default(),
    };
    Ok(Arc::new(OpenCodeHarness::new(config)))
}

/// The Claude Code harness (#831): the `claude` binary, Claude Code's configuration directory
/// and the socket directory under the Holler state directory, or the first configuration error
/// of the three, in that order.
fn claude_harness(env: Lookup<'_>) -> Result<Arc<dyn HarnessPort>, PaneError> {
    let bin = env::claude_bin(env)?;
    let config_dir = env::claude_config_dir(env)?;
    let state = holler_hub::state::resolve_state_dir().ok_or_else(|| PaneError::Unavailable {
        what: "no Holler state directory (HOLLER_STATE_DIR is unset and so is HOME) for the \
               Claude Code harness's socket files"
            .to_owned(),
    })?;
    let config = ClaudeConfig::new(bin, config_dir, state.join(CLAUDE_INBOX_DIR));
    Ok(Arc::new(ClaudeHarness::new(config)))
}

/// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unwired;

impl PaneStore for Unwired {
    fn get(&self, _name: &PaneName) -> Result<Option<Pane>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn cas_put(&self, _pane: &Pane, _expected_generation: u64) -> Result<Pane, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn delete(&self, _name: &PaneName, _expected_generation: u64) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn watch(&self, _since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl ProfileStore for Unwired {
    fn get(&self, _name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn cas_put(
        &self,
        _profile: &Profile,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn delete(
        &self,
        _name: &ProfileName,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn watch(&self, _since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn log(&self, _name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn rename(
        &self,
        _from: &ProfileName,
        _to: &ProfileName,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl HerdrPort for Unwired {
    fn ensure_pane(&self, _spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn send_text(&self, _pane: &PaneId, _text: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn send_keys(&self, _pane: &PaneId, _keys: &[Key]) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn read(&self, _pane: &PaneId, _max_lines: usize) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn close(&self, _pane: &PaneId) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn version(&self) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn observe_pane(&self, _pane: &PaneId) -> Result<PaneObservation, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl HostPort for Unwired {
    fn ensure_session(&self, _name: &PaneName, _cwd: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn run(&self, _name: &PaneName, _cwd: &str, _argv: &Argv) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn stop_owned(&self, _name: &PaneName) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn foreground_commands(&self, _name: &PaneName) -> Result<Vec<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn attached_clients(&self, _name: &PaneName) -> Result<u32, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl HarnessPort for Unwired {
    fn serve_command(
        &self,
        _name: &PaneName,
        _port: u16,
        _spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn verify_serving(&self, _at: &HarnessEndpoint, _spec: &ServeSpec) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn health(&self, _at: &HarnessEndpoint) -> Result<bool, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn create_session(&self, _at: &HarnessEndpoint) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn list_sessions(&self, _at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn abort(&self, _at: &HarnessEndpoint, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn attach_tui(
        &self,
        _pane: &PaneId,
        _at: &HarnessEndpoint,
        _session: &str,
    ) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn shown_session(
        &self,
        _pane: &PaneId,
        _at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    // The four process methods answer `not-implemented` too, never their `process-unsupported`
    // defaults (#831): an unwired port is not a server harness.
    fn process_command(
        &self,
        _name: &PaneName,
        _spec: &ServeSpec,
        _session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn locate_process(&self, _name: &PaneName, _session: &str) -> Result<Option<u32>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn agent_view(
        &self,
        _at: &HarnessEndpoint,
        _session: &str,
        _max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl ProfileScope for Unwired {
    fn resolve(
        &self,
        _profile: &ProfileName,
        _pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn edit_spec(
        &self,
        _profile: Option<&ProfileName>,
        _pane: &PaneName,
        _edit: &SpecEdit,
        _act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl Prober for Unwired {
    fn run_probe(&self, _argv: &Argv, _expect: &[String], _timeout: Duration) -> ProbeResult {
        ProbeResult::Error("not implemented".to_owned())
    }
}

impl HubPort for Unwired {
    fn advertised(&self) -> Result<Vec<Advertised>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn rebind(&self, _driver: &Advertised, _harness_session_id: &str) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn prompt(
        &self,
        _harness_session_id: &str,
        _text: &str,
        _mode: PromptMode,
    ) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }
}
