//! The flag groups the `holler pane` verbs share (story #670, epic #633), defined once:
//!
//! - [`SpecFlags`]: one flag per field of a pane spec, used by `launch` and `relaunch` (and by
//!   the profile spec). Every value stays a string at clap time; [`SpecFlags::validate`] types
//!   them, with the guards of `holler-pane` and their stable error codes, so a bad `--grid` is a
//!   refusal with a code a script can match (exit 1) and not a clap usage error.
//! - [`ProfileOpt`]: `--profile NAME`, on every `pane` verb but `import`, and on `roster`,
//!   `say`, `interrupt` and `answer`.
//! - [`SpecOnly`]: `--spec-only`, on `launch`, `relaunch` and `close`.
//!
//! This is all that #670 declares for the verbs. The positionals and flags that belong to one
//! verb (`pane get PANE`, `pane doctor --fix`) are declared in that verb's own file by its owning
//! story.

use clap::Args;
use holler_pane::pane::PaneRole;
use holler_pane::{Argv, EnvVarName, GridPos, PaneError};

/// `--profile NAME`: scope the verb to a profile.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileOpt {
    /// Act on the panes of this profile; a named pane must belong to it.
    #[arg(long, value_name = "NAME")]
    pub profile: Option<String>,
}

/// `--spec-only`: edit a profile's spec and change nothing live.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecOnly {
    /// Edit the profile's spec for the pane and change nothing live.
    #[arg(long, requires = "profile")]
    pub spec_only: bool,
}

/// One flag per field of a pane spec.
///
/// The text flags keep the user's string, [`SpecFlags::validate`] types it. `--env` names a
/// variable and never carries its value; a command or a health check is given as repeated `ARG`
/// flags or as one JSON array, never both and never as a shell string.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecFlags {
    /// The project directory or worktree the pane works in.
    #[arg(long, value_name = "DIR")]
    pub project: Option<String>,
    /// The Herdr workspace the pane lives in.
    #[arg(long, value_name = "NAME")]
    pub workspace: Option<String>,
    /// Where the pane sits in the Herdr grid, from 1: `r<row>c<col>`, `c<col>r<row>` or `<row>,<col>`.
    #[arg(long, value_name = "POS")]
    pub grid: Option<String>,
    /// The model the pane runs, as PROVIDER/ID.
    #[arg(long, value_name = "PROVIDER/ID")]
    pub model: Option<String>,
    /// The model's effort level.
    #[arg(long, value_name = "LEVEL")]
    pub effort: Option<String>,
    /// The pane's role: agent or orchestrator.
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// An environment variable NAME the pane gets (a name only, never NAME=value). Repeatable.
    #[arg(long, value_name = "NAME")]
    pub env: Vec<String>,
    /// The soft context ceiling a watchdog reads.
    #[arg(long, value_name = "N")]
    pub ctx_soft: Option<u32>,
    /// The hard context ceiling a watchdog reads.
    #[arg(long, value_name = "N")]
    pub ctx_hard: Option<u32>,
    /// How the harness server's port is chosen.
    #[arg(long, value_name = "POLICY")]
    pub port_policy: Option<String>,
    /// One element of the launch command, program first. Repeatable; not with --command-json.
    #[arg(long, value_name = "ARG", conflicts_with = "command_json")]
    pub command_arg: Vec<String>,
    /// The launch command as a JSON array of strings; not with --command-arg.
    #[arg(long, value_name = "JSON")]
    pub command_json: Option<String>,
    /// One element of the health check, program first. Repeatable; not with --check-json.
    #[arg(long, value_name = "ARG", conflicts_with = "check_json")]
    pub check_arg: Vec<String>,
    /// The health check as a JSON array of strings; not with --check-arg.
    #[arg(long, value_name = "JSON")]
    pub check_json: Option<String>,
    /// A string the health check's output must contain. Repeatable.
    #[arg(long, value_name = "STR")]
    pub expect: Vec<String>,
}

/// The spec flags, typed: what [`SpecFlags::validate`] returns.
///
/// The fields that need no check here (`project`, `workspace`, `model`, `effort`, `port_policy`,
/// `expect`) are the user's strings, as given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecValues {
    pub project: Option<String>,
    pub workspace: Option<String>,
    pub grid: Option<GridPos>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub role: Option<PaneRole>,
    pub env: Vec<EnvVarName>,
    pub ctx_soft: Option<u32>,
    pub ctx_hard: Option<u32>,
    pub port_policy: Option<String>,
    pub command: Option<Argv>,
    pub check: Option<Argv>,
    pub expect: Vec<String>,
}

impl SpecFlags {
    /// Type the flags with the guards of `holler-pane`, or refuse with the guard's own code:
    ///
    /// - `--grid`: `grid-ambiguous` (not a cell, or ambiguous) or `grid-out-of-range`;
    /// - `--env`: `profile-secret-refused` for `NAME=value` (the value is never echoed) and
    ///   `env-name-invalid` for an empty name or one with whitespace;
    /// - `--command-json` and `--check-json`: `command-not-argv` for JSON that is not an array of
    ///   strings, and `usage` for text that is not JSON;
    /// - `--role`: `usage` for anything but `agent` or `orchestrator`.
    pub fn validate(&self) -> Result<SpecValues, PaneError> {
        Ok(SpecValues {
            project: self.project.clone(),
            workspace: self.workspace.clone(),
            grid: self.grid.as_deref().map(GridPos::parse).transpose()?,
            model: self.model.clone(),
            effort: self.effort.clone(),
            role: self.role.as_deref().map(parse_role).transpose()?,
            env: self
                .env
                .iter()
                .map(|name| EnvVarName::parse(name))
                .collect::<Result<_, _>>()?,
            ctx_soft: self.ctx_soft,
            ctx_hard: self.ctx_hard,
            port_policy: self.port_policy.clone(),
            command: argv_of(&self.command_arg, self.command_json.as_deref(), "command")?,
            check: argv_of(&self.check_arg, self.check_json.as_deref(), "check")?,
            expect: self.expect.clone(),
        })
    }
}

/// A role from its flag text. The accepted names are `PaneRole`'s serde names, so they cannot
/// drift from the pane record's.
fn parse_role(text: &str) -> Result<PaneRole, PaneError> {
    serde_json::from_value(serde_json::Value::String(text.to_owned())).map_err(|_| {
        PaneError::Usage {
            message: "--role must be agent or orchestrator".to_owned(),
        }
    })
}

/// An argv from `--<name>-arg` flags or `--<name>-json`, or neither.
fn argv_of(args: &[String], json: Option<&str>, name: &str) -> Result<Option<Argv>, PaneError> {
    match (args.is_empty(), json) {
        (true, None) => Ok(None),
        (false, None) => Ok(Some(Argv::new(args.to_vec()))),
        (true, Some(text)) => Argv::from_json(text).map(Some),
        (false, Some(_)) => Err(PaneError::Usage {
            message: format!("--{name}-arg and --{name}-json cannot be used together"),
        }),
    }
}
