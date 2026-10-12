//! `interrupt`'s pure logic (issue #191), split out of `main.rs` the same
//! way `say_cmd.rs` (issue #190) is — this file returns a plain
//! [`InterruptResult`]; only the bin turns that into an actual exit.
//!
//! **A pane's interrupt by its harness (#831, the operator's decision of 2026-10-11).** For
//! `--pane NAME`, [`interrupt_pane`] reads the pane's record (the pane gates of
//! `say_cmd::resolve_pane_record` first). A harness that aborts in place (OpenCode) is
//! interrupted through the hub, as before. One whose abort style is a relaunch (Claude Code) has
//! no body for the hub to reach and no in-place abort: the CLI relaunches the pane on its session
//! of record through the pane engine (`tx_launch::relaunch_recorded`), which loses the turn in
//! flight and keeps the session, and never types into the pane. Replacement text on such a pane
//! is refused (`interrupt-text-unsupported`).

use holler_pane::pane::AbortStyle;
use holler_pane::tx_launch::{relaunch_recorded, ProcessSession, TxOptions};
use holler_pane::{PaneError, Ports};

use crate::pane::wiring::Wiring;
use crate::prompt_target::{PromptArgs, PromptTarget, Routed, Stop};
use crate::say_cmd::{
    pane_hold_refusal, resolve_pane_record, route_target, routing_stop, session_of_record,
    INTERRUPT_TEXT_UNSUPPORTED,
};
use crate::Interrupt;

/// What `main.rs` should print and exit with.
pub struct InterruptResult {
    /// The line to print — to stdout on success, stderr on every refusal.
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout.
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> InterruptResult {
    InterruptResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> InterruptResult {
    InterruptResult { message, to_stderr: true, exit_code }
}

/// What `interrupt --pane` does for a pane, by its harness (#831).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneInterrupt {
    /// The pane's harness aborts in place: the interrupt goes to the hub, addressed to the
    /// pane's session of record. Nothing live was touched.
    Hub { session: String },
    /// The pane's process was relaunched on its session of record: the turn in flight is lost,
    /// the session kept.
    Relaunched { session: String },
}

/// The interrupt of the pane `pane` (`--pane NAME [--profile P]`, with `text` its replacement
/// text): the pane gates of `say_cmd::resolve_pane_record` first (`pane-parked`,
/// `pane-unhealthy`, a SHOWN and DRIVEN mismatch), then by the abort style of its harness's
/// kind. In place (OpenCode): [`PaneInterrupt::Hub`], for the caller to send through the hub.
/// A relaunch (Claude Code): `interrupt-text-unsupported` with text; otherwise the pane is
/// relaunched on its session of record, a busy one included, through the pane engine, and the
/// answer is [`PaneInterrupt::Relaunched`] with the session its process holds.
pub fn interrupt_pane(
    ports: Ports<'_>,
    pane: &str,
    profile: Option<&str>,
    text: Option<&str>,
) -> Result<PaneInterrupt, PaneError> {
    let record = resolve_pane_record(ports, pane, profile)?;
    let session = session_of_record(&record)?;
    match record.harness.kind.abort_style() {
        AbortStyle::InPlace => Ok(PaneInterrupt::Hub { session }),
        AbortStyle::Relaunch if text.is_some() => Err(PaneError::Refused {
            code: INTERRUPT_TEXT_UNSUPPORTED,
            message: format!(
                "{} is interrupted by a relaunch on its session of record, which runs no \
                 replacement text; interrupt it without text, then say the text",
                record.name
            ),
        }),
        AbortStyle::Relaunch => {
            let keep = ProcessSession::Keep;
            let launched = relaunch_recorded(ports, &record, &keep, &TxOptions::default())
                .map_err(|failure| failure.error)?;
            let held = launched.pane.and_then(|pane| pane.session_of_record);
            Ok(PaneInterrupt::Relaunched {
                session: held.unwrap_or(session),
            })
        }
    }
}

/// Where `interrupt` goes next: to the hub, or nowhere, its pane relaunched.
enum Next {
    Hub(Routed),
    Relaunched { pane: String, session: String },
}

/// Route `interrupt`: a SESSION, and a malformed tail, through `say`'s routing unchanged; a
/// `--pane` target through [`interrupt_pane`] over the run's wiring.
fn route(interrupt: &Interrupt) -> Result<Next, Stop> {
    let Ok(PromptArgs {
        target: PromptTarget::Pane(pane),
        arg,
    }) = interrupt.resolve()
    else {
        return route_target(interrupt.resolve(), &interrupt.profile).map(Next::Hub);
    };
    let profile = interrupt.profile.profile.as_deref();
    let answer = Wiring::connect()
        .and_then(|wiring| interrupt_pane(wiring.ports(), &pane, profile, arg.as_deref()))
        .map_err(|error| routing_stop(&error))?;
    Ok(match answer {
        PaneInterrupt::Hub { session } => Next::Hub(Routed { session, arg }),
        PaneInterrupt::Relaunched { session } => Next::Relaunched { pane, session },
    })
}

/// What `interrupt --pane` prints for a pane it relaunched: one line, or (JSON) the object
/// `{"interrupted": true, "pane", "session", "relaunched": true}`.
fn relaunched(pane: &str, session: &str, json: bool) -> InterruptResult {
    if json {
        let doc = serde_json::json!({
            "interrupted": true,
            "pane": pane,
            "session": session,
            "relaunched": true,
        });
        return ok(doc.to_string());
    }
    let session = holler_pane::findings::quoted(session);
    ok(format!(
        "interrupted {pane}: relaunched on its session of record {session}"
    ))
}

/// `holler interrupt SESSION [TEXT]` (issue #191): cancel `SESSION`'s
/// in-flight turn and confirm the cancel was applied — without `TEXT`,
/// print `interrupted <session>`; with `TEXT`, run it as a redirect prompt
/// right after and print its reply exactly like `say` does.
///
/// Exit codes: `0` interrupted (and, with `TEXT`, a reply arrived); `1`
/// every runtime refusal (no live hub, not connected, unknown session, ack
/// timeout, connection lost, the redirect prompt itself failing) — each with
/// the spec's own wording; `2` an ambiguous session or a malformed positional
/// tail.
///
/// `--pane NAME [--profile P]` (story #646) interrupts the pane's session of record,
/// routed by `say`'s engine (`say_cmd::route_target`), with its refusals and exit
/// codes; a redirect the hub's pane-state gate refuses prints as `say`'s does. A Claude Code
/// pane (#831) is relaunched on its session of record instead ([`interrupt_pane`]), and the run
/// prints `interrupted <pane>: relaunched on its session of record "<session>"`, exit 0.
pub fn run(interrupt: &Interrupt, json: bool) -> InterruptResult {
    let Routed { session, arg } = match route(interrupt) {
        Ok(Next::Hub(routed)) => routed,
        Ok(Next::Relaunched { pane, session }) => return relaunched(&pane, &session, json),
        Err(stop) => return err(stop.message, stop.exit_code),
    };
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    let call = holler_hub::control::ControlCall::interrupt(&session, arg.as_deref());
    match crate::transport::call(interrupt.server.as_deref(), &call) {
        Ok(doc) => {
            if json {
                return ok(doc.to_string());
            }
            match arg {
                // The redirect form streams its reply exactly like `say`.
                Some(_) => {
                    let text = doc.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    ok(text.to_string())
                }
                None => ok(format!("interrupted {session}")),
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::RemotePolicyRefused(msg)) => err(msg, 3),
        Err(holler_hub::control::ControlError::Refused(e)) => {
            // The pane arm comes first (story #646): the redirect passes
            // `send_prompt`, whose pane-state gate is no hold to release.
            if let Some(refused) = pane_hold_refusal(&session, &e, json) {
                return InterruptResult {
                    message: refused.message,
                    to_stderr: refused.to_stderr,
                    exit_code: refused.exit_code,
                };
            }
            if crate::hold_cmd::is_held(&e) {
                let (message, to_stderr) = crate::hold_cmd::held_refusal(&session, &e, json);
                return InterruptResult { message, to_stderr, exit_code: crate::hold_cmd::HELD_EXIT_CODE };
            }
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            err(e.message.clone(), if is_ambiguous { 2 } else { 1 })
        }
        Err(e) => err(e.to_string(), 1),
    }
}
