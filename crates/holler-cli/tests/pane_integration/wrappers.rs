//! The glue `bind` and `observe` share to check the wiring's wrapper ports (#846; the #649
//! gate's W-7): [`CallLog`], the call log of a recording port; [`herdr_suite_over`], the Herdr
//! conformance suite over a wrapper of the fake; and [`unbuildable_lazy_herdr`], a `LazyHerdr`
//! whose build fails.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use holler_cli::pane::wiring::bind::LazyHerdr;
use holler_pane::{HerdrPort, PaneError, PaneId};
use holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture};
use holler_pane_testkit::conformance::Conformance;
use holler_pane_testkit::herdr::FakeHerdr;

/// The methods a recording port was called by, oldest first.
#[derive(Default)]
pub struct CallLog(Mutex<Vec<&'static str>>);

impl CallLog {
    pub fn note(&self, method: &'static str) {
        self.lock().push(method);
    }

    pub fn list(&self) -> Vec<&'static str> {
        self.lock().clone()
    }

    pub fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<&'static str>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The Herdr conformance suite over the wrapper `wrap` builds around a fresh fake (one session
/// and one workspace `scratch`, two rows by one column), the label hook renaming in that fake.
pub fn herdr_suite_over<W: HerdrPort>(wrap: impl Fn(Arc<dyn HerdrPort>) -> W) -> Conformance {
    let rename = |_: &HerdrFixture<W>, fake: &Arc<FakeHerdr>, pane: &PaneId, label: &str| {
        fake.rename(pane, label)
    };
    run_herdr_conformance(
        || {
            let fake = Arc::new(
                FakeHerdr::new("scratch")
                    .with_workspace("scratch", 2, 1)
                    .unwrap(),
            );
            let inner: Arc<dyn HerdrPort> = fake.clone();
            let fixture = HerdrFixture {
                port: wrap(inner),
                session: "scratch".into(),
                workspace: "scratch".into(),
            };
            (fixture, fake)
        },
        Some(&rename),
    )
}

/// A `LazyHerdr` whose build fails with `usage` naming `HOLLER_HERDR_SESSION`, and the count of
/// its build attempts.
pub fn unbuildable_lazy_herdr() -> (LazyHerdr, Arc<AtomicUsize>) {
    let built = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&built);
    let lazy = LazyHerdr::new(move || {
        count.fetch_add(1, Ordering::SeqCst);
        Err(PaneError::Usage {
            message: "no Herdr session: set HOLLER_HERDR_SESSION".to_owned(),
        })
    });
    (lazy, built)
}
