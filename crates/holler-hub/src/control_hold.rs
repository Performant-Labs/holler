//! `control/hold` and `control/release` (issue #442): the control-socket half
//! of the session hold. The registry and its semantics live in
//! [`crate::holds`]; enforcement is in `circuit::dispatch::send_prompt`. This
//! module only resolves the name an operator typed to the registry's key and
//! maps the outcome to a control reply.
//!
//! # Which sessions can be named
//!
//! A name resolves against every roster row the hub still remembers (live,
//! reconnecting or `gone`, so a session whose body dropped can still be held)
//! **and** every session already held (so a hold on a session that has
//! aged out of the roster, or that has not re-joined since a hub restart, can
//! still be released). A name that matches neither is `-32003 unknown_session`:
//! there are no phantom holds on sessions the hub has never seen. A bare name
//! matching more than one is refused as ambiguous, like `say`.

use holler_proto::{Code, CorrelationId, WireError};

use crate::control_server::{encode_error, encode_error_frame, encode_response};
use crate::live::Registry;
use crate::roster::Roster;

enum Resolved {
    Key(String),
    Unknown,
    Ambiguous(Vec<String>),
}

/// `name` is `<label>/<session>` or a bare `<session>`; a bare name matches
/// any known key ending in `/<name>`, an exact match always wins.
fn resolve(name: &str, registry: &Registry, roster: &Roster) -> Resolved {
    let mut known: Vec<String> = roster.rows(Some(true)).into_iter().map(|r| r.name).collect();
    known.extend(registry.holds().snapshot().into_keys());
    known.sort();
    known.dedup();
    if known.iter().any(|k| k == name) {
        return Resolved::Key(name.to_owned());
    }
    let suffix = format!("/{name}");
    let mut matches: Vec<String> = known.into_iter().filter(|k| k.ends_with(&suffix)).collect();
    match matches.len() {
        0 => Resolved::Unknown,
        1 => Resolved::Key(matches.remove(0)),
        _ => Resolved::Ambiguous(matches),
    }
}

fn refusal(cid: &CorrelationId, name: &str, r: Resolved) -> Result<String, String> {
    match r {
        Resolved::Key(k) => Ok(k),
        Resolved::Unknown => Err(encode_error(cid, Code::UnknownSession, format!("unknown session: {name}"))),
        Resolved::Ambiguous(candidates) => {
            let message = format!("ambiguous session {name}: candidates are {}", candidates.join(", "));
            Err(encode_error_frame(cid, &WireError::new(Code::UnknownSession, message, Some("ambiguous"))))
        }
    }
}

fn session_param<'a>(cid: &CorrelationId, obj: &'a serde_json::Value, method: &str) -> Result<&'a str, String> {
    match obj.get("params").and_then(|p| p.get("session")).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(encode_error(cid, Code::InvalidParams, format!("{method} needs params.session"))),
    }
}

/// `control/hold {session, reason?}` → `{session, hold: true, reason?, since,
/// newly_held, persisted}`.
pub(crate) fn hold(cid: &CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let name = match session_param(cid, obj, "control/hold") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let key = match refusal(cid, name, resolve(name, registry, roster)) {
        Ok(k) => k,
        Err(e) => return e,
    };
    let reason = obj.get("params").and_then(|p| p.get("reason")).and_then(|v| v.as_str());
    let out = registry.holds().hold(&key, reason);
    encode_response(
        cid,
        serde_json::json!({
            "session": key,
            "hold": true,
            "reason": out.info.reason,
            "since": out.info.since,
            "newly_held": out.newly_held,
            "persisted": out.persisted,
        }),
    )
}

/// `control/release {session}` → `{session, hold: false, was_held, persisted}`.
pub(crate) fn release(cid: &CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let name = match session_param(cid, obj, "control/release") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let key = match refusal(cid, name, resolve(name, registry, roster)) {
        Ok(k) => k,
        Err(e) => return e,
    };
    let out = registry.holds().release(&key);
    encode_response(
        cid,
        serde_json::json!({ "session": key, "hold": false, "was_held": out.was_held, "persisted": out.persisted }),
    )
}
