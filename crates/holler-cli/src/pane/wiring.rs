//! The ports a `holler pane` or `holler profile` verb runs against (epic #633).
//!
//! [`Wiring`] owns them: [`Wiring::connect`] builds them, which can fail (no hub, no Herdr
//! socket), and [`Wiring::ports`] lends them as the [`Ports`] bundle a verb holds. `main.rs` calls
//! `connect` once per run and reports its error the way a verb reports one; `pane::run` and
//! `profile::run` never see it.
//!
//! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
//! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
//! Story #649 replaces the body of `connect` (and the fields of `Wiring`) with the real stores and
//! adapters, and keeps `Unwired`: it is the one not-implemented port set, and the in-process verb
//! harness (`tests/verb_harness`) builds its `Ports` from it until the test kit's fakes (#638)
//! replace it there.

pub mod bind;
pub mod env;
pub mod store;

use std::time::Duration;

use holler_pane::{
    Actor, Argv, Cursor, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort,
    Key, Pane, PaneError, PaneEvent, PaneId, PaneName, PaneStore, Ports, ProbeResult, Prober,
    Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope, ProfileStore, ResolvedScope,
    ServeSpec, SpecEdit, Watch,
};

/// The ports a verb runs against, owned.
pub struct Wiring {
    #[allow(dead_code)] // #649
    ports: Unwired,
    panes: std::sync::Arc<store::HubPaneStore>,
    profiles: std::sync::Arc<store::HubProfileStore>,
    scope: crate::pane::profile_scope::StoreScope,
    herdr: bind::BindingHerdr,
    host: Box<dyn HostPort>,
    harness: bind::WiredHarness,
    prober: holler_pane::SystemProber,
}

impl Wiring {
    /// Build the ports of a run. Fails when something they need cannot be reached.
    pub fn connect() -> Result<Self, PaneError> {
        crate::transport::call(None, &holler_hub::control::ControlCall::status()).map_err(|e| match e {
            holler_hub::control::ControlError::NoLiveHub => PaneError::Unavailable { what: format!("no live holler hub reachable at {}", holler_hub::state::resolve_state_dir().map(|d| d.display().to_string()).unwrap_or_default()) },
            other => PaneError::Unavailable { what: other.to_string() },
        })?;
        let (panes, profiles) = store::stores();
        let scope = crate::pane::profile_scope::StoreScope::new(profiles.clone(), panes.clone(), Actor::parse("holler pane")?);
        use std::sync::Arc;
        let lookup = |k: &str| std::env::var(k).ok();
        let cfg = env::herdr_config(&lookup);
        let session = cfg.as_ref().map(|c| c.session.clone()).unwrap_or_default();
        let lazy: Arc<dyn HerdrPort> = Arc::new(bind::LazyHerdr::new(move || {
            let c = cfg.clone()?;
            Ok(Arc::new(holler_adapter_herdr::adapter::HerdrAdapter::connect(c)?) as Arc<dyn HerdrPort>)
        }));
        let bindings = bind::Bindings::new();
        let herdr = bind::BindingHerdr::new(lazy.clone(), bindings.clone());
        let tmux = env::tmux_socket(&lookup);
        let host: Box<dyn HostPort> = match &tmux {
            Ok(sock) => Box::new(bind::BindingHost::new(Arc::new(holler_adapter_host::TmuxHost::new(sock.clone())), bindings.clone())),
            Err(_) => Box::new(Unwired),
        };
        let oc_sock = match tmux { Ok(holler_adapter_host::TmuxSocket::Path(p)) => holler_adapter_opencode::TmuxSocket::Path(p), _ => holler_adapter_opencode::TmuxSocket::Default };
        let pane_dyn: Arc<dyn PaneStore> = panes.clone();
        let harness = match (env::opencode_bin(&lookup), env::serve_args(&lookup)) {
            (Ok(bin), Ok(args)) => bind::WiredHarness::new(Ok(Arc::new(holler_adapter_opencode::OpenCodeHarness::new(holler_adapter_opencode::OpenCodeConfig {
                opencode_bin: bin, serve_args: args, env: holler_adapter_opencode::ProcessEnv::Inherit,
                tmux: holler_adapter_opencode::TmuxConfig { tmux_bin: "tmux".into(), socket: oc_sock },
                tui_session: bind::tui_session(&session, bindings, pane_dyn, lazy),
                timeouts: holler_adapter_opencode::Timeouts::default(),
            })) as Arc<dyn HarnessPort>)),
            (Err(e), _) | (_, Err(e)) => bind::WiredHarness::new(Err(e)),
        };
        Ok(Self { ports: Unwired, panes, profiles, scope, herdr, host, harness, prober: holler_pane::SystemProber })
    }

    /// The ports, lent for one verb.
    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: self.panes.as_ref(),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: self.host.as_ref(),
            harness: &self.harness,
            scope: &self.scope,
            prober: &self.prober,
        }
    }
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
