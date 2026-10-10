//! The launch/relaunch transaction (plan, act, observe, record; I3 and I8). Story #644.
//!
//! **RED stub (#644, T):** every public item of the brief's API is declared with its value,
//! and the three functions answer `not-implemented`; F fills the bodies.

use std::time::Duration;

use crate::error::RefusalCode;
use crate::{Pane, PaneError, PaneName, Ports, Profile, ProfileName, ProfileSpec};

/// `pane-exists`: `launch` of a name that already has a record (relaunch it instead). A
/// refusal, exit 3.
pub const PANE_EXISTS: RefusalCode = RefusalCode::from_static("pane-exists");
/// `grid-occupied`: the target cell already holds a Herdr pane (never adopted, recorded or
/// not). A refusal, exit 3.
pub const GRID_OCCUPIED: RefusalCode = RefusalCode::from_static("grid-occupied");
/// `port-in-use`: a harness server already answers on the port before this verb started one
/// (never adopted). A refusal, exit 3.
pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
/// The budget of one whole verb run (I5's default bound), checked before every step.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
/// The timeout handed to `Prober::run_probe`.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// `HostInfo.name` of every pane recorded here (epic decision 3: Herdr, tmux and the harness
/// run on the hub's machine).
pub const HOST_NAME: &str = "localhost";
/// The `op` of the `timeout` a launch answers when its budget runs out.
pub const OP_LAUNCH: &str = "pane.launch";
/// The `op` of the `timeout` a relaunch answers when its budget runs out.
pub const OP_RELAUNCH: &str = "pane.relaunch";

/// The port a policy names: only `fixed:<port>`, `<port>` in 1..=65535 in canonical decimal.
/// Anything else is `usage`.
pub fn port_of_policy(policy: &str) -> Result<u16, PaneError> {
    // stub (#644 RED): F fills
    let _ = policy;
    Err(PaneError::NotImplemented)
}

/// How one engine run is bounded and clocked.
#[derive(Debug, Clone, Copy)]
pub struct TxOptions {
    /// The budget of the whole run ([`DEFAULT_BUDGET`]).
    pub budget: Duration,
    /// The timeout of the health probe ([`PROBE_TIMEOUT`]).
    pub probe_timeout: Duration,
    /// The wall clock stamped into `last_observed.at` (`holler_proto::clock::now_millis`).
    pub now_ms: fn() -> i64,
}

impl Default for TxOptions {
    fn default() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
            probe_timeout: PROBE_TIMEOUT,
            now_ms: holler_proto::clock::now_millis,
        }
    }
}

/// What `launch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub name: PaneName,
    /// Required for a live launch (`usage` otherwise); unused with `spec_only`.
    pub herdr_session: Option<String>,
    /// The effective spec, complete; the engine checks `spec.pane == name` (`usage`).
    pub spec: ProfileSpec,
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}

/// What `relaunch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaunchRequest {
    /// As the caller read it; its generation is the expected generation of the record write.
    pub record: Pane,
    /// The effective spec, complete.
    pub spec: ProfileSpec,
    /// The caller was asked to place the pane (`--grid`); the engine derives the move.
    pub grid_given: bool,
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}

/// What a run that succeeded stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launched {
    /// The stored record (`None` with `spec_only`).
    pub pane: Option<Pane>,
    /// The profile as `edit_spec` stored it (`None` without a profile).
    pub profile: Option<Profile>,
}

/// Why a run failed, and whether it got as far as the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxFailure {
    pub error: PaneError,
    /// `edit_spec` entered the act, or a step after it failed: the CLI prints the reconcile
    /// step.
    pub acted: bool,
}

impl From<PaneError> for TxFailure {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            acted: false,
        }
    }
}

/// Create a pane: plan, act, observe, record.
pub fn launch(
    ports: Ports<'_>,
    request: &LaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    // stub (#644 RED): F fills
    let _ = (ports, request, options);
    Err(TxFailure::from(PaneError::NotImplemented))
}

/// Launch a pane again, replacing its process.
pub fn relaunch(
    ports: Ports<'_>,
    request: &RelaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    // stub (#644 RED): F fills
    let _ = (ports, request, options);
    Err(TxFailure::from(PaneError::NotImplemented))
}
