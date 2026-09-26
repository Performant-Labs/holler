//! ACP auth-method selection, the `session/new` auth sequence and the text of
//! every auth-related startup reason (issues #439, #459). Version-neutral:
//! both protocol paths run the same code. `connection_v1.rs` sends v1's
//! `authenticate`, `connection.rs` sends v2's `auth/login` (v2 has no
//! `authenticate`); each reduces its `initialize` response's advertised
//! methods to [`AdvertisedMethod`]s and hands its own `session/new` and login
//! requests to [`open_session`], so the trigger (a `session/new` that failed
//! with auth-required, JSON-RPC `-32000`), [`select`], the request order and
//! the [`AuthStage`] transitions exist once. `spawn.rs` appends
//! [`stage_suffix`] at each spawn attempt's one common failure point.
//!
//! The reasons call the login step `authenticate` on both protocols (the
//! existing wording, which v2 keeps); the debug log names the wire request
//! each path actually sends.
//!
//! Every string in a reason that is not the driver's own text goes through
//! [`quote_id`] (a method id, advertised or configured) or
//! [`cap_adapter_message`] (an adapter's JSON-RPC error message): control
//! characters become a space and the length is capped, so a hostile adapter
//! can neither flood a startup error nor forge a log line through it. A
//! method's `description`/`_meta` never reach this module, and a JSON-RPC
//! error's `data` never reaches a reason built here, except through
//! [`startup_error_text`] with no `auth_method` set, which keeps an error's
//! display byte for byte.

use std::future::Future;

use agent_client_protocol::ErrorCode;
use holler_proto::log::Direction as LogDirection;

use super::log_debug;

/// At most this many advertised ids are listed in a refusal; the rest are
/// counted as `(+N more)`.
pub(super) const MAX_ECHOED_IDS: usize = 8;
/// An echoed method id (advertised or configured) keeps at most this many
/// characters.
pub(super) const MAX_ID_CHARS: usize = 64;
/// An echoed adapter error message keeps at most this many characters.
pub(super) const MAX_ADAPTER_MSG_CHARS: usize = 200;
/// The JSON-RPC code that starts the auth flow (`ErrorCode::AuthRequired`).
const AUTH_REQUIRED: i32 = -32000;

/// What selection needs to know about an advertised method's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MethodKind {
    /// The adapter authenticates itself when sent the login request (v1
    /// `authenticate`, v2 `auth/login`); v1's default when a method carries
    /// no `type`.
    Agent,
    /// The client must run an interactive login instead; the protocol says a
    /// client never passes it to the login request, so it is never selected.
    Terminal,
    /// A v1 kind this driver does not know (a variant a newer SDK adds to the
    /// `#[non_exhaustive]` `v1::AuthMethod`). Only `Terminal` is refused, so
    /// this is selectable like `Agent`. Not produced with the pinned SDK:
    /// v1's `AuthMethod::Agent` variant is untagged, so a v1 method with an
    /// unrecognised `type` value deserializes as `Agent`, and the "never send
    /// a terminal-type method" guard can only recognise the literal
    /// `terminal` type. Never produced on v2 (#459): v2 tags `Agent`, so an
    /// unrecognised `type` deserializes as `v2::AuthMethod::Other`, which
    /// `connection.rs` drops before selection (fail closed, as for any
    /// variant a newer SDK adds): it is never selected or sent, and a
    /// configured id that only it carries is unadvertised.
    Other,
}

/// One advertised auth method, reduced to what selection and the reasons
/// need. Never its description or `_meta`, which may carry hints (an env-var
/// name, a URL) this driver must not echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdvertisedMethod {
    pub(super) id: String,
    pub(super) kind: MethodKind,
}

/// Choose the method id to send in the login request (v1 `authenticate`, v2
/// `auth/login`), after `session/new` failed with auth-required. `Ok` only
/// when `configured` exactly equals (case-sensitive, untrimmed) an advertised
/// id and no advertised entry with that id is terminal-type; the id returned
/// is the configured string. Otherwise `Err` with the refusal, which always
/// names `auth_method`, and no login request is sent.
pub(super) fn select(advertised: &[AdvertisedMethod], configured: Option<&str>) -> Result<String, String> {
    if advertised.is_empty() {
        return Err(match configured {
            Some(id) => format!(
                "auth_method {} cannot be applied: the adapter lists no auth methods (none advertised)",
                quote_id(id)
            ),
            None => "auth_method is not set, and the adapter lists no auth methods (none advertised)".to_string(),
        });
    }
    let listed = list_ids(advertised);
    let Some(id) = configured else {
        return Err(format!("auth_method is not set; set it to one of the advertised methods: {listed}"));
    };
    if !advertised.iter().any(|m| m.id == id) {
        return Err(format!(
            "auth_method {} is not advertised by the adapter; advertised methods: {listed}",
            quote_id(id)
        ));
    }
    if advertised.iter().any(|m| m.id == id && m.kind == MethodKind::Terminal) {
        return Err(format!(
            "auth_method {} is a terminal-type method: it needs a manual login (#440) and is never sent \
             in authenticate; advertised methods: {listed}",
            quote_id(id)
        ));
    }
    Ok(id.to_string())
}

/// The advertised ids as a refusal (or the debug log) lists them: the first
/// [`MAX_ECHOED_IDS`] through [`quote_id`], comma-separated, then
/// `(+N more)` for the rest.
pub(super) fn list_ids(advertised: &[AdvertisedMethod]) -> String {
    let shown: Vec<String> = advertised.iter().take(MAX_ECHOED_IDS).map(|m| quote_id(&m.id)).collect();
    match advertised.len().saturating_sub(MAX_ECHOED_IDS) {
        0 => shown.join(", "),
        more => format!("{} (+{more} more)", shown.join(", ")),
    }
}

/// A method id as every reason and log field shows it: control characters
/// replaced by a space, at most [`MAX_ID_CHARS`] characters (then `...`),
/// `"` and `\` escaped, inside double quotes. So `" stub-key "` stays
/// visibly distinct from `"stub-key"`, and no id can pass for the end of
/// another in a list.
pub(super) fn quote_id(raw: &str) -> String {
    let capped = sanitize(raw, MAX_ID_CHARS);
    let mut out = String::with_capacity(capped.len() + 2);
    out.push('"');
    for c in capped.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// An adapter's JSON-RPC error message as a reason echoes it: control
/// characters replaced by a space, at most [`MAX_ADAPTER_MSG_CHARS`]
/// characters (then `...`). Only ever the message, never the error's `data`.
pub(super) fn cap_adapter_message(msg: &str) -> String {
    sanitize(msg, MAX_ADAPTER_MSG_CHARS)
}

/// `raw` with control characters replaced by a space, cut to `max`
/// characters on a char boundary, with `...` appended when anything was cut.
fn sanitize(raw: &str, max: usize) -> String {
    let mut chars = raw.chars();
    let mut out: String = chars.by_ref().take(max).map(|c| if c.is_control() { ' ' } else { c }).collect();
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

/// `<message> (JSON-RPC code <code>)` with the adapter's message capped, or
/// just the code when the adapter sent no message.
pub(super) fn rpc_error(message: &str, code: i32) -> String {
    let message = cap_adapter_message(message);
    if message.trim().is_empty() {
        format!("JSON-RPC code {code}")
    } else {
        format!("{message} (JSON-RPC code {code})")
    }
}

/// The reason when `session/new` answered auth-required and [`select`]
/// refused: the adapter's own message, then the refusal.
pub(super) fn refused_reason(adapter_message: &str, refusal: &str) -> String {
    format!("session/new: {}; {refusal}", rpc_error(adapter_message, AUTH_REQUIRED))
}

/// The reason when the login request (`authenticate` or `auth/login`) itself
/// failed. Fatal: `session/new` is not retried.
pub(super) fn authenticate_failed_reason(configured: &str, code: i32, adapter_message: &str) -> String {
    format!(
        "authenticate with auth_method {} failed: {}",
        quote_id(configured),
        rpc_error(adapter_message, code)
    )
}

/// The reason when the login request succeeded but the one retried
/// `session/new` answered auth-required again.
pub(super) fn did_not_clear_reason(configured: &str, adapter_message: &str) -> String {
    format!(
        "authentication with auth_method {} did not clear the requirement: the retried session/new failed: {}",
        quote_id(configured),
        rpc_error(adapter_message, AUTH_REQUIRED)
    )
}

/// The reason when the login request succeeded but the one retried
/// `session/new` failed with any other code.
pub(super) fn retry_failed_reason(configured: &str, code: i32, adapter_message: &str) -> String {
    format!(
        "session/new failed after authenticate with auth_method {}: {}",
        quote_id(configured),
        rpc_error(adapter_message, code)
    )
}

/// How far a handshake's auth flow got. Written by [`open_session`], read by
/// the spawn attempt (`spawn_v1` or `spawn_v2`) at its one common failure
/// point, so every startup failure (a failed `initialize`, a timeout, a dead
/// connection task, an error from the first `session/new`) can say what
/// happened to a configured `auth_method` without each path building its own
/// text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthStage {
    /// The login request was never sent (startup ended in `initialize` or the
    /// first `session/new`).
    NotReached,
    /// The login request (`authenticate` or `auth/login`) was sent and is
    /// unanswered.
    AuthenticatePending,
    /// The login request succeeded; the one retried `session/new` is
    /// unanswered.
    RetryPending,
    /// The flow ended and any failure already carries its own reason (an
    /// auth-flow refusal or error), or startup succeeded.
    Settled,
}

/// Shared cell holding the current [`AuthStage`].
#[derive(Debug, Default)]
pub(super) struct AuthProgress(std::sync::atomic::AtomicU8);

impl AuthProgress {
    pub(super) fn set(&self, stage: AuthStage) {
        self.0.store(stage as u8, std::sync::atomic::Ordering::SeqCst);
    }

    pub(super) fn get(&self) -> AuthStage {
        match self.0.load(std::sync::atomic::Ordering::SeqCst) {
            1 => AuthStage::AuthenticatePending,
            2 => AuthStage::RetryPending,
            3 => AuthStage::Settled,
            _ => AuthStage::NotReached,
        }
    }
}

/// Appended by both spawn attempts to any startup failure whose text does
/// not already say what happened to the configured `auth_method`; empty once
/// the stage is [`AuthStage::Settled`].
pub(super) fn stage_suffix(configured: &str, stage: AuthStage) -> String {
    let id = quote_id(configured);
    match stage {
        AuthStage::NotReached => format!(
            "; auth_method {id} was configured but not applied (startup ended before session/new answered \
             auth-required {AUTH_REQUIRED})"
        ),
        AuthStage::AuthenticatePending => {
            format!("; timed out or ended while authenticate with auth_method {id} was pending")
        }
        AuthStage::RetryPending => format!(
            "; authenticate with auth_method {id} was sent, but startup ended before the retried session/new completed"
        ),
        AuthStage::Settled => String::new(),
    }
}

/// A startup error outside the auth flow, as its reason. With no
/// `auth_method` this is the error's display, byte for byte, the text such a
/// failure has always had. With one set, the display would carry the
/// adapter's `data` (newlines, no length limit), so the reason is rebuilt from
/// the code and the capped, sanitized message alone. `spawn_v2` applies it to
/// a failed v2 `initialize` only after checking the raw error for the v1
/// negotiation failure, whose phrase the SDK puts only in `data` (#459).
pub(super) fn startup_error_text(e: &agent_client_protocol::Error, auth_method: Option<&str>) -> String {
    match auth_method {
        None => e.to_string(),
        Some(_) => rpc_error(&e.message, i32::from(e.code)),
    }
}

/// `session/new` plus the auth flow: the one sequence both protocols run (see
/// the module doc). `new_session` sends one `session/new` and is called at
/// most twice; `login` sends the one login request for the id [`select`]
/// chose, only after the first `session/new` failed with auth-required. The
/// trigger is the JSON-RPC code alone, never message text. A first failure
/// with any other code is [`startup_error_text`]; every auth-flow reason is
/// built from an error's code and capped, sanitized message, never its
/// `data`. The spawn attempt adds the not-applied/pending clause from
/// `progress`.
pub(super) async fn open_session<S, N, L>(
    advertised: &[AdvertisedMethod],
    auth_method: Option<&str>,
    progress: &AuthProgress,
    mut new_session: impl FnMut() -> N,
    login: impl FnOnce(String) -> L,
) -> Result<S, String>
where
    N: Future<Output = Result<S, agent_client_protocol::Error>>,
    L: Future<Output = Result<(), agent_client_protocol::Error>>,
{
    let first = match new_session().await {
        Ok(session) => {
            progress.set(AuthStage::Settled);
            return Ok(session);
        }
        Err(e) => e,
    };
    if first.code != ErrorCode::AuthRequired {
        return Err(startup_error_text(&first, auth_method));
    }
    log_debug(
        LogDirection::In,
        "session/new",
        vec![("event", "auth_required".to_string()), ("advertised", list_ids(advertised))],
        None,
    );
    let method_id = select(advertised, auth_method).map_err(|refusal| {
        progress.set(AuthStage::Settled);
        refused_reason(&first.message, &refusal)
    })?;
    progress.set(AuthStage::AuthenticatePending);
    if let Err(e) = login(method_id.clone()).await {
        progress.set(AuthStage::Settled);
        return Err(authenticate_failed_reason(&method_id, i32::from(e.code), &e.message));
    }
    progress.set(AuthStage::RetryPending);
    let retried = new_session().await;
    progress.set(AuthStage::Settled);
    match retried {
        Ok(session) => Ok(session),
        Err(e) if e.code == ErrorCode::AuthRequired => Err(did_not_clear_reason(&method_id, &e.message)),
        Err(e) => Err(retry_failed_reason(&method_id, i32::from(e.code), &e.message)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #439
mod tests {
    use super::*;

    fn agent(id: &str) -> AdvertisedMethod {
        AdvertisedMethod { id: id.to_string(), kind: MethodKind::Agent }
    }

    fn terminal(id: &str) -> AdvertisedMethod {
        AdvertisedMethod { id: id.to_string(), kind: MethodKind::Terminal }
    }

    fn refusal(advertised: &[AdvertisedMethod], configured: Option<&str>) -> String {
        let reason = select(advertised, configured).expect_err("must refuse");
        assert!(reason.contains("auth_method"), "every refusal names the config key: {reason}");
        reason
    }

    #[test]
    fn configured_and_advertised_agent_method_is_selected_verbatim() {
        let got = select(&[agent("other"), agent("stub-key")], Some("stub-key"));
        assert_eq!(got, Ok("stub-key".to_string()));
    }

    #[test]
    fn missing_configuration_refuses_listing_advertised_ids() {
        let r = refusal(&[agent("stub-key"), agent("api-key")], None);
        assert!(r.contains("\"stub-key\"") && r.contains("\"api-key\""), "{r}");
    }

    #[test]
    fn nothing_advertised_refuses_saying_none_advertised() {
        let r = refusal(&[], Some("stub-key"));
        assert!(r.contains("none advertised"), "{r}");
    }

    #[test]
    fn unadvertised_method_refuses_naming_both_sides() {
        let r = refusal(&[agent("stub-key")], Some("nope"));
        assert!(r.contains("\"nope\"") && r.contains("\"stub-key\"") && r.contains("not advertised"), "{r}");
    }

    #[test]
    fn ids_compare_exactly_case_sensitive_and_untrimmed() {
        let r = refusal(&[agent("Stub-Key")], Some("stub-key"));
        assert!(r.contains("not advertised"), "{r}");
        let r = refusal(&[agent("stub-key")], Some(" stub-key "));
        assert!(r.contains("\" stub-key \"") && r.contains("not advertised"), "padded value renders distinctly: {r}");
    }

    #[test]
    fn terminal_method_refuses_as_manual_login() {
        let r = refusal(&[terminal("stub-tty"), agent("stub-key")], Some("stub-tty"));
        assert!(r.contains("\"stub-tty\"") && r.contains("manual login"), "{r}");
        assert!(!r.contains("not advertised"), "{r}");
    }

    #[test]
    fn duplicate_ids_refuse_if_any_is_terminal_else_select_once() {
        let r = refusal(&[agent("dup"), terminal("dup")], Some("dup"));
        assert!(r.contains("manual login"), "{r}");
        assert_eq!(select(&[agent("dup"), agent("dup")], Some("dup")), Ok("dup".to_string()));
    }

    #[test]
    fn refusal_reasons_are_distinct_per_case() {
        let reasons = [
            refusal(&[agent("stub-key")], None),
            refusal(&[agent("stub-key")], Some("nope")),
            refusal(&[terminal("stub-key")], Some("stub-key")),
            refusal(&[], Some("stub-key")),
        ];
        for (i, a) in reasons.iter().enumerate() {
            for b in &reasons[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn a_flood_of_long_ids_lists_at_most_eight_capped_ids_then_a_count() {
        let flood: Vec<_> = (0..100).map(|n| agent(&format!("id{n:03}{}", "z".repeat(495)))).collect();
        let r = refusal(&flood, None);
        assert_eq!(r.matches("\"id").count(), 8, "{r}");
        assert!(r.contains("(+92 more)"), "{r}");
        let longest_z = r.split(|c| c != 'z').map(str::len).max().unwrap_or(0);
        assert!(longest_z <= 64, "each id at most 64 chars: {r}");
    }

    #[test]
    fn quote_id_quotes_sanitizes_and_caps() {
        assert_eq!(quote_id("stub-key"), "\"stub-key\"");
        assert_eq!(quote_id(" stub-key "), "\" stub-key \"");
        assert_ne!(quote_id(" stub-key "), quote_id("stub-key"));
        assert_eq!(quote_id("a\nb"), "\"a b\"");
        let hostile = quote_id("x\u{1b}[31m\ny");
        assert!(!hostile.chars().any(char::is_control), "{hostile:?}");
        assert!(hostile.starts_with('"') && hostile.ends_with('"'), "{hostile}");
        let long = quote_id(&"a".repeat(500));
        assert_eq!(long.matches('a').count(), MAX_ID_CHARS);
        assert!(long.starts_with('"') && long.ends_with('"'), "{long}");
        // Cut on a char boundary: a multi-byte id must not panic.
        assert_eq!(quote_id(&"é".repeat(100)).matches('é').count(), MAX_ID_CHARS);
    }

    #[test]
    fn configured_value_goes_through_the_same_sanitize_and_cap() {
        let configured = format!("x\n\u{1b}[31m{}", "c".repeat(500));
        let r = refusal(&[agent("stub-key")], Some(&configured));
        assert!(!r.chars().any(char::is_control), "{r:?}");
        assert!(r.matches('c').count() <= MAX_ID_CHARS, "{r}");
    }

    #[test]
    fn adapter_message_is_capped_and_sanitized() {
        assert_eq!(cap_adapter_message("Authentication required"), "Authentication required");
        let capped = cap_adapter_message(&"Q".repeat(5000));
        assert!(capped.matches('Q').count() <= MAX_ADAPTER_MSG_CHARS, "{} Qs", capped.matches('Q').count());
        assert!(capped.chars().count() <= MAX_ADAPTER_MSG_CHARS + 3);
        let multibyte = cap_adapter_message(&"é".repeat(5000));
        assert!(multibyte.matches('é').count() <= MAX_ADAPTER_MSG_CHARS);
        assert!(!cap_adapter_message("bad\r\n\u{1b}[0mtext").chars().any(char::is_control));
    }

    #[test]
    fn caps_are_the_briefs_values() {
        assert_eq!((MAX_ECHOED_IDS, MAX_ID_CHARS, MAX_ADAPTER_MSG_CHARS), (8, 64, 200));
    }

    #[test]
    fn v1_stage_suffixes_are_exact_and_settled_is_empty() {
        assert_eq!(
            stage_suffix("stub-key", AuthStage::NotReached),
            "; auth_method \"stub-key\" was configured but not applied (startup ended before session/new answered auth-required -32000)"
        );
        assert!(stage_suffix("k", AuthStage::AuthenticatePending).contains("authenticate with auth_method \"k\" was pending"));
        assert!(stage_suffix("k", AuthStage::RetryPending).contains("retried session/new"));
        assert_eq!(stage_suffix("k", AuthStage::Settled), "");
    }

    #[test]
    fn auth_progress_round_trips_every_stage() {
        let p = AuthProgress::default();
        assert_eq!(p.get(), AuthStage::NotReached);
        for stage in [AuthStage::AuthenticatePending, AuthStage::RetryPending, AuthStage::Settled, AuthStage::NotReached] {
            p.set(stage);
            assert_eq!(p.get(), stage);
        }
    }

    #[test]
    fn suffixes_render_the_configured_id_through_quote_id() {
        let id = "a\nb";
        for stage in [AuthStage::NotReached, AuthStage::AuthenticatePending, AuthStage::RetryPending] {
            let suffix = stage_suffix(id, stage);
            assert!(suffix.contains("auth_method \"a b\""), "{suffix}");
            assert!(!suffix.chars().any(char::is_control));
        }
    }
}
