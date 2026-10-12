//! The hub port (#839, ADR-0021 section 11): what pane control reads and asks of the hub's live
//! sessions, through its control socket. The real one is `holler-cli`'s `pane/wiring/hub.rs`; the
//! test kit's is `FakeHub`.
//!
//! The session a pane's body drives is the harness session a live attach-mode body advertises
//! (`SessionAd.harness_session_id`, ADR 0014 point 6). [`HubPort::advertised`] lists them, so the
//! engines join a pane to its body by that id (`crate::driven`); [`HubPort::rebind`] asks a body
//! to drive another session of its endpoint (`session/rebind`); and [`HubPort::prompt`] is the
//! hub's one prompt path, the gate of `send_prompt`, asked without sending ([`PromptMode::Check`])
//! or sending ([`PromptMode::Send`]).
//!
//! **Blocking**, as every port: call it from `spawn_blocking` (or a thread) in async code, and it
//! returns within I5's bound or with [`PaneError::Timeout`]. Every method is **required**, with
//! no default, so the compiler finds every implementation, a wrapper included (#835's rule: a
//! default answers silently for a wrapper that forgot to forward).

use crate::error::RefusalCode;
use crate::ports::Activity;
use crate::PaneError;

/// `pane-not-driven`: no body drives the pane, or more than one could, or a listed body cannot
/// be shown to be its own; for `say`, `interrupt` and `answer` with `--pane`, the hub knows no
/// live body for the pane's session of record. Refusal, exit 3.
pub const PANE_NOT_DRIVEN: RefusalCode = RefusalCode::from_static("pane-not-driven");
/// `session-held`: an operator or default hold on the body's session refuses `pane reset
/// --first`'s message (the three `--pane` verbs keep their own held refusal, exit 4). Refusal,
/// exit 3.
pub const SESSION_HELD: RefusalCode = RefusalCode::from_static("session-held");

/// One live attach-mode body session the hub knows, with the harness session it advertises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advertised {
    /// The token of the body's connection (the roster row's `token_id`): how a rebind names
    /// its body.
    pub token: String,
    /// The session's routable name, `label/session`.
    pub name: String,
    /// The harness session it drives. Advertised text, so untrusted: a message quotes it.
    pub harness_session_id: String,
    /// What it is doing: the roster's `idle` is idle, `input-required` a held question, and
    /// anything else busy.
    pub state: Activity,
}

/// How [`HubPort::prompt`] asks the hub's prompt path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// Run the gate and send nothing: the question `pane reset --first` asks before the act.
    Check,
    /// Queue the prompt through `send_prompt`, accepted and returned, never waiting for the turn.
    Send,
}

/// The hub's live sessions, as pane control reads and drives them. See the module docs.
pub trait HubPort: Send + Sync {
    /// The live attach-mode sessions the hub knows, each with the harness session it advertises.
    fn advertised(&self) -> Result<Vec<Advertised>, PaneError>;

    /// Ask the body behind `driver` to drive `harness_session_id` instead, on the same endpoint;
    /// returns the id the body acknowledged. A hold does not refuse it (a hold stops prompts).
    fn rebind(&self, driver: &Advertised, harness_session_id: &str) -> Result<String, PaneError>;

    /// The hub's prompt path for the session advertising `harness_session_id`: with
    /// [`PromptMode::Check`] its gate (the hold, the pane gate, a held question) answers and
    /// nothing is sent; with [`PromptMode::Send`] `text` is queued through `send_prompt`.
    fn prompt(
        &self,
        harness_session_id: &str,
        text: &str,
        mode: PromptMode,
    ) -> Result<(), PaneError>;
}
