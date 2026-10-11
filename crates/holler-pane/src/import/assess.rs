//! The join, per pane (#650): what one pane directory becomes, from its files, Herdr's
//! snapshot and what its chain observed. Pure: the port calls are `observe`'s.
//!
//! 1. **The Herdr pane** is the snapshot entry in the run's Herdr session whose label is the
//!    pane's name. None is `pane-unlabelled`, two or more `pane-label-ambiguous`; either
//!    leaves the pane unplaced. The record takes the entry with its label cleared (a label is
//!    an observation, never recorded state, as launch records it).
//! 2. **The position** is the entry's cell. A name `hj-<cell>` whose cell differs is only
//!    information (`name-cell-differs`).
//! 3. **The tmux session and the server**: `observe`'s findings.
//! 4. **The session of record** is `last-session`, when every other file present names it
//!    and the healthy server's list holds it. Recency is never a tie-break, and a session id
//!    read from a file is never recorded as observed (I6).
//! 5. **The role** is the orchestrator's for a pane named by `--orchestrator`.
//! 6. **The command** is the launcher entry's argv; a string is `command-not-argv`.
//!
//! A pane with a problem gets no record. Every other pane gets the record a real run writes,
//! at generation 0, in the profile `fleet`.

use crate::findings::quoted;
use crate::pane::{
    HarnessInfo, HarnessKind, Health, Hold, HostInfo, LastObserved, PaneProbe, PaneRole,
};
use crate::tx_launch::HOST_NAME;
use crate::{Argv, GridPos, HerdrPane, Pane, PaneName, Ports, ProfileName};

use super::files::{FileState, LauncherCommand, PaneFiles, SessionFiles};
use super::findings::{Finding, ImportFinding, Level};
use super::observe::{observe_all, tmux_finding, Observed, Target};
use super::{Action, PaneRow};

/// The prefix of the old launcher's pane names, `hj-<cell>` (`hj-c1r2`): the only names whose
/// cell the import compares with Herdr's, for information.
const CELL_NAME_PREFIX: &str = "hj-";

/// What every pane of a run is joined against.
pub(super) struct Context<'a> {
    /// The Herdr session the run names (`--herdr-session`).
    pub session: &'a str,
    /// Every pane Herdr's snapshot listed, in every session.
    pub snapshot: &'a [HerdrPane],
    /// Herdr's version, recorded as `host.herdr_api_version`.
    pub version: &'a str,
    /// The panes `--orchestrator` names.
    pub orchestrators: &'a [PaneName],
    /// The profile every record joins.
    pub fleet: &'a ProfileName,
}

/// What one pane directory became.
pub(super) struct Outcome {
    pub row: PaneRow,
    pub findings: Vec<Finding>,
    /// The record a real run writes; `None` unless the row's action is `record`.
    pub record: Option<Pane>,
}

impl Outcome {
    /// A pane that is left alone: no position, role or session, and one finding.
    fn left_alone(name: &PaneName, action: Action, kind: ImportFinding, message: &str) -> Self {
        Self {
            row: PaneRow {
                name: name.clone(),
                action,
                role: None,
                pos: None,
                session_of_record: None,
            },
            findings: vec![Finding::about(name, kind, message.to_owned())],
            record: None,
        }
    }

    /// A pane that already has a record: skipped before any other check.
    pub(super) fn already_imported(name: &PaneName) -> Self {
        let message = "the pane already has a record, so the import leaves it as it is";
        Self::left_alone(
            name,
            Action::AlreadyImported,
            ImportFinding::AlreadyImported,
            message,
        )
    }

    /// A pane that is not an OpenCode pane: no record, never blocking.
    pub(super) fn not_opencode(name: &PaneName) -> Self {
        let message = "its sessions.toml names a harness that is not OpenCode, and only an \
                       OpenCode pane is imported";
        Self::left_alone(
            name,
            Action::Skipped,
            ImportFinding::HarnessNotOpencode,
            message,
        )
    }
}

/// Observe every pane of `panes` at once, then join each.
pub(super) fn assess_all(
    ports: Ports<'_>,
    context: &Context<'_>,
    panes: &[&PaneFiles],
) -> Vec<Outcome> {
    let targets: Vec<Target<'_>> = panes
        .iter()
        .map(|files| Target {
            name: &files.name,
            port: files.launcher.as_ref().map(|entry| entry.port),
        })
        .collect();
    let observed = observe_all(ports, &targets);
    panes
        .iter()
        .zip(observed)
        .map(|(files, seen)| assess(context, files, &seen))
        .collect()
}

/// What a pane's chain gave the record.
struct Seen {
    health: Health,
    session: Option<String>,
}

/// Join one pane: its findings, its row and (when it has no problem) its record.
fn assess(context: &Context<'_>, files: &PaneFiles, observed: &Observed) -> Outcome {
    let name = &files.name;
    let mut findings = files.problems.clone();
    let herdr = join(context, name, &mut findings);
    let port = files.launcher.as_ref().map(|entry| entry.port);
    let (health, listed) = observations(name, port, observed, &mut findings);
    let session = session_of_record(name, port, &files.sessions, listed, &mut findings);
    let command = command_of(files, &mut findings);
    let role = if context.orchestrators.contains(name) {
        PaneRole::Orchestrator
    } else {
        PaneRole::Agent
    };
    let pos = herdr.as_ref().map(|pane| pane.grid);
    let blocked = findings.iter().any(|f| f.level() == Level::Problem);
    let seen = Seen { health, session };
    let record = herdr
        .filter(|_| !blocked)
        .and_then(|herdr| record(context, files, herdr, seen, role, command));
    let row = PaneRow {
        name: name.clone(),
        action: if record.is_some() {
            Action::Record
        } else {
            Action::Blocked
        },
        role: Some(role),
        pos,
        session_of_record: record.as_ref().and_then(|r| r.session_of_record.clone()),
    };
    Outcome {
        row,
        findings,
        record,
    }
}

/// The Herdr panes in the run's session that carry `name` as their label.
fn labelled<'s>(context: &Context<'s>, name: &PaneName) -> Vec<&'s HerdrPane> {
    context
        .snapshot
        .iter()
        .filter(|pane| pane.session == context.session)
        .filter(|pane| pane.label.as_deref() == Some(name.as_str()))
        .collect()
}

/// Step 1 and 2: the Herdr pane labelled `name`, with its label cleared, or the finding that
/// says why there is none; and `name-cell-differs` for a name that spells another cell.
fn join(context: &Context<'_>, name: &PaneName, findings: &mut Vec<Finding>) -> Option<HerdrPane> {
    let session = quoted(context.session);
    match labelled(context, name).as_slice() {
        [] => {
            let message = format!(
                "no Herdr pane in session {session} has the label {name}, so the pane has no \
                 place to be recorded at"
            );
            findings.push(Finding::about(name, ImportFinding::PaneUnlabelled, message));
            None
        }
        [one] => {
            findings.extend(name_cell(name, one.grid));
            Some(HerdrPane {
                label: None,
                ..(*one).clone()
            })
        }
        many => {
            let ids: Vec<String> = many
                .iter()
                .map(|pane| quoted(pane.pane_id.as_str()))
                .collect();
            let message = format!(
                "Herdr panes {} in session {session} all have the label {name}, so the import \
                 cannot tell which one is the pane",
                ids.join(", ")
            );
            findings.push(Finding::about(
                name,
                ImportFinding::PaneLabelAmbiguous,
                message,
            ));
            None
        }
    }
}

/// `name-cell-differs` when `name` is `hj-<cell>` and that cell is not `grid`; a name that
/// spells no cell says nothing.
fn name_cell(name: &PaneName, grid: GridPos) -> Option<Finding> {
    let spelled = name
        .as_str()
        .strip_prefix(CELL_NAME_PREFIX)
        .and_then(|cell| GridPos::parse(cell).ok())?;
    (spelled != grid).then(|| {
        let message = format!(
            "the name spells the cell {spelled}, and Herdr shows the pane at {grid}; the record \
             takes Herdr's position"
        );
        Finding::about(name, ImportFinding::NameCellDiffers, message)
    })
}

/// Step 3: the findings of what the chain observed, the health the record takes and the
/// sessions of a healthy server whose list was read.
fn observations<'o>(
    name: &PaneName,
    port: Option<u16>,
    observed: &'o Observed,
    findings: &mut Vec<Finding>,
) -> (Health, Option<&'o [String]>) {
    let Observed::Seen { tmux, server } = observed else {
        let message = "the pane was not observed: its check did not finish".to_owned();
        findings.push(Finding::about(name, ImportFinding::ObserveFailed, message));
        return (Health::Unknown, None);
    };
    findings.extend(tmux_finding(name, tmux));
    let (Some(server), Some(port)) = (server, port) else {
        return (Health::Unknown, None);
    };
    findings.extend(server.finding(name, port));
    (server.health(), server.sessions())
}

/// What the three session files say together.
enum Agreement<'f> {
    /// A file could not be read: its own problem is reported, and nothing is judged.
    Unjudged,
    /// `last-session` is absent or empty.
    NotRecorded,
    /// `last-session` names a session these present files do not.
    Disagree(Vec<&'static str>),
    /// `last-session` names this session, and every other file present names it too.
    Agree(&'f str),
}

/// The files agree when `last-session` is present and every other present file names it.
fn agreement(files: &SessionFiles) -> Agreement<'_> {
    let last = match &files.last {
        FileState::Unreadable => return Agreement::Unjudged,
        FileState::Absent => return Agreement::NotRecorded,
        FileState::Present(last) => last.as_str(),
    };
    let mut disagree = Vec::new();
    match &files.table {
        FileState::Unreadable => return Agreement::Unjudged,
        FileState::Present(table) if !table.session_ids.iter().any(|id| id == last) => {
            disagree.push("sessions.toml");
        }
        FileState::Present(_) | FileState::Absent => {}
    }
    match &files.owned {
        FileState::Unreadable => return Agreement::Unjudged,
        FileState::Present(owned) if !owned.iter().any(|id| id == last) => {
            disagree.push("owned-sessions");
        }
        FileState::Present(_) | FileState::Absent => {}
    }
    if disagree.is_empty() {
        Agreement::Agree(last)
    } else {
        Agreement::Disagree(disagree)
    }
}

/// Step 4: the session of record, or the finding that says why there is none. `listed` is
/// the healthy server's list; when there is none, the server's own finding says why.
fn session_of_record(
    name: &PaneName,
    port: Option<u16>,
    files: &SessionFiles,
    listed: Option<&[String]>,
    findings: &mut Vec<Finding>,
) -> Option<String> {
    let last = match agreement(files) {
        Agreement::Unjudged => return None,
        Agreement::NotRecorded => {
            let message = "last-session is absent or empty, so the record gets no session of \
                           record";
            findings.push(Finding::about(
                name,
                ImportFinding::NoSessionRecorded,
                message.to_owned(),
            ));
            return None;
        }
        Agreement::Disagree(others) => {
            let message = format!(
                "the session last-session names is not in {}; the files must name one session \
                 before the pane is recorded",
                others.join(" or ")
            );
            findings.push(Finding::about(
                name,
                ImportFinding::SourcesDisagree,
                message,
            ));
            return None;
        }
        Agreement::Agree(last) => last,
    };
    let (ids, port) = (listed?, port?);
    if ids.iter().any(|id| id == last) {
        return Some(last.to_owned());
    }
    let message = format!(
        "the harness server on port {port} does not have the session {} that the files name, so \
         the record gets no session of record",
        quoted(last)
    );
    findings.push(Finding::about(
        name,
        ImportFinding::SessionNotFound,
        message,
    ));
    None
}

/// Step 6: the launcher entry's command as an argv; a string is `command-not-argv`.
fn command_of(files: &PaneFiles, findings: &mut Vec<Finding>) -> Option<Argv> {
    match files.launcher.as_ref().map(|entry| &entry.command) {
        Some(LauncherCommand::Argv(argv)) => Some(argv.clone()),
        Some(LauncherCommand::Shell) => {
            let message = "the launcher table gives the command as a string, and a stored \
                           command is an argv array, so the pane is recorded without one";
            findings.push(Finding::about(
                &files.name,
                ImportFinding::CommandNotArgv,
                message.to_owned(),
            ));
            None
        }
        Some(LauncherCommand::Absent) | None => None,
    }
}

/// The record a real run writes for `files`: what the files and this run's observations
/// give, `generation` 0, in the profile `fleet`, with nothing observed yet (`at` 0: the first
/// doctor pass fills it). `None` when `pane.env` or the launcher entry was not read whole
/// (a problem then says why).
fn record(
    context: &Context<'_>,
    files: &PaneFiles,
    herdr: HerdrPane,
    seen: Seen,
    role: PaneRole,
    command: Option<Argv>,
) -> Option<Pane> {
    let (env, launcher) = (files.env.as_ref()?, files.launcher.as_ref()?);
    Some(Pane {
        name: files.name.clone(),
        generation: 0,
        herdr,
        host: HostInfo {
            name: HOST_NAME.to_owned(),
            tmux: files.name.as_str().to_owned(),
            cwd: launcher.cwd.clone(),
            herdr_api_version: Some(context.version.to_owned()),
        },
        harness: HarnessInfo {
            kind: HarnessKind::Opencode,
            port: launcher.port,
            pid: None,
            health: seen.health,
        },
        session_of_record: seen.session,
        role,
        hold: Hold::None,
        last_observed: LastObserved {
            shown: None,
            driven: None,
            at: 0,
        },
        profile: Some(context.fleet.clone()),
        model: env.model.clone(),
        opencode_agent: env.opencode_agent.clone(),
        env: env.env.clone(),
        context: env.context,
        command,
        probe: PaneProbe {
            check: None,
            expect: Vec::new(),
            last: None,
        },
    })
}
