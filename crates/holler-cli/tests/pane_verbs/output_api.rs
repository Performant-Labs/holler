//! The output module's API (`holler_cli::output`, story #670): the one place that
//! writes a verb's result, so every pane and profile verb answers in one shape.
//!
//! Routing under test: text mode writes ok data to `out` and any error to `err`
//! (nothing on `out` for an error); JSON mode writes exactly one envelope to `out`
//! (error envelopes included) and nothing to `err`. Exit codes are the same in both
//! formats: 0 ok, 1 refused or failed, 2 usage (an error coded `usage` exits 2, so a
//! run-time `PaneError::Usage` from a guard does not exit 1).

use holler_cli::output::{emit, emit_stream, emit_usage_error, ErrorBody, ErrorCode, Format, Sink};
use holler_pane::error::{is_valid_code, ALL_CODES};
use holler_pane::PaneError;
use serde_json::{json, Value};

use crate::verb_harness::one_envelope;

/// Run `f` over a fresh sink; returns `(exit code, out, err)`.
fn with_sink(f: impl FnOnce(&mut Sink) -> i32) -> (i32, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = f(&mut Sink {
        out: &mut out,
        err: &mut err,
    });
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn error_body(code: &str, message: &str) -> ErrorBody {
    ErrorBody {
        code: ErrorCode::new(code).expect("a valid code"),
        message: message.to_string(),
    }
}

// --- ErrorCode ---------------------------------------------------------------

/// `ErrorCode::new` accepts exactly what `holler_pane::error::is_valid_code` accepts:
/// the CLI has no second validator (the pane crate's is the only one in the workspace).
#[test]
fn error_code_new_agrees_with_the_one_validator() {
    let mut candidates: Vec<&str> = ALL_CODES.to_vec();
    candidates.extend([
        "",
        "-",
        "a-",
        "-a",
        "a--b",
        "Usage",
        "not_implemented",
        "not implemented",
        "two-words-ok",
        "has1digit",
        "ünï",
        "a-b-c-d",
    ]);
    for code in candidates {
        assert_eq!(
            ErrorCode::new(code).is_ok(),
            is_valid_code(code),
            "ErrorCode::new({code:?}) must agree with is_valid_code"
        );
    }
}

#[test]
fn error_code_new_rejects_an_invalid_code() {
    for bad in [
        "",
        "Not-Implemented",
        "not_implemented",
        "-lead",
        "trail-",
        "double--hyphen",
    ] {
        assert!(
            ErrorCode::new(bad).is_err(),
            "{bad:?} is not a stable kebab-case code"
        );
    }
}

/// A `PaneError` always has a valid code, so the conversion cannot fail.
#[test]
fn error_code_from_pane_error_is_infallible_and_keeps_the_wire_code() {
    let from_variant = ErrorCode::from(&PaneError::NotImplemented);
    assert_eq!(
        serde_json::to_value(&from_variant).unwrap(),
        json!("not-implemented")
    );
    let from_data_variant = ErrorCode::from(&PaneError::Usage {
        message: "x".to_string(),
    });
    assert_eq!(
        serde_json::to_value(&from_data_variant).unwrap(),
        json!("usage")
    );
}

// --- emit --------------------------------------------------------------------

#[test]
fn emit_ok_in_text_mode_writes_the_rendered_text_to_out_only() {
    let (code, out, err) = with_sink(|sink| {
        emit(sink, Format::Text, Ok::<_, ErrorBody>(vec![1, 2, 3]), |v| {
            format!("{} items", v.len())
        })
    });
    assert_eq!(code, 0);
    assert_eq!(out.trim_end_matches('\n'), "3 items");
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn emit_ok_in_json_mode_writes_one_envelope_and_does_not_render_text() {
    let (code, out, err) = with_sink(|sink| {
        emit(
            sink,
            Format::Json,
            Ok::<_, ErrorBody>(json!({"n": 3})),
            |_| panic!("text mode only"),
        )
    });
    assert_eq!(code, 0);
    assert!(err.is_empty(), "{err:?}");
    assert_eq!(
        one_envelope(&out),
        json!({"schema_version": 1, "ok": true, "data": {"n": 3}, "error": null})
    );
}

#[test]
fn emit_error_in_text_mode_writes_the_message_to_err_and_nothing_to_out() {
    let (code, out, err) = with_sink(|sink| {
        emit(
            sink,
            Format::Text,
            Err::<Value, _>(error_body("pane-not-found", "no pane named demo-c1r1")),
            |_| panic!("an error has no data to render"),
        )
    });
    assert_eq!(code, 1);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("no pane named demo-c1r1"), "{err:?}");
}

#[test]
fn emit_error_in_json_mode_writes_one_error_envelope_to_out_and_nothing_to_err() {
    let (code, out, err) = with_sink(|sink| {
        emit(
            sink,
            Format::Json,
            Err::<Value, _>(error_body("pane-not-found", "no pane named demo-c1r1")),
            |_| panic!("text mode only"),
        )
    });
    assert_eq!(code, 1);
    assert!(err.is_empty(), "{err:?}");
    assert_eq!(
        one_envelope(&out),
        json!({
            "schema_version": 1,
            "ok": false,
            "data": null,
            "error": {"code": "pane-not-found", "message": "no pane named demo-c1r1"}
        })
    );
}

/// The envelope's key order is part of the contract (`schema_version` first), and the
/// line is compact: a reader that scans the first bytes must not need a JSON parser.
#[test]
fn emit_json_envelope_is_compact_with_schema_version_first() {
    let (_, out, _) = with_sink(|sink| {
        emit(
            sink,
            Format::Json,
            Err::<Value, _>(error_body("timeout", "took too long")),
            |_| String::new(),
        )
    });
    assert_eq!(
        out.trim_end(),
        r#"{"schema_version":1,"ok":false,"data":null,"error":{"code":"timeout","message":"took too long"}}"#
    );
}

/// An error coded `usage` exits 2 in both formats, however it reached `emit` (a
/// run-time `PaneError::Usage` from `PaneName::parse` or `Argv::from_json`, not only a
/// clap error); every other error exits 1.
#[test]
fn emit_exits_2_for_a_usage_coded_error_in_both_formats_and_1_for_any_other() {
    for format in [Format::Text, Format::Json] {
        let usage = ErrorBody::from(&PaneError::Usage {
            message: "not valid JSON".to_string(),
        });
        let (code, _, _) =
            with_sink(|sink| emit(sink, format, Err::<Value, _>(usage), |_| String::new()));
        assert_eq!(code, 2, "{format:?}: a usage error exits 2");

        let (code, _, _) = with_sink(|sink| {
            emit(
                sink,
                format,
                Err::<Value, _>(error_body("probe-failed", "x")),
                |_| String::new(),
            )
        });
        assert_eq!(code, 1, "{format:?}: any other error exits 1");
    }
}

// --- emit_usage_error --------------------------------------------------------

#[test]
fn emit_usage_error_returns_2_and_routes_by_format() {
    let message = "the following required arguments were not provided: --profile <PROFILE>";

    let (code, out, err) = with_sink(|sink| emit_usage_error(sink, Format::Text, message));
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains(message), "{err:?}");

    let (code, out, err) = with_sink(|sink| emit_usage_error(sink, Format::Json, message));
    assert_eq!(code, 2);
    assert!(err.is_empty(), "{err:?}");
    let envelope = one_envelope(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(envelope["error"]["code"], "usage");
    assert_eq!(envelope["error"]["message"], message);
}

// --- emit_stream -------------------------------------------------------------

#[test]
fn emit_stream_writes_one_line_per_item_in_text_mode() {
    let items = vec![Ok::<_, ErrorBody>("alpha"), Ok("beta"), Ok("gamma")];
    let (code, out, err) =
        with_sink(|sink| emit_stream(sink, Format::Text, items.into_iter(), |s| s.to_uppercase()));
    assert_eq!(code, 0);
    assert!(err.is_empty(), "{err:?}");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["ALPHA", "BETA", "GAMMA"]);
}

/// `pane watch` in JSON mode is NDJSON: one complete envelope per line.
#[test]
fn emit_stream_writes_one_envelope_per_line_in_json_mode() {
    let items = vec![Ok::<_, ErrorBody>(json!({"seq": 1})), Ok(json!({"seq": 2}))];
    let (code, out, err) =
        with_sink(|sink| emit_stream(sink, Format::Json, items.into_iter(), |_| String::new()));
    assert_eq!(code, 0);
    assert!(err.is_empty(), "{err:?}");
    let lines: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        lines,
        [
            json!({"schema_version": 1, "ok": true, "data": {"seq": 1}, "error": null}),
            json!({"schema_version": 1, "ok": true, "data": {"seq": 2}, "error": null}),
        ]
    );
}

/// An error item ends the stream's exit code at 1 and is reported the way `emit`
/// reports an error; the items written before it stay written.
#[test]
fn emit_stream_exits_1_on_an_error_item_and_keeps_the_earlier_lines() {
    let items = vec![
        Ok::<_, ErrorBody>("alpha"),
        Err(error_body("unavailable", "the feed closed")),
    ];
    let (code, out, err) =
        with_sink(|sink| emit_stream(sink, Format::Text, items.into_iter(), |s| s.to_string()));
    assert_eq!(code, 1);
    assert_eq!(out.lines().collect::<Vec<_>>(), ["alpha"]);
    assert!(err.contains("the feed closed"), "{err:?}");
}
