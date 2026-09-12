#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! RED unit tests for the `holler_proto::log` module (story #144).
//!
//! These pin the *core* (no-OS) contract of the debug-logging primitive:
//!
//! - `resolve` — flag beats env, defaults are none/text, an unrecognised value
//!   is an `Err` (fail-closed), and an *empty* flag value is treated as
//!   "absent" (it falls through to the env, then the default).
//! - `redact` / `redact_frame` — case-insensitive key substrings and
//!   value-prefix matches are replaced with `[redacted]`, recursively, at every
//!   depth; everything else is left intact.
//! - `Event::render` — the text layout's fixed-width component column, and one
//!   JSON object per line for the `json` format.
//! - Severity independence — `debug` events are gated by the level, while
//!   `info`/`warn` always render.
//!
//! The banner is exercised in `banner_test.rs` (its own binary) because it
//! guards on a process-global `OnceLock`/`AtomicU8` that can only be observed
//! in "exactly once" form from a fresh process.

use holler_proto::log::{
    resolve, redact, redact_frame, Component, Config, DebugLevel, Direction, Event, LogFormat,
    REDACTED, Severity,
};
use serde_json::json;

// --- resolve: flag beats env, then default ------------------------------------

#[test]
fn resolve_flag_beats_env() {
    let c = resolve(Some("noisy"), Some("json"), Some("none"), Some("text"))
        .expect("valid values");
    assert_eq!(c.debug, DebugLevel::Noisy, "the --debug flag beats HOLLER_DEBUG");
    assert_eq!(c.format, LogFormat::Json, "the --log-format flag beats HOLLER_LOG_FORMAT");
}

#[test]
fn resolve_defaults_when_neither_flag_nor_env() {
    let c = resolve(None, None, None, None).expect("defaults are valid");
    assert_eq!(c.debug, DebugLevel::None, "default level is none");
    assert_eq!(c.format, LogFormat::Text, "default format is text");
}

#[test]
fn resolve_env_used_when_flag_absent() {
    let c = resolve(None, None, Some("quiet"), Some("json")).expect("valid");
    assert_eq!(c.debug, DebugLevel::Quiet);
    assert_eq!(c.format, LogFormat::Json);
}

#[test]
fn resolve_empty_flag_is_treated_as_absent() {
    // The CLI captures the dials with `default_value = ""`, so an *absent*
    // flag arrives as `Some("")`. The resolver must treat that as "not
    // given" (fall through to the env), not as the invalid value `""`.
    let c = resolve(Some(""), None, Some("noisy"), None).expect("empty flag is absent");
    assert_eq!(c.debug, DebugLevel::Noisy, "empty --debug must not shadow the env");
    let c = resolve(None, Some(""), None, Some("json")).expect("empty flag is absent");
    assert_eq!(c.format, LogFormat::Json, "empty --log-format must not shadow the env");
}

#[test]
fn resolve_invalid_value_is_err() {
    let err = resolve(Some("verbose"), None, None, None)
        .expect_err("an unrecognised level is a fail-closed refusal");
    assert!(err.contains("--debug"), "the error names the offending flag: {err}");
    assert!(err.contains("verbose"), "the error echoes the bad value: {err}");

    let err = resolve(None, Some("yaml"), None, None)
        .expect_err("an unrecognised format is a fail-closed refusal");
    assert!(err.contains("--log-format"), "the error names the offending flag: {err}");
}

#[test]
fn resolve_invalid_env_value_is_err() {
    // The env is a fallback, but an unrecognised env value is *also* a
    // fail-closed refusal (we never silently downgrade on a typo).
    assert!(resolve(None, None, Some("chatty"), None).is_err());
    assert!(resolve(None, None, None, Some("xml")).is_err());
}

// --- redaction ---------------------------------------------------------------

#[test]
fn redact_key_substrings_are_case_insensitive() {
    // The story's rule: any key containing secret|credential|ticket|
    // authorization|pepper|private_key|signing_key (case-insensitive) is
    // redacted.
    let inputs = [
        "secret", "SECRET", "Secret",
        "client_secret", "x-api-key-secret", "SECRET_KEY",
        "credential", "Credential", "credentials",
        "ticket", "TICKET", "auth_ticket",
        "authorization", "Authorization", "AUTHORIZATION",
        "pepper", "Pepper", "HOLLER_PEPPER",
        // issue #322: the hub's X25519 private key.
        "private_key", "PRIVATE_KEY", "hub_private_key", "body_private_key",
        // issue #323: `BodyIdentity::signing_key` is the field's literal
        // name (not `*private_key*`), so it needs its own substring.
        "signing_key", "SIGNING_KEY", "body_signing_key",
    ];
    for key in inputs {
        let v = json!({ key: "some-value" });
        let v = redact(&v);
        assert_eq!(
            v[key], "[redacted]",
            "key {key:?} should be redacted, got {v:?}"
        );
    }
}

#[test]
fn redact_non_secret_keys_are_left_alone() {
    // Ordinary keys must survive verbatim — over-redaction would make the
    // noise useless.
    let v = json!({ "id": "abc", "hostname": "h", "session": "s1", "peer": "p" });
    let v = redact(&v);
    assert_eq!(v["id"], "abc");
    assert_eq!(v["hostname"], "h");
    assert_eq!(v["session"], "s1");
    assert_eq!(v["peer"], "p");
}

#[test]
fn redact_value_prefixes() {
    // The story's value rule: a string value that *begins* with a token
    // prefix is redacted.
    let a = json!({ "token": "hlr_join_abc123" });
    assert_eq!(redact(&a)["token"], "[redacted]");
    let b = json!({ "token": "hlr_live_xyz789" });
    assert_eq!(redact(&b)["token"], "[redacted]");
    // …but the same words inside a larger value are *not* secrets.
    let c = json!({ "note": "see hlr_join_ in the docs" });
    assert_eq!(redact(&c)["note"], "see hlr_join_ in the docs");
}

#[test]
fn redact_recurses_into_nested_objects_and_arrays() {
    let input = json!({
        "id": "abc",
        "nested": { "inner_secret": "top", "deeper": { "deep_pepper": "zz" } },
        "list": [ "keep", { "ticket": "t1" }, 42 ]
    });
    let out = redact(&input);
    assert_eq!(out["id"], "abc", "non-secret top key survives");
    assert_eq!(out["nested"]["inner_secret"], "[redacted]");
    assert_eq!(out["nested"]["deeper"]["deep_pepper"], "[redacted]");
    assert_eq!(out["list"][0], "keep");
    assert_eq!(out["list"][1]["ticket"], "[redacted]");
    assert_eq!(out["list"][2], json!(42), "non-string values are untouched");
}

#[test]
fn redact_frame_round_trips_through_json() {
    // `redact_frame` parses a raw wire-frame string, redacts it, re-serialises
    // it. A secret key inside the frame comes back `[redacted]`; the frame is
    // still valid JSON.
    let frame = r#"{"id":"c14fb1a960b3","secret":"hlr_join_123","ok":"plain"}"#;
    let out = redact_frame(frame);
    let v: serde_json::Value = serde_json::from_str(&out).expect("still JSON after redaction");
    assert_eq!(v["secret"], "[redacted]");
    assert_eq!(v["id"], "c14fb1a960b3", "non-secret fields survive");
    assert_eq!(v["ok"], "plain");
}

#[test]
fn redact_frame_leaves_non_json_verbatim() {
    // A frame that is not a JSON object (e.g. a bare token string) is returned
    // verbatim — it cannot be a secret *shape* to recursively scrub.
    let out = redact_frame("not-json-at-all");
    assert_eq!(out, "not-json-at-all");
}

// --- rendering ---------------------------------------------------------------

fn event(component: Component, severity: Severity, method: &'static str) -> Event {
    Event {
        component,
        severity,
        direction: Direction::Local,
        method,
        id: Some("c14fb1a960b3"),
        peer: None,
        fields: Vec::new(),
        frame: None,
    }
}

#[test]
fn render_text_component_column_is_12_wide() {
    // `wire` is 4 chars → 8 leading spaces → a 12-char column.
    let config = Config { debug: DebugLevel::Noisy, format: LogFormat::Text };
    let line = event(Component::Wire, Severity::Info, "hello").render(&config);
    // Layout: `<ts> INFO <12-char column> …`. We anchor on the level token
    // (robust to the timestamp's exact length, which varies with the year)
    // and slice the 12-char column relative to it: `<level>` is 4 chars, then
    // a space, then the 12-char (left-padded) column. So the column spans
    // offsets [level+5, level+17).
    //
    // (We can't use `str::split(' ')` here — it *collapses* the column's
    // leading pad spaces, turning the 12-char field into a bare 4-char `wire`.)
    let level = line.find("INFO").expect("the level token is present");
    assert_eq!(
        line.get(level + 5..level + 17).expect("the 12-char column"),
        "        wire",
        "the component column must be exactly 12 wide (8 pad + `wire`), got: {line:?}"
    );
}

#[test]
fn render_json_is_one_object_with_expected_keys() {
    let config = Config { debug: DebugLevel::Noisy, format: LogFormat::Json };
    let line = event(Component::Wire, Severity::Warn, "boom").render(&config);
    let v: serde_json::Value =
        serde_json::from_str(&line).expect("the json line must parse as one object");
    assert!(v.is_object());
    for key in ["ts", "level", "component", "dir", "type"] {
        assert!(v.get(key).is_some(), "missing `{key}` in {v:?}");
    }
    assert_eq!(v["component"], "wire");
    assert_eq!(v["level"], "WARN");
    assert_eq!(v["type"], "boom");
}

#[test]
fn redacted_constant_is_the_placeholder() {
    assert_eq!(REDACTED, "[redacted]");
}

// --- severity independence ---------------------------------------------------

#[test]
fn debug_severity_is_gated_by_level() {
    let debug_ev = event(Component::Wire, Severity::Debug, "tick");
    // At `none`, a debug event renders to the empty string (a no-op emit).
    assert_eq!(
        debug_ev.render(&Config { debug: DebugLevel::None, format: LogFormat::Text }),
        "",
        "a debug event must be silent at --debug none"
    );
    // …but at `quiet`/`noisy` it renders a real line.
    assert!(!debug_ev.render(&Config { debug: DebugLevel::Quiet, format: LogFormat::Text }).is_empty());
    assert!(!debug_ev.render(&Config { debug: DebugLevel::Noisy, format: LogFormat::Text }).is_empty());
}

#[test]
fn info_and_warn_are_always_emitted() {
    // Regardless of the level, info and warn render a non-empty line.
    for severity in [Severity::Info, Severity::Warn] {
        for level in [DebugLevel::None, DebugLevel::Quiet, DebugLevel::Noisy] {
            let ev = event(Component::Cli, severity, "op");
            let rendered = ev.render(&Config { debug: level, format: LogFormat::Text });
            assert!(
                !rendered.is_empty(),
                "{severity:?} must always render at {level:?}, got: {rendered:?}"
            );
        }
    }
}

// --- log injection (#237) ------------------------------------------------------

#[test]
fn render_text_escapes_newlines_in_field_values() {
    // Regression for #237: a body-supplied field (e.g. `hostname`) that
    // embeds a `\n` (plus a forged timestamp/level/component prefix) must
    // not be able to inject a second, fabricated log line into the
    // text-format renderer's single-line output.
    let forged = "evil\n2026-09-09T00:00:00.000000Z INFO         wire -- lockout client_id=forged";
    let mut ev = event(Component::Registry, Severity::Info, "conn_connected");
    ev.fields = vec![("hostname", forged.to_owned())];
    let config = Config { debug: DebugLevel::None, format: LogFormat::Text };
    let line = ev.render(&config);

    assert_eq!(
        line.lines().count(),
        1,
        "an attacker-controlled field value must never produce more than one line, got: {line:?}"
    );
    assert!(
        !line.contains('\n'),
        "the rendered line must not contain a raw newline, got: {line:?}"
    );
    assert!(
        line.contains("hostname=evil\\n2026-09-09"),
        "the newline must be rendered as the escaped two-character sequence \\n, got: {line:?}"
    );
}

#[test]
fn render_text_escapes_carriage_returns_and_other_control_chars() {
    let ev_fields = vec![("hostname", "host\rname\t\u{0}end".to_owned())];
    let mut ev = event(Component::Registry, Severity::Debug, "presence");
    ev.fields = ev_fields;
    let config = Config { debug: DebugLevel::Noisy, format: LogFormat::Text };
    let line = ev.render(&config);

    assert!(!line.contains('\r'), "a raw carriage return must not reach the line: {line:?}");
    assert!(line.contains("\\r"), "carriage return must be escaped: {line:?}");
    assert!(line.contains("\\t"), "tab must be escaped: {line:?}");
    assert!(line.contains("\\u{0}"), "NUL must be escaped: {line:?}");
}

#[test]
fn render_text_leaves_printable_field_values_untouched() {
    // Ordinary hostnames (and any other printable value, including non-ASCII
    // text) must render verbatim — the fix must not over-escape.
    let mut ev = event(Component::Registry, Severity::Info, "conn_connected");
    ev.fields = vec![("hostname", "laptop-42.local".to_owned())];
    let config = Config { debug: DebugLevel::None, format: LogFormat::Text };
    let line = ev.render(&config);
    assert!(
        line.contains("hostname=laptop-42.local"),
        "a normal hostname must render verbatim, got: {line:?}"
    );
}

#[test]
fn render_json_already_escapes_control_characters() {
    // The JSON path was already safe (serde_json escapes control characters
    // in strings); this pins that guarantee so a future change can't
    // silently regress it.
    let mut ev = event(Component::Registry, Severity::Info, "conn_connected");
    ev.fields = vec![("hostname", "evil\nforged line".to_owned())];
    let config = Config { debug: DebugLevel::None, format: LogFormat::Json };
    let line = ev.render(&config);
    assert_eq!(line.lines().count(), 1, "the JSON line must stay a single line: {line:?}");
    let v: serde_json::Value = serde_json::from_str(&line).expect("valid JSON despite the embedded newline");
    assert_eq!(v["hostname"], "evil\nforged line", "serde_json round-trips the raw value safely");
}
