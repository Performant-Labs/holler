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

use std::time::Duration;

use holler_pane::{
    Actor, Argv, Cursor, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort,
    Key, Pane, PaneError, PaneEvent, PaneId, PaneName, PaneStore, Ports, ProbeResult, Prober,
    Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope, ProfileStore, ResolvedScope,
    SpecEdit, Watch,
};

/// The ports a verb runs against, owned.
pub struct Wiring {
    ports: Unwired,
}

impl Wiring {
    /// Build the ports of a run. Fails when something they need cannot be reached.
    pub fn connect() -> Result<Self, PaneError> {
        Ok(Self { ports: Unwired })
    }

    /// The ports, lent for one verb.
    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: &self.ports,
            profile_store: &self.ports,
            herdr: &self.ports,
            host: &self.ports,
            harness: &self.ports,
            scope: &self.ports,
            prober: &self.ports,
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

    fn run(&self, _name: &PaneName, _argv: &Argv) -> Result<(), PaneError> {
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
    fn serve(&self, _name: &PaneName, _port: u16) -> Result<u32, PaneError> {
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
