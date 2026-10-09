//! The health probe of a pane (epic #633, B1): a `check` argv whose output must
//! contain every `expect` string.
//!
//! **Frozen by #637:** [`ProbeResult`] (which `Pane.probe.last` persists) and the
//! signature of [`run_probe`]. The body of [`run_probe`] is #663's; until it lands
//! the function is a stub that never reports success, so a launch that should be
//! refused with `probe-failed` cannot sail through on a stub.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::argv::Argv;

/// What one run of a health probe found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeResult {
    /// The probe ran and every expected string was in its output.
    Ok,
    /// The probe ran and these expected strings were not in its output.
    Failed { missing: Vec<String> },
    /// The probe could not be run to a verdict (the program is missing, it timed
    /// out, ...); the reason is plain text.
    Error(String),
}

/// Run the health probe `argv` (never through a shell) and look for every string of
/// `expect` in its output, giving up after `timeout`.
///
/// **Stub (#637):** the real runner is built by #663. This one always answers
/// [`ProbeResult::Error`], never [`ProbeResult::Ok`].
pub fn run_probe(argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
    let _ = (argv, expect, timeout);
    ProbeResult::Error("the probe runner is not implemented yet (story #663)".to_owned())
}
