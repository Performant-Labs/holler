//! The vocabulary of [`HarnessPort::compact`] (#878, ADR-0021 "Compaction"): what a compaction
//! answers, the model it may be asked to run on, and its two refusals.
//!
//! - [`CompactOutcome`]: the context size of the session before and after, both read from the
//!   harness, never inferred.
//! - [`CompactModel`]: a provider and model id, a name pair and never a secret (I7). The verb
//!   names none, so the harness runs the compaction on the model its own config pins.
//! - [`COMPACT_UNSUPPORTED`]: the default, for a harness that cannot compact.
//! - [`COMPACT_MODEL_UNPINNED`]: neither the caller nor the harness's own config named a model,
//!   so nothing is sent: a compaction is a model call and never runs on a model nobody chose.
//!
//! [`HarnessPort::compact`]: super::HarnessPort::compact

use crate::error::{PaneError, RefusalCode};

/// `compact-unsupported`: the default of `HarnessPort::compact`, for a harness that cannot
/// compact a session's context through its API (Claude Code until the spike of #878 Part B
/// decides otherwise). Refusal, exit 3.
pub const COMPACT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("compact-unsupported");

/// `compact-model-unpinned`: `HarnessPort::compact` named no model and the harness's own config
/// pins no compaction model (OpenCode's `agent.compaction.model`), so nothing was sent.
/// Refusal, exit 3.
pub const COMPACT_MODEL_UNPINNED: RefusalCode = RefusalCode::from_static("compact-model-unpinned");

/// What a compaction did to the context of a session: its size in tokens before the act and
/// after it, each read from the harness. A count the harness cannot read is `unavailable`,
/// never zero; whether the context shrank is the engine's to judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactOutcome {
    pub tokens_before: u64,
    pub tokens_after: u64,
}

/// The model a compaction runs on: a provider and a model id, a name pair (never a key, I7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactModel {
    pub provider: String,
    pub model_id: String,
}

/// The default answer of `HarnessPort::compact`: [`COMPACT_UNSUPPORTED`].
pub(super) fn compact_unsupported() -> PaneError {
    PaneError::Refused {
        code: COMPACT_UNSUPPORTED,
        message: "this harness does not compact a session's context".to_owned(),
    }
}
