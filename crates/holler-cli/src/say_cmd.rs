//! `say`'s pure logic (issue #190), split out of `main.rs` to keep that file
//! under the workspace's 900-line build guard (`scripts/lint.sh` check 4) —
//! the same split `query_cmd.rs` (issue #185) already made for `hub query`'s
//! own resolve/param logic. This module returns a plain [`SayResult`]; only
//! the bin (`main.rs`, the one file allowed to exit the process) turns that
//! into an actual exit.
//!
//! It also owns what story #646 (part 3) adds to the three prompt verbs, which
//! `interrupt_cmd.rs` and `answer_cmd.rs` call from here:
//!
//! - **Routing** ([`route_target`], [`resolve_pane_target`]): `--pane NAME` addresses
//!   the pane's session of record, refused (exit 3) when the record says the pane is
//!   parked, unhealthy or shows a session other than the one it drives. `--profile P`
//!   scopes `--pane` and needs it. `prompt_target::route` stays as #670 froze it.
//! - **The routed `say --queue`** ([`QUEUE_ACCEPT_WAIT`], [`queue_outcome`]): returns
//!   once the hub has accepted the prompt instead of waiting for the turn's reply.
//! - **The pane arm of a held refusal** ([`pane_hold_refusal`]): the hub's pane-state
//!   gate at `send_prompt` refuses with `session_held` and `hold_kind: "pane"`, which
//!   is a refusal (exit 3) with the pane's remedy, not a hold to release.

use std::io::ErrorKind;
use std::time::Duration;

use holler_hub::control::ControlError;
use holler_pane::error::{class_of, ErrorClass, RefusalCode};
use holler_pane::findings::{doctor_command, quoted};
use holler_pane::pane::{Health, Hold};
use holler_pane::{Pane, PaneError, PaneName, Ports, ProfileName};
use serde::Serialize;

use crate::cli::Usage;
use crate::pane::args::ProfileOpt;
use crate::pane::wiring::Wiring;
use crate::prompt_target::{PromptArgs, PromptTarget, Routed, Stop};
use crate::Say;

/// What `say_command` (in `main.rs`) should print and exit with.
pub struct SayResult {
    /// The line to print — to stdout on success, stderr on every refusal.
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout (a reply).
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> SayResult {
    SayResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> SayResult {
    SayResult { message, to_stderr: true, exit_code }
}

/// Parse a `<number><s|m|h>` duration (the `say --timeout` grammar). `None`
/// on anything else — the call site fails closed (exit 3) on `None`, same
/// discipline as `main.rs`'s own `parse_ttl`.
pub(crate) fn parse_duration(s: &str) -> Option<Duration> {
    let digits: &str = s.split(|c: char| !c.is_ascii_digit()).next()?;
    if digits.is_empty() {
        return None;
    }
    let rest = &s[digits.len()..];
    if rest.len() != 1 {
        return None;
    }
    let num: u64 = digits.parse().ok()?;
    match rest {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 3600)),
        _ => None,
    }
}

/// `say`'s prompt text (issue #190): `TEXT` normally, or the concatenated
/// text parts of `--parts-file`'s A2A `Message` when given instead. `Say::resolve`
/// (the `--pane` accessor, `prompt_target.rs`) guarantees one of the two is
/// present by the time this runs; `text` is its `TEXT`.
fn resolve_say_text(say: &Say, text: Option<&str>) -> Result<String, String> {
    if let Some(path) = &say.parts_file {
        let content = std::fs::read_to_string(path).map_err(|e| format!("cannot read --parts-file {path}: {e}"))?;
        let message: holler_proto::Message = serde_json::from_str(&content)
            .map_err(|e| format!("--parts-file {path} is not a valid A2A Message: {e}"))?;
        Ok(message.parts.iter().filter_map(|p| p.text()).collect::<Vec<_>>().join(""))
    } else {
        Ok(text.unwrap_or_default().to_string())
    }
}

/// The `-32009 session_busy` refusal's human hint (issue #190's exact
/// wording): `"<session> is <state> (turn <age>, last update <age> ago); use
/// 'interrupt <session> \"…\"' to replace it, or 'say --queue' to append"`.
///
/// A held permission/elicitation (issue #151, `data.state ==
/// "input-required"`) is a distinct hint: neither `interrupt` nor `--queue`
/// resolves it — only `answer` does, so the wording points there instead,
/// and drops the `--queue` suggestion entirely (`--queue` is refused on
/// `input-required`, never silently accepted — see `talk::say`'s own gate).
fn busy_hint(session: &str, e: &holler_proto::WireError) -> String {
    let data = e.data.as_ref();
    let state = data.and_then(|d| d.state.clone()).unwrap_or_else(|| "working".to_string());
    let turn_age = fmt_age(data.and_then(|d| d.turn_age_ms).unwrap_or(0));
    let last_update_age = fmt_age(data.and_then(|d| d.last_update_age_ms).unwrap_or(0));
    if state == "input-required" {
        format!(
            "session_busy: {session} is input-required (turn {turn_age}, last update {last_update_age} ago); \
             use 'answer {session} <choice>' to resolve it"
        )
    } else {
        format!(
            "session_busy: {session} is {state} (turn {turn_age}, last update {last_update_age} ago); \
             use 'interrupt {session} \"…\"' to replace it, or 'say --queue' to append"
        )
    }
}

/// Render a millisecond age as `<m>m<s>s` (or bare `<s>s` under a minute) —
/// the busy hint's own age formatting.
fn fmt_age(ms: u64) -> String {
    let total_secs = ms / 1000;
    let (m, s) = (total_secs / 60, total_secs % 60);
    if m > 0 {
        format!("{m}m{s}s")
    } else {
        format!("{s}s")
    }
}

/// `holler say SESSION TEXT [--timeout 600s] [--queue] [--json]` (issue
/// #190): resolve `SESSION` against the live hub, run one prompt turn, and
/// report the reply (or `--json`'s full result document) — or a refusal.
/// Exit codes: `0` a reply arrived, `1` every runtime refusal (no live hub,
/// not connected, unknown/ambiguous session, busy, connection lost, timeout,
/// cancelled — each with the spec's own wording; `2` an ambiguous session or
/// a malformed positional tail, `3` a malformed `--timeout`/`--parts-file`.
///
/// `--pane NAME [--profile P]` (story #646) prompts the pane's session of record
/// ([`route_target`]); every pane-routed refusal is exit 3 with its stable code in
/// the message, and a prompt the hub's pane-state gate refuses is too
/// ([`pane_hold_refusal`]). The routed `--queue` form returns once the prompt is
/// accepted ([`QUEUE_ACCEPT_WAIT`]); the bare `say --queue SESSION` still waits for
/// the reply (#190).
pub fn run(say: &Say, json: bool) -> SayResult {
    let Routed { session, arg } = match route_target(say.resolve(), &say.profile) {
        Ok(routed) => routed,
        Err(stop) => return err(stop.message, stop.exit_code),
    };
    let timeout = match parse_duration(&say.timeout) {
        Some(d) => d,
        None => return err(format!("invalid --timeout {:?} (use e.g. 30s, 5m, 1h)", say.timeout), 3),
    };
    let text = match resolve_say_text(say, arg.as_deref()) {
        Ok(t) => t,
        Err(msg) => return err(msg, 3),
    };
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    let mut call = holler_hub::control::ControlCall::say_with(
        &session,
        &text,
        say.queue,
        say.grant.as_deref(),
        timeout,
    );
    let early = returns_on_acceptance(say);
    if early {
        call.timeout = QUEUE_ACCEPT_WAIT;
    }
    let reply = crate::transport::call(say.server.as_deref(), &call);
    match wait_ended(reply, early) {
        Ok(wait) => queue_outcome(&session, wait, json),
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(e) => control_refusal(&session, e, json),
    }
}

/// A control call that did not answer with a reply, as `say` prints it.
fn control_refusal(session: &str, e: ControlError, json: bool) -> SayResult {
    match e {
        ControlError::RemotePolicyRefused(msg) => err(msg, 3),
        ControlError::Refused(e) => {
            // The pane arm comes first (story #646): a pane-state refusal is
            // `session_held` on the wire but no hold to release.
            if let Some(result) = pane_hold_refusal(session, &e, json) {
                return result;
            }
            // `ambiguous` (issue #190 spec: "lists candidates") is the one
            // `say` refusal that exits 2, not 1 — every other refusal
            // (unknown session, session_busy, not_connected, connection_lost,
            // cancelled) is a runtime failure (exit 1).
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            if crate::hold_cmd::is_held(&e) {
                let (message, to_stderr) = crate::hold_cmd::held_refusal(session, &e, json);
                return SayResult { message, to_stderr, exit_code: crate::hold_cmd::HELD_EXIT_CODE };
            }
            if crate::hold_cmd::is_invalid_grant(&e) {
                let (message, to_stderr) =
                    crate::hold_cmd::invalid_grant_refusal(session, &e, json);
                return SayResult { message, to_stderr, exit_code: crate::hold_cmd::INVALID_GRANT_EXIT_CODE };
            }
            let message = if e.code == holler_proto::Code::SessionBusy.jsonrpc() {
                busy_hint(session, &e)
            } else {
                e.message.clone()
            };
            err(message, if is_ambiguous { 2 } else { 1 })
        }
        e => err(e.to_string(), 1),
    }
}

// --- routing by pane (story #646, part 3) ----------------------------------------

/// A pane that is parked (`holler pane park`): its prompts are refused until it is
/// unparked. The three open codes of the routed verbs are declared here, the file
/// that owns the routing (ADR-0021 section 9); each is a refusal, exit 3.
pub const PANE_PARKED: RefusalCode = RefusalCode::from_static("pane-parked");
/// A pane whose record says its harness server is unhealthy.
pub const PANE_UNHEALTHY: RefusalCode = RefusalCode::from_static("pane-unhealthy");
/// A pane whose record says it shows one session while the hub drives another.
pub const PANE_SHOWN_DRIVEN_MISMATCH: RefusalCode =
    RefusalCode::from_static("pane-shown-driven-mismatch");

/// What `--profile` without `--pane` is told: the prompt verbs still need a pane name
/// (ADR-0021 section 3, the scoping class).
const PROFILE_NEEDS_PANE: &str =
    "--profile NAME scopes --pane NAME, so it needs one: `--pane NAME --profile NAME`";

/// Clear a resolved prompt verb to run, or say why it stops; `say`, `interrupt` and
/// `answer` all call it. A malformed tail is `usage` (exit 2), as before. A SESSION is
/// sent to as it is, and a SESSION with `--profile` is `usage`, naming `--pane`. A
/// `--pane` target is resolved by [`resolve_pane_target`] over the run's wiring, before
/// any hub is contacted: its error prints with the exit of its code's class, a refusal
/// (exit 3) naming its stable code.
pub fn route_target(
    resolved: Result<PromptArgs, Usage>,
    profile: &ProfileOpt,
) -> Result<Routed, Stop> {
    let args = resolved.map_err(|usage| Stop {
        message: usage.to_string(),
        exit_code: ErrorClass::Usage.exit_code(),
    })?;
    let session = match (args.target, profile.profile.as_deref()) {
        (PromptTarget::Session(session), None) => session,
        (PromptTarget::Session(_), Some(_)) => {
            return Err(Stop {
                message: PROFILE_NEEDS_PANE.to_owned(),
                exit_code: ErrorClass::Usage.exit_code(),
            })
        }
        (PromptTarget::Pane(pane), profile) => Wiring::connect()
            .and_then(|wiring| resolve_pane_target(wiring.ports(), &pane, profile))
            .map_err(|error| routing_stop(&error))?,
    };
    Ok(Routed {
        session,
        arg: args.arg,
    })
}

/// A routing error as the prompt verbs print it (they have no envelope): its message,
/// naming its stable code when it is a refusal, and the exit code of its class.
fn routing_stop(error: &PaneError) -> Stop {
    let (code, text) = (error.code(), error.to_string());
    let class = class_of(code);
    let message = if class == ErrorClass::Refusal {
        coded(code, text)
    } else {
        text
    };
    Stop {
        message,
        exit_code: class.exit_code(),
    }
}

/// `text`, with `code: ` in front unless it already names `code`, so a refusal's
/// stable code appears in its message exactly once.
fn coded(code: &str, text: String) -> String {
    if text.contains(code) {
        text
    } else {
        format!("{code}: {text}")
    }
}

/// Resolve `--pane NAME [--profile P]` to the pane's session of record, checking in
/// this order: the pane name (`usage`); with `--profile`, the profile scope
/// (`ProfileScope::resolve`, so `profile-not-found` and `pane-not-in-profile` come
/// from the one helper, as for `pane park`); the record (`pane-not-found`); its session
/// of record (`session-not-found`, naming the pane); then what the record says about
/// the pane: parked ([`PANE_PARKED`]), unhealthy ([`PANE_UNHEALTHY`], with the reason)
/// or SHOWN and DRIVEN both observed and differing ([`PANE_SHOWN_DRIVEN_MISMATCH`]).
/// The record is the source; nothing live is probed.
///
/// The pane-state predicate has a twin at the hub's `send_prompt` gate
/// (`holler-hub/src/circuit/dispatch.rs`, `pane_state_refusal`), which also refuses a
/// bare `say` to the session; it exists twice only because `holler-pane` is frozen and
/// the hub cannot depend on this crate. Keep the two equal.
pub fn resolve_pane_target(
    ports: Ports<'_>,
    pane: &str,
    profile: Option<&str>,
) -> Result<String, PaneError> {
    let name = PaneName::parse(pane)?;
    let profile = profile.map(ProfileName::parse).transpose()?;
    let record = routed_record(ports, &name, profile.as_ref())?;
    let Some(session) = record.session_of_record.clone() else {
        return Err(PaneError::SessionNotFound {
            what: format!("{name} has no session of record"),
        });
    };
    match pane_state_refusal(&record) {
        Some(refusal) => Err(refusal),
        None => Ok(session),
    }
}

/// The record of `name`: with a profile, the one the profile scope answers (it refuses
/// a pane outside P); without, the pane store's.
fn routed_record(
    ports: Ports<'_>,
    name: &PaneName,
    profile: Option<&ProfileName>,
) -> Result<Pane, PaneError> {
    let record = match profile {
        Some(profile) => ports
            .scope
            .resolve(profile, Some(name))?
            .panes
            .into_iter()
            .find(|pane| pane.name == *name),
        None => ports.pane_store.get(name)?,
    };
    record.ok_or_else(|| PaneError::PaneNotFound {
        what: name.to_string(),
    })
}

/// What the record says against prompting the pane, in the order parked, unhealthy,
/// SHOWN differing from DRIVEN (one side absent is not a mismatch); each message names
/// the pane, the code and the remedy on one line (stored text is `quoted`).
fn pane_state_refusal(record: &Pane) -> Option<PaneError> {
    let name = &record.name;
    let doctor = doctor_command(Some(name), false);
    let (code, message) = if let Hold::Parked { reason, .. } = &record.hold {
        let reason = quoted(reason);
        let message = format!("{name} is parked ({PANE_PARKED}, reason {reason}); run holler pane unpark {name} first");
        (PANE_PARKED, message)
    } else if let Health::Unhealthy(reason) = &record.harness.health {
        let reason = quoted(reason);
        (
            PANE_UNHEALTHY,
            format!("{name} is unhealthy ({PANE_UNHEALTHY}): {reason}; run {doctor}"),
        )
    } else {
        let seen = &record.last_observed;
        let (Some(shown), Some(driven)) = (seen.shown.as_deref(), seen.driven.as_deref()) else {
            return None;
        };
        if shown == driven {
            return None;
        }
        let (shown, driven) = (quoted(shown), quoted(driven));
        let message = format!(
            "{name} shows session {shown} but the hub drives {driven} ({PANE_SHOWN_DRIVEN_MISMATCH}); run {doctor}"
        );
        (PANE_SHOWN_DRIVEN_MISMATCH, message)
    };
    Some(PaneError::Refused { code, message })
}

// --- the routed `say --queue` (story #646, part 3) -------------------------------

/// How long the routed `say --pane NAME --queue` waits for the hub before it reports
/// the prompt queued. It replaces only the control call's own wait: the prompt's
/// `timeout_ms` stays `--timeout`, so the hub keeps waiting for the turn after the CLI
/// has returned. A refusal (busy is not one with `--queue`; a hold or the pane gate is)
/// answers well inside it.
pub const QUEUE_ACCEPT_WAIT: Duration = Duration::from_secs(2);

/// What the wait of a `say` ended with.
#[derive(Debug)]
pub enum QueueWait {
    /// The turn finished inside the wait; the control call answered this document.
    Reply(serde_json::Value),
    /// The CLI's own acceptance deadline ([`QUEUE_ACCEPT_WAIT`]) expired after the
    /// prompt was accepted.
    Deadline,
}

/// Whether this `say` returns once its prompt is accepted: the routed (`--pane`)
/// `--queue` form over the local control socket. A remote hub (`--server`) cannot tell
/// a read deadline from a dropped connection, so it keeps the reply wait; the bare
/// `say --queue SESSION` keeps it too (#190's contract).
fn returns_on_acceptance(say: &Say) -> bool {
    say.queue && say.pane.is_some() && say.server.is_none()
}

/// The control call's result as a [`QueueWait`]: a reply, or, when the call waited
/// only [`QUEUE_ACCEPT_WAIT`] (`returns_on_acceptance`), its read deadline expiring
/// with no answer, which means the hub accepted the prompt. Any other error is passed
/// on to be printed as a refusal.
fn wait_ended(
    reply: Result<serde_json::Value, ControlError>,
    returns_on_acceptance: bool,
) -> Result<QueueWait, ControlError> {
    match reply {
        Ok(doc) => Ok(QueueWait::Reply(doc)),
        Err(ControlError::Io(e))
            if returns_on_acceptance
                && matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
        {
            Ok(QueueWait::Deadline)
        }
        Err(other) => Err(other),
    }
}

/// What `say` prints for the end of its wait, exit 0 either way: a reply exactly as
/// the bare form prints it (text: the `text` member; JSON: the whole document), or,
/// once the acceptance deadline passed, `queued <session>` (JSON:
/// `{"queued":true,"session":"<session>"}`) on stdout: the prompt was accepted and the
/// turn runs on, so returning is not a failure.
pub fn queue_outcome(session: &str, wait: QueueWait, json: bool) -> SayResult {
    match wait {
        QueueWait::Reply(doc) if json => ok(doc.to_string()),
        QueueWait::Reply(doc) => {
            let text = doc.get("text").and_then(|v| v.as_str()).unwrap_or("");
            ok(text.to_string())
        }
        QueueWait::Deadline if json => {
            ok(serde_json::json!({ "queued": true, "session": session }).to_string())
        }
        QueueWait::Deadline => ok(format!("queued {session}")),
    }
}

// --- the pane arm of a held refusal (story #646, part 3) -------------------------

/// The `-32011 session_held` of the hub's pane-state gate, as `--json` prints it: the
/// held form's object (`hold_cmd::held_refusal`) with `hold_kind: "pane"` and no
/// `since`, in this key order.
#[derive(Serialize)]
struct PaneHeld<'a> {
    error: &'static str,
    session: &'a str,
    reason: &'a str,
    hold_kind: &'static str,
}

/// Render a `-32011 session_held` refusal of `session` whose `data.hold_kind` is
/// `"pane"`: the hub's pane-state gate refused the prompt (`data.reason` is the pane
/// code). One line naming the code with its remedy (`holler pane unpark` for
/// `pane-parked`, `holler pane doctor` otherwise), exit 3 like every pane-routed
/// refusal; never the generic held tail nor `HELD_EXIT_CODE`, because `holler release`
/// does not lift it. JSON: the held form's object with `hold_kind: "pane"`, on stdout.
/// `None` for every other error, which the caller's generic arms then render.
pub fn pane_hold_refusal(
    session: &str,
    e: &holler_proto::WireError,
    json: bool,
) -> Option<SayResult> {
    let data = e.data.as_deref()?;
    if !crate::hold_cmd::is_held(e) || data.hold_kind.as_deref() != Some("pane") {
        return None;
    }
    let code = data.reason.as_deref().unwrap_or_default();
    let exit_code = ErrorClass::Refusal.exit_code();
    if json {
        let doc = PaneHeld {
            error: "session_held",
            session,
            reason: code,
            hold_kind: "pane",
        };
        return Some(match serde_json::to_string(&doc) {
            Ok(line) => SayResult {
                message: line,
                to_stderr: false,
                exit_code,
            },
            Err(cause) => err(format!("cannot encode the refusal: {cause}"), exit_code),
        });
    }
    let remedy = if code == PANE_PARKED.as_str() {
        "unpark the pane with holler pane unpark"
    } else {
        "check the pane with holler pane doctor"
    };
    let message = coded(code, e.message.clone());
    Some(err(format!("{message}; {remedy}"), exit_code))
}
