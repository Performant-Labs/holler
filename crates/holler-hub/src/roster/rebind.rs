//! A re-pointed session on the roster (#839): `control/rebind` names the id a body acknowledged
//! on that one row the instant the body answers, ahead of the body's next presence beat. A child
//! module of `roster`, so its `impl Roster` reaches the private table (`roster.rs` is at the
//! file-size limit).
//!
//! It is a forward-fill of one field on one row, as `set_turn_id` and `set_last_turn` are, and
//! not a whole-set `Roster::advertise`: a control-socket task that advertised the body's cached
//! presence could put a stale frame back on the body's other rows, racing the connection's own
//! presence handler (which writes the roster first and the cache after), until the next beat.

use super::{row_matches_bare, Roster};

impl Roster {
    /// Set the `harness_session_id` of the row of `bare_name` that `token_id` holds (#839), the
    /// id its body acknowledged to a `session/rebind`. Matched as [`Roster::set_turn_id`]
    /// matches; a no-op (no generation bump) when no row matches or the row already names it.
    pub fn set_harness_session_id(&self, token_id: &str, bare_name: &str, harness_session_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut changed = false;
        for row in inner.rows.values_mut() {
            let ours = row.token_id == token_id && row_matches_bare(&row.name, bare_name);
            if ours && row.harness_session_id.as_deref() != Some(harness_session_id) {
                row.harness_session_id = Some(harness_session_id.to_owned());
                changed = true;
            }
        }
        if changed {
            inner.bump_gen();
        }
    }
}
