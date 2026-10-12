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
//! - #835's acceptance item (brief R3-3): `pane list` over the wiring's Herdr chain shows an
//!   unmanaged pane's working directory, and its last screen change only when the adapter
//!   reports one. D33 made this hold before #835, so it is a regression pin: it guards the
//!   chain once `observe_pane` loses its default (#835).

use std::sync::atomic::Ordering;
use std::sync::Arc;

use holler_cli::output::Format;
use holler_cli::pane::wiring::bind::{BindingHerdr, Bindings, LazyHerdr};
use holler_pane::{
    Actor, GridPos, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId,
    PaneObservation, Ports,
};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::seed_unmanaged_pane;
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::FakePaneStore;
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;

use crate::verb_harness::{run_verb_with, Outcome};
use crate::wrappers::{herdr_suite_over, unbuildable_lazy_herdr, CallLog};

#[test]
fn the_lazy_herdr_passes_the_herdr_conformance_suite() {
    let result = herdr_suite_over(|inner| LazyHerdr::new(move || Ok(Arc::clone(&inner))));
    assert_eq!(result, Ok(()));
}

// --- the recording checks -----------------------------------------------------------------

/// A Herdr port that records which observe method was called, and answers each pane with an
/// observation the trait's defaults never make (its own id as the working directory). Its
/// other methods are never called by these checks.
#[derive(Default)]
struct Observing {
    calls: CallLog,
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
        self.calls.note("observe_pane");
        Ok(seen(pane))
    }
    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
        self.calls.note("observe_panes");
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
        inner.calls.list(),
        vec!["observe_pane"],
        "{which}: observe_pane reached the inner port once"
    );

    inner.calls.clear();
    let panes = three_panes();
    assert_eq!(
        wrapper.observe_panes(&panes),
        panes.iter().map(seen).collect::<Vec<_>>(),
        "{which}: observe_panes answers the inner port's observations, in order"
    );
    assert_eq!(
        inner.calls.list(),
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
    let (lazy, built) = unbuildable_lazy_herdr();
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

// --- #835 R3-3: `pane list` over the wiring's Herdr chain -------------------------------

const LABEL: &str = "seeding-pane";
const CWD: &str = "/work/seeding";

/// `holler pane list` over an empty registry and the wiring's production Herdr chain
/// (`BindingHerdr` over `LazyHerdr`, as `Wiring::connect` builds it) over `herdr`.
fn list_over_the_chain(herdr: Arc<FakeHerdr>, format: Format) -> Outcome {
    let inner: Arc<dyn HerdrPort> = herdr;
    let lazy: Arc<dyn HerdrPort> = Arc::new(LazyHerdr::new(move || Ok(Arc::clone(&inner))));
    let chain = BindingHerdr::new(lazy, Bindings::new());
    let panes = Arc::new(FakePaneStore::new());
    let profiles = Arc::new(FakeProfileStore::new());
    let scope = FakeProfileScope::new(
        profiles.clone(),
        panes.clone(),
        Actor::parse("holler pane").unwrap(),
    );
    let (host, harness, prober) = (FakeHost::new(), FakeHarness::new(), FakeProber::new());
    let ports = Ports {
        pane_store: panes.as_ref(),
        profile_store: profiles.as_ref(),
        herdr: &chain,
        host: &host,
        harness: &harness,
        scope: &scope,
        prober: &prober,
    };
    run_verb_with(&["pane", "list"], format, ports)
}

/// A fake Herdr holding one unmanaged pane at `r1c1` that works in [`CWD`], whose screen last
/// changed at `changed` (`None`: Herdr reports no time, as the real adapter does today, #837).
fn unmanaged(changed: Option<i64>) -> (Arc<FakeHerdr>, String) {
    let herdr = Arc::new(FakeHerdr::new("scratch").with_workspace("scratch", 3, 3).unwrap());
    let seeded = seed_unmanaged_pane(&herdr, GridPos { row: 1, col: 1 }, LABEL, CWD, changed).unwrap();
    (herdr, seeded.pane_id.as_str().to_owned())
}

/// The unmanaged table's row of `id` in the text form, as its cells.
fn text_row(out: &str, id: &str) -> Vec<String> {
    let rows: Vec<Vec<String>> = out
        .lines()
        .map(|line| line.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
        .filter(|cells| cells.first().map(String::as_str) == Some(id))
        .collect();
    assert_eq!(rows.len(), 1, "one row for {id}: {out}");
    rows[0].clone()
}

#[test]
fn pane_list_over_the_wirings_herdr_chain_shows_the_cwd_and_the_screen_change() {
    let changed = 1_700_000_000_000;
    let (herdr, id) = unmanaged(Some(changed));
    let run = list_over_the_chain(Arc::clone(&herdr), Format::Json);
    let data = check_envelope(&run.out, run.code).unwrap().data;
    let row = &data["unmanaged"][0];
    assert_eq!(row["pane_id"], id.as_str(), "{data}");
    assert_eq!(row["cwd"], CWD, "the working directory reaches the list: {row}");
    assert_eq!(row["last_screen_change"], changed, "the adapter's time reaches the list: {row}");

    let text = list_over_the_chain(herdr, Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    let cells = text_row(&text.out, &id);
    assert_eq!(cells[1..4], [LABEL, "r1c1", CWD], "{}", text.out);
    assert_ne!(cells.last().map(String::as_str), Some("-"), "an idle time: {}", text.out);
}

#[test]
fn pane_list_over_the_wirings_herdr_chain_invents_no_screen_change() {
    let (herdr, id) = unmanaged(None);
    let run = list_over_the_chain(Arc::clone(&herdr), Format::Json);
    let data = check_envelope(&run.out, run.code).unwrap().data;
    let row = &data["unmanaged"][0];
    assert_eq!(row["cwd"], CWD, "{row}");
    assert!(row["last_screen_change"].is_null(), "never guessed: {row}");

    let text = list_over_the_chain(herdr, Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    let cells = text_row(&text.out, &id);
    assert_eq!(cells[3], CWD, "{}", text.out);
    assert_eq!(cells.last().map(String::as_str), Some("-"), "{}", text.out);
}
