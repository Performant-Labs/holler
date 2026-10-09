//! The conformance suites: one generic function per port that runs a table of cases
//! against any implementation of the port (a fake of this crate, the hub's registry,
//! the CLI's client, an adapter) and returns every case that did not hold.
//!
//! A suite returns a [`Conformance`] instead of asserting: library code never panics,
//! and a caller can see which case failed, which the mutation tests rely on. Each
//! suite keeps its cases in one `(id, case)` table that its runner iterates and its
//! `*_cases()` function lists, so the two cannot drift. Each case runs against a fresh
//! implementation.
//!
//! - [`pane_store`] — `PaneStore` (slice a, #638).
//! - [`profile_store`] and [`profile_scope`] — `ProfileStore` and `ProfileScope`
//!   (slice c, #682).
//! - [`herdr`] — `HerdrPort` (slice d, #683).
//! - [`host`] and [`harness`] — `HostPort` and `HarnessPort` (slice e, #684).

pub mod harness;
pub mod herdr;
pub mod host;
pub mod pane_store;
pub mod profile_scope;
pub mod profile_store;

use std::fmt::Debug;

use holler_pane::{PaneError, Watch};

/// The most items [`drain`] reads before it gives up on a watch going idle.
const DRAIN_LIMIT: usize = 1_000;

/// One conformance case that did not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFailure {
    /// The case's id, as the suite's `*_cases()` function lists it.
    pub case: &'static str,
    /// Why the case did not hold.
    pub detail: String,
}

/// `Ok` when every case held; otherwise every failure, in case order.
pub type Conformance = Result<(), Vec<CaseFailure>>;

/// Run every case of `cases`, each against a fresh subject, and gather the failures in
/// case order. `fresh` returns the subject and a guard that is kept alive for that case
/// only (a temporary directory, say); the subject is dropped before its guard. `check`
/// runs one case against the subject.
pub(crate) fn run_cases<S, K, C: Copy>(
    cases: &[(&'static str, C)],
    mut fresh: impl FnMut() -> (S, K),
    mut check: impl FnMut(C, &S) -> Result<(), String>,
) -> Conformance {
    let failures: Vec<CaseFailure> = cases
        .iter()
        .filter_map(|&(case, run)| {
            let (subject, guard) = fresh();
            let outcome = check(run, &subject);
            drop(subject);
            drop(guard);
            outcome.err().map(|detail| CaseFailure { case, detail })
        })
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

/// The value of a call that must succeed; otherwise why it failed. `call` names the
/// call in the detail.
pub(crate) fn succeeds<T>(call: &str, result: Result<T, PaneError>) -> Result<T, String> {
    result.map_err(|e| format!("{call} failed with `{}`: {e}", e.code()))
}

/// `Ok` when `result` is an error with the code `code`; otherwise what the call did
/// instead. `call` names the call in the detail.
pub(crate) fn expect_code<T>(
    call: &str,
    result: Result<T, PaneError>,
    code: &str,
) -> Result<(), String> {
    match result {
        Err(e) if e.code() == code => Ok(()),
        Err(e) => Err(format!(
            "{call}: expected `{code}`, got `{}`: {e}",
            e.code()
        )),
        Ok(_) => Err(format!("{call}: expected `{code}`, but it succeeded")),
    }
}

/// `Ok` when `got` equals `want`; otherwise both, in the detail. `what` names the
/// value compared.
pub(crate) fn expect_eq<T: PartialEq + Debug>(what: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what}: expected {want:?}, got {got:?}"))
    }
}

/// The next item of `watch`, which must be `Ok`: a change, or `None` for idle. The end
/// of the stream or an error is a failure.
pub(crate) fn next_item<T>(watch: &mut Watch<T>) -> Result<Option<T>, String> {
    match watch.next() {
        Some(Ok(item)) => Ok(item),
        Some(Err(e)) => Err(format!("next() failed with `{}`: {e}", e.code())),
        None => Err("next() ended the stream; an open watch never ends".to_owned()),
    }
}

/// Every change `watch` yields before it goes idle; the idle `Ok(None)` is read too.
/// Reaching 1,000 items, the end of the stream or an error first is a failure, with
/// the reason.
pub(crate) fn drain<T>(watch: &mut Watch<T>) -> Result<Vec<T>, String> {
    let mut changes = Vec::new();
    while changes.len() < DRAIN_LIMIT {
        match watch.next() {
            Some(Ok(Some(change))) => changes.push(change),
            Some(Ok(None)) => return Ok(changes),
            Some(Err(e)) => {
                return Err(format!(
                    "the watch failed with `{}` before it went idle: {e}",
                    e.code()
                ))
            }
            None => {
                return Err("the watch ended before it went idle; an idle watch yields \
                    Ok(None) and stays open"
                    .to_owned())
            }
        }
    }
    Err(format!(
        "the watch did not go idle within {DRAIN_LIMIT} items"
    ))
}
