//! The budget of one launch or relaunch run (I5): started when the engine is entered, checked
//! before every live step, and the deadline of the run's waits.

use std::time::{Duration, Instant};

use crate::PaneError;

/// The budget of one run, started when the engine is entered. Its end is fixed once, when it
/// starts, so [`Budget::check`] and the waits ([`Budget::deadline`]) agree on when it runs out.
pub(super) struct Budget {
    end: Instant,
    op: &'static str,
}

impl Budget {
    /// A budget of `limit` from now. A limit the clock cannot hold (hundreds of billions of
    /// years) is halved until it can, so such a budget still never runs out in practice, for
    /// `check` and the waits alike.
    pub(super) fn start(limit: Duration, op: &'static str) -> Self {
        let started = Instant::now();
        let mut limit = limit;
        let end = loop {
            if let Some(end) = started.checked_add(limit) {
                break end;
            }
            limit /= 2;
        };
        Self { end, op }
    }

    /// `timeout` once the budget has run out: a reading of the monotonic clock against its end,
    /// no port call.
    pub(super) fn check(&self) -> Result<(), PaneError> {
        if Instant::now() < self.end {
            return Ok(());
        }
        Err(self.expired())
    }

    /// The `timeout` of a run whose budget ran out, its `op` the verb's.
    pub(super) fn expired(&self) -> PaneError {
        let op = self.op.to_owned();
        PaneError::Timeout { op }
    }

    /// When the budget runs out, the waits' deadline: the instant from which `check` fails, so
    /// a wait asks only while `check` would pass.
    pub(super) fn deadline(&self) -> Instant {
        self.end
    }
}
