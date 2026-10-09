//! The in-process harness for the pane and profile verbs (story #670).
//!
//! Included by `pane_verbs` and `profile_verbs` through `#[path]`. It runs one verb
//! **in this process** over the stub wiring's `Ports` and two captured writers, so a
//! verb story tests its routing (text goes to `err`/`out`, JSON is one envelope on
//! `out`) without a subprocess. Subprocess helpers stay in `tests/support/`; the
//! envelope checks here are deliberately minimal (a `serde_json` parse plus
//! `schema_version`, `ok` and `error.code`), and the conformance helper of #638 is
//! what later verb stories use for full envelope validation.

use clap::Parser;
use holler_cli::output::{Format, Sink, VerbCtx};
use holler_cli::pane::wiring::Wiring;
use holler_cli::{pane, profile, Cli, Command};
use serde_json::Value;

/// What one verb run produced.
#[derive(Debug)]
pub struct Outcome {
    pub code: i32,
    pub out: String,
    pub err: String,
}

/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and the stub wiring's ports.
pub fn run_verb(argv: &[&str], format: Format) -> Outcome {
    let mut full = vec!["holler"];
    full.extend_from_slice(argv);
    let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{full:?} must parse: {e}"));
    let wiring = Wiring::connect().expect("the stub wiring connects without a hub");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = {
        let mut ctx = VerbCtx {
            format,
            ports: wiring.ports(),
            sink: Sink {
                out: &mut out,
                err: &mut err,
            },
        };
        match &cli.command {
            Command::Pane(cmd) => pane::run(cmd, &mut ctx),
            Command::Profile(cmd) => profile::run(cmd, &mut ctx),
            other => panic!("not a pane or profile verb: {other:?}"),
        }
    };
    Outcome {
        code,
        out: String::from_utf8(out).expect("out is UTF-8"),
        err: String::from_utf8(err).expect("err is UTF-8"),
    }
}

/// The one envelope in `out`: exactly one line, a JSON object. Panics otherwise.
pub fn one_envelope(out: &str) -> Value {
    assert!(
        out.ends_with('\n'),
        "an envelope line ends with a newline: {out:?}"
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one envelope line on out, got {out:?}"
    );
    let value: Value =
        serde_json::from_str(lines[0]).unwrap_or_else(|e| panic!("{:?} is JSON: {e}", lines[0]));
    assert!(value.is_object(), "an envelope is a JSON object: {value}");
    value
}

/// A stub verb's routing, in both formats (the seam test of #670, one verb at a time).
///
/// `argv` is the verb after `holler` (`["pane", "list"]`); `story` is the issue number
/// the stub names as its owner. Text mode writes the refusal to `err` and nothing to
/// `out`; JSON mode writes exactly one `not-implemented` envelope to `out` and nothing
/// to `err`; both exit 1.
pub fn assert_stub_routes(argv: &[&str], story: u32) {
    let phrase = format!("not implemented (story #{story})");

    let text = run_verb(argv, Format::Text);
    assert_eq!(text.code, 1, "{argv:?} text: a stub exits 1: {text:?}");
    assert!(
        text.out.is_empty(),
        "{argv:?} text: nothing on out: {text:?}"
    );
    assert_eq!(
        text.err.trim_end(),
        format!("error: {phrase}"),
        "{argv:?} text: err holds the one-line refusal: {text:?}"
    );

    let json = run_verb(argv, Format::Json);
    assert_eq!(json.code, 1, "{argv:?} json: a stub exits 1: {json:?}");
    assert!(
        json.err.is_empty(),
        "{argv:?} json: nothing on err: {json:?}"
    );
    let envelope = one_envelope(&json.out);
    assert_eq!(envelope["schema_version"], 1, "{envelope}");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["data"], Value::Null, "{envelope}");
    assert_eq!(envelope["error"]["code"], "not-implemented", "{envelope}");
    let message = envelope["error"]["message"]
        .as_str()
        .expect("error.message is a string");
    assert!(
        message.contains(&phrase),
        "{argv:?} json: message names the story: {message:?}"
    );
    assert!(
        !message.contains('\n'),
        "{argv:?} json: the message is one line: {message:?}"
    );
}
