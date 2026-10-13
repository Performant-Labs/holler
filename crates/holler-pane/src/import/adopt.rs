//! The adoption of a pane whose tmux session runs Claude Code (#831, ADR-0021 section 15 item
//! 17): with `--adopt-claude-code`, such a pane gets a record with the Claude Code harness
//! (`kind: claude`) instead of being left alone. Reads only; never a guess.
//!
//! **The process** ([`read`]). tmux names the pane's first process (`ps`, the shell a person
//! typed `claude` into), so the chain asks the host for the pids of every tmux pane's
//! foreground process group (`host.foreground_pids`), then the harness, by pid, which session
//! each holds (`harness.shown_session`, Claude Code's `agents --json`), in the order the pids
//! came, until the second pid the harness knows. Exactly one known pid holding a non-empty
//! session is the pane's process; none, more than one, or one holding no session is
//! `claude-process-unmatched`; a read that failed is `harness-unreadable`. A pane with no single
//! Herdr label reads nothing ([`Adoption::Unlabelled`]): the join's own label finding blocks it.
//!
//! **The record** ([`assess_adopted`]) is the join's (`assess::begin` and `assess::finish`,
//! shared with an OpenCode pane), with the Claude Code harness part: the process's pid, no port,
//! health `unknown`, the session it holds (never the old `last-session` files), no command
//! (Holler cannot see its argv) and no agent (`relaunch` refuses one for Claude Code, so
//! `OC_DEFAULT_AGENT` is ignored). `pane.env` gives the model and effort as written, the context
//! ceilings and the other variables' names, as for any pane. None of the OpenCode-only checks
//! applies: no server is asked, and the session files and the launcher's `command` are not
//! read. The launcher entry gives the directory and may omit its port.
//!
//! **Its findings**: `claude-login-env` (a problem) for a `pane.env` variable that changes
//! Claude Code's login ([`HarnessKind::login_variables`]); `claude-model-not-anthropic` (a
//! warning) when `OC_MODEL`'s provider is not `anthropic`; and, for a pane adopted, one
//! `claude-code-adopted` (info) naming the next step, `holler pane relaunch NAME`: a process
//! Holler did not start holds a cross-session message, and the relaunch restarts it under
//! Holler on the same session. Each names the pane and a variable's name only, never a value
//! (I7).

use crate::pane::{HarnessInfo, HarnessKind, Health, PaneId};
use crate::{HarnessEndpoint, PaneName, Ports};

use super::assess::{begin, finish, labelled, Context, HarnessPart, Outcome};
use super::files::{EnvValues, PaneFiles};
use super::findings::{Finding, ImportFinding};
use super::observe::{unread_finding, Adopt, Unread};

/// The calls of the adoption, named as `<port>.<method>` in a finding's message.
const HOST_FOREGROUND_PIDS: &str = "host.foreground_pids";
const HARNESS_SHOWN_SESSION: &str = "harness.shown_session";

/// The provider an adopted pane's model is expected to name (compared without case).
const ANTHROPIC: &str = "anthropic";

/// What the chain of a pane whose tmux session runs Claude Code did about its adoption.
pub(super) enum Adoption {
    /// The run was not asked to adopt: the pane is left alone (`harness-claude-code`).
    NotAsked,
    /// Asked, with no single Herdr pane labelled with the pane's name: nothing was read.
    Unlabelled,
    /// Asked, and read: the pane's process, or why there is none.
    Read(Result<Process, Miss>),
}

/// The pane's one Claude Code process: its pid and the session it holds.
pub(super) struct Process {
    pid: u32,
    session: String,
}

/// Why no process was found for the pane.
pub(super) enum Miss {
    /// A read failed (`host.foreground_pids` or `harness.shown_session`).
    Unread(Unread),
    /// The harness knows none of the pids.
    NoMatch,
    /// The harness knows more than one.
    Several,
    /// The one it knows holds an empty session.
    NoSession,
}

/// The adoption state of the pane `name` in this run (the three of [`Adopt`]): resolved from the
/// snapshot the run already read, before the chain, since `shown_session` takes the Herdr pane.
pub(super) fn target(context: &Context<'_>, name: &PaneName) -> Adopt {
    if !context.adopt {
        return Adopt::Off;
    }
    match labelled(context, name).as_slice() {
        [one] => Adopt::On(one.pane_id.clone()),
        _ => Adopt::OnUnlabelled,
    }
}

/// The pane's process: `host.foreground_pids`, then `harness.shown_session` per pid, in that
/// order, stopping at the second pid the harness knows (the outcome is then already "more than
/// one"). A failed `foreground_pids` reads no session: no partial list is walked.
pub(super) fn read(ports: Ports<'_>, name: &PaneName, pane: &PaneId) -> Result<Process, Miss> {
    let pids = ports.host.foreground_pids(name).map_err(|error| {
        Miss::Unread(Unread {
            op: HOST_FOREGROUND_PIDS,
            error,
        })
    })?;
    let mut found = None;
    for pid in pids {
        let at = HarnessEndpoint::Process {
            pane: name.clone(),
            pid,
        };
        let shown = ports.harness.shown_session(pane, &at).map_err(|error| {
            Miss::Unread(Unread {
                op: HARNESS_SHOWN_SESSION,
                error,
            })
        })?;
        let Some(session) = shown else {
            continue;
        };
        if found.is_some() {
            return Err(Miss::Several);
        }
        found = Some(Process { pid, session });
    }
    match found {
        None => Err(Miss::NoMatch),
        Some(process) if process.session.is_empty() => Err(Miss::NoSession),
        Some(process) => Ok(process),
    }
}

/// Join a pane whose tmux session runs Claude Code, in a run that adopts it: the join's start
/// and end shared with an OpenCode pane, the Claude Code harness part, and the adoption's own
/// findings (see the module docs).
pub(super) fn assess_adopted(
    context: &Context<'_>,
    files: &PaneFiles,
    adoption: Adoption,
) -> Outcome {
    let name = &files.name;
    let (mut findings, herdr) = begin(context, files);
    let process = match adoption {
        Adoption::Read(Ok(process)) => Some(process),
        Adoption::Read(Err(miss)) => {
            findings.push(miss.finding(name));
            None
        }
        Adoption::NotAsked | Adoption::Unlabelled => None,
    };
    if let Some(env) = &files.env {
        findings.extend(login_env(name, env));
        findings.extend(model_not_anthropic(name, env));
    }
    let part = process.map(|process| HarnessPart {
        harness: HarnessInfo {
            kind: HarnessKind::Claude,
            port: None,
            pid: Some(process.pid),
            health: Health::Unknown,
        },
        session: Some(process.session),
        keeps_agent: false,
        command: None,
    });
    let mut outcome = finish(context, files, herdr, findings, part);
    if outcome.record.is_some() {
        let message = format!(
            "the pane is adopted with the Claude Code harness; run holler pane relaunch {name} \
             to restart its process under Holler on the same session, so that it accepts \
             messages"
        );
        outcome.findings.push(Finding::about(
            name,
            ImportFinding::ClaudeCodeAdopted,
            message,
        ));
    }
    outcome
}

impl Miss {
    /// The finding of a pane whose process was not found. No session id or pid is named.
    fn finding(&self, name: &PaneName) -> Finding {
        let why = match self {
            Miss::Unread(unread) => return unread_finding(name, unread),
            Miss::NoMatch => "no process of its tmux session is one the Claude Code harness lists",
            Miss::Several => {
                "more than one process of its tmux session is one the Claude Code \
                              harness lists"
            }
            Miss::NoSession => {
                "the one process of its tmux session that the Claude Code \
                                harness lists holds no session"
            }
        };
        Finding::about(
            name,
            ImportFinding::ClaudeProcessUnmatched,
            format!("{why}, so the pane is not adopted on a guess"),
        )
    }
}

/// `claude-login-env` for each variable of `pane.env` that changes Claude Code's login: its
/// name, never its value.
fn login_env(name: &PaneName, env: &EnvValues) -> Vec<Finding> {
    let login = HarnessKind::Claude.login_variables();
    env.env
        .iter()
        .filter(|variable| login.contains(&variable.as_str()))
        .map(|variable| {
            let message = format!(
                "its pane.env sets {}, which silently changes a Claude Code login, so the pane \
                 is not adopted; remove it from pane.env",
                variable.as_str()
            );
            Finding::about(name, ImportFinding::ClaudeLoginEnv, message)
        })
        .collect()
}

/// `claude-model-not-anthropic` when `OC_MODEL`'s provider is not `anthropic` (compared without
/// case): the variable's name, never its value.
fn model_not_anthropic(name: &PaneName, env: &EnvValues) -> Option<Finding> {
    (!env.model.provider.eq_ignore_ascii_case(ANTHROPIC)).then(|| {
        let message = "its pane.env's OC_MODEL names a provider other than anthropic; the pane is \
                       adopted with that model, which a relaunch passes to claude as it is";
        Finding::about(
            name,
            ImportFinding::ClaudeModelNotAnthropic,
            message.to_owned(),
        )
    })
}
