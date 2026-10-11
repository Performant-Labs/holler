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
//! - the tmux host (`TmuxHost`) and the OpenCode harness (`OpenCodeHarness`), on one tmux server;
//!   building either does no I/O;
//! - `SystemProber`, and `StoreScope` over the two clients as the `ProfileScope`. The scope logs
//!   every profile write as `holler pane`: what a `--profile` edit of a pane verb writes, and
//!   through it what `profile apply`'s creates and relaunches write (one actor; per-verb actors are
//!   #825).
//!
//! **What `connect` needs: the hub, and nothing else (D2).** It makes one `control/status`
//! exchange, and any failure of it (no hub, an error, no answer within 5 s) is `unavailable`
//! naming the hub, so with no hub every verb exits 1 and touches no adapter. It touches no Herdr,
//! tmux or OpenCode, and a configuration variable that is missing or malformed never fails it
//! (D26): the error is kept, and the port it configures answers it at its first use. So `pane list`,
//! `pane get`, `pane watch`, `profile list` and `profile show` work with nothing but the hub.
//!
//! **The environment (D5 to D7).** Read once, by `connect`, through [`env`]:
//!
//! | Variable | What it sets | When it is unset |
//! |---|---|---|
//! | `HOLLER_HERDR_SESSION` | the one Herdr session the panes live in (Herdr's `HERDR_SESSION` is not read) | each Herdr call answers `usage` naming it |
//! | `HOLLER_HERDR_SOCKET` | that session's socket, an absolute path | `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`$HOME/.config` for `XDG_CONFIG_HOME`) |
//! | `HOLLER_HERDR_WORKSPACES` | each workspace's rows by columns, `label=RxC[,label=RxC]...` (a launch into an unlisted workspace is `unavailable`) | no workspace |
//! | `HOLLER_TMUX_SOCKET` | the tmux server of the host and the harness, an absolute path (`tmux` and `kill` come from `PATH`) | tmux's default socket |
//! | `HOLLER_OPENCODE_BIN` | the `opencode` binary, an absolute path | `opencode` found on `PATH` (none: each harness call answers `unavailable` naming the variable) |
//! | `HOLLER_OPENCODE_SERVE_ARGS` | a JSON array of strings, appended to `opencode serve --port P --hostname 127.0.0.1` | none |
//!
//! A malformed value (a relative or empty path, a workspace list that does not parse, serve
//! arguments that are not a JSON array of strings) is `usage` naming its variable, at the first
//! use of the port it configures; a bad `HOLLER_TMUX_SOCKET` fails the host and the harness both,
//! and never falls back to the default tmux server. The OpenCode processes run with
//! `ProcessEnv::Inherit`: they inherit the tmux server's environment, never the CLI's, so a
//! provider credential reaches a harness server only through that tmux server's environment. No
//! variable here carries a credential, and nothing of this module puts one on an argv, in a file
//! or in a message.
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
pub mod store;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
use holler_adapter_host::{TmuxHost, TmuxSocket as HostSocket};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Resolver, Timeouts, TmuxConfig,
    TmuxSocket as TuiSocket,
};
use holler_pane::{
    Actor, Argv, Cursor, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort,
    Key, Pane, PaneError, PaneEvent, PaneId, PaneName, PaneStore, Ports, ProbeResult, Prober,
    Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope, ProfileStore, ResolvedScope,
    ServeSpec, SpecEdit, SystemProber, Watch,
};

use self::bind::{
    tui_session, BindingHerdr, BindingHost, Bindings, LazyHerdr, WiredHarness, WiredHost,
};
use self::env::Lookup;
use crate::pane::profile_scope::StoreScope;

/// Who the profile scope logs every profile write of a wired run as (D1).
const ACTOR: &str = "holler pane";

/// The ports a verb runs against, owned (D1).
pub struct Wiring {
    panes: Arc<dyn PaneStore>,
    profiles: Arc<dyn ProfileStore>,
    herdr: BindingHerdr,
    host: BindingHost,
    harness: WiredHarness,
    scope: StoreScope,
    prober: SystemProber,
}

impl Wiring {
    /// Build the ports of a run from this process's environment, once the hub has answered.
    /// Fails `unavailable`, naming the hub, only when the hub cannot be reached (D2).
    pub fn connect() -> Result<Self, PaneError> {
        store::reach_hub()?;
        let actor = Actor::parse(ACTOR)?;
        Ok(Self::from_env(&|name| std::env::var(name).ok(), actor))
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
            harness: WiredHarness::new(opencode_harness(env, &tmux, lookup)),
            scope: StoreScope::new(Arc::clone(&profiles), Arc::clone(&panes), actor),
            panes,
            profiles,
            prober: SystemProber,
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

    fn verify_serving(&self, _port: u16, _spec: &ServeSpec) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn health(&self, _port: u16) -> Result<bool, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn create_session(&self, _port: u16) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn list_sessions(&self, _port: u16) -> Result<Vec<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn abort(&self, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn attach_tui(&self, _pane: &PaneId, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn shown_session(&self, _pane: &PaneId) -> Result<Option<String>, PaneError> {
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
