//! Session names (ADR 0005) and the feature / harness id vocabulary.
//!
//! A **session name** is a string that is also a valid routable name. The
//! grammar (ADR 0005): `^[a-z0-9]([a-z0-9-]*[a-z0-9])?(/([a-z0-9]([a-z0-9-]*[a-z0-9])?))?$`
//!
//! - Lowercase ASCII letters and digits only; `-` allowed, but a segment must
//!   not start or end with one.
//! - A name is `<session>`, or `<label>/<session>` (the hub's roster key).
//! - Names are **immutable** — the only way to change one is to delete and
//!   recreate (ADR 0005).

use std::collections::BTreeSet;

/// A session name: `<session>` or `<label>/<session>`.
///
/// Parsing enforces the full ADR 0005 grammar. This is the *locator* the
/// routing key is derived from — but it is **not itself** the locator (peer
/// address, connection handle, and `client_id` are separate fields on the
/// roster row).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionName(String);

impl SessionName {
    /// Parse a wire string into a `SessionName`.
    ///
    /// Succeeds iff the string matches the ADR 0005 grammar (see the module
    /// docs). Returns `NameError` naming the first violation.
    pub fn parse(s: &str) -> Result<Self, NameError> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return Err(NameError::Empty);
        }
        let parts: Vec<&[u8]> = split_segments(bytes);
        for (i, seg) in parts.iter().enumerate() {
            if seg.is_empty() {
                return if i == 0 {
                    Err(NameError::Empty)
                } else {
                    Err(NameError::EmptySegment)
                };
            }
            if !segment_ok(seg) {
                return Err(NameError::BadCharacter);
            }
        }
        Ok(Self(s.to_owned()))
    }

    /// The bare session part — the text after the label's `/` (or the whole
    /// name when there is no label).
    #[inline]
    pub fn session(&self) -> &str {
        match self.0.rfind('/') {
            Some(pos) => &self.0[pos + 1..],
            None => &self.0,
        }
    }

    /// The optional label part — the text before the first `/` (or `""` when
    /// the name has no label).
    #[inline]
    pub fn label(&self) -> &str {
        match self.0.find('/') {
            Some(pos) => &self.0[..pos],
            None => "",
        }
    }

    /// `true` when the name carries a label (`<label>/<session>`).
    #[inline]
    pub fn has_label(&self) -> bool {
        self.0.contains('/')
    }

    /// The full routable string, verbatim.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Debug for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionName({:?})", self.0)
    }
}
impl From<&SessionName> for String {
    fn from(n: &SessionName) -> String {
        n.0.clone()
    }
}

fn split_segments(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for i in 0..bytes.len() {
        if bytes[i] == b'/' {
            out.push(&bytes[start..i]);
            start = i + 1;
        }
    }
    out.push(&bytes[start..]);
    out
}

fn segment_ok(seg: &[u8]) -> bool {
    if seg.is_empty() {
        return false;
    }
    let first = seg[0];
    let last = seg[seg.len() - 1];
    if !is_word(first) || !is_word(last) {
        return false;
    }
    for &c in seg {
        if !(is_word(c) || c == b'-') {
            return false;
        }
    }
    true
}

#[inline]
fn is_word(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z')
}

/// Why a string was not a valid session name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// The string was empty.
    Empty,
    /// A segment after a `/` (or the whole name) was empty — e.g. a trailing
    /// or doubled `/`.
    EmptySegment,
    /// A segment contained a character outside the grammar (uppercase, a
    /// leading/trailing `-`, or other).
    BadCharacter,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::Empty => f.write_str("name is empty"),
            NameError::EmptySegment => f.write_str("name has an empty segment"),
            NameError::BadCharacter => f.write_str("name has a disallowed character"),
        }
    }
}
impl std::error::Error for NameError {}

/// The protocol **feature** ids (docs §9). Feature ids are **reserved words**
/// in the CLI.
pub const FEATURES: &[&str] = &[
    "interrupt",
    "presence",
    "ping",
    "query",
    "roster",
    "token",
    "attach",
    "opencode-http",
];

/// The **harness** id vocabulary (docs §9). Only `opencode` is a v2 adapter;
/// the rest are reserved for future adapters (a new harness is a new config
/// row + a new id, not a protocol bump).
pub const HARNESS_IDS: &[&str] = &[
    "opencode", "claude", "codex", "grok", "hermes", "pi", "cursor", "copilot", "droid", "kimi",
    "qwen", "kilo", "goose",
];

/// The closed set of known feature / harness ids, for `query/support`
/// unknown-feature detection (docs §8, `-32006`).
#[derive(Debug, Default, Clone)]
pub struct Vocab {
    features: BTreeSet<&'static str>,
    harnesses: BTreeSet<&'static str>,
}

impl Vocab {
    /// The full v2 vocabulary (all protocol features + all harness ids).
    pub fn v2() -> Self {
        let features = FEATURES.iter().copied().collect();
        let harnesses = HARNESS_IDS.iter().copied().collect();
        Self {
            features,
            harnesses,
        }
    }

    /// `true` iff `id` is a known feature or harness id.
    pub fn is_known(&self, id: &str) -> bool {
        self.features.contains(id) || self.harnesses.contains(id)
    }

    /// `true` iff `id` is one of the known protocol **feature** ids.
    pub fn is_feature(&self, id: &str) -> bool {
        self.features.contains(id)
    }

    /// `true` iff `id` is one of the known **harness** ids.
    pub fn is_harness(&self, id: &str) -> bool {
        self.harnesses.contains(id)
    }
}
