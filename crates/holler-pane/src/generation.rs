//! The compare-and-swap rule on a record's `generation` (epic #633, invariant I3).
//!
//! Every record the hub owns ([`crate::Pane`], [`crate::Profile`]) carries a
//! `generation: u64` that is bumped on every change, and every write is a
//! compare-and-swap on it: the caller names the generation it read, and the store
//! applies the write only if the record is still at that generation. A write that
//! lost the race is [`PaneError::Conflict`] (`generation-conflict`), and changes
//! nothing.
//!
//! A record that does not exist yet is at generation `0`, so a create names
//! `expected_generation: 0`, and its first stored generation is `1`.
//!
//! The rule is written here once; the fakes of #638 and the stores of #639 and #661
//! all call [`next_generation`] instead of writing it again.

use crate::error::PaneError;

/// The generation a record moves to when a write that read `expected` is applied
/// to a record now at `current`.
///
/// `Err(PaneError::Conflict)` when `expected != current`: the caller read an older
/// version and must read again before retrying. The counter cannot realistically
/// overflow; if it ever did the store is not trustworthy, so that is
/// `store-corrupt` rather than a wrap.
pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
    if current != expected {
        return Err(PaneError::Conflict);
    }
    current
        .checked_add(1)
        .ok_or_else(|| PaneError::StoreCorrupt {
            what: "a generation counter overflowed".to_owned(),
        })
}
