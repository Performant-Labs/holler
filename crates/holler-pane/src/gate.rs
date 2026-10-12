//! The prompt gate's one predicate and its one selection (#839, ADR-0021 section 11 point 1).
//!
//! A prompt to a pane's session is refused while the pane's record says it is parked,
//! unhealthy, or shows a session other than the one its body drives. Three places ask:
//!
//! - the hub's gate at `send_prompt` (`holler-hub`'s `panes/gate.rs`), for every prompt, keyed
//!   on the advertised `harness_session_id` of the session being prompted;
//! - the `--pane` routing of `say`, `interrupt` and `answer` (`holler-cli`'s `say_cmd`), before
//!   any hub is contacted;
//! - the test kit's `FakeHub`, which stands for the hub's gate in the verbs' tests.
//!
//! They share [`prompt_refusal`] (the predicate) and [`first_refusal`] (the selection), so the
//! rule is written once; each keeps its own message text. `holler roster`'s SHOWN/DRIVEN column
//! calls [`shown_driven_pair_differs`], the pair rule the predicate uses.
//!
//! The selection keys on the id being prompted: the panes whose session of record **or recorded
//! DRIVEN** it is. The recorded DRIVEN closes the case the session of record cannot see: a body
//! that is behind (it still drives an earlier session of a pane) advertises an id no record names
//! as its session of record, and once a pass has recorded that id as the pane's DRIVEN, the pair
//! refuses it. A body that is behind and that no pass or verb has observed yet is not gated: the
//! window ADR-0021 section 11 states.

use crate::error::RefusalCode;
use crate::pane::{Health, Hold, LastObserved};
use crate::Pane;

/// `pane-parked`: the pane is parked (`holler pane park`). Refusal, exit 3.
pub const PANE_PARKED: RefusalCode = RefusalCode::from_static("pane-parked");
/// `pane-unhealthy`: the pane's record says its harness is unhealthy. Refusal, exit 3.
pub const PANE_UNHEALTHY: RefusalCode = RefusalCode::from_static("pane-unhealthy");
/// `pane-shown-driven-mismatch`: the pane's record says it shows one session while its body
/// drives another. Refusal, exit 3.
pub const PANE_SHOWN_DRIVEN_MISMATCH: RefusalCode =
    RefusalCode::from_static("pane-shown-driven-mismatch");

/// Why a pane's record refuses a prompt to its session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptRefusal {
    /// The pane is parked.
    Parked,
    /// The pane's harness is unhealthy.
    Unhealthy,
    /// SHOWN and DRIVEN were both observed, and they differ.
    ShownDrivenMismatch,
}

impl PromptRefusal {
    /// The refusal's open code.
    pub fn code(self) -> RefusalCode {
        match self {
            PromptRefusal::Parked => PANE_PARKED,
            PromptRefusal::Unhealthy => PANE_UNHEALTHY,
            PromptRefusal::ShownDrivenMismatch => PANE_SHOWN_DRIVEN_MISMATCH,
        }
    }
}

/// Whether the pane shows one session and drives another: both sides observed, and different.
/// One side absent is never a mismatch.
pub fn shown_driven_pair_differs(seen: &LastObserved) -> bool {
    matches!(
        (seen.shown.as_deref(), seen.driven.as_deref()),
        (Some(shown), Some(driven)) if shown != driven
    )
}

/// What `pane`'s record says against prompting its session, checked in the order parked,
/// unhealthy, then the SHOWN/DRIVEN pair; `None` when nothing does. The record is the source,
/// never a live probe.
pub fn prompt_refusal(pane: &Pane) -> Option<PromptRefusal> {
    if matches!(pane.hold, Hold::Parked { .. }) {
        return Some(PromptRefusal::Parked);
    }
    if matches!(pane.harness.health, Health::Unhealthy(_)) {
        return Some(PromptRefusal::Unhealthy);
    }
    shown_driven_pair_differs(&pane.last_observed).then_some(PromptRefusal::ShownDrivenMismatch)
}

/// The prompt gate's selection for the harness session `id`: of the panes whose session of
/// record or recorded DRIVEN is `id`, the first that [`prompt_refusal`] refuses, with why, in
/// the slice's order (a store lists by name). Fail closed: panes that record one id are judged
/// together, and any bad one refuses.
pub fn first_refusal<'a>(panes: &'a [Pane], id: &str) -> Option<(&'a Pane, PromptRefusal)> {
    panes
        .iter()
        .filter(|pane| {
            pane.session_of_record.as_deref() == Some(id)
                || pane.last_observed.driven.as_deref() == Some(id)
        })
        .find_map(|pane| prompt_refusal(pane).map(|refusal| (pane, refusal)))
}
