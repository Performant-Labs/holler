//! Where `say`, `interrupt` and `answer` send their prompt: a SESSION or, since epic #633, a pane
//! (story #670, decision 5).
//!
//! The three verbs used to take `SESSION` as a fixed first positional. `say --pane NAME TEXT`
//! cannot parse that way (`TEXT` would bind to `SESSION`), so each verb now captures one optional
//! tail in clap and resolves it here, in code, behind [`Say::resolve`], [`Interrupt::resolve`] and
//! [`Answer::resolve`]: the same pattern as [`crate::Query::resolve`]. Every existing form resolves
//! to the same session and text as before, and a missing or surplus positional is the existing
//! [`Usage`] error (exit 2).
//!
//! `--pane` and `--profile` parse but are not routed yet: story #646 does that. Until then the
//! three verbs refuse them (exit 1, `not implemented (story #646)`, plain text under every
//! format, before any hub is contacted), because a `--pane` that parsed and was ignored would
//! deliver a prompt unchecked. [`route`] is that guard, and the one place that knows the exit
//! codes of these forms: 2 for a malformed tail, 1 for a refused pane or profile.

use crate::cli::{Answer, Interrupt, Say, Usage};
use crate::output::not_implemented_message;
use crate::pane::args::ProfileOpt;

/// The story that routes `--pane` and `--profile`.
pub const ROUTING_STORY: u32 = 646;

/// Who a prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptTarget {
    /// A session address (`<label>/<session>`, or a bare `<session>`).
    Session(String),
    /// A pane, by `--pane NAME`.
    Pane(String),
}

/// A verb's target and the argument after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptArgs {
    pub target: PromptTarget,
    /// `TEXT` for `say`, the optional redirect text for `interrupt`, `CHOICE` for `answer`.
    pub arg: Option<String>,
}

/// A prompt verb that is cleared to contact a hub: the session to prompt and the argument after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routed {
    pub session: String,
    pub arg: Option<String>,
}

/// Why a prompt verb stops before it contacts a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    /// What `main` prints to stderr after `error: `.
    pub message: String,
    /// 2 for a malformed tail (a usage error), 1 for a `--pane` or `--profile` that is not routed.
    pub exit_code: i32,
}

/// The exit code of a usage error (ADR 0003).
const USAGE_EXIT: i32 = 2;

/// The exit code of a refusal (ADR 0003: a runtime failure).
const REFUSED_EXIT: i32 = 1;

/// Clear a resolved verb to run, or say why it stops.
///
/// `resolved` is the verb's `resolve()`: a malformed tail has already stopped it as a usage error,
/// so that a malformed form is never reported as the refusal below. A `--pane` target, and any
/// `--profile`, are then refused until story #646 routes them.
pub fn route(resolved: Result<PromptArgs, Usage>, profile: &ProfileOpt) -> Result<Routed, Stop> {
    let args = resolved.map_err(|usage| Stop {
        message: usage.to_string(),
        exit_code: USAGE_EXIT,
    })?;
    match (args.target, &profile.profile) {
        (PromptTarget::Session(session), None) => Ok(Routed {
            session,
            arg: args.arg,
        }),
        _ => Err(Stop {
            message: not_implemented_message(ROUTING_STORY),
            exit_code: REFUSED_EXIT,
        }),
    }
}

impl Say {
    /// The target and the text. `TEXT` may be absent only when `--parts-file` supplies the message.
    pub fn resolve(&self) -> Result<PromptArgs, Usage> {
        let tail = Tail {
            verb: "say",
            arg: "TEXT",
            required: self.parts_file.is_none(),
            hint: " (or --parts-file FILE)",
        };
        resolve_tail(&self.rest, self.pane.as_deref(), &tail)
    }
}

impl Interrupt {
    /// The target and the optional redirect text.
    pub fn resolve(&self) -> Result<PromptArgs, Usage> {
        let tail = Tail {
            verb: "interrupt",
            arg: "TEXT",
            required: false,
            hint: "",
        };
        resolve_tail(&self.rest, self.pane.as_deref(), &tail)
    }
}

impl Answer {
    /// The target and the choice.
    pub fn resolve(&self) -> Result<PromptArgs, Usage> {
        let tail = Tail {
            verb: "answer",
            arg: "CHOICE",
            required: true,
            hint: "",
        };
        resolve_tail(&self.rest, self.pane.as_deref(), &tail)
    }
}

/// What follows the target in one verb's tail.
struct Tail {
    /// The verb, for messages.
    verb: &'static str,
    /// The argument's name, for messages.
    arg: &'static str,
    /// The argument must be given.
    required: bool,
    /// What else can stand in for a missing required argument, for messages.
    hint: &'static str,
}

impl Tail {
    /// The forms of the verb, for messages.
    fn forms(&self) -> String {
        let arg = if self.required {
            self.arg.to_owned()
        } else {
            format!("[{}]", self.arg)
        };
        let verb = self.verb;
        format!("`holler {verb} SESSION {arg}`, or `holler {verb} --pane NAME {arg}`")
    }
}

/// Split a verb's tail into its target and argument.
///
/// With `--pane NAME` the tail holds at most the argument. Without it, the first positional is the
/// SESSION and the second the argument; clap has already refused a third.
fn resolve_tail(rest: &[String], pane: Option<&str>, tail: &Tail) -> Result<PromptArgs, Usage> {
    let (target, arg) = match pane {
        Some(name) => {
            if rest.len() > 1 {
                return Err(Usage::new(format!(
                    "--pane NAME takes the place of SESSION, so only {} may follow it, got {} \
                     positionals: {}",
                    tail.arg,
                    rest.len(),
                    tail.forms()
                )));
            }
            (PromptTarget::Pane(name.to_owned()), rest.first())
        }
        None => match rest.first() {
            Some(session) => (PromptTarget::Session(session.clone()), rest.get(1)),
            None => {
                return Err(Usage::new(format!(
                    "a SESSION is required: {}",
                    tail.forms()
                )));
            }
        },
    };
    if tail.required && arg.is_none() {
        return Err(Usage::new(format!(
            "{} is required{}: {}",
            tail.arg,
            tail.hint,
            tail.forms()
        )));
    }
    Ok(PromptArgs {
        target,
        arg: arg.cloned(),
    })
}
