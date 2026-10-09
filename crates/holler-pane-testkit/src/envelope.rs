//! The JSON-envelope checker every verb's `--format=json` tests reuse: stdout is one
//! envelope (or NDJSON lines of them) with `schema_version` 1, `ok` agreeing with the
//! exit code, `error` null exactly when `ok`, and nothing else. Empty stub declared by
//! #638 so that no two slices edit `lib.rs`; slice b (#681) fills it.
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
