//! The JSON-envelope checker that the `--format=json` tests of every `holler pane` and
//! `holler profile` verb reuse (ADR-0021 section 9).
//!
//! - [`check_envelope`] takes a verb's stdout and exit code. It returns the one envelope
//!   on stdout, or an [`EnvelopeFault`] naming the first rule the output breaks.
//! - [`check_ndjson`] does the same for an NDJSON stream (`pane watch`): one envelope per
//!   line, and only the last line may be a failure.
//!
//! [`check_envelope`] checks these rules in this order. The first rule broken is the
//! fault, so output that breaks two rules always gets the same fault.
//!
//! 0. The exit code is 0, 1, 2 or 3. This is checked before stdout is read.
//! 1. Stdout is one JSON value, followed by at most one `\n`. A value that starts at the
//!    first byte and is followed by anything else (a `\r`, a blank line, a second
//!    envelope) is `TextAfter`. Otherwise, output that does not start with `{` but has a
//!    later `{` that begins valid JSON (a log line or a space before the envelope) is
//!    `TextBefore`. Anything else (empty, blank, truncated, not JSON) is `NotJson`.
//! 2. The value is an object.
//! 3. It has the keys `schema_version`, `ok`, `data` and `error`. The first missing one,
//!    in that order, is named.
//! 4. It has no other key. The smallest extra key is named. Map order is not used: it
//!    differs between builds, because `serde_json/preserve_order` (on in a `--workspace`
//!    build) keeps document order.
//! 5. `schema_version` is the JSON integer 1.
//! 6. `ok` is the boolean `true` when the exit code is 0, and `false` otherwise.
//! 7. On success, `error` is `null`. `data` may be any value, `null` included.
//! 8. On failure, `error` is an object.
//! 9. On failure, `data` is `null`.
//! 10. `error` has exactly the keys `code` and `message`. A missing one is named first
//!     (`error.code`, then `error.message`), then the smallest extra one (`error.<key>`).
//! 11. `code` is a string that [`is_valid_code`] accepts.
//! 12. `message` is a string that is not blank and has no `\n` or `\r`.
//! 13. The exit code matches the class of the code, `class_of(code).exit_code()`.
//!
//! [`check_ndjson`] checks the exit code first, then drops one final `\n`. If nothing is
//! left, the stream is empty. Otherwise it checks each line alone with
//! [`check_envelope`], in order, and the first fault wins, so a blank line is `NotJson`.
//! A line before the last is checked at exit 0, and one whose `ok` is not `true` is
//! `NotLastFailure`. The last line is checked at the process's exit code, so with an exit
//! code other than 0 it must be the failure, as the CLI's `emit_stream` writes it.
//!
//! The checker parses with `serde_json` alone: the testkit cannot depend on the CLI's own
//! `Envelope` type (ADR-0021 section 5). It classifies a code with [`class_of`] and keeps
//! no table of codes or exit codes of its own. The key set pins what
//! `holler-cli/src/output.rs` writes today. A field added to the envelope (ADR-0021
//! section 9 allows that at `schema_version` 1) is added to this checker's key list in
//! the same change.
//!
//! ```
//! use holler_pane_testkit::envelope::{check_envelope, EnvelopeFault};
//!
//! let stdout = "{\"schema_version\":1,\"ok\":true,\"data\":{},\"error\":null}\n";
//! assert!(check_envelope(stdout, 0).is_ok());
//! assert_eq!(
//!     check_envelope(stdout, 3),
//!     Err(EnvelopeFault::OkDisagreesWithExit)
//! );
//! ```

use std::fmt;

use holler_pane::error::{class_of, is_valid_code};
use serde_json::{Deserializer, Map, Value};

/// The `schema_version` of every envelope.
const SCHEMA_VERSION: u64 = 1;

/// The keys of an envelope, each with the name a fault gives it, in the order a missing
/// one is named.
const ENVELOPE_KEYS: [(&str, &str); 4] = [
    ("schema_version", "schema_version"),
    ("ok", "ok"),
    ("data", "data"),
    ("error", "error"),
];

/// The keys of a failed envelope's `error` object, the same way.
const ERROR_KEYS: [(&str, &str); 2] = [("code", "error.code"), ("message", "error.message")];

/// One envelope that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// Always 1 once checked.
    pub schema_version: u64,
    /// `true` exactly when the exit code is 0.
    pub ok: bool,
    /// The verb's result when `ok`, which may be any JSON value, `null` included. Always
    /// `null` on failure.
    pub data: Value,
    /// The error on failure. `None` exactly when `ok`.
    pub error: Option<EnvelopeError>,
}

/// The `error` member of a failed envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeError {
    /// A kebab-case code whose class agrees with the exit code.
    pub code: String,
    /// One line that is not blank.
    pub message: String,
}

/// The first rule a verb's output breaks. The module doc lists the rules in the order
/// they are checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeFault {
    /// Stdout is not one JSON value: it is empty, blank, truncated or not JSON, or it is
    /// broken JSON that starts with `{`.
    NotJson,
    /// Text comes before the envelope: stdout does not start with `{`, but a later `{`
    /// begins valid JSON (a log line or a space before the envelope).
    TextBefore,
    /// Something other than one final `\n` follows the value: a `\r`, a blank line, a
    /// second envelope or any other text.
    TextAfter,
    /// The value is valid JSON but not an object.
    NotAnObject,
    /// A key is missing: `schema_version`, `ok`, `data` or `error`, or, inside `error`,
    /// `error.code` or `error.message`. The first missing one is named.
    MissingKey(&'static str),
    /// The object has a key it must not have. The smallest extra key is named, written
    /// `error.<key>` inside `error`.
    UnknownKey(String),
    /// `schema_version` is not the JSON integer 1.
    SchemaVersion,
    /// The exit code is not 0, 1, 2 or 3.
    ExitCodeUnknown(i32),
    /// `ok` is not the boolean `true` at exit 0, or not the boolean `false` at any other
    /// exit code.
    OkDisagreesWithExit,
    /// `ok` is true, but `error` is not `null`.
    ErrorWhileOk,
    /// `ok` is false, but `error` is not an object.
    NoErrorWhenFailed,
    /// `ok` is false, but `data` is not `null`.
    DataOnFailure,
    /// `error.code` is not a kebab-case string. The code is named: a string as it is,
    /// any other value by its JSON text.
    CodeNotKebab(String),
    /// `error.message` is not a string, is blank, or contains `\n` or `\r`.
    MessageNotOneLine,
    /// The exit code does not match the class of `error.code` (`class_of(code).exit_code()`).
    ClassDisagreesWithExit,
    /// The NDJSON stream has no line.
    EmptyStream,
    /// A line before the last of an NDJSON stream is not a success. Only the last line
    /// may be the failure.
    NotLastFailure,
}

/// One line that names the rule. A key or a code taken from stdout is quoted, so it
/// prints on one line even when it holds a line break.
impl fmt::Display for EnvelopeFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeFault::NotJson => f.write_str("stdout is not one JSON value"),
            EnvelopeFault::TextBefore => f.write_str("stdout has text before the envelope"),
            EnvelopeFault::TextAfter => f.write_str(
                "stdout has text after the envelope (only one final newline may follow it)",
            ),
            EnvelopeFault::NotAnObject => f.write_str("the envelope is not a JSON object"),
            EnvelopeFault::MissingKey(key) => write!(f, "the envelope has no {key:?} key"),
            EnvelopeFault::UnknownKey(key) => {
                write!(f, "the envelope has the unknown key {key:?}")
            }
            EnvelopeFault::SchemaVersion => f.write_str("schema_version is not the integer 1"),
            EnvelopeFault::ExitCodeUnknown(code) => {
                write!(f, "exit code {code} is not 0, 1, 2 or 3")
            }
            EnvelopeFault::OkDisagreesWithExit => f.write_str(
                "ok does not match the exit code (true at exit 0, false at any other exit code)",
            ),
            EnvelopeFault::ErrorWhileOk => f.write_str("error is not null although ok is true"),
            EnvelopeFault::NoErrorWhenFailed => {
                f.write_str("error is not an object although ok is false")
            }
            EnvelopeFault::DataOnFailure => f.write_str("data is not null although ok is false"),
            EnvelopeFault::CodeNotKebab(code) => {
                write!(f, "error.code {code:?} is not a kebab-case code")
            }
            EnvelopeFault::MessageNotOneLine => {
                f.write_str("error.message is not a single non-blank line")
            }
            EnvelopeFault::ClassDisagreesWithExit => {
                f.write_str("the exit code does not match the class of error.code")
            }
            EnvelopeFault::EmptyStream => f.write_str("the NDJSON stream has no envelope"),
            EnvelopeFault::NotLastFailure => {
                f.write_str("a line before the last of the NDJSON stream is not a success")
            }
        }
    }
}

impl std::error::Error for EnvelopeFault {}

/// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
/// code of the verb that wrote it.
///
/// Returns the envelope, or the fault of the first rule it breaks, in the order the
/// module doc lists.
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
    check_exit_code(exit_code)?;
    let Value::Object(object) = frame(stdout)? else {
        return Err(EnvelopeFault::NotAnObject);
    };
    let [schema_version, ok, data, error] = exact_members(object, ENVELOPE_KEYS, "")?;
    if schema_version.as_u64() != Some(SCHEMA_VERSION) {
        return Err(EnvelopeFault::SchemaVersion);
    }
    let succeeded = exit_code == 0;
    if ok.as_bool() != Some(succeeded) {
        return Err(EnvelopeFault::OkDisagreesWithExit);
    }
    if succeeded {
        return if error.is_null() {
            Ok(Envelope {
                schema_version: SCHEMA_VERSION,
                ok: true,
                data,
                error: None,
            })
        } else {
            Err(EnvelopeFault::ErrorWhileOk)
        };
    }
    let error = check_failure(error, &data, exit_code)?;
    Ok(Envelope {
        schema_version: SCHEMA_VERSION,
        ok: false,
        data: Value::Null,
        error: Some(error),
    })
}

/// Checks that `stdout` is an NDJSON stream of envelopes, one per line, that agrees with
/// `exit_code`: every line but the last is a success, and the last line is the failure
/// when `exit_code` is not 0.
///
/// Returns every line's envelope, in order, or the first fault.
pub fn check_ndjson(stdout: &str, exit_code: i32) -> Result<Vec<Envelope>, EnvelopeFault> {
    check_exit_code(exit_code)?;
    let lines: Vec<&str> = match stdout.strip_suffix('\n').unwrap_or(stdout) {
        "" => Vec::new(),
        body => body.split('\n').collect(),
    };
    let Some((last, before)) = lines.split_last() else {
        return Err(EnvelopeFault::EmptyStream);
    };
    let mut envelopes = before
        .iter()
        .copied()
        .map(check_line_before_last)
        .collect::<Result<Vec<_>, _>>()?;
    envelopes.push(check_envelope(last, exit_code)?);
    Ok(envelopes)
}

/// Rule 0: `exit_code` is one a verb ends with: 0 ok, 1 runtime failure, 2 usage,
/// 3 refusal (ADR-0021 section 9).
fn check_exit_code(exit_code: i32) -> Result<(), EnvelopeFault> {
    if (0..=3).contains(&exit_code) {
        Ok(())
    } else {
        Err(EnvelopeFault::ExitCodeUnknown(exit_code))
    }
}

/// Rule 1: the one JSON value on `stdout`. One final `\n` is dropped first, and any
/// other text around the value is a fault.
fn frame(stdout: &str) -> Result<Value, EnvelopeFault> {
    let body = stdout.strip_suffix('\n').unwrap_or(stdout);
    // `serde_json` skips whitespace before a value, so a value that starts at the first
    // byte needs a first byte that is not whitespace.
    if body.starts_with(|c: char| !c.is_ascii_whitespace()) {
        let mut values = Deserializer::from_str(body).into_iter::<Value>();
        if let Some(Ok(value)) = values.next() {
            return if values.byte_offset() < body.len() {
                Err(EnvelopeFault::TextAfter)
            } else {
                Ok(value)
            };
        }
    }
    // Broken JSON that starts with `{` is never text before an envelope. `match_indices`
    // gives char boundaries, and `get` cannot panic.
    let text_before = !body.starts_with('{')
        && body
            .match_indices('{')
            .any(|(at, _)| body.get(at..).is_some_and(starts_with_value));
    Err(if text_before {
        EnvelopeFault::TextBefore
    } else {
        EnvelopeFault::NotJson
    })
}

/// Whether a JSON value parses at the start of `text`, whatever follows it.
fn starts_with_value(text: &str) -> bool {
    matches!(
        Deserializer::from_str(text).into_iter::<Value>().next(),
        Some(Ok(_))
    )
}

/// Rules 3 and 4 (and 10, for the `error` object): the members of `object` in the order
/// of `keys`, when it has exactly those keys.
///
/// The first key it lacks, in that order, is `MissingKey` with the key's name. Then the
/// smallest key left over is `UnknownKey`, written `<path><key>`. It is the smallest and
/// not the first in map order, because map order is document order in a build with
/// `serde_json/preserve_order` and sorted order in one without it.
fn exact_members<const N: usize>(
    mut object: Map<String, Value>,
    keys: [(&str, &'static str); N],
    path: &str,
) -> Result<[Value; N], EnvelopeFault> {
    if let Some(&(_, name)) = keys.iter().find(|(key, _)| !object.contains_key(*key)) {
        return Err(EnvelopeFault::MissingKey(name));
    }
    // Every key is present (checked just above), so the `null` default is never used.
    let members = keys.map(|(key, _)| object.remove(key).unwrap_or_default());
    match object.keys().min() {
        Some(extra) => Err(EnvelopeFault::UnknownKey(format!("{path}{extra}"))),
        None => Ok(members),
    }
}

/// Rules 8 to 13: the `error` member of a failed envelope, checked with its `data` and
/// the exit code.
fn check_failure(
    error: Value,
    data: &Value,
    exit_code: i32,
) -> Result<EnvelopeError, EnvelopeFault> {
    let Value::Object(error) = error else {
        return Err(EnvelopeFault::NoErrorWhenFailed);
    };
    if !data.is_null() {
        return Err(EnvelopeFault::DataOnFailure);
    }
    let [code, message] = exact_members(error, ERROR_KEYS, "error.")?;
    let code = kebab_code(code)?;
    let message = one_line_message(message)?;
    if class_of(&code).exit_code() != exit_code {
        return Err(EnvelopeFault::ClassDisagreesWithExit);
    }
    Ok(EnvelopeError { code, message })
}

/// Rule 11: `code` as a string that [`is_valid_code`] accepts. Any other code is named
/// in the fault: a string as it is, any other value by its JSON text.
fn kebab_code(code: Value) -> Result<String, EnvelopeFault> {
    match code {
        Value::String(code) if is_valid_code(&code) => Ok(code),
        Value::String(code) => Err(EnvelopeFault::CodeNotKebab(code)),
        other => Err(EnvelopeFault::CodeNotKebab(other.to_string())),
    }
}

/// Rule 12: `message` as a string that is not blank and has no `\n` or `\r`.
fn one_line_message(message: Value) -> Result<String, EnvelopeFault> {
    match message {
        Value::String(text) if !text.trim().is_empty() && !text.contains(['\n', '\r']) => Ok(text),
        _ => Err(EnvelopeFault::MessageNotOneLine),
    }
}

/// A line of an NDJSON stream before the last. It is checked at exit 0, because only the
/// last line may be the failure, and an `ok` that is not `true` there is `NotLastFailure`.
fn check_line_before_last(line: &str) -> Result<Envelope, EnvelopeFault> {
    check_envelope(line, 0).map_err(|fault| match fault {
        EnvelopeFault::OkDisagreesWithExit => EnvelopeFault::NotLastFailure,
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: &str = "{\"schema_version\":1,\"ok\":true,\"data\":{},\"error\":null}";

    #[test]
    fn exit_code_is_checked_before_stdout() {
        assert_eq!(
            check_envelope("{", 7),
            Err(EnvelopeFault::ExitCodeUnknown(7))
        );
        assert_eq!(check_ndjson("", 7), Err(EnvelopeFault::ExitCodeUnknown(7)));
    }

    #[test]
    fn only_one_final_newline_is_stripped() {
        assert!(check_envelope(&format!("{G}\n"), 0).is_ok());
        assert_eq!(
            check_envelope(&format!("{G}\r\n"), 0),
            Err(EnvelopeFault::TextAfter)
        );
        assert_eq!(
            check_envelope(&format!("{G}\n\n"), 0),
            Err(EnvelopeFault::TextAfter)
        );
    }

    #[test]
    fn broken_json_that_starts_with_a_brace_is_not_text_before() {
        assert_eq!(
            check_envelope("{\"a\": {\"b\":1}", 0),
            Err(EnvelopeFault::NotJson)
        );
        assert_eq!(
            check_envelope(&format!("{{partial\n{G}\n"), 0),
            Err(EnvelopeFault::NotJson)
        );
        assert_eq!(
            check_envelope(&format!("x{G}\n"), 0),
            Err(EnvelopeFault::TextBefore)
        );
        assert_eq!(check_envelope("   ", 0), Err(EnvelopeFault::NotJson));
        assert_eq!(check_envelope("null", 0), Err(EnvelopeFault::NotAnObject));
    }

    #[test]
    fn every_fault_displays_as_one_line() {
        // The two payloads that come straight from stdout carry a newline (A's W-3), so
        // the one-line rule is pinned for any value, not just a harmless one.
        let faults = [
            EnvelopeFault::NotJson,
            EnvelopeFault::TextBefore,
            EnvelopeFault::TextAfter,
            EnvelopeFault::NotAnObject,
            EnvelopeFault::MissingKey("error.code"),
            EnvelopeFault::UnknownKey("a\nb".to_owned()),
            EnvelopeFault::SchemaVersion,
            EnvelopeFault::ExitCodeUnknown(4),
            EnvelopeFault::OkDisagreesWithExit,
            EnvelopeFault::ErrorWhileOk,
            EnvelopeFault::NoErrorWhenFailed,
            EnvelopeFault::DataOnFailure,
            EnvelopeFault::CodeNotKebab("a\r\nb".to_owned()),
            EnvelopeFault::MessageNotOneLine,
            EnvelopeFault::ClassDisagreesWithExit,
            EnvelopeFault::EmptyStream,
            EnvelopeFault::NotLastFailure,
        ];
        assert_eq!(faults.len(), 17);
        for fault in faults {
            let text = fault.to_string();
            assert!(!text.is_empty(), "{fault:?} displays as nothing");
            assert!(
                !text.contains('\n') && !text.contains('\r'),
                "{fault:?} displays on more than one line: {text:?}"
            );
        }
    }
}
