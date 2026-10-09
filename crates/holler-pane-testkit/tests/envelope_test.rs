//! The JSON-envelope checker (`holler_pane_testkit::envelope`, #681, slice b of #638):
//! the good envelopes it accepts, and a mutation table showing that every way a verb's
//! `--format=json` output can break ADR-0021 section 9 is rejected with the one fault
//! that names the first broken rule.

use std::collections::BTreeSet;

use holler_pane::error::{class_of, ALL_CODES};
use holler_pane_testkit::envelope::{
    check_envelope, check_ndjson, Envelope, EnvelopeError, EnvelopeFault,
};
use serde_json::{json, Value};

// ---- builders --------------------------------------------------------------------

fn good_success() -> Value {
    json!({"schema_version": 1, "ok": true, "data": {"name": "hj-c1r1"}, "error": null})
}

fn good_failure(code: &str) -> Value {
    json!({
        "schema_version": 1,
        "ok": false,
        "data": null,
        "error": {"code": code, "message": "something went wrong"},
    })
}

/// One compact envelope the way the CLI writes it: the value, then `\n`.
fn line(value: &Value) -> String {
    format!("{value}\n")
}

fn lines(values: &[Value]) -> String {
    values.iter().map(line).collect()
}

fn with(base: &Value, key: &str, value: Value) -> Value {
    let mut out = base.clone();
    if let Some(map) = out.as_object_mut() {
        map.insert(key.to_owned(), value);
    }
    out
}

fn without(base: &Value, key: &str) -> Value {
    let mut out = base.clone();
    if let Some(map) = out.as_object_mut() {
        map.remove(key);
    }
    out
}

/// Replace the whole `error` member of a failure.
fn with_error(code: &str, error: Value) -> Value {
    with(&good_failure(code), "error", error)
}

/// A failure whose `message` is `message`.
fn with_message(message: Value) -> Value {
    with_error("timeout", json!({"code": "timeout", "message": message}))
}

/// A failure whose `code` is `code`.
fn with_code(code: Value) -> Value {
    with_error("timeout", json!({"code": code, "message": "x"}))
}

// ---- AC1: the good envelopes are accepted -----------------------------------------

#[test]
fn a_success_at_exit_0_is_accepted() {
    let got = check_envelope(&line(&good_success()), 0);
    assert_eq!(
        got,
        Ok(Envelope {
            schema_version: 1,
            ok: true,
            data: json!({"name": "hj-c1r1"}),
            error: None,
        })
    );

    let null_data = with(&good_success(), "data", Value::Null);
    let got = check_envelope(&line(&null_data), 0);
    assert_eq!(
        got,
        Ok(Envelope {
            schema_version: 1,
            ok: true,
            data: Value::Null,
            error: None,
        })
    );
}

#[test]
fn a_failure_is_accepted_at_the_exit_code_of_its_class() {
    for (code, exit) in [
        ("timeout", 1),
        ("usage", 2),
        ("pane-not-found", 3),
        ("quota-exceeded", 3),
    ] {
        let got = check_envelope(&line(&good_failure(code)), exit);
        assert_eq!(
            got,
            Ok(Envelope {
                schema_version: 1,
                ok: false,
                data: Value::Null,
                error: Some(EnvelopeError {
                    code: code.to_owned(),
                    message: "something went wrong".to_owned(),
                }),
            }),
            "{code} at exit {exit}"
        );
    }
}

#[test]
fn the_adr_examples_are_accepted() {
    let ok = r#"{"schema_version": 1, "ok": true, "data": {}, "error": null}"#;
    let failed = r#"{"schema_version": 1, "ok": false, "data": null, "error": {"code": "pane-not-found", "message": "pane not found: hj-c1r1"}}"#;
    assert!(check_envelope(&format!("{ok}\n"), 0).is_ok());
    let got = check_envelope(&format!("{failed}\n"), 3);
    assert_eq!(
        got.map(|e| e.error),
        Ok(Some(EnvelopeError {
            code: "pane-not-found".to_owned(),
            message: "pane not found: hj-c1r1".to_owned(),
        }))
    );
}

#[test]
fn a_missing_final_newline_is_accepted() {
    assert!(check_envelope(&good_success().to_string(), 0).is_ok());
}

#[test]
fn every_closed_code_is_accepted_only_at_the_exit_code_of_its_class() {
    for code in ALL_CODES {
        for exit in 1..=3 {
            let got = check_envelope(&line(&good_failure(code)), exit);
            if class_of(code).exit_code() == exit {
                assert!(got.is_ok(), "{code} at exit {exit}: {got:?}");
            } else {
                assert_eq!(
                    got,
                    Err(EnvelopeFault::ClassDisagreesWithExit),
                    "{code} at exit {exit}"
                );
            }
        }
    }
}

// ---- AC2: every mutant is rejected with its fault ---------------------------------

struct Row {
    name: &'static str,
    stdout: String,
    exit: i32,
    fault: EnvelopeFault,
}

fn row(name: &'static str, stdout: impl Into<String>, exit: i32, fault: EnvelopeFault) -> Row {
    Row {
        name,
        stdout: stdout.into(),
        exit,
        fault,
    }
}

/// The exit code, the framing and the shape of the top-level object.
fn framing_rows() -> Vec<Row> {
    let g = line(&good_success());
    vec![
        row("broken", "{", 0, EnvelopeFault::NotJson),
        row("empty", "", 0, EnvelopeFault::NotJson),
        row("prose", "not json\n", 1, EnvelopeFault::NotJson),
        row(
            "text-before",
            format!("warning: hub slow\n{g}"),
            0,
            EnvelopeFault::TextBefore,
        ),
        row(
            "leading-space",
            format!(" {g}"),
            0,
            EnvelopeFault::TextBefore,
        ),
        row(
            "text-after",
            format!("{g}trailing\n"),
            0,
            EnvelopeFault::TextAfter,
        ),
        row(
            "two-envelopes",
            format!("{g}{g}"),
            0,
            EnvelopeFault::TextAfter,
        ),
        row(
            "blank-line-after",
            format!("{g}\n"),
            0,
            EnvelopeFault::TextAfter,
        ),
        row("array", "[1]", 0, EnvelopeFault::NotAnObject),
        row("exit-4", g.clone(), 4, EnvelopeFault::ExitCodeUnknown(4)),
        row("exit-negative", g, -1, EnvelopeFault::ExitCodeUnknown(-1)),
    ]
}

/// Extra keys placed before and among the known ones, so that neither document order nor
/// the order a map leaves after the known keys are removed is the smallest key
/// (`serde_json::Map::remove` is a `swap_remove` under `preserve_order`).
fn extra_key_order_rows() -> Vec<Row> {
    vec![
        row(
            "extra-keys-among-known-keys-smallest-first",
            r#"{"zz":1,"mm":3,"schema_version":1,"ok":true,"data":null,"error":null,"aa":2}
"#
            .to_owned(),
            0,
            EnvelopeFault::UnknownKey("aa".into()),
        ),
        row(
            "error-extra-keys-among-known-keys-smallest-first",
            r#"{"schema_version":1,"ok":false,"data":null,"error":{"zz":1,"mm":3,"code":"timeout","message":"x","aa":2}}
"#
            .to_owned(),
            1,
            EnvelopeFault::UnknownKey("error.aa".into()),
        ),
    ]
}

/// The four keys, `schema_version` and `ok` against the exit code.
fn key_rows() -> Vec<Row> {
    let g = good_success();
    let two_extra = r#"{"schema_version":1,"ok":true,"data":null,"error":null,"zz":1,"aa":2}"#;
    vec![
        row(
            "missing-schema-version",
            line(&without(&g, "schema_version")),
            0,
            EnvelopeFault::MissingKey("schema_version"),
        ),
        row(
            "missing-ok",
            line(&without(&g, "ok")),
            0,
            EnvelopeFault::MissingKey("ok"),
        ),
        row(
            "missing-data",
            line(&without(&g, "data")),
            0,
            EnvelopeFault::MissingKey("data"),
        ),
        row(
            "missing-error",
            line(&without(&g, "error")),
            0,
            EnvelopeFault::MissingKey("error"),
        ),
        row(
            "missing-data-and-error",
            line(&without(&without(&g, "error"), "data")),
            0,
            EnvelopeFault::MissingKey("data"),
        ),
        row(
            "detail-key",
            line(&with(&g, "detail", json!("x"))),
            0,
            EnvelopeFault::UnknownKey("detail".into()),
        ),
        // Two extra keys in reverse order: the smallest one is named whatever order the
        // JSON map keeps (`preserve_order` is on in a `--workspace` build, A's W-1).
        row(
            "two-extra-keys-smallest-first",
            format!("{two_extra}\n"),
            0,
            EnvelopeFault::UnknownKey("aa".into()),
        ),
        row(
            "schema-2",
            line(&with(&g, "schema_version", json!(2))),
            0,
            EnvelopeFault::SchemaVersion,
        ),
        row(
            "schema-string",
            line(&with(&g, "schema_version", json!("1"))),
            0,
            EnvelopeFault::SchemaVersion,
        ),
        row(
            "schema-float",
            format!(
                "{}\n",
                r#"{"schema_version":1.0,"ok":true,"data":null,"error":null}"#
            ),
            0,
            EnvelopeFault::SchemaVersion,
        ),
        row(
            "schema-null",
            line(&with(&g, "schema_version", Value::Null)),
            0,
            EnvelopeFault::SchemaVersion,
        ),
        row(
            "schema-checked-before-ok",
            line(&with(&g, "schema_version", json!(2))),
            3,
            EnvelopeFault::SchemaVersion,
        ),
        row(
            "ok-true-exit-3",
            line(&g),
            3,
            EnvelopeFault::OkDisagreesWithExit,
        ),
        row(
            "ok-false-exit-0",
            line(&good_failure("timeout")),
            0,
            EnvelopeFault::OkDisagreesWithExit,
        ),
        row(
            "ok-not-bool",
            line(&with(&g, "ok", json!("true"))),
            0,
            EnvelopeFault::OkDisagreesWithExit,
        ),
    ]
}

/// `error` and `data` against `ok`, then the body of the `error` object.
fn failure_rows() -> Vec<Row> {
    let f = good_failure("timeout");
    vec![
        row(
            "error-while-ok",
            line(&with(
                &good_success(),
                "error",
                json!({"code": "timeout", "message": "x"}),
            )),
            0,
            EnvelopeFault::ErrorWhileOk,
        ),
        row(
            "no-error",
            line(&with(&f, "error", Value::Null)),
            1,
            EnvelopeFault::NoErrorWhenFailed,
        ),
        row(
            "error-not-object",
            line(&with(&f, "error", json!("boom"))),
            1,
            EnvelopeFault::NoErrorWhenFailed,
        ),
        row(
            "data-on-failure",
            line(&with(&f, "data", json!({}))),
            1,
            EnvelopeFault::DataOnFailure,
        ),
        row(
            "no-code",
            line(&with_error("timeout", json!({"message": "x"}))),
            1,
            EnvelopeFault::MissingKey("error.code"),
        ),
        row(
            "no-message",
            line(&with_error("timeout", json!({"code": "timeout"}))),
            1,
            EnvelopeFault::MissingKey("error.message"),
        ),
        row(
            "empty-error-object",
            line(&with_error("timeout", json!({}))),
            1,
            EnvelopeFault::MissingKey("error.code"),
        ),
        row(
            "error-detail",
            line(&with_error(
                "timeout",
                json!({"code": "timeout", "message": "x", "detail": "x"}),
            )),
            1,
            EnvelopeFault::UnknownKey("error.detail".into()),
        ),
        row(
            "error-two-extra-keys-smallest-first",
            r#"{"schema_version":1,"ok":false,"data":null,"error":{"code":"timeout","message":"x","zz":1,"aa":2}}
"#,
            1,
            EnvelopeFault::UnknownKey("error.aa".into()),
        ),
    ]
}

/// The `code` and the `message` of the error, and the exit code of the code's class.
fn code_and_message_rows() -> Vec<Row> {
    vec![
        row(
            "code-not-kebab",
            line(&with_code(json!("Pane_Not_Found"))),
            1,
            EnvelopeFault::CodeNotKebab("Pane_Not_Found".into()),
        ),
        row(
            "code-empty",
            line(&with_code(json!(""))),
            1,
            EnvelopeFault::CodeNotKebab(String::new()),
        ),
        row(
            "code-not-string",
            line(&with_code(json!(5))),
            1,
            EnvelopeFault::CodeNotKebab("5".into()),
        ),
        row(
            "two-line-message",
            line(&with_message(json!("line one\nline two"))),
            1,
            EnvelopeFault::MessageNotOneLine,
        ),
        row(
            "cr-message",
            line(&with_message(json!("a\rb"))),
            1,
            EnvelopeFault::MessageNotOneLine,
        ),
        row(
            "empty-message",
            line(&with_message(json!(""))),
            1,
            EnvelopeFault::MessageNotOneLine,
        ),
        row(
            "blank-message",
            line(&with_message(json!("  \t "))),
            1,
            EnvelopeFault::MessageNotOneLine,
        ),
        row(
            "message-not-string",
            line(&with_message(json!(5))),
            1,
            EnvelopeFault::MessageNotOneLine,
        ),
        row(
            "class-mismatch",
            line(&good_failure("timeout")),
            3,
            EnvelopeFault::ClassDisagreesWithExit,
        ),
        row(
            "usage-at-1",
            line(&good_failure("usage")),
            1,
            EnvelopeFault::ClassDisagreesWithExit,
        ),
        row(
            "message-checked-before-class",
            line(&with_message(json!(""))),
            3,
            EnvelopeFault::MessageNotOneLine,
        ),
    ]
}

fn mutant_rows() -> Vec<Row> {
    let mut rows = framing_rows();
    rows.extend(key_rows());
    rows.extend(extra_key_order_rows());
    rows.extend(failure_rows());
    rows.extend(code_and_message_rows());
    rows
}

#[test]
fn every_mutant_is_rejected_with_its_fault() {
    for r in mutant_rows() {
        assert_eq!(
            check_envelope(&r.stdout, r.exit),
            Err(r.fault),
            "mutant `{}` (stdout {:?}, exit {})",
            r.name,
            r.stdout,
            r.exit
        );
    }
}

// ---- AC3: NDJSON -------------------------------------------------------------------

#[test]
fn a_stream_of_ok_lines_at_exit_0_is_accepted() {
    let items = [
        with(&good_success(), "data", json!({"n": 1})),
        with(&good_success(), "data", json!({"n": 2})),
        with(&good_success(), "data", json!({"n": 3})),
    ];
    let got = check_ndjson(&lines(&items), 0).unwrap_or_default();
    assert_eq!(got.len(), 3);
    assert!(got.iter().all(|e| e.ok));
    let data: Vec<Value> = got.into_iter().map(|e| e.data).collect();
    assert_eq!(
        data,
        vec![json!({"n": 1}), json!({"n": 2}), json!({"n": 3})]
    );
}

#[test]
fn a_stream_ending_in_one_failure_is_accepted_at_its_exit_code() {
    let ok = good_success();
    let cases = [
        (
            vec![ok.clone(), ok.clone(), good_failure("timeout")],
            1,
            "timeout",
        ),
        (
            vec![ok, good_failure("pane-not-found")],
            3,
            "pane-not-found",
        ),
        (vec![good_failure("usage")], 2, "usage"),
    ];
    for (items, exit, code) in cases {
        let got = check_ndjson(&lines(&items), exit).unwrap_or_default();
        assert_eq!(got.len(), items.len(), "{code}: one envelope per line");
        let last = got
            .last()
            .and_then(|e| e.error.as_ref().map(|x| x.code.as_str()));
        assert_eq!(last, Some(code), "the last envelope is the failure");
        assert!(got.last().is_some_and(|e| !e.ok));
        assert!(got[..got.len() - 1].iter().all(|e| e.ok));
    }
}

fn stream_rows() -> Vec<Row> {
    let ok = good_success();
    let timeout = good_failure("timeout");
    let refusal = good_failure("pane-not-found");
    let detail = with(&ok, "detail", json!("x"));
    let g = line(&ok);
    vec![
        row("empty-stream", "", 0, EnvelopeFault::EmptyStream),
        row("newline-only", "\n", 0, EnvelopeFault::EmptyStream),
        row(
            "failure-in-the-middle",
            lines(&[ok.clone(), refusal.clone(), ok.clone()]),
            3,
            EnvelopeFault::NotLastFailure,
        ),
        row(
            "failure-then-ok",
            lines(&[refusal.clone(), ok.clone()]),
            0,
            EnvelopeFault::NotLastFailure,
        ),
        row(
            "non-bool-ok-before-the-last",
            lines(&[with(&ok, "ok", json!("true")), ok.clone()]),
            0,
            EnvelopeFault::NotLastFailure,
        ),
        row(
            "failure-last-at-exit-0",
            lines(&[ok.clone(), refusal.clone()]),
            0,
            EnvelopeFault::OkDisagreesWithExit,
        ),
        row(
            "ok-last-at-exit-3",
            lines(&[ok.clone(), ok.clone()]),
            3,
            EnvelopeFault::OkDisagreesWithExit,
        ),
        row(
            "broken-line",
            format!("{g}{{\n{g}"),
            0,
            EnvelopeFault::NotJson,
        ),
        row("blank-line", format!("{g}\n{g}"), 0, EnvelopeFault::NotJson),
        row(
            "detail-line",
            lines(&[detail, ok.clone()]),
            0,
            EnvelopeFault::UnknownKey("detail".into()),
        ),
        row(
            "class-mismatch-on-the-last",
            lines(&[ok.clone(), timeout]),
            3,
            EnvelopeFault::ClassDisagreesWithExit,
        ),
        row(
            "unknown-exit",
            line(&ok),
            5,
            EnvelopeFault::ExitCodeUnknown(5),
        ),
    ]
}

#[test]
fn every_stream_mutant_is_rejected_with_its_fault() {
    for r in stream_rows() {
        assert_eq!(
            check_ndjson(&r.stdout, r.exit),
            Err(r.fault),
            "stream mutant `{}` (stdout {:?}, exit {})",
            r.name,
            r.stdout,
            r.exit
        );
    }
}

// ---- AC2: the tables cover every fault ----------------------------------------------

/// The variant's name. The `match` has no `_` arm on purpose: a variant added later
/// fails to compile here until a row of a table covers it.
fn fault_name(fault: &EnvelopeFault) -> &'static str {
    match fault {
        EnvelopeFault::NotJson => "NotJson",
        EnvelopeFault::TextBefore => "TextBefore",
        EnvelopeFault::TextAfter => "TextAfter",
        EnvelopeFault::NotAnObject => "NotAnObject",
        EnvelopeFault::MissingKey(_) => "MissingKey",
        EnvelopeFault::UnknownKey(_) => "UnknownKey",
        EnvelopeFault::SchemaVersion => "SchemaVersion",
        EnvelopeFault::ExitCodeUnknown(_) => "ExitCodeUnknown",
        EnvelopeFault::OkDisagreesWithExit => "OkDisagreesWithExit",
        EnvelopeFault::ErrorWhileOk => "ErrorWhileOk",
        EnvelopeFault::NoErrorWhenFailed => "NoErrorWhenFailed",
        EnvelopeFault::DataOnFailure => "DataOnFailure",
        EnvelopeFault::CodeNotKebab(_) => "CodeNotKebab",
        EnvelopeFault::MessageNotOneLine => "MessageNotOneLine",
        EnvelopeFault::ClassDisagreesWithExit => "ClassDisagreesWithExit",
        EnvelopeFault::EmptyStream => "EmptyStream",
        EnvelopeFault::NotLastFailure => "NotLastFailure",
    }
}

#[test]
fn the_mutant_tables_cover_every_fault() {
    let covered: BTreeSet<&str> = mutant_rows()
        .iter()
        .chain(stream_rows().iter())
        .map(|r| fault_name(&r.fault))
        .collect();
    let all: BTreeSet<&str> = [
        "NotJson",
        "TextBefore",
        "TextAfter",
        "NotAnObject",
        "MissingKey",
        "UnknownKey",
        "SchemaVersion",
        "ExitCodeUnknown",
        "OkDisagreesWithExit",
        "ErrorWhileOk",
        "NoErrorWhenFailed",
        "DataOnFailure",
        "CodeNotKebab",
        "MessageNotOneLine",
        "ClassDisagreesWithExit",
        "EmptyStream",
        "NotLastFailure",
    ]
    .into_iter()
    .collect();
    assert_eq!(all.len(), 17);
    assert_eq!(covered, all);
}
