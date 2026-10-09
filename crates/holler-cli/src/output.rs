//! What the `holler pane` and `holler profile` verbs print (story #670, epic #633): one module
//! decides it, so every verb answers in one shape and each verb supplies only its data.
//!
//! - **Text mode** (the default): the data of a successful verb goes to `out`; an error goes to
//!   `err` as `error: <message>`, with nothing on `out`.
//! - **JSON mode** (`--format=json`, or `--json`): exactly one envelope goes to `out` per result,
//!   an error included, and nothing goes to `err`. `pane watch` is the one stream: one envelope
//!   per line ([`emit_stream`]).
//!
//! The exit code is the same in both modes: `0` ok, `1` refused or failed, `2` usage. An error
//! coded `usage` exits 2 however it got here, so a run-time guard of `holler-pane` (a bad pane
//! name, `--command-json` that is not JSON) is a usage error and not a runtime failure.
//!
//! Writing goes through an injected [`Sink`], so a test captures both streams. Nothing here exits
//! the process: every function returns the code and `main.rs`, the one file allowed to exit,
//! applies it (`scripts/lint.sh` check 2).
//!
//! A write that fails (a closed pipe) turns an exit 0 into 1 and ends a stream, so a verb never
//! reports success for output nobody received.

use std::io::{self, Write};

use clap::error::ErrorKind;
use clap::ValueEnum;
use holler_pane::error::is_valid_code;
use holler_pane::{PaneError, Ports};
use serde::Serialize;

/// The envelope's `schema_version` (epic #633, "Output"). A change to the envelope's shape bumps it.
pub const SCHEMA_VERSION: u32 = 1;

/// The two subcommands whose usage errors are envelopes in JSON mode (decision 1(d) of #670).
/// Every other verb keeps its own usage-error output, as ADR 0003 requires.
const ENVELOPE_NAMESPACES: [&str; 2] = ["pane", "profile"];

/// The output format: the global `--format`, and what `--json` means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Human-readable text (the default).
    Text,
    /// One JSON envelope per result.
    Json,
}

/// How the global `--format` and `--json` resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatChoice {
    /// The format every verb prints in.
    pub format: Format,
    /// `--format=json` was given itself (`--json` alone does not set it). `roster` prints the
    /// envelope only for this, so `roster --json` keeps its legacy shape (ADR 0003: the `--json`
    /// shape is part of the surface); the `pane` and `profile` verbs treat either flag the same.
    pub json_explicit: bool,
}

/// Resolve the global `--json` and `--format` once, after parsing.
///
/// `--format=json` is `--json`; `--json` with `--format=text` contradict each other and are a
/// usage error (exit 2).
pub fn resolve_format(json: bool, format: Option<Format>) -> Result<FormatChoice, ErrorBody> {
    match (json, format) {
        (true, Some(Format::Text)) => {
            Err(usage_body("--json cannot be combined with --format=text"))
        }
        (_, Some(format)) => Ok(FormatChoice {
            format,
            json_explicit: format == Format::Json,
        }),
        (true, None) => Ok(FormatChoice {
            format: Format::Json,
            json_explicit: false,
        }),
        (false, None) => Ok(FormatChoice {
            format: Format::Text,
            json_explicit: false,
        }),
    }
}

/// A stable kebab-case error code: `^[a-z]+(-[a-z]+)*$`, checked by the one validator of the
/// workspace, `holler_pane::error::is_valid_code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ErrorCode(String);

/// The text that is not a valid [`ErrorCode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidCode(String);

impl std::fmt::Display for InvalidCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} is not a kebab-case error code", self.0)
    }
}

impl std::error::Error for InvalidCode {}

impl ErrorCode {
    /// A code from its text, if it is well-formed.
    pub fn new(code: &str) -> Result<Self, InvalidCode> {
        if is_valid_code(code) {
            Ok(Self(code.to_owned()))
        } else {
            Err(InvalidCode(code.to_owned()))
        }
    }

    /// The code, as it appears in the envelope.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&PaneError> for ErrorCode {
    /// A `PaneError` always has a valid code, so this cannot fail.
    fn from(error: &PaneError) -> Self {
        Self(error.code().to_owned())
    }
}

/// The `error` member of a failed envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorBody {
    /// The stable code a script matches on.
    pub code: ErrorCode,
    /// One line for a person. In JSON mode [`emit`] puts it on one line.
    pub message: String,
}

impl From<&PaneError> for ErrorBody {
    fn from(error: &PaneError) -> Self {
        Self {
            code: ErrorCode::from(error),
            message: error.to_string(),
        }
    }
}

/// An `ErrorBody` coded `usage`.
fn usage_body(message: &str) -> ErrorBody {
    ErrorBody::from(&PaneError::Usage {
        message: message.to_owned(),
    })
}

/// The one envelope of every `holler pane` and `holler profile` result in JSON mode.
///
/// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
/// `ok` is false and `error` is `null` when it is true.
#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
}

impl<T> Envelope<T> {
    /// A successful result.
    pub fn success(data: T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// A failed result.
    pub fn failure(error: ErrorBody) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

/// The two writers a verb prints to: the real `main` builds it over stdout and stderr, a test
/// over buffers.
pub struct Sink<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// What a `holler pane` or `holler profile` verb runs with.
///
/// `ports` is held by value: [`Ports`] is `Copy`. A verb takes `&mut VerbCtx` because writing to
/// the sink needs a mutable borrow.
pub struct VerbCtx<'a> {
    pub format: Format,
    pub ports: Ports<'a>,
    pub sink: Sink<'a>,
}

/// Print one result and return the exit code: 0 ok, 1 error, 2 for an error coded `usage`.
///
/// `text` renders the data for text mode (JSON mode serializes the data itself and never calls
/// it). The text ends with one newline, which `emit` adds when it is missing; empty text writes
/// nothing.
pub fn emit<T: Serialize>(
    sink: &mut Sink<'_>,
    format: Format,
    result: Result<T, ErrorBody>,
    text: impl FnOnce(&T) -> String,
) -> i32 {
    match format {
        Format::Text => emit_text(sink, result, text),
        Format::Json => emit_json(sink, result),
    }
}

/// Print a stream of results, one line each, and return the exit code.
///
/// Text mode writes `text(item)` per item; JSON mode writes one envelope per line (NDJSON, which is
/// what `pane watch` prints). Every line is flushed as it is written. The stream ends at the first
/// error item, which is reported the way [`emit`] reports an error, and at the first write that
/// fails; lines already written stay written.
pub fn emit_stream<T: Serialize>(
    sink: &mut Sink<'_>,
    format: Format,
    items: impl Iterator<Item = Result<T, ErrorBody>>,
    text: impl Fn(&T) -> String,
) -> i32 {
    for item in items {
        let code = emit(sink, format, item, &text);
        if code != 0 {
            return code;
        }
    }
    0
}

/// Print an error (a failed `emit` with no data) and return its exit code.
pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
    emit(sink, format, Err::<(), _>(error), |()| String::new())
}

/// Print a usage error and return 2. Text mode writes `error: <message>` to `err`; JSON mode
/// writes an envelope coded `usage`.
pub fn emit_usage_error(sink: &mut Sink<'_>, format: Format, message: &str) -> i32 {
    emit_error(sink, format, usage_body(message))
}

/// What a verb says until its story lands: `not implemented (story #N)`. The one wording of the
/// stubs and of the `--pane` and `--profile` refusals of the legacy verbs.
pub fn not_implemented_message(story: u32) -> String {
    format!("not implemented (story #{story})")
}

/// The refusal of a stub verb: code `not-implemented`, naming the story that owns the verb.
pub fn not_implemented(story: u32) -> ErrorBody {
    ErrorBody {
        code: ErrorCode::from(&PaneError::NotImplemented),
        message: not_implemented_message(story),
    }
}

fn emit_text<T>(
    sink: &mut Sink<'_>,
    result: Result<T, ErrorBody>,
    text: impl FnOnce(&T) -> String,
) -> i32 {
    match result {
        Ok(data) => {
            let rendered = text(&data);
            let written = if rendered.is_empty() {
                Ok(())
            } else {
                write_line(sink.out, &rendered)
            };
            settle(written, 0)
        }
        Err(error) => {
            let written = write_line(sink.err, &format!("error: {}", error.message));
            settle(written, exit_code(&error))
        }
    }
}

fn emit_json<T: Serialize>(sink: &mut Sink<'_>, result: Result<T, ErrorBody>) -> i32 {
    match result {
        Ok(data) => settle(write_envelope(sink, &Envelope::success(data)), 0),
        Err(error) => {
            let code = exit_code(&error);
            let error = ErrorBody {
                message: one_line(&error.message),
                ..error
            };
            settle(write_envelope(sink, &Envelope::<()>::failure(error)), code)
        }
    }
}

/// Write one envelope as one compact line. Data that cannot be encoded is reported on `err` and
/// fails the write, so no half-written envelope reaches `out`.
fn write_envelope<T: Serialize>(sink: &mut Sink<'_>, envelope: &Envelope<T>) -> io::Result<()> {
    match serde_json::to_string(envelope) {
        Ok(line) => write_line(sink.out, &line),
        Err(cause) => {
            write_line(
                sink.err,
                &format!("error: cannot encode the result: {cause}"),
            )?;
            Err(io::Error::other(cause))
        }
    }
}

/// Write `line` and end it with a newline (unless it has one), then flush.
fn write_line(to: &mut dyn Write, line: &str) -> io::Result<()> {
    to.write_all(line.as_bytes())?;
    if !line.ends_with('\n') {
        to.write_all(b"\n")?;
    }
    to.flush()
}

/// The exit code of a verb that wrote `written`: `code`, except that output nobody received
/// cannot be a success.
fn settle(written: io::Result<()>, code: i32) -> i32 {
    if written.is_err() && code == 0 {
        1
    } else {
        code
    }
}

/// The exit code of an error: 2 for `usage`, 1 for every other.
fn exit_code(error: &ErrorBody) -> i32 {
    if error.code
        == ErrorCode::from(&PaneError::Usage {
            message: String::new(),
        })
    {
        2
    } else {
        1
    }
}

/// `message` on one line, as the envelope requires: each line break becomes a space.
fn one_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the raw command line says, for a parse that failed: a failed parse has no parsed flags, so
/// the choice between an envelope and clap's own message is made from the arguments themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawArgs<'a> {
    /// The first token that is neither a flag nor a flag's value: the subcommand the user meant.
    pub namespace: Option<&'a str>,
    /// `--json`, `--format=json` or `--format json` is on the line.
    pub json: bool,
}

impl<'a> RawArgs<'a> {
    /// The namespace whose usage error is an envelope: `pane` or `profile`, in JSON mode.
    /// Every other verb keeps its own usage-error output (decision 1(d) of #670).
    pub fn envelope_namespace(&self) -> Option<&'a str> {
        self.namespace
            .filter(|ns| self.json && ENVELOPE_NAMESPACES.contains(ns))
    }
}

/// Scan a command line (the program name first) for [`RawArgs`].
///
/// `value_flags` are the global flags that take a value (`--debug`, `--log-format`, `--format`),
/// so that a value such as the `pane` of `--debug pane` is not mistaken for the namespace. The
/// caller derives them from the clap tree, so a new global flag cannot be missed.
pub fn scan_args<'a>(argv: &'a [String], value_flags: &[String]) -> RawArgs<'a> {
    let mut raw = RawArgs {
        namespace: None,
        json: false,
    };
    let mut tokens = argv.iter().skip(1).map(String::as_str);
    while let Some(token) = tokens.next() {
        match classify(token, value_flags) {
            Token::EndOfFlags => break,
            Token::Json => raw.json = true,
            Token::Format(value) => raw.json |= value == "json",
            Token::FormatNext => raw.json |= tokens.next() == Some("json"),
            Token::ValueNext => {
                tokens.next();
            }
            Token::Flag => {}
            Token::Word => {
                raw.namespace.get_or_insert(token);
            }
        }
    }
    raw
}

/// One command-line token, as far as [`scan_args`] needs to tell.
enum Token<'a> {
    /// `--`: everything after it is a value.
    EndOfFlags,
    /// `--json`.
    Json,
    /// `--format=VALUE`.
    Format(&'a str),
    /// `--format`, whose value is the next token.
    FormatNext,
    /// A global flag that takes a value; the value is the next token.
    ValueNext,
    /// Any other flag (`--x=v` carries its own value, and a flag with none has nothing to skip).
    Flag,
    /// A word: a subcommand, a positional, or a value.
    Word,
}

fn classify<'a>(token: &'a str, value_flags: &[String]) -> Token<'a> {
    if token == "--" {
        return Token::EndOfFlags;
    }
    if token == "--json" {
        return Token::Json;
    }
    if let Some(value) = token.strip_prefix("--format=") {
        return Token::Format(value);
    }
    if token == "--format" {
        return Token::FormatNext;
    }
    if value_flags.iter().any(|flag| flag == token) {
        return Token::ValueNext;
    }
    if token.starts_with('-') {
        Token::Flag
    } else {
        Token::Word
    }
}

/// The message of a clap usage error on one line, for the envelope of `namespace` (`pane` or
/// `profile`): clap's reason without the `error: ` prefix, the `Usage:` block and the closing
/// hint, joined with spaces. A bare namespace gets a message of its own, because clap answers it
/// with the whole help text.
pub fn usage_message(error: &clap::Error, namespace: &str) -> String {
    match error.kind() {
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand | ErrorKind::MissingSubcommand => {
            format!("a subcommand is required: run `holler {namespace} --help` to list them")
        }
        _ => flatten(&error.to_string()),
    }
}

fn flatten(rendered: &str) -> String {
    let reason = rendered
        .split("\n\nUsage:")
        .next()
        .unwrap_or(rendered)
        .trim();
    let reason = reason.strip_prefix("error: ").unwrap_or(reason);
    let lines: Vec<&str> = reason
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("For more information"))
        .collect();
    lines.join(" ")
}
