//! The hub port's client (#839, D3): [`HubClient`] implements `HubPort` over the hub's control
//! socket, one request per call, through `crate::transport::call(None, ..)`, the CLI's one dispatch
//! point. It holds nothing: building one does no I/O, and every call goes to the hub that
//! `HOLLER_STATE_DIR` names when it is made, as the registries' clients do (`super::store`).
//!
//! - **`advertised`** reads `control/roster` and keeps the rows that are attach-mode, `connected`
//!   and advertise a `harness_session_id`; a row's `state` is `idle`, `input-required` (a held
//!   question or permission) or busy (`working`, `stalled`, and anything else: fail closed).
//! - **`rebind`** is `control/rebind` (the body's bare session name, by its token). It comes after
//!   the act, so a refusal is `unavailable` carrying the body's reason, never a refusal code.
//! - **`prompt`** is `control/say`: with `Check`, `check_only` (the hub's gate answers and nothing
//!   is sent); with `Send`, the queued prompt built as `say --pane --queue` builds it
//!   ([`send_call`]), which returns once the hub accepted it. A refusal is classified once
//!   (`pane::hub_refusal`) and mapped to the code `pane reset --first` ends with.
//!
//! **A failed exchange** goes through the registries' one mapping (`store`'s `timed_out` and
//! `hub_unavailable`): `rebind` and `prompt` with `Send` are writes (a reply lost in flight may
//! have landed, so it is a `timeout`), `advertised` and `prompt` with `Check` are reads. The `op`
//! of a `timeout` is `hub.advertised`, `hub.rebind` or `hub.prompt`.

use std::time::Duration;

use holler_hub::control::{ControlCall, ControlError};
use holler_pane::error::RefusalCode;
use holler_pane::findings::quoted;
use holler_pane::hub::{session_held, Advertised, HubPort, PromptMode, PANE_NOT_DRIVEN};
use holler_pane::ports::Activity;
use holler_pane::tx_switch::SESSION_BUSY;
use holler_pane::PaneError;
use holler_proto::rebind::RebindResult;
use holler_proto::WireError;
use serde::Deserialize;

use super::store::{hub_unavailable, timed_out, undecodable, Effect};
use crate::pane::hub_refusal::{classify, HubRefusal};
use crate::say_cmd::{wait_ended, QUEUE_ACCEPT_WAIT};

/// The turn budget of a first message (`pane reset --first`): `say --timeout`'s default. It is
/// the hub's `timeout_ms`, so the hub keeps waiting for the turn after the CLI has returned.
pub const SEND_TURN_BUDGET: Duration = Duration::from_secs(600);

const ADVERTISED: &str = "hub.advertised";
const REBIND: &str = "hub.rebind";
const PROMPT: &str = "hub.prompt";

/// The hub port's client: `HubPort` over the hub's control socket. See the module docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct HubClient;

impl HubClient {
    /// A client of the hub that `HOLLER_STATE_DIR` names when each call is made. No I/O.
    pub fn new() -> Self {
        Self
    }
}

/// The `control/say` of a first message: built as `say_cmd::run` builds the accept-and-return
/// call, `ControlCall::say_with` with the turn's budget as its timeout (so the hub's `timeout_ms`
/// is the turn's) and only the CLI's own read wait set to the accept wait, queued, with no grant.
/// Built with the accept wait as its timeout, the hub would time the turn out at 2 s.
pub fn send_call(harness_session_id: &str, text: &str) -> ControlCall {
    let mut call = ControlCall::say_with(harness_session_id, text, true, None, SEND_TURN_BUDGET);
    call.timeout = QUEUE_ACCEPT_WAIT;
    call
}

/// The rows of a `control/roster` reply this client reads.
#[derive(Deserialize)]
struct Roster {
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    name: String,
    mode: String,
    state: String,
    conn_state: String,
    token_id: String,
    #[serde(default)]
    harness_session_id: Option<String>,
}

impl HubPort for HubClient {
    fn advertised(&self) -> Result<Vec<Advertised>, PaneError> {
        let call = ControlCall::roster(false, None);
        let reply = crate::transport::call(None, &call)
            .map_err(|error| failed(&call, Effect::Read, ADVERTISED, &error))?;
        let roster: Roster =
            serde_json::from_value(reply).map_err(|error| undecodable(call.method, &error))?;
        Ok(roster.rows.into_iter().filter_map(advertised).collect())
    }

    fn rebind(&self, driver: &Advertised, harness_session_id: &str) -> Result<String, PaneError> {
        let bare = driver
            .name
            .rsplit_once('/')
            .map_or(driver.name.as_str(), |(_, session)| session);
        let call = ControlCall::rebind(&driver.token, bare, harness_session_id);
        match crate::transport::call(None, &call) {
            Ok(reply) => {
                let acknowledged: RebindResult = serde_json::from_value(reply)
                    .map_err(|error| undecodable(call.method, &error))?;
                Ok(acknowledged.harness_session_id)
            }
            Err(ControlError::Refused(refusal)) => Err(PaneError::Unavailable {
                what: format!(
                    "the body {} did not drive session {}: {}",
                    quoted(&driver.name),
                    quoted(harness_session_id),
                    reason_of(&refusal)
                ),
            }),
            Err(error) => Err(failed(&call, Effect::Write, REBIND, &error)),
        }
    }

    fn prompt(
        &self,
        harness_session_id: &str,
        text: &str,
        mode: PromptMode,
    ) -> Result<(), PaneError> {
        let (call, effect) = match mode {
            PromptMode::Check => (
                ControlCall::say_check(harness_session_id, text),
                Effect::Read,
            ),
            PromptMode::Send => (send_call(harness_session_id, text), Effect::Write),
        };
        let reply = crate::transport::call(None, &call);
        let answered = match mode {
            PromptMode::Check => reply.map(drop),
            PromptMode::Send => wait_ended(reply, true).map(drop),
        };
        match answered {
            Ok(()) => Ok(()),
            Err(ControlError::Refused(refusal)) => Err(self.refused(harness_session_id, &refusal)),
            Err(error) => Err(failed(&call, effect, PROMPT, &error)),
        }
    }
}

impl HubClient {
    /// The hub's refusal of a prompt to `id`, as `pane reset --first` ends with it (D5, D7).
    fn refused(&self, id: &str, refusal: &WireError) -> PaneError {
        match classify(refusal) {
            HubRefusal::NotDriven(text) => PaneError::Refused {
                code: PANE_NOT_DRIVEN,
                message: format!("no live body drives session {}: {text}", quoted(id)),
            },
            HubRefusal::PaneGate { code, message } => match RefusalCode::parse(code) {
                Ok(code) => PaneError::Refused { code, message },
                Err(_) => hub_unavailable("control/say", &ControlError::Refused(refusal.clone())),
            },
            HubRefusal::Held { kind, reason } => self.held(id, &kind, reason.as_deref()),
            HubRefusal::Busy(message) => PaneError::Refused {
                code: SESSION_BUSY,
                message,
            },
            HubRefusal::Other => {
                hub_unavailable("control/say", &ControlError::Refused(refusal.clone()))
            }
        }
    }

    /// The `session-held` refusal (D4) in `holler_pane::hub::session_held`'s words, naming the
    /// body session that advertises `id`: one more roster read, on this path only, since the
    /// refusal's data carries no hold key. When the read fails or no live body advertises `id`,
    /// the message names the id instead.
    fn held(&self, id: &str, kind: &str, reason: Option<&str>) -> PaneError {
        let name = self
            .advertised()
            .ok()
            .and_then(|bodies| {
                bodies
                    .into_iter()
                    .find(|body| body.harness_session_id == id)
            })
            .map(|body| body.name);
        session_held(id, name.as_deref(), kind, reason)
    }
}

/// A roster row as a live attach body, or `None` for a row that is not one (spawn mode, not
/// `connected`, no harness session id).
fn advertised(row: Row) -> Option<Advertised> {
    let id = row
        .harness_session_id
        .filter(|_| row.mode == "attach" && row.conn_state == "connected")?;
    let state = match row.state.as_str() {
        "idle" => Activity::Idle,
        "input-required" => Activity::HoldingQuestion,
        _ => Activity::Busy,
    };
    Some(Advertised {
        token: row.token_id,
        name: row.name,
        harness_session_id: id,
        state,
    })
}

/// A body's refusal of a rebind, as its message carries it: its reason, then the hub's text.
fn reason_of(refusal: &WireError) -> String {
    match refusal
        .data
        .as_deref()
        .and_then(|data| data.reason.as_deref())
    {
        Some(reason) => format!("{reason} ({})", refusal.message),
        None => refusal.message.clone(),
    }
}

/// A failed exchange of `call`, through the registries' one mapping (`store`'s table).
fn failed(call: &ControlCall, effect: Effect, op: &str, error: &ControlError) -> PaneError {
    if timed_out(error, effect) {
        PaneError::Timeout { op: op.to_owned() }
    } else {
        hub_unavailable(call.method, error)
    }
}
