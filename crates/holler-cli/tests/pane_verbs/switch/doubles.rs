//! The two harness doubles of `switch.rs` (#645), moved here unchanged so that file stays
//! under the 900-line gate (#647 part 2): `WriterInSelect`, a writer between the act and the
//! record, and `SeamHarness`, the recording wrapper of the prompt and activity seam that
//! `reset.rs` also uses.

use std::sync::Mutex;

use holler_pane::ports::Activity;
use holler_pane::{
    Argv, HarnessEndpoint, HarnessPort, Pane, PaneError, PaneId, PaneName, PaneStore, ServeSpec,
};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::pane_store::FakePaneStore;

use crate::doctor::rig::Rig;

/// A harness whose `select_session` first lets another writer store `other` (P's record,
/// changed), then delegates to the rig's fake: the record write after the act meets a
/// newer generation.
pub(super) struct WriterInSelect<'a> {
    pub(super) inner: &'a FakeHarness,
    pub(super) panes: &'a FakePaneStore,
    pub(super) other: &'a Pane,
}

impl HarnessPort for WriterInSelect<'_> {
    fn serve_command(&self, n: &PaneName, port: u16, s: &ServeSpec) -> Result<Argv, PaneError> {
        self.inner.serve_command(n, port, s)
    }
    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.inner.verify_serving(at, spec)
    }
    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.inner.health(at)
    }
    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.inner.create_session(at)
    }
    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(at)
    }
    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.inner.abort(at, session)
    }
    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        self.inner.attach_tui(pane, at, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.panes.concurrent_put(self.other)?;
        self.inner.select_session(pane, session)
    }
    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane, at)
    }
}

/// One prompt a [`SeamHarness`] queued: the port and session it went to, the text,
/// and the pane's session of record at prompt time — the prompt follows the record
/// write only if that is the prompted session (#645 part 2's ordering pin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedPrompt {
    pub port: u16,
    pub session: String,
    pub text: String,
    pub then_of_record: Option<String>,
}

/// What the seam wrappers saw, in order: every prompt queued through
/// `HarnessPort::send_prompt` and every activity query through
/// `HarnessPort::session_activity`. Those two run the trait's defaults on the kit's
/// fake, which do not enter its call log, so this log is their assertion surface.
#[derive(Default)]
pub(crate) struct SeamLog {
    prompts: Mutex<Vec<QueuedPrompt>>,
    activity: Mutex<Vec<(u16, String)>>,
}

impl SeamLog {
    fn prompt(&self, port: u16, session: &str, text: &str, then_of_record: Option<String>) {
        self.prompts
            .lock()
            .expect("the prompt log")
            .push(QueuedPrompt {
                port,
                session: session.to_owned(),
                text: text.to_owned(),
                then_of_record,
            });
    }

    fn activity_query(&self, port: u16, session: &str) {
        self.activity
            .lock()
            .expect("the activity log")
            .push((port, session.to_owned()));
    }

    /// The prompts queued so far, oldest first.
    pub(crate) fn prompts(&self) -> Vec<QueuedPrompt> {
        self.prompts.lock().expect("the prompt log").clone()
    }

    /// The activity queries answered so far, oldest first.
    pub(crate) fn activity(&self) -> Vec<(u16, String)> {
        self.activity.lock().expect("the activity log").clone()
    }
}

/// A `FakeHarness` wrapper for the two seam methods #645 part 2 adds (the
/// `WriterInSelect` pattern): it delegates the eight existing methods unchanged and
/// overrides only `send_prompt` and `session_activity`, recording each call into
/// `log`, answering every activity query with `activity` (busy for any query, when
/// the case wants that) and every prompt with `prompt` (`Ok` unless the case faults
/// it). At prompt time it also reads `pane`'s record, so each queued prompt carries
/// the ordering pin. It holds no session or TUI state of its own: a wrapper that
/// starts modelling behaviour is a second `FakeHarness` (#684's, not ours).
pub(crate) struct SeamHarness<'a> {
    inner: &'a FakeHarness,
    panes: &'a FakePaneStore,
    pane: PaneName,
    log: &'a SeamLog,
    activity: Activity,
    prompt: Option<PaneError>,
}

impl<'a> SeamHarness<'a> {
    /// Wrap the rig's own fake for `pane`, answering the seam from that table.
    pub(crate) fn new(
        rig: &'a Rig,
        pane: &str,
        log: &'a SeamLog,
        activity: Activity,
        prompt: Option<PaneError>,
    ) -> Self {
        Self {
            inner: &rig.harness,
            panes: &rig.panes,
            pane: PaneName::parse(pane).expect("a pane name"),
            log,
            activity,
            prompt,
        }
    }
}

impl HarnessPort for SeamHarness<'_> {
    fn serve_command(&self, n: &PaneName, port: u16, s: &ServeSpec) -> Result<Argv, PaneError> {
        self.inner.serve_command(n, port, s)
    }
    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.inner.verify_serving(at, spec)
    }
    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.inner.health(at)
    }
    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.inner.create_session(at)
    }
    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(at)
    }
    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.inner.abort(at, session)
    }
    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        self.inner.attach_tui(pane, at, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.inner.select_session(pane, session)
    }
    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane, at)
    }
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let then = self
            .panes
            .get(&self.pane)
            .expect("the store answers")
            .and_then(|record| record.session_of_record);
        self.log.prompt(at.port().unwrap(), session, text, then);
        match self.prompt.clone() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        self.log.activity_query(at.port().unwrap(), session);
        Ok(self.activity)
    }
}
