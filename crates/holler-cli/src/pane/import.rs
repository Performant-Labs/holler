//! `holler pane import --from STATE_ROOT --herdr-session NAME [--launcher FILE]
//! [--orchestrator PANE]... [--dry-run] [--adopt-claude-code]` (stories #650, #835, #831):
//! adopt the fleet the old launcher runs. Read each pane's files, join each pane to what Herdr,
//! tmux and the harness show, and write one record per pane and the profile `fleet`, or (with
//! `--dry-run`) only report. A pane whose tmux session runs Claude Code is left alone (#835),
//! or, with `--adopt-claude-code`, adopted with the Claude Code harness, and the switch then
//! extends an existing `fleet` instead of being refused (#831, ADR-0021 section 15, item 17).
//!
//! In order:
//!
//! 1. **Arguments.** `--herdr-session` is required, as on `launch`; STATE_ROOT must be a
//!    directory; each `--orchestrator` must name a pane directory. Each is `usage` (exit 2),
//!    before any file is read or any port is called.
//! 2. **The files** (`sources`, `pane_env`, `launcher`): read here, so `holler-pane` knows no
//!    file format (ADR-0021 section 5). The verb never writes them.
//! 3. **The plan**: `holler_pane::import::plan`, which reads Herdr, the pane store, tmux and the
//!    harness, and changes nothing. A `herdr.version()` or `herdr.snapshot()` error ends the
//!    run with that error and no report.
//! 4. **A real run's writes** (`writes`): the profile, then each record; or, extending `fleet`,
//!    each record, then `fleet`; or nothing at all.
//!
//! A dry run exits 0 with the whole report, whatever it found. A real run exits 0 when it
//! wrote the plan or had nothing to write, 3 with `import-blocked` when a pane has a problem
//! and with `profile-exists` when `fleet` exists and the switch is absent, and 1 when a write
//! failed.

mod launcher;
mod pane_env;
mod sources;
mod writes;

use std::path::{Path, PathBuf};

use clap::Args;
use holler_pane::import::files::PaneFiles;
use holler_pane::import::findings::{Finding, ImportFinding};
use holler_pane::import::{plan, Action, FleetPlan, ImportRequest, PaneRow};
use holler_pane::{PaneError, PaneName, Ports, Profile};
use serde::Serialize;

use super::list::{optional_text, NO_VALUE};
use crate::output::{emit, ErrorBody, VerbCtx};
use crate::profile::list::count;
use crate::profile::show::push_spec;
use launcher::LauncherTable;
use sources::Listing;

/// The launcher table's file in the state root, when `--launcher` does not name one.
const LAUNCHER_FILE: &str = "launcher.toml";

/// What separates two fields of a pane's line in the text form.
const GAP: &str = "  ";

/// Import the panes the old launcher runs into the pane registry, as the profile `fleet`.
///
/// STATE_ROOT holds one directory per pane, named as the pane, with the launcher's files:
/// `pane.env` (the model, effort, context ceilings and agent, and the names of every other
/// variable, never a value), `last-session`, `sessions.toml` and `owned-sessions` (the session
/// each names). The launcher table (default STATE_ROOT/launcher.toml) holds, per pane, a TOML
/// table named as the pane with `port`, `cwd` (an absolute path) and `command` (an array of
/// strings); a pane that runs Claude Code needs no `port`. A pane is joined to the Herdr pane in --herdr-session that carries its name as its
/// label, and takes its position from there, never from its name. Its session of record is the
/// one `last-session` names, when every other file names it too and its healthy server has it.
/// Every pane is an agent unless --orchestrator names it.
///
/// A pane whose tmux session runs Claude Code is left alone. The old files cannot tell it from
/// an OpenCode pane, so the import reads the command name of the foreground process of each
/// tmux pane in the pane's tmux session (a name only, never a command line). A pane whose tmux
/// session has a foreground process named exactly `claude` gets the finding harness-claude-code
/// (info), the action `skipped` and no record, and blocks nothing; `pane doctor` then reports
/// it as unregistered-herdr-pane. Any other name (`bash`, which the launcher's OpenCode panes
/// show), or no process, is imported as OpenCode. The rule is a default, not a proof: tmux
/// names only the foreground process, so a Claude Code pane is taken for OpenCode when its
/// `claude` has exited (it shows its shell), is not in the foreground (a wrapper script runs
/// it, or it is suspended) or was started under another name. Read the dry run first. A pane
/// whose tmux session cannot be read gets the problem harness-unreadable and is never recorded
/// on a guess.
///
/// --adopt-claude-code adopts those panes too: each gets a record with the Claude Code harness
/// (claude), the pid of its `claude` process and the session that process holds, both read from
/// Claude Code's own listing by pid (never from the old files), and the finding
/// claude-code-adopted (info), which names the next step, `holler pane relaunch NAME`. A pane is
/// never adopted on a guess: no such process, more than one, or one holding no session is
/// claude-process-unmatched, and a pane.env variable that changes Claude Code's login is
/// claude-login-env (both problems). A pane whose sessions.toml names another harness stays
/// skipped. With the switch, a run whose profile `fleet` exists extends it with the specs of the
/// panes it records, Claude Code or OpenCode, by one compare-and-swap, instead of refusing it.
///
/// The run never starts, stops, types into or moves anything, and changes no file. With
/// --dry-run it writes nothing and reports what a real run would write. A real run writes the
/// profile `fleet` and one record per pane (with the switch and an existing `fleet`, the records
/// and then `fleet`), or nothing at all: it is refused (import-blocked) while any pane has a
/// problem, and a failed write is undone. Running it again after an import writes nothing.
///
/// The text form has one line per pane, `PANE ACTION ROLE POS SESSION` (`-` for none), each
/// followed by its findings as `  LEVEL KIND: MESSAGE`; findings about no pane come first,
/// under `-`. A dry run then prints the profile it would create, spec by spec, as `profile
/// show` does, or the specs it would add to an existing `fleet`. A last line sums up.
///
/// `--format=json` prints one envelope whose data is `{"dry_run", "state_root", "recorded",
/// "profile", "panes", "findings"}`: `recorded` is the number of records written; `profile` is
/// the profile a dry run would create (generation 0) or extend (at the generation read, with the
/// specs it would add), or the one a real run stored, or null;
/// each of `panes` is `{"name", "action", "role", "pos", "session_of_record"}`, `action` being
/// `record`, `already-imported`, `skipped` or `blocked`; each of `findings` is `{"pane",
/// "level", "kind", "message"}`, `level` being `problem` (blocks), `warning`, `skipped` or
/// `info`.
#[derive(Args, Debug)]
pub struct PaneImport {
    /// The directory that holds one directory per pane (the launcher's state root).
    #[arg(long, value_name = "STATE_ROOT")]
    pub from: String,
    /// The Herdr session the panes live in. Required.
    #[arg(long, value_name = "NAME")]
    pub herdr_session: Option<String>,
    /// The launcher's port and directory table (default: STATE_ROOT/launcher.toml).
    #[arg(long, value_name = "FILE")]
    pub launcher: Option<String>,
    /// A pane that is the orchestrator; repeat for each. Every other pane is an agent.
    #[arg(long, value_name = "PANE")]
    pub orchestrator: Vec<String>,
    /// Report what a real run would write, and write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Also adopt the panes whose tmux session runs Claude Code, with the Claude Code harness,
    /// and extend the profile fleet when it exists instead of refusing it.
    #[arg(long)]
    pub adopt_claude_code: bool,
}

/// Run `holler pane import`: read the files, plan, write (unless `--dry-run`) and print the
/// report once.
pub fn run(args: &PaneImport, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = import(args, ctx.ports);
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `pane import`. Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize)]
struct Imported {
    dry_run: bool,
    /// `--from`, as given.
    state_root: String,
    /// The number of records written (0 in a dry run).
    recorded: usize,
    /// The profile a dry run would create or extend, or the one a real run stored.
    profile: Option<Profile>,
    panes: Vec<PaneRow>,
    findings: Vec<Finding>,
    /// When the run extends an existing `fleet`, the panes whose specs it adds (the text form
    /// says so; the JSON has no such key).
    #[serde(skip)]
    extended: Option<Vec<PaneName>>,
}

/// Steps 1 to 4 of the module docs.
fn import(args: &PaneImport, ports: Ports<'_>) -> Result<Imported, ErrorBody> {
    let body = |error: PaneError| ErrorBody::from(&error);
    let input = Input::read(args).map_err(body)?;
    let planned = plan(ports, &input.request()).map_err(body)?;
    let extended = match &planned.fleet {
        FleetPlan::Extend { added, .. } => Some(added.clone()),
        FleetPlan::None | FleetPlan::Create(_) => None,
    };
    let (profile, recorded) = if args.dry_run {
        (planned.fleet.profile().cloned(), 0)
    } else {
        writes::write(&planned, ports)?.map_or((None, 0), |written| {
            (Some(written.profile), written.recorded)
        })
    };
    Ok(Imported {
        dry_run: args.dry_run,
        state_root: args.from.clone(),
        recorded,
        profile,
        panes: planned.panes,
        findings: planned.findings,
        extended,
    })
}

/// What the plan is asked about: the typed arguments and every pane directory's files.
struct Input {
    herdr_session: String,
    orchestrators: Vec<PaneName>,
    panes: Vec<PaneFiles>,
    others: Vec<Finding>,
    adopt: bool,
}

impl Input {
    /// Step 1, then step 2.
    fn read(args: &PaneImport) -> Result<Self, PaneError> {
        let herdr_session = args
            .herdr_session
            .clone()
            .filter(|session| !session.trim().is_empty())
            .ok_or_else(|| {
                usage("--herdr-session is required: name the Herdr session the panes live in")
            })?;
        let root = Path::new(&args.from);
        if !root.is_dir() {
            return Err(usage(&format!(
                "--from {:?} is not a directory: name the directory that holds one directory \
                 per pane",
                args.from
            )));
        }
        let listing = sources::list(root)?;
        let orchestrators = orchestrators(&args.orchestrator, &listing, &args.from)?;
        let launcher_path = args
            .launcher
            .as_ref()
            .map_or_else(|| root.join(LAUNCHER_FILE), PathBuf::from);
        let table = LauncherTable::read(&launcher_path);
        let panes = listing
            .panes
            .iter()
            .map(|(name, dir)| sources::read(name, dir, &table))
            .collect();
        Ok(Self {
            herdr_session,
            orchestrators,
            panes,
            others: listing.others,
            adopt: args.adopt_claude_code,
        })
    }

    fn request(&self) -> ImportRequest<'_> {
        ImportRequest {
            herdr_session: &self.herdr_session,
            orchestrators: &self.orchestrators,
            panes: &self.panes,
            others: &self.others,
            adopt: self.adopt,
        }
    }
}

/// The panes `--orchestrator` names, each a pane name (`usage`) of a pane directory (`usage`).
fn orchestrators(
    given: &[String],
    listing: &Listing,
    root: &str,
) -> Result<Vec<PaneName>, PaneError> {
    given
        .iter()
        .map(|text| {
            let name = PaneName::parse(text)?;
            if listing.panes.iter().any(|(pane, _)| *pane == name) {
                Ok(name)
            } else {
                Err(usage(&format!(
                    "--orchestrator {name} names no pane directory in {root:?}"
                )))
            }
        })
        .collect()
}

fn usage(message: &str) -> PaneError {
    PaneError::Usage {
        message: message.to_owned(),
    }
}

/// The text form (see [`PaneImport`]).
fn render(imported: &Imported) -> String {
    let mut lines = table(&imported.panes, &imported.findings);
    if imported.dry_run {
        if let Some(profile) = &imported.profile {
            push_profile(&mut lines, profile, imported.extended.as_deref());
        }
    }
    lines.push(summary(imported));
    lines.join("\n")
}

/// The profile a dry run would create, every spec of it; or, for an existing `fleet`, the
/// specs it would add (`added`).
fn push_profile(lines: &mut Vec<String>, profile: &Profile, added: Option<&[PaneName]>) {
    let Some(added) = added else {
        lines.push(format!(
            "profile {} ({}) to create: {}",
            profile.name,
            profile.slug,
            count(profile.panes.len(), "spec")
        ));
        for spec in &profile.panes {
            push_spec(lines, spec);
        }
        return;
    };
    lines.push(format!(
        "profile {} ({}) to extend: {} added, {} total",
        profile.name,
        profile.slug,
        count(added.len(), "spec"),
        profile.panes.len()
    ));
    let added_specs = profile
        .panes
        .iter()
        .filter(|spec| added.iter().any(|name| name.as_str() == spec.pane));
    for spec in added_specs {
        push_spec(lines, spec);
    }
}

/// The findings about no pane under a `-` line, then each pane's line and its findings.
fn table(rows: &[PaneRow], findings: &[Finding]) -> Vec<String> {
    let about = |pane: Option<&PaneName>| -> Vec<String> {
        findings
            .iter()
            .filter(|finding| finding.pane.as_ref() == pane)
            .map(finding_line)
            .collect()
    };
    let mut lines = Vec::new();
    if findings.iter().any(|finding| finding.pane.is_none()) {
        lines.push(NO_VALUE.to_owned());
        lines.extend(about(None));
    }
    for row in rows {
        lines.push(pane_line(row));
        lines.extend(about(Some(&row.name)));
    }
    lines
}

/// `PANE  ACTION  ROLE  POS  SESSION`, `-` for none. A pane name prints as it is (ADR 0005's
/// grammar); a session id read from a file is quoted when it is not plain.
fn pane_line(row: &PaneRow) -> String {
    let role = row
        .role
        .and_then(|role| serde_json::to_value(role).ok())
        .and_then(|role| role.as_str().map(str::to_owned));
    [
        row.name.to_string(),
        row.action.as_str().to_owned(),
        role.unwrap_or_else(|| NO_VALUE.to_owned()),
        row.pos
            .map_or_else(|| NO_VALUE.to_owned(), |pos| pos.to_string()),
        optional_text(row.session_of_record.as_deref()),
    ]
    .join(GAP)
}

/// `  LEVEL KIND: MESSAGE`. A message is one line, with its untrusted values already quoted.
fn finding_line(finding: &Finding) -> String {
    format!(
        "  {} {}: {}",
        finding.level().as_str(),
        finding.kind.code(),
        finding.message
    )
}

/// The last line: what the run did.
fn summary(imported: &Imported) -> String {
    let actions = |action: Action| {
        imported
            .panes
            .iter()
            .filter(|row| row.action == action)
            .count()
    };
    let (record, blocked) = (actions(Action::Record), actions(Action::Blocked));
    let left = format!(
        "{} already imported, {} skipped",
        actions(Action::AlreadyImported),
        actions(Action::Skipped)
    );
    let adopted = match imported
        .findings
        .iter()
        .filter(|finding| finding.kind == ImportFinding::ClaudeCodeAdopted)
        .count()
    {
        0 => String::new(),
        n => format!(" ({n} adopted)"),
    };
    let written = match (&imported.profile, &imported.extended) {
        (Some(profile), None) => format!(
            "created profile {} ({}): {}, generation {}",
            profile.name,
            profile.slug,
            count(profile.panes.len(), "spec"),
            profile.generation
        ),
        (Some(profile), Some(added)) => format!(
            "extended profile {} ({}): {} added, {} total, generation {}",
            profile.name,
            profile.slug,
            count(added.len(), "spec"),
            profile.panes.len(),
            profile.generation
        ),
        (None, _) => String::new(),
    };
    match (imported.profile.is_some(), imported.dry_run) {
        (_, true) => format!(
            "dry run: {record} to record{adopted}, {blocked} blocked, {left}; nothing was written"
        ),
        (true, false) => format!(
            "recorded {}{adopted}; {written}",
            count(imported.recorded, "pane")
        ),
        (false, false) => format!("nothing to record: {left}; nothing was written"),
    }
}
