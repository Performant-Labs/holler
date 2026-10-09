//! `FakeProber`, a `Prober` that answers a scripted `ProbeResult` per argv and records
//! every run, so no verb test waits for the real probe runner (#663) (slice d of #638,
//! #683).
//!
//! An argv nobody scripted answers [`ProbeResult::Error`], never [`ProbeResult::Ok`],
//! so a test that forgot to script a probe cannot pass it (the same reason the stub
//! `holler_pane::run_probe` never answers `Ok`). The fake has no fault switch: a fault
//! answers a `PaneError`, and a `Prober` answers a `ProbeResult`, so a probe that times
//! out or fails is simply a scripted `ProbeResult::Error`.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use holler_pane::{Argv, ProbeResult, Prober};

/// One run of the fake probe, as it was called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCall {
    /// The program and its arguments.
    pub argv: Argv,
    /// The strings the run looked for in the output.
    pub expect: Vec<String>,
    /// How long the run was given.
    pub timeout: Duration,
}

/// A `Prober` that answers a scripted `ProbeResult` per argv.
///
/// It is not `Clone`: share it behind an `Arc`, because a copy would split the script
/// and the record of the runs.
pub struct FakeProber {
    state: Mutex<State>,
}

/// What the fake holds, behind its lock.
struct State {
    /// The answer for each argv, matched element by element.
    scripted: HashMap<Argv, ProbeResult>,
    /// Every run, oldest first.
    calls: Vec<ProbeCall>,
}

impl FakeProber {
    /// Nothing scripted: every run answers `ProbeResult::Error` naming the argv, never
    /// `Ok`.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                scripted: HashMap::new(),
                calls: Vec::new(),
            }),
        }
    }

    /// Answer `result` for every run of exactly `argv` (equal element by element).
    /// Scripting the argv again replaces its result.
    pub fn script(&self, argv: Argv, result: ProbeResult) {
        self.lock().scripted.insert(argv, result);
    }

    /// Every run, oldest first, with its `expect` and `timeout`; the unscripted runs are
    /// included.
    pub fn calls(&self) -> Vec<ProbeCall> {
        self.lock().calls.clone()
    }

    /// The state, locked. A poisoned lock is taken over: a test that panicked while
    /// holding it must not wedge every later call.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for FakeProber {
    fn default() -> Self {
        Self::new()
    }
}

impl Prober for FakeProber {
    /// Record the run, then answer the result scripted for `argv`, or else an
    /// `Error` naming it.
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
        let mut state = self.lock();
        state.calls.push(ProbeCall {
            argv: argv.clone(),
            expect: expect.to_vec(),
            timeout,
        });
        state.scripted.get(argv).cloned().unwrap_or_else(|| {
            ProbeResult::Error(format!("no probe scripted for {:?}", argv.as_slice()))
        })
    }
}
