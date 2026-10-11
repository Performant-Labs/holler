//! The import rig's two port wrappers (#650): a Herdr that also reports panes of another
//! Herdr session, and the failing-N-th-write pane store. Each passes every other call to the
//! test kit's fake, so the fake's call log stays complete.

use std::sync::atomic::{AtomicUsize, Ordering};

use holler_pane::{
    Cursor, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, Pane, PaneError, PaneEvent,
    PaneId, PaneName, PaneStore, Watch,
};
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::pane_store::FakePaneStore;

/// The rig's `FakeHerdr`, with [`ImportRig::extra_herdr`] appended to every snapshot. Every
/// call passes through to the fake, so its call log stays the fake's.
pub(crate) struct ExtraHerdr<'a> {
    pub herdr: &'a FakeHerdr,
    pub extra: &'a [HerdrPane],
}

impl HerdrPort for ExtraHerdr<'_> {
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
        snapshot.panes.extend(self.extra.iter().cloned());
        Ok(snapshot)
    }

    fn version(&self) -> Result<String, PaneError> {
        self.herdr.version()
    }
}

/// What [`NthCasPut`] does to one numbered `cas_put`.
#[derive(Debug, Clone)]
pub(crate) enum NthPut {
    /// Answer this error and write nothing.
    Fail(PaneError),
    /// Apply the write, then answer this error: a write that landed but whose answer was
    /// lost (a `timeout` after the store committed).
    ApplyThenFail(PaneError),
}

/// The failing-N-th-write seam, copied from `profile_verbs/rig.rs` (a separate test target,
/// so it cannot be imported; A's handoff lists the copy as deliberate): a `PaneStore` that
/// delegates every call to a [`FakePaneStore`] and acts on the `cas_put`s its plan numbers
/// (1 is the first `cas_put` made through the seam).
pub(crate) struct NthCasPut<'a> {
    inner: &'a FakePaneStore,
    plan: Vec<(usize, NthPut)>,
    count: AtomicUsize,
}

impl<'a> NthCasPut<'a> {
    pub fn new(inner: &'a FakePaneStore, plan: Vec<(usize, NthPut)>) -> Self {
        Self {
            inner,
            plan,
            count: AtomicUsize::new(0),
        }
    }
}

impl PaneStore for NthCasPut<'_> {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.inner.list()
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        let n = self.count.fetch_add(1, Ordering::SeqCst) + 1;
        match self
            .plan
            .iter()
            .find(|(at, _)| *at == n)
            .map(|(_, step)| step)
        {
            Some(NthPut::Fail(error)) => Err(error.clone()),
            Some(NthPut::ApplyThenFail(error)) => {
                self.inner.cas_put(pane, expected_generation)?;
                Err(error.clone())
            }
            None => self.inner.cas_put(pane, expected_generation),
        }
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.inner.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.inner.watch(since)
    }
}
