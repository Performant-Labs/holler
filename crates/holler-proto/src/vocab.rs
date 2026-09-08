//! The name vocabulary (ADR 0005) and the feature / harness id vocabulary.
//!
//! Three **distinct** name kinds, each a newtype around a `String`, sharing
//! one segment grammar but differing in length and structure:
//!
//! - [`Label`] — the hub-unique hostname / label (the prefix a session is
//!   routed under). 1–32 chars.
//! - [`SessionName`] — a session **within** a single body. 1–32 chars, **no
//!   `/`**.
//! - [`RoutableName`] — the cross-hub router: `<label>/<session>`, or a bare
//!   name. Each `/`-separated segment is 1–32 chars.
//!
//! Segment grammar (ADR 0005): lowercase ASCII letters and digits only; `-`
//! is allowed **between** word chars, but a segment must not start or end
//! with one. The 32-char per-segment limit (docs §3) is enforced here.
//!
//! Names are **immutable** — the only way to change one is to delete and
//! recreate (ADR 0005).

use std::collections::BTreeSet;

/// The per-segment length limit (docs §3). A segment longer than this is
/// rejected by every name kind.
pub const MAX_SEGMENT_LEN: usize = 32;

/// A **label** — the hub-unique hostname a body claims. It is the routing
/// prefix (see [`RoutableName`]) and is 1–32 chars, a single segment.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Label(String);

impl Label {
    /// Parse a wire string into a `Label`.
    ///
    /// Succeeds iff the string is a single 1–32-char segment matching the
    /// grammar (no `/`). A `Label` never contains a `/`.
    pub fn parse(s: &str) -> Result<Self, NameError> {
        if s.is_empty() {
            return Err(NameError::Empty);
        }
        if s.contains('/') {
            return Err(NameError::Slash);
        }
        check_segment(s.as_bytes())?;
        Ok(Self(s.to_owned()))
    }

    /// The label string, verbatim.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Label({:?})", self.0)
    }
}
impl From<&Label> for String {
    fn from(l: &Label) -> String {
        l.0.clone()
    }
}

/// A **session name** — a session within a single body. 1–32 chars, **no
/// `/`**. (The cross-hub router is [`RoutableName`], which *does* allow the
/// `<label>/<session>` form.)
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionName(String);

impl SessionName {
    /// Parse a wire string into a `SessionName`.
    ///
    /// Succeeds iff the string is a single 1–32-char segment matching the
    /// grammar. A `SessionName` never contains a `/` (that is the
    /// [`RoutableName`] shape).
    pub fn parse(s: &str) -> Result<Self, NameError> {
        if s.is_empty() {
            return Err(NameError::Empty);
        }
        if s.contains('/') {
            return Err(NameError::Slash);
        }
        check_segment(s.as_bytes())?;
        Ok(Self(s.to_owned()))
    }

    /// The session name string, verbatim.
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

/// A **routable name** — the cross-hub router key: `<label>/<session>`, or a
/// bare name (no label). Each `/`-separated segment is 1–32 chars and
/// matches the segment grammar.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoutableName(String);

impl RoutableName {
    /// Parse a wire string into a `RoutableName`.
    ///
    /// Succeeds iff the string is one or more `/`-separated segments, each
    /// 1–32 chars, each matching the segment grammar (lowercase letters and
    /// digits; `-` only between word chars; no leading/trailing `-`).
    pub fn parse(s: &str) -> Result<Self, NameError> {
        if s.is_empty() {
            return Err(NameError::Empty);
        }
        let bytes = s.as_bytes();
        let mut start = 0usize;
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'/' {
                if i == start {
                    return Err(NameError::EmptySegment);
                }
                check_segment(&bytes[start..i])?;
                start = i + 1;
            }
        }
        if start == bytes.len() {
            // Trailing `/`.
            return Err(NameError::EmptySegment);
        }
        check_segment(&bytes[start..])?;
        Ok(Self(s.to_owned()))
    }

    /// The **session** part — the text after the label's `/`, or the whole
    /// name when there is no label.
    #[inline]
    pub fn session(&self) -> &str {
        match self.0.rfind('/') {
            Some(pos) => &self.0[pos + 1..],
            None => &self.0,
        }
    }

    /// The **label** part — the text before the first `/`, or `""` when the
    /// name has no label.
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

impl std::fmt::Display for RoutableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Debug for RoutableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RoutableName({:?})", self.0)
    }
}
impl From<&RoutableName> for String {
    fn from(n: &RoutableName) -> String {
        n.0.clone()
    }
}

/// The per-segment checks shared by all three name kinds: length limit,
/// word-char first/last, and no disallowed characters.
fn check_segment(seg: &[u8]) -> Result<(), NameError> {
    if seg.is_empty() {
        return Err(NameError::Empty);
    }
    if seg.len() > MAX_SEGMENT_LEN {
        return Err(NameError::TooLong);
    }
    if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
        return Err(NameError::BadCharacter);
    }
    for &c in seg {
        if !(is_word(c) || c == b'-') {
            return Err(NameError::BadCharacter);
        }
    }
    Ok(())
}

#[inline]
fn is_word(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z')
}

/// Why a string was not a valid name of the given kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// The string was empty.
    Empty,
    /// A segment (or the whole single-segment name) was empty — e.g. a
    /// trailing or doubled `/`.
    EmptySegment,
    /// A `/` appeared where the kind does not allow it (a `Label` or
    /// `SessionName` carrying a `/`).
    Slash,
    /// A segment exceeded [`MAX_SEGMENT_LEN`] (docs §3).
    TooLong,
    /// A segment contained a character outside the grammar (uppercase, a
    /// leading/trailing `-`, or other).
    BadCharacter,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::Empty => f.write_str("name is empty"),
            NameError::EmptySegment => f.write_str("name has an empty segment"),
            NameError::Slash => f.write_str("name has a '/' where one is not allowed"),
            NameError::TooLong => write!(f, "name segment exceeds {MAX_SEGMENT_LEN} chars"),
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
