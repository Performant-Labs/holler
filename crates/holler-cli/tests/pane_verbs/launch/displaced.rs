//! #808 E11: a `HerdrPort` over the rig's `FakeHerdr` whose `snapshot` can report one pane
//! somewhere it is not, so a test can show that O2 (Herdr still lists the pane at its cell)
//! compares the pane's place, not only its id. `FakeHerdr` never moves a pane, so the rig
//! hands the verb this wrapper, which passes every call through to the fake (its call log
//! stays the fake's) until a hook arms it.

use std::sync::{Mutex, PoisonError};

use holler_pane::{
    GridPos, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId,
};
use holler_pane_testkit::herdr::FakeHerdr;

/// Where the armed snapshot reports the pane, its other fields unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Displace {
    /// In another cell of its workspace (`r3c3`, free in the rig's `main`).
    Grid,
    /// In another workspace, at its cell.
    Workspace,
    /// In another Herdr session, at its workspace and cell.
    Session,
}

impl Displace {
    fn apply(self, pane: &mut HerdrPane) {
        match self {
            Displace::Grid => pane.grid = GridPos { row: 3, col: 3 },
            Displace::Workspace => pane.workspace = "elsewhere".to_owned(),
            Displace::Session => pane.session = "elsewhere".to_owned(),
        }
    }
}

/// The pane to displace and how, once armed.
pub(crate) type Armed = Mutex<Option<(PaneId, Displace)>>;

/// The rig's `FakeHerdr`, with `snapshot` reporting the armed pane displaced.
pub(crate) struct DisplacingHerdr<'a> {
    pub herdr: &'a FakeHerdr,
    pub armed: &'a Armed,
}

impl HerdrPort for DisplacingHerdr<'_> {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        self.herdr.ensure_pane(spec)
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.herdr.send_text(pane, text)
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        self.herdr.send_keys(pane, keys)
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        self.herdr.read(pane, max_lines)
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.herdr.close(pane)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        let mut snapshot = self.herdr.snapshot()?;
        let armed = self.armed.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((id, how)) = armed.as_ref() {
            for pane in snapshot.panes.iter_mut().filter(|p| p.pane_id == *id) {
                how.apply(pane);
            }
        }
        Ok(snapshot)
    }

    fn version(&self) -> Result<String, PaneError> {
        self.herdr.version()
    }
}
