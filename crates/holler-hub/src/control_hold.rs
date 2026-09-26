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
    known.extend(registry.holds().keys());
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

/// `control/release {session, once?, ttl_ms?}`.
///
/// Without `once`: lifts the top hold (the operator hold if there is one, else
/// the default hold) and answers `{session, hold: false, was_held, lifted,
/// still_held, persisted}`.
///
/// With `once` (issue #460): lifts nothing; mints a one-time grant for exactly
/// one prompt to this session, valid for `ttl_ms` (default 60 s), and answers
/// `{session, grant, ttl_ms, default_held, operator_held}`. The grant is
/// presented with `say --grant`; it lifts a default hold only.
pub(crate) fn release(cid: &CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let name = match session_param(cid, obj, "control/release") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let key = match refusal(cid, name, resolve(name, registry, roster)) {
        Ok(k) => k,
        Err(e) => return e,
    };
    let params = obj.get("params");
    if params.and_then(|p| p.get("once")).and_then(|v| v.as_bool()) == Some(true) {
        return mint(cid, &key, params, registry);
    }
    let out = registry.holds().release(&key);
    encode_response(
        cid,
        serde_json::json!({
            "session": key,
            "hold": out.still_held,
            "was_held": out.was_held,
            "lifted": out.lifted.map(crate::holds::Kind::as_str),
            "still_held": out.still_held,
            "persisted": out.persisted,
        }),
    )
}

fn mint(cid: &CorrelationId, key: &str, params: Option<&serde_json::Value>, registry: &Registry) -> String {
    let ttl = match params.and_then(|p| p.get("ttl_ms")).and_then(|v| v.as_u64()) {
        None => crate::holds::DEFAULT_TTL,
        Some(0) => return encode_error(cid, Code::InvalidParams, "ttl must be greater than zero".to_string()),
        Some(ms) => std::time::Duration::from_millis(ms),
    };
    if ttl > crate::holds::MAX_TTL {
        return encode_error(cid, Code::InvalidParams, "ttl is longer than the 24h maximum".to_string());
    }
    match registry.holds().mint_grant(key, ttl) {
        Ok(g) => encode_response(
            cid,
            serde_json::json!({
                "session": key,
                "grant": g.grant,
                "ttl_ms": u64::try_from(g.ttl.as_millis()).unwrap_or(u64::MAX),
                "default_held": g.default_held,
                "operator_held": g.operator_held,
            }),
        ),
        Err(crate::holds::MintError::Full) => encode_error(cid, Code::LimitExceeded, "too many live grants".to_string()),
        Err(crate::holds::MintError::NoRandomness) => {
            encode_error(cid, Code::InvalidRequest, "the hub could not generate a grant id".to_string())
        }
    }
}
