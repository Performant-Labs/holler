//! `holler_pane` — the types, error codes and ports of pane control (epic #633).
//!
//! Holler is the only thing that changes a pane: one registry on the hub holds, for
//! every pane, the Herdr pane, the tmux session, the harness server and the one
//! session of record, and Herdr, tmux and OpenCode are reached only through adapters.
//! This crate is the contract that makes that parallel to build. It holds **types and
//! traits only**: no async runtime, no I/O, no behaviour behind a stub. It depends on
//! `serde`, `serde_json` and `holler-proto` (for `SessionName`).
//!
//! The modules:
//!
//! - [`error`] — the closed set of kebab-case error codes, [`PaneError`] with one
//!   variant per code, the one validator ([`error::is_valid_code`]) and the open
//!   `Refused` variant for codes an adapter or a verb owns.
//! - [`reply`] — [`PaneReply`], the wire form of the hub's `pane/*` and `profile/*`
//!   control methods, and their params structs.
//! - [`pane`] — the [`Pane`] record and [`PaneName`].
//! - [`profile`] — [`Profile`], [`ProfileSpec`], [`ProfileName`] with its one slug,
//!   [`Actor`], and the [`ProfileStore`] and [`ProfileScope`] traits.
//! - [`generation`] — the compare-and-swap rule on a record's `generation`.
//! - [`grid`] — [`GridPos`], the one grid-position type (ROWCOL, 1-based).
//! - [`argv`] — [`Argv`] and [`EnvVarName`], the guards that keep a stored command an
//!   array and a stored environment a list of names.
//! - [`ports`] — [`PaneStore`], [`HerdrPort`], [`HostPort`], [`HarnessPort`],
//!   [`Prober`] and the [`Ports`] bundle a verb holds.
//! - [`probe`] — [`ProbeResult`] and the [`run_probe`] stub.
//! - `profile_snapshot`, `profile_diff`, `tx_apply`, `tx_launch`, `tx_switch`,
//!   `reconcile`, `findings`, `import` — empty stubs, declared here so that no two
//!   stories edit this file; each names the story that fills it.
//!
//! **Frozen when #637 merges.** After that a change to a signature or a code goes
//! through the epic's amend-first rule. `HerdrPort` and `HarnessPort` stay
//! provisional until the spikes #636 and #635 report.
//!
//! **Serde policy.** The stored records and the params structs refuse unknown fields:
//! a record read, changed and written back by a peer that does not know a field would
//! drop it silently. A [`PaneReply`], which a client never writes back, ignores them.
//! Optional fields are left out when absent and default to absent on read.

pub mod argv;
pub mod error;
pub mod findings;
pub mod generation;
pub mod grid;
pub mod import;
pub mod pane;
pub mod ports;
pub mod probe;
pub mod profile;
pub mod profile_diff;
pub mod profile_snapshot;
pub mod reconcile;
pub mod reply;
pub mod tx_apply;
pub mod tx_launch;
pub mod tx_switch;

// Flat re-exports so the common path is `holler_pane::{...}`, the way `holler_proto`
// does it. Nothing here shares a name with a root re-export of `holler_proto`
// (that crate's `Role` is the A2A role; the pane role is `PaneRole`). The error
// vocabulary beyond `PaneError` stays under `error::`.
pub use argv::{Argv, EnvVarName};
pub use error::PaneError;
pub use generation::next_generation;
pub use grid::GridPos;
pub use pane::{HerdrPane, Pane, PaneEvent, PaneId, PaneName};
pub use ports::{
    Cursor, HarnessPort, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort, Key, PaneStore, Ports,
    Prober, SystemProber, Watch,
};
pub use probe::{run_probe, ProbeResult};
pub use profile::{
    Actor, Profile, ProfileChange, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope,
    ProfileSpec, ProfileStore, ResolvedScope, SpecEdit,
};
pub use reply::{decode_params, PaneReply, ReplyError, WatchReply};
