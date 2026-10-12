//! The wiring's own port types (#649): what sits between the verbs and the adapters so that the
//! adapters can be built from configuration that may be wrong, and so that the OpenCode adapter's
//! one lookup can answer for a pane that is being launched.
//!
//! - [`LazyHerdr`] builds the Herdr adapter on its first use and keeps the result (D2): building
//!   it pings Herdr, so a verb that never reaches Herdr never needs one.
//! - [`Wired`] holds a port, or the configuration error that keeps it from being built, which
//!   each of its methods then answers (W-4, W-E): [`WiredHarness`] for OpenCode, [`WiredHost`]
//!   for tmux. A misconfigured port fails closed at its first use, never at `connect`, and is
//!   never replaced by another (the default tmux server, or `Unwired`).
//! - [`BindingHerdr`] and [`BindingHost`] forward every method, and bind the pane id that each
//!   `ensure_pane` returns to the name that the next `ensure_session` names (D8 (a)).
//! - [`tui_session`] is the lookup itself (D8).
//!
//! **Every method is forwarded (D31 W-4, D33).** Each wrapper here overrides every method of its
//! port, the ones the port gives a default included: `HarnessPort`'s `send_prompt` and
//! `session_activity`, its four process methods (#831: `process_command`, `new_session_id`,
//! `locate_process` and `login_status`, whose defaults refuse `process-unsupported`), and
//! `HerdrPort`'s `observe_panes` (#832). A default never answers in place of the inner port, and
//! `observe_panes` goes to the inner port's own `observe_panes`, so a list of panes stays one read
//! of Herdr (ADR-0021 section 14), never one `observe_pane` per pane. `HerdrPort::observe_pane`
//! and `HostPort::foreground_commands` have no default (#835), so the compiler holds every wrapper
//! to them.
//!
//! **Why the bindings (D8).** The launch engine attaches the TUI by the Herdr pane id before the
//! pane's record exists (E34): `ensure_pane` (which knows the pane id), then `ensure_session` and
//! `run` (which know the name), then the server, then `attach_tui(pane id)`, and only then the
//! record. So the lookup cannot be a registry read alone, and the bindings remember each launch's
//! pair. This couples the wiring to the engine's call order (`tx_launch::bring_up` calls
//! `ensure_session` right after each `ensure_pane`), which a default-run test pins; carrying the
//! name in `attach_tui` would be a change to the frozen port. The bindings are per run and in
//! memory, and only the TUI methods read them, from any thread. A wiring runs one launch at a
//! time (`profile apply` runs its creates in sequence), with one exception (#831): `pane doctor
//! --fix` can relaunch several Claude Code panes at once, one per pane's thread, and their
//! bindings may then pair one pane's id with another's name. Nothing reads those pairs: a Claude
//! Code relaunch attaches no TUI, and the TUI lookup is asked only for an OpenCode pane's id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use holler_adapter_opencode::Resolver;
use holler_pane::ports::{Activity, LoginStatus, SessionChoice};
use holler_pane::{
    Argv, HarnessEndpoint, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort,
    Key, Pane, PaneError, PaneId, PaneName, PaneObservation, PaneStore, ServeSpec,
};

/// The pairs a run's launches bound: the pane id the last `ensure_pane` answered, until an
/// `ensure_session` claims it, and each pane id bound to its pane's name.
#[derive(Debug, Default)]
struct Bound {
    pending: Option<PaneId>,
    names: HashMap<PaneId, PaneName>,
}

/// The bindings of one run (D8 (a)), shared by [`BindingHerdr`], which offers each pane id,
/// [`BindingHost`], which binds it to a name, and [`tui_session`], which reads them. Cloning one
/// shares it.
#[derive(Debug, Clone, Default)]
pub struct Bindings(Arc<Mutex<Bound>>);

impl Bindings {
    /// No binding yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The bound pairs. A thread that panicked while it held them left them whole (each change
    /// is one assignment or one insert), so a poisoned lock is taken as it is.
    fn lock(&self) -> MutexGuard<'_, Bound> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The pane id the last `ensure_pane` answered (`None` when it failed), for the next
    /// `ensure_session` to claim.
    fn offer(&self, pane: Option<PaneId>) {
        self.lock().pending = pane;
    }

    /// Take the offered pane id: each one is claimed by one `ensure_session`, whatever its outcome.
    fn claim(&self) -> Option<PaneId> {
        self.lock().pending.take()
    }

    /// Bind `pane` to the name `name`.
    fn bind(&self, pane: PaneId, name: PaneName) {
        self.lock().names.insert(pane, name);
    }

    /// The name bound to `pane`, if a launch of this run bound one.
    fn name_of(&self, pane: &PaneId) -> Option<PaneName> {
        self.lock().names.get(pane).cloned()
    }
}

/// The Herdr port, offering the pane id each `ensure_pane` answers to the `ensure_session` that
/// follows it (D8 (a)). Every method forwards to the inner port and answers what it answers,
/// `observe_pane` and `observe_panes` included (D33): a list is one call to the inner port.
pub struct BindingHerdr {
    inner: Arc<dyn HerdrPort>,
    bindings: Bindings,
}

impl BindingHerdr {
    /// `inner`, offering its panes to `bindings`.
    pub fn new(inner: Arc<dyn HerdrPort>, bindings: Bindings) -> Self {
        Self { inner, bindings }
    }
}

impl HerdrPort for BindingHerdr {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        let answer = self.inner.ensure_pane(spec);
        let pane = answer.as_ref().ok().map(|pane| pane.pane_id.clone());
        self.bindings.offer(pane);
        answer
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.inner.send_text(pane, text)
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        self.inner.send_keys(pane, keys)
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        self.inner.read(pane, max_lines)
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.inner.close(pane)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        self.inner.snapshot()
    }

    fn version(&self) -> Result<String, PaneError> {
        self.inner.version()
    }

    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError> {
        self.inner.observe_pane(pane)
    }

    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
        self.inner.observe_panes(panes)
    }
}

/// The host port, binding the pane id offered by the last `ensure_pane` to the name each
/// `ensure_session` makes a session for (D8 (a)). The offer is claimed whatever the outcome, and
/// bound only when the session exists. Every method forwards to the inner port and answers what
/// it answers.
pub struct BindingHost {
    inner: Arc<dyn HostPort>,
    bindings: Bindings,
}

impl BindingHost {
    /// `inner`, binding offered panes in `bindings`.
    pub fn new(inner: Arc<dyn HostPort>, bindings: Bindings) -> Self {
        Self { inner, bindings }
    }
}

impl HostPort for BindingHost {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        let offered = self.bindings.claim();
        self.inner.ensure_session(name, cwd)?;
        if let Some(pane) = offered {
            self.bindings.bind(pane, name.clone());
        }
        Ok(())
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.inner.run(name, cwd, argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.inner.stop_owned(name)
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.inner.ps(name)
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.inner.foreground_commands(name)
    }
}

/// How a [`LazyHerdr`] builds its port.
type Build = Box<dyn Fn() -> Result<Arc<dyn HerdrPort>, PaneError> + Send + Sync>;

/// The Herdr port, built on its first use and kept, the error of a build that failed included
/// (D2): the adapter pings Herdr when it is built, so a verb that never reaches Herdr runs with no
/// Herdr, and its configuration errors (D5) are answered at that first use, never at `connect`.
/// A failed build is not retried: every method answers its error, except `observe_panes`, which
/// returns no error and so observes nothing: one empty observation per pane, the port's rule for
/// a call that failed (D33).
pub struct LazyHerdr {
    build: Build,
    built: OnceLock<Result<Arc<dyn HerdrPort>, PaneError>>,
}

impl LazyHerdr {
    /// A port that `build` makes when it is first used. Calling this does nothing else.
    pub fn new(
        build: impl Fn() -> Result<Arc<dyn HerdrPort>, PaneError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            build: Box::new(build),
            built: OnceLock::new(),
        }
    }

    /// The port, built now if this is its first use.
    fn port(&self) -> Result<&dyn HerdrPort, PaneError> {
        match self.built.get_or_init(|| (self.build)()) {
            Ok(port) => Ok(port.as_ref()),
            Err(error) => Err(error.clone()),
        }
    }
}

impl HerdrPort for LazyHerdr {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        self.port()?.ensure_pane(spec)
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.port()?.send_text(pane, text)
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        self.port()?.send_keys(pane, keys)
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        self.port()?.read(pane, max_lines)
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.port()?.close(pane)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        self.port()?.snapshot()
    }

    fn version(&self) -> Result<String, PaneError> {
        self.port()?.version()
    }

    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError> {
        self.port()?.observe_pane(pane)
    }

    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
        match self.port() {
            Ok(port) => port.observe_panes(panes),
            Err(_) => vec![PaneObservation::default(); panes.len()],
        }
    }
}

/// A port the wiring built from its configuration, or the configuration error that kept it from
/// being built, which every method then answers (W-4, W-E). Every method is forwarded explicitly,
/// so a method with a default (`HarnessPort::send_prompt`, `session_activity` and the four
/// process methods of #831) reaches the inner port, never the default.
pub struct Wired<P: ?Sized>(Result<Arc<P>, PaneError>);

/// A wired `HarnessPort`: OpenCode (D7) or Claude Code (#831), or what keeps it from being
/// configured.
pub type WiredHarness = Wired<dyn HarnessPort>;

/// The wired `HostPort`: tmux (D6), or what keeps it from being configured.
pub type WiredHost = Wired<dyn HostPort>;

impl<P: ?Sized> Wired<P> {
    /// The port, or the configuration error each method answers.
    pub fn new(port: Result<Arc<P>, PaneError>) -> Self {
        Self(port)
    }

    /// The port, or its configuration error.
    fn port(&self) -> Result<&P, PaneError> {
        self.0.as_deref().map_err(PaneError::clone)
    }
}

impl HarnessPort for Wired<dyn HarnessPort> {
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        self.port()?.serve_command(name, port, spec)
    }

    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.port()?.verify_serving(at, spec)
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.port()?.health(at)
    }

    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.port()?.create_session(at)
    }

    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.port()?.list_sessions(at)
    }

    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.port()?.abort(at, session)
    }

    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        self.port()?.attach_tui(pane, at, session)
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.port()?.select_session(pane, session)
    }

    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.port()?.shown_session(pane, at)
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.port()?.send_prompt(at, session, text)
    }

    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        self.port()?.session_activity(at, session)
    }

    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        self.port()?.process_command(name, spec, session)
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        self.port()?.new_session_id()
    }

    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        self.port()?.locate_process(name, session)
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        self.port()?.login_status()
    }
}

impl HostPort for Wired<dyn HostPort> {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.port()?.ensure_session(name, cwd)
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.port()?.run(name, cwd, argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.port()?.stop_owned(name)
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.port()?.ps(name)
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.port()?.foreground_commands(name)
    }
}

/// The OpenCode adapter's `tui_session` lookup (D8, E31): the tmux session of a Herdr pane, which
/// is its Holler pane's name, from the first of these that knows it:
///
/// - (a) the [`Bindings`] of a launch of this run, so a pane being launched is found before its
///   record exists;
/// - (b) the pane registry: the one record whose Herdr pane is `pane` in the Herdr session
///   `session` (Herdr numbers pane ids within a session, so both are the join key, W-5). Two
///   or more such records are `unavailable`, never a pick;
/// - (c) the pane's label in Herdr's snapshot, when it is a pane name (only a pane someone
///   labelled, such as an imported one: launch writes no label, E26).
///
/// Otherwise `pane-not-found`, the lookup's own error (E32). `session` is the Herdr session the
/// adapter serves, empty when none is configured; then (b) matches nothing, and (c) answers the
/// Herdr port's own configuration error. An error of (b) or (c) is answered as it is.
///
/// **Bound (D8, W-C).** The adapter asks after it has fixed its method's deadline, so when (a)
/// misses, (b) is one `pane/list` (10 s) and (c) one snapshot (10 s, and on a first use the
/// adapter's connect ping): a TUI method resolved through them can run past its own bound when the
/// hub or Herdr is slow.
pub fn tui_session(
    session: &str,
    bindings: Bindings,
    panes: Arc<dyn PaneStore>,
    herdr: Arc<dyn HerdrPort>,
) -> Resolver<PaneId, PaneName> {
    let session = session.to_owned();
    Arc::new(move |pane: &PaneId| {
        if let Some(name) = bindings.name_of(pane) {
            return Ok(name);
        }
        if let Some(name) = recorded(panes.as_ref(), &session, pane)? {
            return Ok(name);
        }
        if let Some(name) = labelled(herdr.as_ref(), &session, pane)? {
            return Ok(name);
        }
        Err(PaneError::PaneNotFound {
            what: format!(
                "the Herdr pane {:?} of the session {session:?}: no launch of this run, no pane \
                 record and no label names it",
                pane.as_str()
            ),
        })
    })
}

/// (b): the name of the one record on `pane` of the Herdr session `session`, if there is one.
fn recorded(
    panes: &dyn PaneStore,
    session: &str,
    pane: &PaneId,
) -> Result<Option<PaneName>, PaneError> {
    let on_pane = |record: &Pane| {
        !session.is_empty() && record.herdr.session == session && record.herdr.pane_id == *pane
    };
    let mut names = panes
        .list()?
        .into_iter()
        .filter(on_pane)
        .map(|record| record.name);
    match (names.next(), names.next()) {
        (None, _) => Ok(None),
        (Some(name), None) => Ok(Some(name)),
        (Some(first), Some(second)) => Err(PaneError::Unavailable {
            what: format!(
                "more than one pane record ({first}, {second}) names the Herdr pane {:?} of the \
                 session {session:?}, so its tmux session is not known",
                pane.as_str()
            ),
        }),
    }
}

/// (c): the pane name `pane` of the Herdr session `session` is labelled with, if it has one.
fn labelled(
    herdr: &dyn HerdrPort,
    session: &str,
    pane: &PaneId,
) -> Result<Option<PaneName>, PaneError> {
    let label = herdr
        .snapshot()?
        .panes
        .into_iter()
        .find(|listed| listed.session == session && listed.pane_id == *pane)
        .and_then(|listed| listed.label);
    Ok(label.and_then(|label| PaneName::parse(&label).ok()))
}
