//! TRIAL (T's satisfiability check only, never committed).
#![allow(clippy::unwrap_used, clippy::expect_used)] // #649
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use holler_adapter_opencode::Resolver;
use holler_pane::ports::Activity;
use holler_pane::{Argv, HarnessPort, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort, Key, PaneError, PaneId, PaneName, PaneStore, ServeSpec};
#[derive(Default)]
struct Inner { pending: Option<PaneId>, bound: HashMap<String, PaneName> }
#[derive(Clone, Default)]
pub struct Bindings(Arc<Mutex<Inner>>);
impl Bindings { pub fn new() -> Self { Self::default() } }
pub struct BindingHerdr { inner: Arc<dyn HerdrPort>, b: Bindings }
impl BindingHerdr { pub fn new(inner: Arc<dyn HerdrPort>, b: Bindings) -> Self { Self { inner, b } } }
pub struct BindingHost { inner: Arc<dyn HostPort>, b: Bindings }
impl BindingHost { pub fn new(inner: Arc<dyn HostPort>, b: Bindings) -> Self { Self { inner, b } } }
type Build = Box<dyn Fn() -> Result<Arc<dyn HerdrPort>, PaneError> + Send + Sync>;
pub struct LazyHerdr { build: Build, built: OnceLock<Result<Arc<dyn HerdrPort>, PaneError>> }
impl LazyHerdr {
    pub fn new(build: impl Fn() -> Result<Arc<dyn HerdrPort>, PaneError> + Send + Sync + 'static) -> Self { Self { build: Box::new(build), built: OnceLock::new() } }
    fn port(&self) -> Result<&Arc<dyn HerdrPort>, PaneError> { self.built.get_or_init(|| (self.build)()).as_ref().map_err(Clone::clone) }
}
pub struct WiredHarness(Result<Arc<dyn HarnessPort>, PaneError>);
impl WiredHarness { pub fn new(inner: Result<Arc<dyn HarnessPort>, PaneError>) -> Self { Self(inner) } fn h(&self) -> Result<&Arc<dyn HarnessPort>, PaneError> { self.0.as_ref().map_err(Clone::clone) } }
pub fn tui_session(session: &str, b: Bindings, panes: Arc<dyn PaneStore>, herdr: Arc<dyn HerdrPort>) -> Resolver<PaneId, PaneName> {
    let session = session.to_owned();
    Arc::new(move |id: &PaneId| {
        eprintln!("TRIAL resolve {id:?} bound={:?}", b.0.lock().unwrap().bound);
        if let Some(n) = b.0.lock().unwrap().bound.get(id.as_str()) { return Ok(n.clone()); }
        let hits: Vec<_> = panes.list()?.into_iter().filter(|p| p.herdr.session == session && p.herdr.pane_id == *id).collect();
        match hits.len() { 0 => {}, 1 => return Ok(hits[0].name.clone()), _ => return Err(PaneError::Unavailable { what: "two".into() }) }
        if let Some(label) = herdr.snapshot()?.panes.into_iter().find(|p| p.pane_id == *id).and_then(|p| p.label) { if let Ok(n) = PaneName::parse(&label) { return Ok(n); } }
        Err(PaneError::PaneNotFound { what: id.as_str().to_owned() })
    })
}
impl HerdrPort for BindingHerdr {
    fn ensure_pane(&self, s: &HerdrSpec) -> Result<HerdrPane, PaneError> { let p = self.inner.ensure_pane(s); eprintln!("TRIAL ensure_pane {s:?} -> {p:?}"); let p = p?; self.b.0.lock().unwrap().pending = Some(p.pane_id.clone()); Ok(p) }
    fn send_text(&self, p: &PaneId, t: &str) -> Result<(), PaneError> { self.inner.send_text(p, t) }
    fn send_keys(&self, p: &PaneId, k: &[Key]) -> Result<(), PaneError> { self.inner.send_keys(p, k) }
    fn read(&self, p: &PaneId, n: usize) -> Result<String, PaneError> { self.inner.read(p, n) }
    fn close(&self, p: &PaneId) -> Result<(), PaneError> { self.inner.close(p) }
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> { self.inner.snapshot() }
    fn version(&self) -> Result<String, PaneError> { self.inner.version() }
}
impl HerdrPort for LazyHerdr {
    fn ensure_pane(&self, s: &HerdrSpec) -> Result<HerdrPane, PaneError> { self.port()?.ensure_pane(s) }
    fn send_text(&self, p: &PaneId, t: &str) -> Result<(), PaneError> { self.port()?.send_text(p, t) }
    fn send_keys(&self, p: &PaneId, k: &[Key]) -> Result<(), PaneError> { self.port()?.send_keys(p, k) }
    fn read(&self, p: &PaneId, n: usize) -> Result<String, PaneError> { self.port()?.read(p, n) }
    fn close(&self, p: &PaneId) -> Result<(), PaneError> { eprintln!("TRIAL close {p:?}"); self.port()?.close(p) }
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> { let r = self.port()?.snapshot(); eprintln!("TRIAL snapshot -> {r:?}"); r }
    fn version(&self) -> Result<String, PaneError> { self.port()?.version() }
}
impl HostPort for BindingHost {
    fn ensure_session(&self, n: &PaneName, c: &str) -> Result<(), PaneError> { self.inner.ensure_session(n, c)?; let mut g = self.b.0.lock().unwrap(); if let Some(p) = g.pending.take() { g.bound.insert(p.as_str().to_owned(), n.clone()); } Ok(()) }
    fn run(&self, n: &PaneName, c: &str, a: &Argv) -> Result<(), PaneError> { self.inner.run(n, c, a) }
    fn stop_owned(&self, n: &PaneName) -> Result<(), PaneError> { self.inner.stop_owned(n) }
    fn ps(&self, n: &PaneName) -> Result<Vec<u32>, PaneError> { self.inner.ps(n) }
}
impl HarnessPort for WiredHarness {
    fn serve_command(&self, n: &PaneName, p: u16, s: &ServeSpec) -> Result<Argv, PaneError> { self.h()?.serve_command(n, p, s) }
    fn verify_serving(&self, p: u16, s: &ServeSpec) -> Result<(), PaneError> { self.h()?.verify_serving(p, s) }
    fn health(&self, p: u16) -> Result<bool, PaneError> { self.h()?.health(p) }
    fn create_session(&self, p: u16) -> Result<String, PaneError> { self.h()?.create_session(p) }
    fn list_sessions(&self, p: u16) -> Result<Vec<String>, PaneError> { self.h()?.list_sessions(p) }
    fn abort(&self, p: u16, s: &str) -> Result<(), PaneError> { self.h()?.abort(p, s) }
    fn attach_tui(&self, x: &PaneId, p: u16, s: &str) -> Result<(), PaneError> { let r = self.h()?.attach_tui(x, p, s); eprintln!("TRIAL attach_tui {x:?} -> {r:?}"); r }
    fn select_session(&self, x: &PaneId, s: &str) -> Result<(), PaneError> { self.h()?.select_session(x, s) }
    fn shown_session(&self, x: &PaneId) -> Result<Option<String>, PaneError> { let r = self.h()?.shown_session(x); eprintln!("TRIAL shown {x:?} -> {r:?}"); r }
    fn send_prompt(&self, p: u16, s: &str, t: &str) -> Result<(), PaneError> { self.h()?.send_prompt(p, s, t) }
    fn session_activity(&self, p: u16, s: &str) -> Result<Activity, PaneError> { self.h()?.session_activity(p, s) }
}
