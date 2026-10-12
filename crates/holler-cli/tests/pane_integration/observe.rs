//! D33 (#832; S's defect 1, A-dup's W-5): the wiring's two Herdr wrappers, [`BindingHerdr`]
//! and [`LazyHerdr`], delegate `HerdrPort::observe_pane` and `observe_panes`, the two
//! default-armed methods #832 added, as ADR-0021 section 14 asks of a wrapper port ("A
//! wrapper port must delegate it") and D31 W-4 asks of every delegating impl.
//!
//! - The Herdr conformance suite runs over `LazyHerdr` as well (the run over `BindingHerdr`
//!   is in `bind.rs`), so #832's case 13 (`observe_pane` of an unknown or a closed pane is
//!   `pane-not-found`) covers both wrappers.
//! - One recording check per wrapper: `observe_pane` answers the inner port's observation,
//!   and `observe_panes` reaches the inner `observe_panes` once and the inner `observe_pane`
//!   not at all, so the list stays section 14's one call to Herdr.
//! - A `LazyHerdr` whose build failed answers one empty observation per pane from
//!   `observe_panes`, the trait's contract for a failed call; its `observe_pane` answers the
//!   stored error (pinned in `bind.rs`'s every-method test).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use holler_cli::pane::wiring::bind::{BindingHerdr, Bindings, LazyHerdr};
use holler_pane::{
    HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId, PaneObservation,
};
use holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture};
use holler_pane_testkit::herdr::FakeHerdr;

/// The fake the suite runs over: one session, one workspace two rows by one column.
fn suite_fake() -> Arc<FakeHerdr> {
    Arc::new(
        FakeHerdr::new("scratch")
            .with_workspace("scratch", 2, 1)
            .unwrap(),
    )
}

#[test]
fn the_lazy_herdr_passes_the_herdr_conformance_suite() {
    let rename = |_: &HerdrFixture<LazyHerdr>,
                  fake: &Arc<FakeHerdr>,
                  pane: &PaneId,
                  label: &str| { fake.rename(pane, label) };
    let result = run_herdr_conformance(
        || {
            let fake = suite_fake();
            let inner: Arc<dyn HerdrPort> = fake.clone();
            let fixture = HerdrFixture {
                port: LazyHerdr::new(move || Ok(Arc::clone(&inner))),
                session: "scratch".into(),
                workspace: "scratch".into(),
            };
            (fixture, fake)
        },
        Some(&rename),
    );
    assert_eq!(result, Ok(()));
}

// --- the recording checks -----------------------------------------------------------------

/// A Herdr port that records which observe method was called, and answers each pane with an
/// observation the trait's defaults never make (its own id as the working directory). Its
/// other methods are never called by these checks.
#[derive(Default)]
struct Observing {
    calls: Mutex<Vec<&'static str>>,
}

impl Observing {
    fn note(&self, method: &'static str) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(method);
    }

    fn calls(&self) -> Vec<&'static str> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// What [`Observing`] reports for `pane`.
fn seen(pane: &PaneId) -> PaneObservation {
    PaneObservation::default()
        .with_cwd(format!("/observed/{}", pane.as_str()))
        .with_last_screen_change(1_000)
}

fn not_called(method: &str) -> PaneError {
    PaneError::Unavailable {
        what: format!("{method} is not part of the observe checks"),
    }
}

impl HerdrPort for Observing {
    fn ensure_pane(&self, _: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        Err(not_called("ensure_pane"))
    }
    fn send_text(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
        Err(not_called("send_text"))
    }
    fn send_keys(&self, _: &PaneId, _: &[Key]) -> Result<(), PaneError> {
        Err(not_called("send_keys"))
    }
    fn read(&self, _: &PaneId, _: usize) -> Result<String, PaneError> {
        Err(not_called("read"))
    }
    fn close(&self, _: &PaneId) -> Result<(), PaneError> {
        Err(not_called("close"))
    }
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        Err(not_called("snapshot"))
    }
    fn version(&self) -> Result<String, PaneError> {
        Err(not_called("version"))
    }
    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError> {
        self.note("observe_pane");
        Ok(seen(pane))
    }
    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
        self.note("observe_panes");
        panes.iter().map(seen).collect()
    }
}

fn three_panes() -> Vec<PaneId> {
    ["w1:p1", "w1:p2", "w1:p3"].map(PaneId::new).to_vec()
}

/// `observe_pane` through `wrapper` answers the inner port's observation; `observe_panes` of
/// three panes answers the inner one's three, from one call to the inner `observe_panes` and
/// none to its `observe_pane`.
fn assert_delegates_observing(wrapper: &dyn HerdrPort, inner: &Observing, which: &str) {
    let pane = PaneId::new("w1:p9");
    assert_eq!(
        wrapper.observe_pane(&pane),
        Ok(seen(&pane)),
        "{which}: observe_pane answers the inner port's observation, not the default empty one"
    );
    assert_eq!(
        inner.calls(),
        vec!["observe_pane"],
        "{which}: observe_pane reached the inner port once"
    );

    inner
        .calls
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    let panes = three_panes();
    assert_eq!(
        wrapper.observe_panes(&panes),
        panes.iter().map(seen).collect::<Vec<_>>(),
        "{which}: observe_panes answers the inner port's observations, in order"
    );
    assert_eq!(
        inner.calls(),
        vec!["observe_panes"],
        "{which}: observe_panes is one call to the inner observe_panes, never one observe_pane \
         per pane (ADR-0021 section 14)"
    );
}

#[test]
fn the_binding_herdr_delegates_observe_pane_and_observe_panes() {
    let inner = Arc::new(Observing::default());
    let port: Arc<dyn HerdrPort> = inner.clone();
    let wrapper = BindingHerdr::new(port, Bindings::new());
    assert_delegates_observing(&wrapper, &inner, "BindingHerdr");
}

#[test]
fn the_lazy_herdr_delegates_observe_pane_and_observe_panes() {
    let inner = Arc::new(Observing::default());
    let port: Arc<dyn HerdrPort> = inner.clone();
    let wrapper = LazyHerdr::new(move || Ok(Arc::clone(&port)));
    assert_delegates_observing(&wrapper, &inner, "LazyHerdr");
}

/// The production chain is `BindingHerdr` over `LazyHerdr` over the adapter
/// (`Wiring::connect`): the list still reaches the adapter as one `observe_panes`.
#[test]
fn the_wirings_herdr_chain_delegates_observe_panes_end_to_end() {
    let inner = Arc::new(Observing::default());
    let port: Arc<dyn HerdrPort> = inner.clone();
    let lazy: Arc<dyn HerdrPort> = Arc::new(LazyHerdr::new(move || Ok(Arc::clone(&port))));
    let wrapper = BindingHerdr::new(lazy, Bindings::new());
    assert_delegates_observing(&wrapper, &inner, "BindingHerdr over LazyHerdr");
}

/// A failed build: `observe_panes` answers one empty observation per pane, the trait's
/// contract for a call that failed, and does not retry the build.
#[test]
fn a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane() {
    let built = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&built);
    let lazy = LazyHerdr::new(move || {
        count.fetch_add(1, Ordering::SeqCst);
        Err(PaneError::Usage {
            message: "no Herdr session: set HOLLER_HERDR_SESSION".to_owned(),
        })
    });
    let panes = three_panes();
    assert_eq!(
        lazy.observe_panes(&panes),
        vec![PaneObservation::default(); 3]
    );
    assert_eq!(
        lazy.observe_panes(&[]),
        Vec::new(),
        "no pane, no observation"
    );
    assert_eq!(
        built.load(Ordering::SeqCst),
        1,
        "the failed build is kept, not retried"
    );
}
