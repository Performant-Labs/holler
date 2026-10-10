//! The error taxonomy of pane control (epic #633, decision 2 of #637).
//!
//! One closed set of kebab-case codes, one validator ([`is_valid_code`], the only
//! one in the workspace), and an open [`PaneError::Refused`] variant for the codes
//! an adapter or a verb owns.
//!
//! - **Closed codes** have one [`PaneError`] variant each. The list is closed:
//!   a later story fills behaviour, never the list ([`ALL_CODES`] is exactly the
//!   set below). Each variant's doc names the story that owns it.
//! - **Open codes** travel in [`PaneError::Refused`]. A verb or an adapter declares
//!   the code it owns as a `const` ([`RefusalCode::from_static`]), so an invalid
//!   literal fails the build and the call site needs no `Result`. A code read off
//!   the wire that is not closed lands in `Refused` too (see
//!   [`crate::PaneReply::into_result`]).
//! - **One representation per code**: a closed code is never carried by `Refused`
//!   ([`RefusalCode`] refuses it), so a `match` on the closed variants cannot be
//!   bypassed.
//! - **One exit class per code** (ADR-0021 section 9): [`class_of`] sorts any code
//!   into an [`ErrorClass`] (a usage error, a refusal or a runtime failure), and
//!   [`ErrorClass::exit_code`] is the exit code a `pane` or `profile` verb ends
//!   with (2, 3 or 1). It is the one place that decides which code is a refusal
//!   and which a failure; the CLI calls it, and so will the test kit (#638).
//!
//! Every code and message is plain data and none echoes a secret: the
//! environment-name guards ([`PaneError::ProfileSecretRefused`],
//! [`PaneError::EnvNameInvalid`]) say what was refused, never the offending text.
//!
//! The wire form of an error is [`crate::PaneReply`]; the single optional
//! `detail` string it carries is the payload of the variant (the `what`, `op` or
//! `message` field), so a closed error survives the wire without loss.

use std::borrow::Cow;
use std::fmt;

use serde::de::{self, Deserializer};
use serde::Deserialize;

/// The number of closed codes.
const CODE_COUNT: usize = 22;

/// The closed set of codes as a plain enum: the one table that
/// [`ALL_CODES`], [`PaneError::code`] and the wire parse-back all derive from, so
/// the three cannot drift (the same single-source rule as `holler_proto::Code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneCode {
    NotImplemented,
    Usage,
    GridAmbiguous,
    GridOutOfRange,
    CommandNotArgv,
    EnvNameInvalid,
    GenerationConflict,
    ProbeFailed,
    ProfileConflict,
    ProfileNotFound,
    ProfileExists,
    ProfileHasLivePanes,
    PaneNotInProfile,
    PaneInOtherProfile,
    ProfileSecretRefused,
    HerdrVersionUnsupported,
    Timeout,
    PaneNotFound,
    SessionNotFound,
    StoreCorrupt,
    Unavailable,
    ProfileDrift,
}

impl PaneCode {
    /// Every closed code, in the epic's order.
    pub(crate) const ALL: [PaneCode; CODE_COUNT] = [
        PaneCode::NotImplemented,
        PaneCode::Usage,
        PaneCode::GridAmbiguous,
        PaneCode::GridOutOfRange,
        PaneCode::CommandNotArgv,
        PaneCode::EnvNameInvalid,
        PaneCode::GenerationConflict,
        PaneCode::ProbeFailed,
        PaneCode::ProfileConflict,
        PaneCode::ProfileNotFound,
        PaneCode::ProfileExists,
        PaneCode::ProfileHasLivePanes,
        PaneCode::PaneNotInProfile,
        PaneCode::PaneInOtherProfile,
        PaneCode::ProfileSecretRefused,
        PaneCode::HerdrVersionUnsupported,
        PaneCode::Timeout,
        PaneCode::PaneNotFound,
        PaneCode::SessionNotFound,
        PaneCode::StoreCorrupt,
        PaneCode::Unavailable,
        PaneCode::ProfileDrift,
    ];

    /// The kebab-case wire string of this code.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            PaneCode::NotImplemented => "not-implemented",
            PaneCode::Usage => "usage",
            PaneCode::GridAmbiguous => "grid-ambiguous",
            PaneCode::GridOutOfRange => "grid-out-of-range",
            PaneCode::CommandNotArgv => "command-not-argv",
            PaneCode::EnvNameInvalid => "env-name-invalid",
            PaneCode::GenerationConflict => "generation-conflict",
            PaneCode::ProbeFailed => "probe-failed",
            PaneCode::ProfileConflict => "profile-conflict",
            PaneCode::ProfileNotFound => "profile-not-found",
            PaneCode::ProfileExists => "profile-exists",
            PaneCode::ProfileHasLivePanes => "profile-has-live-panes",
            PaneCode::PaneNotInProfile => "pane-not-in-profile",
            PaneCode::PaneInOtherProfile => "pane-in-other-profile",
            PaneCode::ProfileSecretRefused => "profile-secret-refused",
            PaneCode::HerdrVersionUnsupported => "herdr-version-unsupported",
            PaneCode::Timeout => "timeout",
            PaneCode::PaneNotFound => "pane-not-found",
            PaneCode::SessionNotFound => "session-not-found",
            PaneCode::StoreCorrupt => "store-corrupt",
            PaneCode::Unavailable => "unavailable",
            PaneCode::ProfileDrift => "profile-drift",
        }
    }

    /// The closed code spelled `text`, if there is one.
    pub(crate) fn parse(text: &str) -> Option<PaneCode> {
        PaneCode::ALL.iter().copied().find(|c| c.as_str() == text)
    }

    /// `text` split into a leading closed code and the rest, when it starts with
    /// `"<code>: "` (the convention of the serde errors this crate's guards raise).
    fn split_prefix(text: &str) -> Option<(PaneCode, &str)> {
        PaneCode::ALL.iter().copied().find_map(|c| {
            let rest = text.strip_prefix(c.as_str())?.strip_prefix(": ")?;
            Some((c, rest))
        })
    }
}

/// Every closed code, as its kebab-case wire string, in the epic's order.
///
/// The set is closed: the invariants of this crate (one variant per code, unique,
/// valid under [`is_valid_code`]) are tested against it, and no story edits it.
/// `profile-drift` is listed for convenience: it is a reconcile finding kind
/// (#647/#665 put it in `findings.rs`), not an error a port returns.
pub const ALL_CODES: &[&str] = &CLOSED_CODES;

const CLOSED_CODES: [&str; CODE_COUNT] = {
    let mut codes = [""; CODE_COUNT];
    let mut i = 0;
    while i < CODE_COUNT {
        codes[i] = PaneCode::ALL[i].as_str();
        i += 1;
    }
    codes
};

/// Whether `code` is a well-formed error code: `^[a-z]+(-[a-z]+)*$` (lower-case
/// ASCII words joined by single hyphens, no leading or trailing hyphen).
///
/// This is the only code validator in the workspace; the CLI's output module calls
/// it. It is a `const fn`, so a verb's code constant can be checked at build time.
pub const fn is_valid_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // A leading hyphen is refused the same way as a doubled one: start as if the
    // previous byte had been a hyphen.
    let mut after_hyphen = true;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            if after_hyphen {
                return false;
            }
            after_hyphen = true;
        } else if b.is_ascii_lowercase() {
            after_hyphen = false;
        } else {
            return false;
        }
        i += 1;
    }
    !after_hyphen
}

/// Byte-wise string equality usable in a `const fn`.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether `code` is one of the closed codes.
const fn is_closed_code(code: &str) -> bool {
    let mut i = 0;
    while i < CODE_COUNT {
        if str_eq(PaneCode::ALL[i].as_str(), code) {
            return true;
        }
        i += 1;
    }
    false
}

/// What kind of error a code stands for, which decides the exit code of a `pane` or
/// `profile` verb that ends with it (ADR-0021 section 9). [`class_of`] gives the
/// class of a code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// The request is malformed: a missing or conflicting argument, a bad name, a
    /// params object that does not decode (`usage`). Exit 2.
    Usage,
    /// The request was understood and declined, working as designed, so nothing was
    /// wrong with the system (`pane-in-other-profile`, `grid-ambiguous`, every open
    /// code). Exit 3.
    Refusal,
    /// Something went wrong while doing the work: a timeout, an unreachable hub or
    /// adapter, a conflict between writers (`timeout`, `unavailable`,
    /// `generation-conflict`). Exit 1.
    Failure,
}

impl ErrorClass {
    /// The exit code of a verb that ends with an error of this class, the same in
    /// text and JSON mode: 2 usage, 3 refusal, 1 runtime failure (ADR 0003's exit
    /// codes; success is 0 and has no class).
    pub const fn exit_code(self) -> i32 {
        match self {
            ErrorClass::Usage => 2,
            ErrorClass::Refusal => 3,
            ErrorClass::Failure => 1,
        }
    }
}

/// The class of the error coded `code`: the one place that decides which code is a
/// refusal and which a runtime failure. ADR-0021 section 9 has the table, with the
/// reason for every row. The CLI's output module calls this, and so will the test
/// kit (#638); neither keeps a table of its own.
///
/// It takes the code, not a [`PaneError`], because a code is what an envelope
/// carries. A caller that holds a `PaneError` passes [`PaneError::code`]:
/// `class_of(error.code())`. (It is not `PaneError::classify`, a crate-private
/// helper that only tells a closed code from an open one.)
///
/// - A **closed** code has the class of its arm below. The `match` names every
///   closed code and has no catch-all arm, so a closed code added without a class
///   does not compile.
/// - Any other **well-formed** code is an open code, carried by
///   [`PaneError::Refused`]: a refusal, always. A verb that reports a runtime
///   failure uses a closed failure code.
/// - A **malformed** code is a failure: it cannot be trusted, the same rule that
///   turns a garbled wire reply into `unavailable`.
pub fn class_of(code: &str) -> ErrorClass {
    let Some(closed) = PaneCode::parse(code) else {
        return if is_valid_code(code) {
            ErrorClass::Refusal
        } else {
            ErrorClass::Failure
        };
    };
    match closed {
        PaneCode::Usage => ErrorClass::Usage,
        // Understood and declined: a guard, a policy or a gate said no, the name is
        // taken, or the request named something that does not exist.
        PaneCode::GridAmbiguous
        | PaneCode::GridOutOfRange
        | PaneCode::CommandNotArgv
        | PaneCode::EnvNameInvalid
        | PaneCode::ProfileSecretRefused
        | PaneCode::ProfileExists
        | PaneCode::ProfileHasLivePanes
        | PaneCode::PaneNotInProfile
        | PaneCode::PaneInOtherProfile
        | PaneCode::ProbeFailed
        | PaneCode::HerdrVersionUnsupported
        | PaneCode::ProfileNotFound
        | PaneCode::PaneNotFound
        | PaneCode::SessionNotFound => ErrorClass::Refusal,
        // Went wrong while doing the work: a race between writers, a bound that ran
        // out, something unreachable or unreadable, live state that disagrees with
        // its spec, or work the verb cannot do yet.
        PaneCode::GenerationConflict
        | PaneCode::ProfileConflict
        | PaneCode::Timeout
        | PaneCode::Unavailable
        | PaneCode::StoreCorrupt
        | PaneCode::NotImplemented
        | PaneCode::ProfileDrift => ErrorClass::Failure,
    }
}

/// A code an adapter or a verb owns: well-formed ([`is_valid_code`]) and not one
/// of the closed codes ([`ALL_CODES`]), so each code has exactly one representation.
///
/// The field is private and there is no `From<&str>`, `From<String>` or `Default`:
/// the only ways to get one are [`RefusalCode::from_static`] (for a `const`) and
/// [`RefusalCode::parse`] (for a code read off the wire). A [`PaneError::Refused`]
/// therefore cannot carry an unvalidated code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefusalCode(Cow<'static, str>);

impl RefusalCode {
    /// A code declared as a constant, checked when the constant is evaluated:
    /// a literal that is not kebab-case, or is a closed code, fails the build.
    /// A verb or an adapter declares the code it owns with
    /// `const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");`,
    /// so the call site needs no `Result`.
    ///
    /// An invalid literal in a `const` does not compile:
    ///
    /// ```compile_fail,E0080
    /// use holler_pane::error::RefusalCode;
    /// const BAD: RefusalCode = RefusalCode::from_static("Not A Code");
    /// ```
    ///
    /// Call it from a `const` item. Called at run time with a literal that is not
    /// an open code it panics, which only a programming error can cause.
    pub const fn from_static(code: &'static str) -> Self {
        assert!(
            is_valid_code(code) && !is_closed_code(code),
            "RefusalCode::from_static: not a kebab-case code, or a closed code"
        );
        Self(Cow::Borrowed(code))
    }

    /// A code read off the wire. A code that is not kebab-case, or that is one of
    /// the closed codes (which have their own variants), is refused.
    pub fn parse(code: String) -> Result<Self, RefusalCodeError> {
        if !is_valid_code(&code) {
            return Err(RefusalCodeError::Invalid(code));
        }
        if PaneCode::parse(&code).is_some() {
            return Err(RefusalCodeError::Closed(code));
        }
        Ok(Self(Cow::Owned(code)))
    }

    /// The code string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RefusalCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string cannot be a [`RefusalCode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalCodeError {
    /// Not `^[a-z]+(-[a-z]+)*$`.
    Invalid(String),
    /// A closed code: it has its own [`PaneError`] variant, so a `Refused` must not
    /// carry it.
    Closed(String),
}

impl fmt::Display for RefusalCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefusalCodeError::Invalid(code) => {
                write!(f, "{} is not a kebab-case code", excerpt(code))
            }
            RefusalCodeError::Closed(code) => {
                write!(
                    f,
                    "{} is a closed code and has its own error variant",
                    excerpt(code)
                )
            }
        }
    }
}

impl std::error::Error for RefusalCodeError {}

/// An error of pane control. Every variant has a stable kebab-case [`code`](Self::code)
/// and a human [`Display`](fmt::Display) message; none carries a secret.
///
/// The variants are one-to-one with the closed codes in [`ALL_CODES`], plus the open
/// [`Refused`](PaneError::Refused). A payload field is a plain `String` so the error
/// can be rebuilt from the wire.
///
/// The payload names by convention: `what` is the thing the error is about (a
/// pane, a profile, a socket, the text that was refused), `message` is free text,
/// `op` is the operation that timed out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneError {
    /// `not-implemented`: the skeleton's answer for a verb or method whose story has
    /// not landed. (#637 defines it; every stub returns it.)
    NotImplemented,
    /// `usage`: the request is malformed: a missing or conflicting argument, a bad
    /// name, a params object that does not decode. (#637, every verb.)
    Usage { message: String },
    /// `grid-ambiguous`: a grid position that can be read more than one way, or not
    /// at all. `what` quotes the text that was refused and says what to write
    /// instead. (#637, `GridPos`.)
    GridAmbiguous { what: String },
    /// `grid-out-of-range`: a row or column of zero, or above `u16::MAX`. `what`
    /// quotes the text that was refused and gives the bounds. (#637, `GridPos`.) Also,
    /// from `HerdrPort::ensure_pane`, a cell outside the Herdr workspace's extent; `what`
    /// then names the cell and the extent. (#640.)
    GridOutOfRange { what: String },
    /// `command-not-argv`: a command given where an argv array is required, such as
    /// a shell string. (#637, `Argv`.)
    CommandNotArgv,
    /// `env-name-invalid`: an environment variable name that is empty or contains
    /// whitespace. The text is not echoed. (#637, `EnvVarName`.)
    EnvNameInvalid,
    /// `generation-conflict`: a compare-and-swap write named a generation the record
    /// has moved past (the CAS conflict; see [`crate::generation`]).
    Conflict,
    /// `probe-failed`: the health probe of a pane did not pass; `message` says what
    /// was missing. (#644/#663.)
    ProbeFailed { message: String },
    /// `profile-conflict`: the profile moved after the live change was made (the I8
    /// write order); `what` names the profile and the reconcile step. (#644/#663.)
    ProfileConflict { what: String },
    /// `profile-not-found`: no profile of that name; `what` is the name.
    /// (#644/#663.)
    ProfileNotFound { what: String },
    /// `profile-exists`: a profile with that name or slug already exists; `what` is
    /// the name. (#661.)
    ProfileExists { what: String },
    /// `profile-has-live-panes`: the profile cannot be deleted while panes of it are
    /// live; `what` names the profile. (#662.)
    ProfileHasLivePanes { what: String },
    /// `pane-not-in-profile`: a named pane does not belong to the profile the verb
    /// is scoped to; `what` names both. (#643/#663.)
    PaneNotInProfile { what: String },
    /// `pane-in-other-profile`: the pane already belongs to another profile (a pane
    /// belongs to at most one); `what` names both. (#661.)
    PaneInOtherProfile { what: String },
    /// `profile-secret-refused`: an environment entry carried a value (`NAME=value`);
    /// a profile holds names only (I7). The value is never echoed. (#661; raised by
    /// `EnvVarName`.)
    ProfileSecretRefused,
    /// `herdr-version-unsupported`: Herdr reports an API version the adapter does not
    /// know; `message` names the version and the supported ones. (#640.)
    HerdrVersionUnsupported { message: String },
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names the port method, as `<port>.<method>` (`herdr.ensure_pane`).
    /// (#638-#642.)
    Timeout { op: String },
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
    /// `session-not-found`: no harness session of that id; `what` is the id.
    /// (#638-#642.)
    SessionNotFound { what: String },
    /// `store-corrupt`: a stored file or record cannot be read back; the store fails
    /// closed rather than dropping state. `what` names the store. (#639/#661.)
    StoreCorrupt { what: String },
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
    /// `profile-drift`: a live pane differs from its profile's spec. Listed for
    /// convenience: it is a reconcile finding kind (#647/#665), not an error a port
    /// returns.
    ProfileDrift { message: String },
    /// An open code that an adapter or a verb owns (not in [`ALL_CODES`]), and the
    /// landing place for a code read off the wire that is not closed. The code is
    /// validated by construction; see [`RefusalCode`].
    ///
    /// An unvalidated code cannot be built here from outside the crate:
    ///
    /// ```compile_fail,E0277
    /// use holler_pane::PaneError;
    /// let _ = PaneError::Refused { code: "Not A Code".into(), message: String::new() };
    /// ```
    ///
    /// Nor can a [`RefusalCode`] be built around the private field (this pins field
    /// privacy; the stable toolchain checks only that it fails to compile, not the
    /// error code):
    ///
    /// ```compile_fail,E0423
    /// use holler_pane::error::RefusalCode;
    /// let _ = RefusalCode(std::borrow::Cow::Borrowed("Not A Code"));
    /// ```
    Refused { code: RefusalCode, message: String },
}

impl PaneError {
    /// The kebab-case code of this error: one of [`ALL_CODES`], or the open code of a
    /// [`Refused`](PaneError::Refused).
    pub fn code(&self) -> &str {
        match self.classify() {
            Ok(closed) => closed.as_str(),
            Err(open) => open.as_str(),
        }
    }

    /// The closed code of this error, or the open code of a `Refused`.
    pub(crate) fn classify(&self) -> Result<PaneCode, &RefusalCode> {
        match self {
            PaneError::NotImplemented => Ok(PaneCode::NotImplemented),
            PaneError::Usage { .. } => Ok(PaneCode::Usage),
            PaneError::GridAmbiguous { .. } => Ok(PaneCode::GridAmbiguous),
            PaneError::GridOutOfRange { .. } => Ok(PaneCode::GridOutOfRange),
            PaneError::CommandNotArgv => Ok(PaneCode::CommandNotArgv),
            PaneError::EnvNameInvalid => Ok(PaneCode::EnvNameInvalid),
            PaneError::Conflict => Ok(PaneCode::GenerationConflict),
            PaneError::ProbeFailed { .. } => Ok(PaneCode::ProbeFailed),
            PaneError::ProfileConflict { .. } => Ok(PaneCode::ProfileConflict),
            PaneError::ProfileNotFound { .. } => Ok(PaneCode::ProfileNotFound),
            PaneError::ProfileExists { .. } => Ok(PaneCode::ProfileExists),
            PaneError::ProfileHasLivePanes { .. } => Ok(PaneCode::ProfileHasLivePanes),
            PaneError::PaneNotInProfile { .. } => Ok(PaneCode::PaneNotInProfile),
            PaneError::PaneInOtherProfile { .. } => Ok(PaneCode::PaneInOtherProfile),
            PaneError::ProfileSecretRefused => Ok(PaneCode::ProfileSecretRefused),
            PaneError::HerdrVersionUnsupported { .. } => Ok(PaneCode::HerdrVersionUnsupported),
            PaneError::Timeout { .. } => Ok(PaneCode::Timeout),
            PaneError::PaneNotFound { .. } => Ok(PaneCode::PaneNotFound),
            PaneError::SessionNotFound { .. } => Ok(PaneCode::SessionNotFound),
            PaneError::StoreCorrupt { .. } => Ok(PaneCode::StoreCorrupt),
            PaneError::Unavailable { .. } => Ok(PaneCode::Unavailable),
            PaneError::ProfileDrift { .. } => Ok(PaneCode::ProfileDrift),
            PaneError::Refused { code, .. } => Err(code),
        }
    }

    /// The payload that travels beside the code on the wire (the `detail` of
    /// [`crate::PaneReply`]): the single string a variant carries, if it carries one.
    pub(crate) fn detail(&self) -> Option<&str> {
        match self {
            PaneError::NotImplemented
            | PaneError::CommandNotArgv
            | PaneError::EnvNameInvalid
            | PaneError::Conflict
            | PaneError::ProfileSecretRefused
            | PaneError::Refused { .. } => None,
            PaneError::Usage { message }
            | PaneError::ProbeFailed { message }
            | PaneError::HerdrVersionUnsupported { message }
            | PaneError::ProfileDrift { message } => Some(message),
            PaneError::GridAmbiguous { what }
            | PaneError::GridOutOfRange { what }
            | PaneError::ProfileConflict { what }
            | PaneError::ProfileNotFound { what }
            | PaneError::ProfileExists { what }
            | PaneError::ProfileHasLivePanes { what }
            | PaneError::PaneNotInProfile { what }
            | PaneError::PaneInOtherProfile { what }
            | PaneError::PaneNotFound { what }
            | PaneError::SessionNotFound { what }
            | PaneError::StoreCorrupt { what }
            | PaneError::Unavailable { what } => Some(what),
            PaneError::Timeout { op } => Some(op),
        }
    }

    /// Rebuild the error a closed `code` stands for from what travelled on the wire:
    /// the payload is the `detail` when there is one, else the whole `message`.
    fn from_closed(code: PaneCode, message: String, detail: Option<String>) -> PaneError {
        let text = detail.unwrap_or(message);
        match code {
            PaneCode::NotImplemented => PaneError::NotImplemented,
            PaneCode::Usage => PaneError::Usage { message: text },
            PaneCode::GridAmbiguous => PaneError::GridAmbiguous { what: text },
            PaneCode::GridOutOfRange => PaneError::GridOutOfRange { what: text },
            PaneCode::CommandNotArgv => PaneError::CommandNotArgv,
            PaneCode::EnvNameInvalid => PaneError::EnvNameInvalid,
            PaneCode::GenerationConflict => PaneError::Conflict,
            PaneCode::ProbeFailed => PaneError::ProbeFailed { message: text },
            PaneCode::ProfileConflict => PaneError::ProfileConflict { what: text },
            PaneCode::ProfileNotFound => PaneError::ProfileNotFound { what: text },
            PaneCode::ProfileExists => PaneError::ProfileExists { what: text },
            PaneCode::ProfileHasLivePanes => PaneError::ProfileHasLivePanes { what: text },
            PaneCode::PaneNotInProfile => PaneError::PaneNotInProfile { what: text },
            PaneCode::PaneInOtherProfile => PaneError::PaneInOtherProfile { what: text },
            PaneCode::ProfileSecretRefused => PaneError::ProfileSecretRefused,
            PaneCode::HerdrVersionUnsupported => {
                PaneError::HerdrVersionUnsupported { message: text }
            }
            PaneCode::Timeout => PaneError::Timeout { op: text },
            PaneCode::PaneNotFound => PaneError::PaneNotFound { what: text },
            PaneCode::SessionNotFound => PaneError::SessionNotFound { what: text },
            PaneCode::StoreCorrupt => PaneError::StoreCorrupt { what: text },
            PaneCode::Unavailable => PaneError::Unavailable { what: text },
            PaneCode::ProfileDrift => PaneError::ProfileDrift { message: text },
        }
    }

    /// Rebuild an error from the three strings of a wire reply: a closed code maps
    /// to its own variant, any other well-formed code to [`PaneError::Refused`], and
    /// a code that is not well-formed to [`PaneError::Unavailable`] (the reply is
    /// garbled, so the peer cannot be trusted).
    pub(crate) fn from_wire(code: String, message: String, detail: Option<String>) -> PaneError {
        if let Some(closed) = PaneCode::parse(&code) {
            return PaneError::from_closed(closed, message, detail);
        }
        match RefusalCode::parse(code) {
            Ok(code) => PaneError::Refused { code, message },
            Err(bad) => PaneError::Unavailable {
                what: format!(
                    "the reply carried an error with an unusable code ({bad}): {}",
                    excerpt(&message)
                ),
            },
        }
    }

    /// The text of the serde error a guard raises for this error: `"<code>: <detail>"`,
    /// the detail being the variant's payload, or its message when it has none. The
    /// convention lets [`PaneError::from_decode`] get the code back out.
    pub(crate) fn coded_message(&self) -> String {
        let text = match self.detail() {
            Some(detail) => detail.to_owned(),
            None => self.to_string(),
        };
        format!("{}: {}", self.code(), text)
    }

    /// The error for a failed decode: a guard of this crate that raised a coded serde
    /// error maps back to its own code; anything else is a `usage` error.
    pub(crate) fn from_decode(err: &serde_json::Error) -> PaneError {
        let text = err.to_string();
        match PaneCode::split_prefix(&text) {
            Some((code, rest)) => PaneError::from_closed(code, rest.to_owned(), None),
            None => PaneError::Usage { message: text },
        }
    }
}

impl fmt::Display for PaneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneError::NotImplemented => f.write_str("not implemented"),
            PaneError::Usage { message } => f.write_str(message),
            PaneError::GridAmbiguous { what } => write!(f, "ambiguous grid position: {what}"),
            PaneError::GridOutOfRange { what } => {
                write!(f, "grid position out of range: {what}")
            }
            PaneError::CommandNotArgv => {
                f.write_str("a command must be a JSON array of strings, never a shell string")
            }
            PaneError::EnvNameInvalid => f.write_str(
                "an environment variable name must be non-empty and contain no whitespace",
            ),
            PaneError::Conflict => f.write_str(
                "the record changed since it was read (generation conflict); read it again and retry",
            ),
            PaneError::ProbeFailed { message } => write!(f, "health probe failed: {message}"),
            PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
            PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
            PaneError::ProfileExists { what } => write!(f, "profile already exists: {what}"),
            PaneError::ProfileHasLivePanes { what } => {
                write!(f, "profile has live panes: {what}")
            }
            PaneError::PaneNotInProfile { what } => {
                write!(f, "pane is not in the profile: {what}")
            }
            PaneError::PaneInOtherProfile { what } => {
                write!(f, "pane belongs to another profile: {what}")
            }
            PaneError::ProfileSecretRefused => f.write_str(
                "a profile holds environment variable names only, never a value (an entry with '=' was refused)",
            ),
            PaneError::HerdrVersionUnsupported { message } => {
                write!(f, "unsupported Herdr version: {message}")
            }
            PaneError::Timeout { op } => write!(f, "timed out: {op}"),
            PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
            PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
            PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
            PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
            PaneError::ProfileDrift { message } => write!(f, "profile drift: {message}"),
            PaneError::Refused { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for PaneError {}

/// `text` quoted for an error message, cut to 64 characters so an oversized input
/// cannot produce an oversized message.
pub(crate) fn excerpt(text: &str) -> String {
    const LIMIT: usize = 64;
    if text.chars().count() <= LIMIT {
        return format!("{text:?}");
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head:?}...")
}

/// Deserialize a string and run it through `parse`, turning a refusal into a serde
/// error whose text is the coded message of the [`PaneError`] (so the code survives
/// in the text, and no guard echoes a secret). Every name and guard type of this
/// crate deserializes through here.
pub(crate) fn deserialize_parsed<'de, D, T>(
    deserializer: D,
    parse: fn(&str) -> Result<T, PaneError>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    parse(&text).map_err(|e| de::Error::custom(e.coded_message()))
}
