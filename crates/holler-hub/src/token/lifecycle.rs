//! How a token ends (issue #454), split out of `token.rs` to keep that file
//! under the 900-line build guard (the `holds.rs` → `holds/grants.rs` split).
//!
//! Two operator verbs, two meanings:
//!
//! - `hub token revoke ID` is [`delete`]: the token stops working, and its row
//!   is **kept** (an audit trail of who was cut off), so it still holds its
//!   label.
//! - `hub token delete ID` is [`purge`]: an `unused` or `revoked` token's row
//!   is **removed**, which frees its label for a new mint. A `bound` token is
//!   refused: it must be revoked first.
//!
//! (The store names predate #454, when both verbs ran [`delete`].) Both reuse
//! the parent's lock and load/save cycle, so they serialize with every other
//! store operation and fail closed on a corrupt store without writing.

use holler_proto::TokenError;

use super::{acquire_lock_retrying, state_str, tokens_path, Record, Store, TokenState};
use crate::state::HubState;

/// `hub token revoke ID`: inactivate a token and **keep its row**. If it is
/// `unused` the join secret is invalid; if it is `bound` the credential is
/// invalid, and the row keeps its hostname / last_seen so `list` shows who
/// was cut off. The kept row still holds its label until [`purge`] removes
/// it. The CLI then asks a live hub to close the socket
/// (`control::revoke_live`, best-effort).
///
/// Returns the record's new shape. The operator-facing message distinguishes
/// `invalidated` (unused) from `revoked` (bound) in the CLI, not here.
pub fn delete(token_id: &str, state: &HubState) -> Result<Record, TokenError> {
    let _lock = acquire_lock_retrying(state)?;
    let mut store = Store::load(&tokens_path(state))?;
    let Some(record) = store.by_mut(token_id) else {
        return Err(TokenError::new(format!("no such token {token_id}")));
    };
    record.state = TokenState::Revoked;
    let out = record.clone();
    store.save(&tokens_path(state))?;
    Ok(out)
}

/// `hub token delete ID`: remove an `unused` or `revoked` token's row, which
/// frees its label for a new mint. A `bound` token is refused and the store is
/// left as it was: it must be revoked first ([`delete`]), so the record of
/// which machine was cut off is never erased in the same step as the cut. The
/// state is checked here, under the lock, never trusted from a caller's
/// earlier read. An unused or revoked token has no live socket, so there is
/// nothing to close.
///
/// Returns the removed record as it was, its prior state included.
pub fn purge(token_id: &str, state: &HubState) -> Result<Record, TokenError> {
    let _lock = acquire_lock_retrying(state)?;
    let mut store = Store::load(&tokens_path(state))?;
    let Some(record) = store.by_mut(token_id) else {
        return Err(TokenError::new(format!("no such token {token_id}")));
    };
    if record.state == TokenState::Bound {
        return Err(TokenError::new(format!(
            "token {token_id} ({}) is bound; revoke it first: holler hub token revoke {token_id}",
            record.label
        )));
    }
    let removed = record.clone();
    store.records.retain(|r| r.token_id != token_id);
    store.save(&tokens_path(state))?;
    Ok(removed)
}

/// Refuse a mint over a label that a record on file holds (ADR 0005 §1). The
/// refusal names the holder and the commands that free the label: `delete`
/// for an unused or revoked holder, `revoke` and then `delete` for a bound
/// one. The message keeps `label` and `already in use`, which the CLI's exit
/// code mapping and older scripts match on.
pub(super) fn ensure_label_free(store: &Store, label: &str) -> Result<(), TokenError> {
    let holder = store
        .label_to_key
        .get(label)
        .and_then(|id| store.records.iter().find(|r| r.token_id == *id));
    let Some(holder) = holder else {
        return Ok(());
    };
    let id = &holder.token_id;
    let fix = match holder.state {
        TokenState::Bound => format!("holler hub token revoke {id}, then holler hub token delete {id}"),
        TokenState::Unused | TokenState::Revoked => format!("holler hub token delete {id}"),
    };
    Err(TokenError::new(format!(
        "label {label:?} already in use by {} token {id}; free it with: {fix}",
        state_str(holder.state)
    )))
}
