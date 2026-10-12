//! A pane's `pane.env` (#650): the grammar the old launcher writes, read for five values and
//! the names of every other variable, never their values (I7).
//!
//! Each line is one of:
//!
//! - blank (empty, or only spaces and tabs), or starting with `#`: skipped;
//! - an assignment: an optional `export` and one or more spaces or tabs, then NAME, then `=`
//!   directly after it, then the value, which is the rest of the line. NAME is the text before
//!   the line's first `=` and must pass `EnvVarName::parse` (not empty, no whitespace or
//!   control character), whether or not it starts with `OC_`.
//!
//! Any other line does not parse: a line that starts with a space or a tab, a line with no
//! `=`, `export OC_MODEL = x` (its NAME ends in a space). The values read are `OC_MODEL`
//! (`PROVIDER/MODEL_ID`, split at the first `/`, both halves non-empty), `OC_EFFORT` (not
//! empty), `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD` (unsigned integers) and `OC_DEFAULT_AGENT`
//! (an agent key; empty is none). A later assignment of a variable replaces an earlier one,
//! as the shell's does. Every other variable is kept by name, in file order, once.
//!
//! An error names a line by its 1-based number, every line of the file counted, and never
//! the text of the line: a line that does not parse has no safe name to give.

use holler_pane::import::files::EnvValues;
use holler_pane::pane::{ContextCeilings, ModelSpec};
use holler_pane::{AgentKey, EnvVarName};

/// The variables whose values become record fields.
const MODEL: &str = "OC_MODEL";
const EFFORT: &str = "OC_EFFORT";
const SOFT: &str = "OC_CONTEXT_SOFT";
const HARD: &str = "OC_CONTEXT_HARD";
const AGENT: &str = "OC_DEFAULT_AGENT";

/// Why a `pane.env` gives no values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EnvError {
    /// The line with this number is not blank, a comment or an assignment.
    Grammar(usize),
    /// The line with this number assigns a variable a value it does not accept.
    Value(usize),
    /// The file parses and does not set these required variables.
    Missing(Vec<&'static str>),
}

/// The values `text` gives (see the module docs).
pub(super) fn parse(text: &str) -> Result<EnvValues, EnvError> {
    let mut read = Read::default();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let Some((name, value)) = assignment(line).ok_or(EnvError::Grammar(number))? else {
            continue;
        };
        read.take(name, value).ok_or(EnvError::Value(number))?;
    }
    read.finish()
}

/// `Some(None)` for a line that is skipped, `Some(Some((NAME, value)))` for an assignment, and
/// `None` for a line that does not parse.
fn assignment(line: &str) -> Option<Option<(EnvVarName, &str)>> {
    if line.chars().all(|c| c == ' ' || c == '\t') || line.starts_with('#') {
        return Some(None);
    }
    let (name, value) = without_export(line).split_once('=')?;
    let name = EnvVarName::parse(name).ok()?;
    Some(Some((name, value)))
}

/// `line` without its `export` prefix: the word and the spaces or tabs after it, when there is
/// at least one.
fn without_export(line: &str) -> &str {
    match line.strip_prefix("export") {
        Some(rest) if rest.starts_with([' ', '\t']) => rest.trim_start_matches([' ', '\t']),
        _ => line,
    }
}

/// The values read so far.
#[derive(Default)]
struct Read {
    model: Option<(String, String)>,
    effort: Option<String>,
    soft: Option<u32>,
    hard: Option<u32>,
    agent: Option<AgentKey>,
    names: Vec<EnvVarName>,
}

impl Read {
    /// Take one assignment; `None` when the variable does not accept the value.
    fn take(&mut self, name: EnvVarName, value: &str) -> Option<()> {
        match name.as_str() {
            MODEL => self.model = Some(model(value)?),
            EFFORT if value.is_empty() => return None,
            EFFORT => self.effort = Some(value.to_owned()),
            SOFT => self.soft = Some(value.parse().ok()?),
            HARD => self.hard = Some(value.parse().ok()?),
            AGENT => self.agent = agent(value)?,
            _ if self.names.contains(&name) => {}
            _ => self.names.push(name),
        }
        Some(())
    }

    /// The values, or the required variables the file does not set.
    fn finish(self) -> Result<EnvValues, EnvError> {
        match (self.model, self.effort, self.soft, self.hard) {
            (Some((provider, model_id)), Some(effort), Some(soft), Some(hard)) => Ok(EnvValues {
                model: ModelSpec {
                    provider,
                    model_id,
                    effort,
                },
                context: ContextCeilings { soft, hard },
                opencode_agent: self.agent,
                env: self.names,
            }),
            (model, effort, soft, hard) => {
                let unset = [
                    (MODEL, model.is_none()),
                    (EFFORT, effort.is_none()),
                    (SOFT, soft.is_none()),
                    (HARD, hard.is_none()),
                ];
                let missing = unset
                    .into_iter()
                    .filter_map(|(name, unset)| unset.then_some(name))
                    .collect();
                Err(EnvError::Missing(missing))
            }
        }
    }
}

/// `OC_MODEL`'s value split at its first `/`, when both halves are non-empty.
fn model(value: &str) -> Option<(String, String)> {
    let (provider, model_id) = value.split_once('/')?;
    let both = !provider.is_empty() && !model_id.is_empty();
    both.then(|| (provider.to_owned(), model_id.to_owned()))
}

/// `OC_DEFAULT_AGENT`'s value: `Some(None)` when it is empty (no agent), `None` when it is not
/// an agent key.
fn agent(value: &str) -> Option<Option<AgentKey>> {
    if value.is_empty() {
        return Some(None);
    }
    AgentKey::parse(value).ok().map(Some)
}
