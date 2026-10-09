//! The fault switch every fake of this crate shares: a standing fault (a port that
//! does not answer, or one that fails every call), one-shot errors queued per method,
//! a delay before every call, and the log of the calls made through the port.
//!
//! A fake of this crate calls the switch (its crate-private `enter`) first in every
//! port method. When that returns an error, the method returns the error and changes
//! nothing. A test reaches the switch through the fake's `faults()`, both to inject a
//! fault and to read back the calls the code under test made.

use std::fmt::Debug;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use holler_pane::PaneError;

/// A port method that a fault can target and the call log records. Each fake has its
/// own enum of them, e.g. [`crate::pane_store::PaneStoreOp`].
pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static {
    /// `"<port>.<method>"`, e.g. `"pane_store.cas_put"`. It is also the `op` of the
    /// `timeout` a wedged call answers.
    fn as_str(self) -> &'static str;
}

/// A standing fault: it applies to every call until it is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
    /// timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
    Fail(PaneError),
}

/// The faults of one fake port and the log of the calls made through it.
///
/// Every method takes `&self`, so a test can share the fake behind an `Arc` and still
/// drive its faults.
pub struct FaultSwitch<Op: PortOp> {
    state: Mutex<State<Op>>,
}

/// What the switch holds, behind its lock.
struct State<Op> {
    standing: Option<Fault>,
    /// The one-shot errors, oldest first. Each one fails the next call of its method.
    queued: Vec<(Op, PaneError)>,
    delay: Option<Duration>,
    calls: Vec<Op>,
}

impl<Op: PortOp> FaultSwitch<Op> {
    /// A switch with no fault, no delay and an empty call log.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                standing: None,
                queued: Vec::new(),
                delay: None,
                calls: Vec::new(),
            }),
        }
    }

    /// Turn a standing fault on (`Some`) or off (`None`).
    pub fn set(&self, fault: Option<Fault>) {
        self.lock().standing = fault;
    }

    /// Fail the next call of `op` with `error`, once. The errors queued for one method
    /// come out in the order they were queued, and a call of another method leaves them
    /// queued. A standing fault answers first, also leaving them queued.
    pub fn fail_next(&self, op: Op, error: PaneError) {
        self.lock().queued.push((op, error));
    }

    /// Make every call sleep for `delay` before it answers, as a slow port does
    /// (`None`: no delay).
    pub fn set_delay(&self, delay: Option<Duration>) {
        self.lock().delay = delay;
    }

    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
        self.lock().calls.clone()
    }

    /// What a fake calls first in every port method. It records the call, sleeps for
    /// the delay (without holding the lock, so other calls proceed), and then answers
    /// the standing fault if there is one, or else the oldest error queued for `op`.
    pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
        let delay = {
            let mut state = self.lock();
            state.calls.push(op);
            state.delay
        };
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        self.lock().take_fault(op)
    }

    fn lock(&self) -> MutexGuard<'_, State<Op>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<Op: PortOp> Default for FaultSwitch<Op> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Op: PortOp> State<Op> {
    /// The error a call of `op` answers now: the standing fault, else the oldest
    /// one-shot error queued for `op`, which this consumes.
    fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
        match &self.standing {
            Some(Fault::Wedged) => {
                return Err(PaneError::Timeout {
                    op: op.as_str().to_owned(),
                })
            }
            Some(Fault::Fail(error)) => return Err(error.clone()),
            None => {}
        }
        match self.queued.iter().position(|(queued, _)| *queued == op) {
            Some(at) => Err(self.queued.remove(at).1),
            None => Ok(()),
        }
    }
}
