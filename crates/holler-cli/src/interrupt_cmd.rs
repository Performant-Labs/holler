//! `interrupt`'s pure logic (issue #191), split out of `main.rs` the same
//! way `say_cmd.rs` (issue #190) is — this file returns a plain
//! [`InterruptResult`]; only the bin turns that into an actual exit.

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

/// `holler interrupt SESSION [TEXT]` (issue #191): cancel `SESSION`'s
/// in-flight turn and confirm the cancel was applied — without `TEXT`,
/// print `interrupted <session>`; with `TEXT`, run it as a redirect prompt
/// right after and print its reply exactly like `say` does.
///
/// Exit codes: `0` interrupted (and, with `TEXT`, a reply arrived); `1`
/// every runtime refusal (no live hub, not connected, unknown session, ack
/// timeout, connection lost, the redirect prompt itself failing) — each with
/// the spec's own wording; `2` an ambiguous session.
pub fn run(interrupt: &Interrupt, json: bool) -> InterruptResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::interrupt(&interrupt.session, interrupt.text.as_deref()) {
        Ok(doc) => {
            if json {
                return ok(doc.to_string());
            }
            match interrupt.text {
                // The redirect form streams its reply exactly like `say`.
                Some(_) => {
                    let text = doc.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    ok(text.to_string())
                }
                None => ok(format!("interrupted {}", interrupt.session)),
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::Refused(e)) => {
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            err(e.message.clone(), if is_ambiguous { 2 } else { 1 })
        }
        Err(e) => err(e.to_string(), 1),
    }
}
