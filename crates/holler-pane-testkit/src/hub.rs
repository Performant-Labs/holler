//! `FakeHub`, the test kit's `HubPort` (#839, brief D10), and `HubOp`, the port's methods as its
//! faults and its call log name them.
//!
//! It stands for the hub's live attach bodies and its one prompt path, so that the verbs' tests
//! mean what the hub does:
//!
//! - `advertised` lists the scripted bodies ([`FakeHub::with_body`]) in the order they were
//!   added;
//! - `rebind` moves the matching body to the id and answers that id, or the scripted fault, and
//!   moves nothing on a fault; a hold does not refuse it (a hold stops prompts, not a rebind);
//! - `prompt` judges in the hub's order: the id resolves to exactly one body (none or two:
//!   `pane-not-driven`), then an operator or default hold on that body's session (`session-held`,
//!   in the real client's words, `holler_pane::hub::session_held`, naming `holler release`), then
//!   the pane gate through `holler_pane::gate::first_refusal`, the very selection the hub's
//!   `pane_gate` calls, over the records of the store the fake was built over, then a held
//!   question or permission (`session-busy`; a busy body is admitted, because
//!   the wiring sends `queue: true`). `Check` sends nothing; `Send` is logged in
//!   [`FakeHub::prompts`]. [`FakeHub::fail_send`] refuses the next `Send` only, as a race between
//!   the check and the send does;
//! - every port method passes [`FakeHub::faults`] first (`hub.advertised`, `hub.rebind`,
//!   `hub.prompt`), and each `rebind` and `prompt` is also logged in [`FakeHub::log`] with
//!   whether a pane in the store had the call's id as its session of record at that moment, as
//!   `SeamHarness` records `then_of_record`: how a verb test orders a call on this fake against a
//!   write on the store;
//! - its reads of the store bypass the store's faults and call log: they stand for the hub's
//!   read of its own registry, so a verb test's store log shows the verb's calls alone.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use holler_pane::findings::quoted;
use holler_pane::gate::{first_refusal, PromptRefusal};
use holler_pane::hub::{session_held, Advertised, HubPort, PromptMode, PANE_NOT_DRIVEN};
use holler_pane::ports::Activity;
use holler_pane::tx_switch::SESSION_BUSY;
use holler_pane::PaneError;

use crate::fault::{FaultSwitch, PortOp};
use crate::pane_store::FakePaneStore;

/// A method of the `HubPort`, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HubOp {
    Advertised,
    Rebind,
    Prompt,
}

impl PortOp for HubOp {
    fn as_str(self) -> &'static str {
        match self {
            HubOp::Advertised => "hub.advertised",
            HubOp::Rebind => "hub.rebind",
            HubOp::Prompt => "hub.prompt",
        }
    }
}

/// Which hold refuses a prompt to a body's session: `holler hold`'s, or the one a session joins
/// with (`--join-held`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldKind {
    Operator,
    Default,
}

impl HoldKind {
    /// The hub's `hold_kind`.
    fn as_str(self) -> &'static str {
        match self {
            HoldKind::Operator => "operator",
            HoldKind::Default => "default",
        }
    }
}

/// One `rebind` or `prompt` the fake was asked, in [`FakeHub::log`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubCall {
    pub op: HubOp,
    /// The harness session id the call named.
    pub id: String,
    /// `Some` for a `prompt`.
    pub mode: Option<PromptMode>,
    /// Whether a pane in the store had `id` as its session of record when the call was made.
    pub of_record: bool,
}

/// One prompt `prompt` with `Send` delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentPrompt {
    pub id: String,
    pub text: String,
    /// Whether a pane in the store had `id` as its session of record when it was sent.
    pub of_record: bool,
}

/// An in-memory `HubPort`. See the module docs. It is not `Clone`: share it behind an `Arc`.
pub struct FakeHub {
    panes: Arc<FakePaneStore>,
    faults: FaultSwitch<HubOp>,
    state: Mutex<State>,
}

/// What the fake holds, behind its lock.
#[derive(Default)]
struct State {
    bodies: Vec<Advertised>,
    holds: BTreeMap<String, HoldKind>,
    fail_send: Option<PaneError>,
    log: Vec<HubCall>,
    prompts: Vec<SentPrompt>,
}

impl FakeHub {
    /// A hub with no body, no hold and no fault, whose gate reads the records of `panes`.
    pub fn new(panes: Arc<FakePaneStore>) -> Self {
        Self {
            panes,
            faults: FaultSwitch::new(),
            state: Mutex::new(State::default()),
        }
    }

    /// A live attach body session `name` (`label/session`) on the connection of `token`,
    /// advertising `harness_session_id`, doing `state`. It bypasses the faults and the log.
    pub fn with_body(&self, token: &str, name: &str, harness_session_id: &str, state: Activity) {
        self.lock().bodies.push(Advertised {
            token: token.to_owned(),
            name: name.to_owned(),
            harness_session_id: harness_session_id.to_owned(),
            state,
        });
    }

    /// The body `name` now does `state`.
    pub fn set_state(&self, name: &str, state: Activity) {
        let mut held = self.lock();
        for body in held.bodies.iter_mut().filter(|body| body.name == name) {
            body.state = state;
        }
    }

    /// Hold the session `name` (keyed by the body session's name, as the hub keys a hold).
    pub fn hold(&self, name: &str, kind: HoldKind) {
        self.lock().holds.insert(name.to_owned(), kind);
    }

    /// Lift the hold on `name`, if any.
    pub fn release(&self, name: &str) {
        self.lock().holds.remove(name);
    }

    /// Refuse the next `prompt` with `Send` with `error`, once; a `Check` does not consume it.
    pub fn fail_send(&self, error: PaneError) {
        self.lock().fail_send = Some(error);
    }

    /// The fault switch of every port method and the log of the calls made through the port.
    pub fn faults(&self) -> &FaultSwitch<HubOp> {
        &self.faults
    }

    /// The bodies as the hub advertises them now.
    pub fn bodies(&self) -> Vec<Advertised> {
        self.lock().bodies.clone()
    }

    /// Every `rebind` and `prompt` the fake was asked, oldest first, refused ones included.
    pub fn log(&self) -> Vec<HubCall> {
        self.lock().log.clone()
    }

    /// Every prompt a `Send` delivered, oldest first.
    pub fn prompts(&self) -> Vec<SentPrompt> {
        self.lock().prompts.clone()
    }

    /// Log one `rebind` or `prompt`, with whether `id` is of record now.
    fn note(&self, op: HubOp, id: &str, mode: Option<PromptMode>) {
        let of_record = self.of_record(id);
        self.lock().log.push(HubCall {
            op,
            id: id.to_owned(),
            mode,
            of_record,
        });
    }

    /// Whether a pane of the store has `id` as its session of record (read off the log).
    fn of_record(&self, id: &str) -> bool {
        self.panes
            .records()
            .iter()
            .any(|pane| pane.session_of_record.as_deref() == Some(id))
    }

    /// The hub's gate for a prompt to `id`, in its order (see the module docs).
    fn judge(&self, id: &str) -> Result<(), PaneError> {
        let (body, hold) = {
            let held = self.lock();
            let on: Vec<&Advertised> = held
                .bodies
                .iter()
                .filter(|body| body.harness_session_id == id)
                .collect();
            let body = match on.as_slice() {
                [body] => (*body).clone(),
                more => return Err(not_driven(id, more.len())),
            };
            let hold = held.holds.get(&body.name).copied();
            (body, hold)
        };
        if let Some(kind) = hold {
            return Err(session_held(id, Some(&body.name), kind.as_str(), None));
        }
        let records = self.panes.records();
        if let Some((pane, refusal)) = first_refusal(&records, id) {
            let code = refusal.code();
            let message = format!(
                "pane {} {}, so it refuses prompts ({code})",
                pane.name,
                why(refusal)
            );
            return Err(PaneError::Refused { code, message });
        }
        let waits = match body.state {
            Activity::HoldingQuestion => "holds a question; answer it first",
            Activity::WaitingPermission => "waits on a permission prompt; answer it first",
            Activity::Idle | Activity::Busy => return Ok(()),
        };
        Err(PaneError::Refused {
            code: SESSION_BUSY,
            message: format!("session {} {waits}", quoted(id)),
        })
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HubPort for FakeHub {
    fn advertised(&self) -> Result<Vec<Advertised>, PaneError> {
        self.faults.enter(HubOp::Advertised)?;
        Ok(self.bodies())
    }

    fn rebind(&self, driver: &Advertised, harness_session_id: &str) -> Result<String, PaneError> {
        self.note(HubOp::Rebind, harness_session_id, None);
        self.faults.enter(HubOp::Rebind)?;
        let mut held = self.lock();
        let body = held
            .bodies
            .iter_mut()
            .find(|body| body.token == driver.token && body.name == driver.name)
            .ok_or_else(|| PaneError::Unavailable {
                what: format!("no live body {}", quoted(&driver.name)),
            })?;
        body.harness_session_id = harness_session_id.to_owned();
        Ok(harness_session_id.to_owned())
    }

    fn prompt(
        &self,
        harness_session_id: &str,
        text: &str,
        mode: PromptMode,
    ) -> Result<(), PaneError> {
        self.note(HubOp::Prompt, harness_session_id, Some(mode));
        self.faults.enter(HubOp::Prompt)?;
        if mode == PromptMode::Send {
            if let Some(error) = self.lock().fail_send.take() {
                return Err(error);
            }
        }
        self.judge(harness_session_id)?;
        if mode == PromptMode::Send {
            let of_record = self.of_record(harness_session_id);
            self.lock().prompts.push(SentPrompt {
                id: harness_session_id.to_owned(),
                text: text.to_owned(),
                of_record,
            });
        }
        Ok(())
    }
}

/// `pane-not-driven` for `id`, which `count` live bodies advertise (none, or more than one).
fn not_driven(id: &str, count: usize) -> PaneError {
    let message = if count == 0 {
        format!("no live body advertises session {}", quoted(id))
    } else {
        format!("{count} live bodies advertise session {}", quoted(id))
    };
    PaneError::Refused {
        code: PANE_NOT_DRIVEN,
        message,
    }
}

/// Why the gate refuses, in the words the hub's gate uses.
fn why(refusal: PromptRefusal) -> &'static str {
    match refusal {
        PromptRefusal::Parked => "is parked",
        PromptRefusal::Unhealthy => "is unhealthy",
        PromptRefusal::ShownDrivenMismatch => "shows a session other than the one it drives",
    }
}
