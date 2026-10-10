//! `Argv`, `EnvVarName` and `AgentKey`: the guards that make a stored command, a
//! stored environment and a stored agent key safe by construction.
//!
//! - [`Argv`] (epic #633, B2): every stored command is an argv array and nothing in
//!   Holler passes it through a shell. A bare string where an array is expected is
//!   refused with `command-not-argv`.
//! - [`EnvVarName`] (I7): an environment entry is a NAME only. It is the one guard
//!   for env entries, so a profile or a pane record cannot hold a value, and the
//!   hub and the importer need no second scan of the raw JSON: a string containing
//!   `=` carries a value and is refused with `profile-secret-refused`; an empty
//!   name, or one with whitespace, is `env-name-invalid`. Neither refusal echoes the
//!   text it refused.
//! - [`AgentKey`] (#700): the OpenCode agent a pane's messages run as, a name and
//!   never a secret. It is a non-empty token of ASCII letters, digits, `-` and `_`;
//!   anything else is refused with the open code `agent-key-invalid`, which does not
//!   echo the text either.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{deserialize_parsed, PaneError, RefusalCode};

/// An argument vector: the program and its arguments, one string each.
///
/// It serializes as a JSON array of strings and reads back from one. Spaces and
/// shell metacharacters inside an element are data, never re-split. A JSON string
/// where an `Argv` is expected is `command-not-argv`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Argv(Vec<String>);

impl Argv {
    /// An argv from its elements.
    pub fn new(parts: Vec<String>) -> Self {
        Self(parts)
    }

    /// The elements: the program first, then its arguments.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// The elements, consuming the argv.
    pub fn into_vec(self) -> Vec<String> {
        self.0
    }

    /// Read an argv from JSON text, as the CLI's `--command-json` and `--check-json`
    /// flags need: text that is not JSON is `usage`; JSON that is not an array of
    /// strings (a bare string most of all) is `command-not-argv`.
    pub fn from_json(text: &str) -> Result<Self, PaneError> {
        serde_json::from_str::<Argv>(text).map_err(|e| {
            if e.is_data() {
                PaneError::CommandNotArgv
            } else {
                PaneError::Usage {
                    message: format!("not valid JSON: {e}"),
                }
            }
        })
    }

    /// An argv from a JSON value: an array whose elements are all strings.
    fn from_value(value: Value) -> Result<Self, PaneError> {
        let Value::Array(items) = value else {
            return Err(PaneError::CommandNotArgv);
        };
        items
            .into_iter()
            .map(|item| match item {
                Value::String(part) => Ok(part),
                _ => Err(PaneError::CommandNotArgv),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

impl<'de> Deserialize<'de> for Argv {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Argv::from_value(value).map_err(|e| de::Error::custom(e.coded_message()))
    }
}

/// The name of an environment variable: a name only, never `NAME=value`.
///
/// Parsing is the guard (see the module docs). It serializes as a plain string and
/// reads back through [`EnvVarName::parse`], so a record that holds a value does not
/// load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Check `text` and keep it as a name.
    ///
    /// - a `=` anywhere: `profile-secret-refused` (the entry carries a value);
    /// - empty, or any whitespace or control character: `env-name-invalid`.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        if text.contains('=') {
            return Err(PaneError::ProfileSecretRefused);
        }
        if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(PaneError::EnvNameInvalid);
        }
        Ok(Self(text.to_owned()))
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for EnvVarName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_parsed(deserializer, EnvVarName::parse)
    }
}

/// `agent-key-invalid`: an OpenCode agent key that is not a non-empty token of ASCII
/// letters, digits, `-` and `_`. A refusal, exit 3 (#700, [`AgentKey`]).
pub const AGENT_KEY_INVALID: RefusalCode = RefusalCode::from_static("agent-key-invalid");

/// The OpenCode agent a pane's hub-delivered messages run as, such as `orchestrator`:
/// a name, never a secret (I7). A record holding `None` instead runs the server's
/// default agent.
///
/// Parsing is the guard (see the module docs). It serializes as a plain string and
/// reads back through [`AgentKey::parse`], so a record holding a malformed key does
/// not load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AgentKey(String);

impl AgentKey {
    /// Check `text` and keep it as an agent key: non-empty, and only ASCII letters,
    /// digits, `-` and `_` (so no whitespace, newline, `=` or `/`). Anything else is
    /// [`AGENT_KEY_INVALID`], in a message that states the rule and not the text.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if text.is_empty() || !text.chars().all(allowed) {
            return Err(PaneError::Refused {
                code: AGENT_KEY_INVALID,
                message: "an OpenCode agent key must be non-empty and use only ASCII letters, \
                          digits, '-' and '_'"
                    .to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// The key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AgentKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_parsed(deserializer, AgentKey::parse)
    }
}

/// Read an `env` field: a JSON array of [`EnvVarName`]s.
///
/// Used by `Pane.env` and `ProfileSpec.env` (beside `#[serde(default)]`) instead of
/// the derived `Vec` impl, whose type error would repeat the value it was given. The
/// field is read as a JSON value first, as [`Argv`] is, so no refusal echoes input:
///
/// - an array: each element goes through [`EnvVarName::parse`], codes unchanged (a
///   non-string element is `env-name-invalid`);
/// - a bare string (the shape a shell `NAME=value` has): `profile-secret-refused` if
///   it contains `=`, else `env-name-invalid`;
/// - any other shape: `env-name-invalid`.
pub(crate) fn deserialize_env_names<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<EnvVarName>, D::Error> {
    let refuse = |e: PaneError| de::Error::custom(e.coded_message());
    match Value::deserialize(deserializer)? {
        Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                Value::String(name) => EnvVarName::parse(&name),
                _ => Err(PaneError::EnvNameInvalid),
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(refuse),
        // A bare string with a `=` is a `NAME=value` entry that lost its list.
        Value::String(text) if text.contains('=') => Err(refuse(PaneError::ProfileSecretRefused)),
        // Anything else is not a list of names: a bare string without a `=` (the
        // wrong shape even when it is a valid name), a map, a number, `null`.
        _ => Err(refuse(PaneError::EnvNameInvalid)),
    }
}
