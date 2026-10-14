//! `holler roster` (issue #186), split out of `main.rs` to keep that file under the workspace's
//! 900-line build guard (`scripts/lint.sh` check 4), the same split `say_cmd.rs` (issue #190) made
//! for `say` and `query_cmd.rs` (issue #185) made for `hub query`.
//!
//! [`run`] makes the hub call, then hands the hub's reply and the pane registry to [`render`], the
//! pure half, which prints. Both print to an injected [`Sink`] and return the exit code, which
//! `main.rs`, the one file allowed to exit the process, applies. (Before story #648 this module
//! returned a `RosterResult` for `main.rs` to print.)
//!
//! **The pane columns (story #648, epic #633).** A hub row is joined to a pane record when the part
//! of its name before the first `/` is the pane's name (`pane_label`); the row then carries six of
//! the cells `holler pane list` prints for that record ([`PANE_COLUMNS`]). They are the registry's
//! values, read on each call, and the roster calls no adapter. When a pane's recorded SHOWN and
//! DRIVEN are both known and differ (`shown_driven_mismatch`), the text output's first line names
//! it. The registry is the local hub's: it is read only after the hub call answered, and never
//! with `--server`, which names another hub.
//!
//! **Three output modes**, from the resolved [`FormatChoice`]: text, the table; a bare `--json`,
//! the roster's own `{"rows": [...]}` with the six pane keys and (#914) `backgroundAgentLastOutput`
//! added to each row (ADR 0003: additive is MINOR); and an explicit `--format=json`, one envelope whose `data` is `{"rows": [...],
//! "mismatches": [...]}`. A usage error is one `error:` line on stderr in every mode, as clap's own
//! are under `roster`; any other error goes through [`emit_error`], an envelope only under
//! `--format=json`. A registry that could not be read is one `note:` line on stderr in every mode,
//! beside the rows.

use std::path::Path;

use holler_hub::control::{ControlCall, ControlError};
use holler_pane::error::{class_of, ErrorClass, RefusalCode};
use holler_pane::pane::Health;
use holler_pane::{GridPos, Pane, PaneError, PaneName, Ports, ProfileName};
use serde::Serialize;
use serde_json::Value;

use crate::output::{emit, emit_error, ErrorBody, ErrorCode, Format, FormatChoice, Sink};
use crate::pane::list::{profile_name, text_value, PaneRow, COLUMNS, NO_VALUE};
use crate::Roster;

/// The six columns the roster appends after HOLD, in order (story #648). Each is a column of
/// `holler pane list` ([`COLUMNS`]): a joined row's cell is the one `pane list` prints for the
/// same record, taken by the column's name, so a column that `pane list` adds or moves cannot
/// shift the roster's.
pub const PANE_COLUMNS: [&str; 6] = ["PANE", "POS", "PROFILE", "HEALTH", "SHOWN", "DRIVEN"];

/// What `holler roster --help` prints after the flags (story #648): the pane columns, and that a
/// pane's name is not its position. Help text, never a line of output.
pub const HELP_LEGEND: &str = "\
The six columns after HOLD are the pane registry's, the cells `holler pane list` prints for the
pane whose name is the part of SESSION before its first `/` (`-` for no value, and in all six
for a session with no pane):
  PANE     the pane's name
  POS      its grid cell, row first: `r2c1` is row 2, column 1. A pane's name is not its
           position: POS is read from Herdr, never from the name, so a pane named `hj-c1r2`
           that sits in column 1, row 2 keeps that name and shows POS `r2c1`
  PROFILE  the profile it belongs to
  HEALTH   healthy, unhealthy or unknown, as its harness server last reported it
  SHOWN    the session its TUI showed when reconcile last observed it
  DRIVEN   the session the hub drives, as its record holds it

When a pane's SHOWN and DRIVEN are both known and differ, the first line of the output says so:
  N pane(s) show a different session than the hub drives: NAME, ...

--profile NAME lists only the rows of that profile's panes; an unknown profile is refused
(profile-not-found, exit 3). The registry is the local hub's: with --server it is not read,
the six columns are `-`, and --profile is a usage error. --format=json prints one envelope
whose data is {\"rows\": [...], \"mismatches\": [...]}; --json keeps the roster's own
{\"rows\": [...]}, each row with the keys pane, pos, profile, health, shown and driven (null
for a session with no pane), and backgroundAgentLastOutput: the epoch seconds of the newest
sub-agent transcript write of a Claude row's session on this machine, else null.";

/// A `--server` that the transport refused before any dial (#508's AC 13: a plaintext `ws://` to
/// a host that is not loopback, an address that does not parse). An open code the roster owns
/// (ADR-0021 section 9), so a refusal, exit 3: today's exit code, which the failure envelope of
/// `--format=json` keeps. The text forms print today's line and no code.
pub const REMOTE_POLICY_REFUSED: RefusalCode = RefusalCode::from_static("remote-policy-refused");

/// What `--profile` with `--server` is told: the registry is the local hub's, so another hub's
/// rows cannot be narrowed to a profile.
const PROFILE_WITH_SERVER: &str = "--profile cannot be combined with --server";

/// The stderr line of a roster run with `--server`: the six columns are `-` because the registry
/// was not read, not because no pane joined.
const NOT_READ_NOTE: &str = "note: pane registry not read (--server names another hub)";

/// The width each column but the last is padded to, in order: today's eight, SESSION to LAST TURN
/// (#186, #142), then HOLD and the first five pane columns. HOLD was the last column before #648
/// and was not padded; DRIVEN is the last now and is not padded, so no line ends in a space.
const WIDTHS: [usize; 14] = [20, 10, 8, 10, 10, 8, 8, 20, 20, 20, 6, 12, 10, 20];

/// The table's columns before the pane columns: SESSION to HOLD, today's header.
const HUB_COLUMNS: [&str; 9] = [
    "SESSION", "HARNESS", "MODE", "STATE", "CONN", "HOSTNAME", "PENDING", "LAST TURN", "HOLD",
];

/// The pane registry as [`render`] receives it.
pub enum Registry<'a> {
    /// The registry's ports. [`render`] reads the pane store, or under `--profile` the profile
    /// scope, through them, and calls no other port.
    Ports(Ports<'a>),
    /// What the wiring answered instead of ports: `Wiring::connect` failed after the hub call had
    /// answered (it makes its own exchange with the hub).
    Failed(PaneError),
    /// Not read: `--server` names another hub, and the registry is the local hub's. Joining one
    /// hub's rows to another hub's records would print a pane's values against a session that is
    /// not the pane's.
    NotRead,
}

/// `holler roster [--all] [--prefix PREFIX] [--server URL] [--profile NAME]` (issue #186;
/// `--prefix`, #236; `--server`, #508; the pane columns and `--profile`, #648): read the hub's
/// roster over the control socket, or another hub's with `--server`, then print it with
/// [`render`]. Returns the exit code.
///
/// In order: a usage error found after parsing is refused before any dial (`--profile` with
/// `--server`, then a malformed profile name), exit 2. Then the hub call, as before #648: a failed
/// one prints today's message, coded `unavailable` (exit 1), or [`REMOTE_POLICY_REFUSED`] (exit
/// 3). Only after a reply, and never with `--server`, is `registry` called for the ports; it is
/// `main.rs`'s deferred `Wiring::connect`, so a hub that is down costs no second wait, and its
/// error goes to [`render`] as [`Registry::Failed`].
pub fn run<'a>(
    roster: &Roster,
    choice: FormatChoice,
    registry: impl FnOnce() -> Result<Ports<'a>, PaneError>,
    sink: &mut Sink<'_>,
) -> i32 {
    let mode = Mode::of(choice);
    if roster.profile.profile.is_some() && roster.server.is_some() {
        return fail(sink, mode, usage(PROFILE_WITH_SERVER));
    }
    if let Err(error) = profile_name(&roster.profile) {
        return fail(sink, mode, ErrorBody::from(&error));
    }
    let call = ControlCall::roster(roster.all, roster.prefix.as_deref());
    let reply = match crate::transport::call(roster.server.as_deref(), &call) {
        Ok(reply) => reply,
        Err(error) => return fail(sink, mode, hub_error(&error)),
    };
    let registry = if roster.server.is_some() {
        Registry::NotRead
    } else {
        registry().map_or_else(Registry::Failed, Registry::Ports)
    };
    render(&reply, registry, roster, choice, sink)
}

/// Print the hub's `control/roster` reply (`{"rows": [...]}`) joined to the pane registry, and
/// return the exit code: the pure half of [`run`], which the tests call with a literal reply.
///
/// The rows are printed in the reply's order, each with the pane it is joined to, if any. With
/// `--profile P`, the registry read is P's panes through the profile scope, and only the rows
/// joined to one of them are printed; its error is the roster's (an unknown P is
/// `profile-not-found`, exit 3, and a registry that cannot answer fails the run, exit 1, rather
/// than list no row, which would read as "P has no pane"). Without it, the read is every record of
/// the pane store, and an error only leaves the six columns empty: `not-implemented` silently, any
/// other error with one `note:` line naming its code. The exit code is then 0.
pub fn render(
    reply: &Value,
    registry: Registry<'_>,
    roster: &Roster,
    choice: FormatChoice,
    sink: &mut Sink<'_>,
) -> i32 {
    // The Claude projects directory is this machine's: another hub's rows (`--server`) name
    // sessions of other machines, so no lookup is made for them.
    let projects = match registry {
        Registry::NotRead => None,
        _ => crate::roster_agent::ambient_projects_dir(),
    };
    render_in(reply, registry, roster, choice, sink, projects.as_deref())
}

/// [`render`] with the Claude projects directory given (issue #914): `None` makes every row's
/// `backgroundAgentLastOutput` null. The tests pass a temp directory.
pub fn render_in(
    reply: &Value,
    registry: Registry<'_>,
    roster: &Roster,
    choice: FormatChoice,
    sink: &mut Sink<'_>,
    projects: Option<&Path>,
) -> i32 {
    let mode = Mode::of(choice);
    let profile = match profile_name(&roster.profile) {
        Ok(profile) => profile,
        Err(error) => return fail(sink, mode, ErrorBody::from(&error)),
    };
    let registered = match read_registry(registry, profile.as_ref()) {
        Ok(registered) => registered,
        Err(error) => return fail(sink, mode, ErrorBody::from(&error)),
    };
    if let Some(note) = &registered.note {
        // Said, never hidden, and the rows still print. A note nobody received changes no exit.
        let _ = writeln!(sink.err, "{note}");
    }
    let hub_rows = reply.get("rows").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let rows: Vec<Joined<'_>> = hub_rows
        .iter()
        .map(|hub| Joined {
            hub,
            pane: pane_of(hub, &registered.panes),
        })
        .filter(|row| profile.is_none() || row.pane.is_some())
        .collect();
    let mismatches = mismatches(&rows);
    print(sink, mode, reply, &rows, &mismatches, profile.as_ref(), projects)
}

/// The roster's three output modes (#648), from the resolved format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The table, with the summary line before it when a pane is a mismatch.
    Text,
    /// A bare `--json`: the roster's own document, as before #648 with the pane keys added.
    Legacy,
    /// An explicit `--format=json`: one envelope (#670 reserved `json_explicit` for this).
    Envelope,
}

impl Mode {
    fn of(choice: FormatChoice) -> Self {
        match (choice.format, choice.json_explicit) {
            (Format::Text, _) => Self::Text,
            (Format::Json, false) => Self::Legacy,
            (Format::Json, true) => Self::Envelope,
        }
    }
}

/// Report `error` and return its exit code. A usage error is one `error:` line on stderr in every
/// mode, as clap's own are under `roster` (its usage errors are not enveloped, #670); any other
/// goes through [`emit_error`]: the failure envelope under `--format=json`, and today's `error:`
/// line in text and under a bare `--json`.
fn fail(sink: &mut Sink<'_>, mode: Mode, error: ErrorBody) -> i32 {
    let usage = class_of(error.code.as_str()) == ErrorClass::Usage;
    let format = if mode == Mode::Envelope && !usage {
        Format::Json
    } else {
        Format::Text
    };
    emit_error(sink, format, error)
}

/// A usage error (exit 2) with `message`.
fn usage(message: &str) -> ErrorBody {
    ErrorBody::from(&PaneError::Usage {
        message: message.to_owned(),
    })
}

/// A failed hub call as the roster has always reported it (#186, #508): today's message, coded
/// `unavailable` (exit 1), or [`REMOTE_POLICY_REFUSED`] (exit 3) for a `--server` refused before
/// any dial. The codes matter only to the envelope; the text forms print the message alone.
fn hub_error(error: &ControlError) -> ErrorBody {
    let message = match error {
        ControlError::NoLiveHub => {
            let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
            format!("no live holler hub reachable at {}", state_root.display())
        }
        other => other.to_string(),
    };
    let coded = match error {
        ControlError::RemotePolicyRefused(_) => PaneError::Refused {
            code: REMOTE_POLICY_REFUSED,
            message: String::new(),
        },
        _ => PaneError::Unavailable {
            what: String::new(),
        },
    };
    ErrorBody {
        code: ErrorCode::from(&coded),
        message,
    }
}

/// What the hub rows are joined to: the registry's records in `pane list`'s row form, and the
/// `note:` line for stderr when the registry was not read or could not answer.
struct Registered {
    panes: Vec<PaneRow>,
    note: Option<String>,
}

impl Registered {
    fn of(panes: &[Pane]) -> Self {
        Self {
            panes: panes.iter().map(PaneRow::from).collect(),
            note: None,
        }
    }

    /// No record, because the registry could not answer (acceptance 8): `not-implemented` (a
    /// store with no registry behind it, which is no fault) says nothing, and any other error is
    /// one `note:` line naming its stable code.
    fn unread(error: &PaneError) -> Self {
        let note = (!matches!(error, PaneError::NotImplemented))
            .then(|| format!("note: pane registry unavailable ({})", error.code()));
        Self {
            panes: Vec::new(),
            note,
        }
    }
}

/// Read the pane registry for the rows: with `--profile P`, P's panes through the profile scope,
/// whose error is the roster's; without it, every record of the pane store, whose error only
/// empties the columns. No other port is called, and no adapter.
fn read_registry(
    registry: Registry<'_>,
    profile: Option<&ProfileName>,
) -> Result<Registered, PaneError> {
    match (registry, profile) {
        (Registry::Ports(ports), Some(profile)) => {
            Ok(Registered::of(&ports.scope.resolve(profile, None)?.panes))
        }
        (Registry::Ports(ports), None) => Ok(ports.pane_store.list().map_or_else(
            |error| Registered::unread(&error),
            |panes| Registered::of(&panes),
        )),
        (Registry::Failed(error), Some(_)) => Err(error),
        (Registry::Failed(error), None) => Ok(Registered::unread(&error)),
        // `run` refuses this pair before any dial; a direct caller gets the same usage error.
        (Registry::NotRead, Some(_)) => Err(PaneError::Usage {
            message: PROFILE_WITH_SERVER.to_owned(),
        }),
        (Registry::NotRead, None) => Ok(Registered {
            panes: Vec::new(),
            note: Some(NOT_READ_NOTE.to_owned()),
        }),
    }
}

/// A hub row the roster prints, and the pane it is joined to, if any.
struct Joined<'a> {
    hub: &'a Value,
    pane: Option<&'a PaneRow>,
}

/// The pane `hub` is joined to (acceptance 10): the one named like the row's [`pane_label`].
fn pane_of<'a>(hub: &Value, panes: &'a [PaneRow]) -> Option<&'a PaneRow> {
    let name = hub.get("name").and_then(Value::as_str)?;
    let label = pane_label(name);
    panes.iter().find(|pane| pane.name.as_str() == label)
}

/// The name of the pane a hub row named `name` is joined to (acceptance 10): the part of `name`
/// before its first `/`. That is the label of a row named `<label>/<session>` (ADR 0005; a label
/// has no `/`) and the whole name of a row with no label. So a labelled row is joined to the pane
/// named like its label, never like its session, and the pane's session of record plays no part.
///
/// This is the roster's relation between a row and a pane. The hub's prompt gate and `say --pane`
/// relate them by the harness session id the row's body advertises, which is the pane's session
/// of record or its recorded DRIVEN (ADR-0021 section 11, #839); this display join is #648's.
fn pane_label(name: &str) -> &str {
    name.split_once('/').map_or(name, |(label, _)| label)
}

/// Whether a pane's recorded SHOWN and DRIVEN are a mismatch: both known and different. One side
/// absent is never one. It is `holler_pane::gate::shown_driven_pair_differs` (#839), the one pair
/// rule of the prompt gate's predicate, which `say --pane`'s refusal and the hub's gate share. It
/// is not `reconcile::shown_differs`, which compares SHOWN with the session of record.
fn shown_driven_mismatch(shown: Option<&str>, driven: Option<&str>) -> bool {
    let pair = holler_pane::pane::LastObserved {
        shown: shown.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 0,
    };
    holler_pane::gate::shown_driven_pair_differs(&pair)
}

/// The panes the summary line names (scope item 3): each joined pane whose recorded pair is a
/// mismatch, once however many rows carry it, in the order the roster prints its rows.
fn mismatches<'a>(rows: &[Joined<'a>]) -> Vec<&'a PaneName> {
    let mut names: Vec<&PaneName> = Vec::new();
    for pane in rows.iter().filter_map(|row| row.pane) {
        let mismatch = shown_driven_mismatch(pane.shown.as_deref(), pane.driven.as_deref());
        if mismatch && !names.contains(&&pane.name) {
            names.push(&pane.name);
        }
    }
    names
}

/// Print the rows in `mode` and return the exit code: 0, or 1 when the output reached nobody.
fn print(
    sink: &mut Sink<'_>,
    mode: Mode,
    reply: &Value,
    rows: &[Joined<'_>],
    mismatches: &[&PaneName],
    profile: Option<&ProfileName>,
    projects: Option<&Path>,
) -> i32 {
    if mode == Mode::Text {
        return emit(sink, Format::Text, Ok(()), |()| text(rows, mismatches, profile));
    }
    let json = match json_rows(rows, projects) {
        Ok(json) => json,
        Err(cause) => {
            let _ = writeln!(sink.err, "error: cannot encode the result: {cause}");
            return 1;
        }
    };
    if mode == Mode::Envelope {
        let data = RosterData {
            rows: json,
            mismatches,
        };
        return emit(sink, Format::Json, Ok(data), |_| String::new());
    }
    // `roster --json`'s bytes are today's but for the six keys: no newline is added after it.
    let document = legacy_document(reply, json).to_string();
    let written = sink
        .out
        .write_all(document.as_bytes())
        .and_then(|()| sink.out.flush());
    i32::from(written.is_err())
}

/// The text output (acceptance 1, 2 and 9): the summary line when a pane is a mismatch, then the
/// table, today's columns with the six pane columns after HOLD. No row to print is one line, as
/// before #648; under `--profile` it names the profile, since the hub may have other sessions.
fn text(rows: &[Joined<'_>], mismatches: &[&PaneName], profile: Option<&ProfileName>) -> String {
    if rows.is_empty() {
        return match profile {
            Some(profile) => format!(
                "(no sessions of profile {} on the hub)\n",
                text_value(profile.as_str())
            ),
            None => "(no sessions on the hub)\n".to_owned(),
        };
    }
    let mut out = String::new();
    if !mismatches.is_empty() {
        let names: Vec<String> = mismatches.iter().map(|name| text_value(name.as_str())).collect();
        out.push_str(&format!(
            "{} pane(s) show a different session than the hub drives: {}\n",
            names.len(),
            names.join(", ")
        ));
    }
    let header = HUB_COLUMNS.iter().chain(&PANE_COLUMNS).map(|column| (*column).to_owned());
    out.push_str(&table_line(header));
    for row in rows {
        out.push_str(&table_line(hub_cells(row.hub).into_iter().chain(pane_cells(row.pane))));
    }
    out
}

/// One line of the table, ending in a newline: each cell but the last left-aligned to its column's
/// width (counted in characters) and followed by one space, the cadence of the table before #648;
/// the last cell is not padded.
fn table_line(cells: impl IntoIterator<Item = String>) -> String {
    let mut cells: Vec<String> = cells.into_iter().collect();
    let last = cells.pop().unwrap_or_default();
    let mut line = String::new();
    for (cell, width) in cells.iter().zip(WIDTHS) {
        line.push_str(&format!("{cell:<width$} "));
    }
    line.push_str(&last);
    line.push('\n');
    line
}

/// A hub row's cells, SESSION to HOLD, exactly as the table printed them before #648. Columns are
/// the row's display fields; a held permission is the `PENDING` column (the human view shows just
/// its count; `--json` carries the full array).
fn hub_cells(row: &Value) -> [String; 9] {
    let field = |key: &str| row.get(key).and_then(Value::as_str).unwrap_or("-").to_owned();
    let pending = row
        .get("pending")
        .and_then(Value::as_array)
        .map_or_else(|| "-".to_owned(), |items| items.len().to_string());
    [
        field("name"),
        field("harness"),
        field("mode"),
        field("state"),
        field("conn_state"),
        field("hostname"),
        pending,
        last_turn_display(row.get("last_turn")),
        hold_display(row),
    ]
}

/// A row's six pane cells: the cells `holler pane list` prints for the joined pane's record, each
/// taken at its column's position in `pane list`'s [`COLUMNS`], never at a fixed index, or `-` in
/// each for a row joined to no pane.
fn pane_cells(pane: Option<&PaneRow>) -> [String; 6] {
    let cells = pane.map(PaneRow::cells);
    PANE_COLUMNS.map(|name| {
        let at = COLUMNS.iter().position(|column| *column == name);
        cells
            .as_ref()
            .zip(at)
            .and_then(|(cells, at)| cells.get(at))
            .map_or_else(|| NO_VALUE.to_owned(), Clone::clone)
    })
}

/// The `data` of `roster --format=json` (scope item 5): exactly these two keys. `rows` is what
/// `roster --json` prints as its own `rows`; `mismatches` names the panes of the text summary line,
/// in the same order, and is `[]` when there is none.
#[derive(Serialize)]
struct RosterData<'a> {
    rows: Vec<Value>,
    mismatches: &'a [&'a PaneName],
}

/// The six pane keys of a JSON row (scope item 6), from the joined pane's [`PaneRow`], so each value
/// is in `pane list`'s JSON form (`pos` is `{"row": R, "col": C, "pos": "rRcC"}`, `health` is
/// `"healthy"`, `"unknown"` or `{"unhealthy": REASON}`); `null` for a value the record does not hold,
/// and in all six for a row joined to no pane. A key is never left out.
#[derive(Serialize)]
struct PaneKeys<'a> {
    pane: Option<&'a PaneName>,
    pos: Option<GridPos>,
    profile: Option<&'a ProfileName>,
    health: Option<&'a Health>,
    shown: Option<&'a str>,
    driven: Option<&'a str>,
}

impl<'a> PaneKeys<'a> {
    fn of(pane: Option<&'a PaneRow>) -> Self {
        Self {
            pane: pane.map(|pane| &pane.name),
            pos: pane.map(|pane| pane.pos),
            profile: pane.and_then(|pane| pane.profile.as_ref()),
            health: pane.map(|pane| &pane.health),
            shown: pane.and_then(|pane| pane.shown.as_deref()),
            driven: pane.and_then(|pane| pane.driven.as_deref()),
        }
    }
}

/// The JSON rows (acceptance 3): each hub row as it arrived, every key and value kept, with the six
/// [`PaneKeys`] added, and (#914) `backgroundAgentLastOutput`: epoch seconds for a Claude row whose
/// session has sub-agent output on this machine, else `null`. The key is never left out.
fn json_rows(rows: &[Joined<'_>], projects: Option<&Path>) -> Result<Vec<Value>, serde_json::Error> {
    rows.iter()
        .map(|row| {
            let mut json = row.hub.clone();
            if let (Value::Object(hub), Value::Object(keys)) =
                (&mut json, serde_json::to_value(PaneKeys::of(row.pane))?)
            {
                hub.extend(keys);
                hub.insert(AGENT_KEY.to_owned(), background_agent_last_output(row.hub, projects));
            }
            Ok(json)
        })
        .collect()
}

/// The JSON key of when a Claude pane's background agent last produced output (#914).
const AGENT_KEY: &str = "backgroundAgentLastOutput";

/// A row's [`AGENT_KEY`] value: only a Claude row with a session id is looked up, and a lookup
/// that cannot tell is `null`.
fn background_agent_last_output(hub: &Value, projects: Option<&Path>) -> Value {
    let claude = hub.get("harness").and_then(Value::as_str) == Some("claude");
    let session = hub.get("harness_session_id").and_then(Value::as_str);
    match (claude, session, projects) {
        (true, Some(session), Some(projects)) => {
            crate::roster_agent::last_output(projects, session).map_or(Value::Null, Value::from)
        }
        _ => Value::Null,
    }
}

/// `roster --json`'s document (scope item 6): the hub's reply as it arrived, with `rows` the rows
/// printed in their JSON form. It has no `mismatches`.
fn legacy_document(reply: &Value, rows: Vec<Value>) -> Value {
    let mut document = reply.clone();
    if let Value::Object(object) = &mut document {
        object.insert("rows".to_owned(), Value::Array(rows));
    }
    document
}

/// The `LAST TURN` column (issue #142): `<state> <stop_reason> <age> ago`
/// (e.g. `completed end_turn 2m ago`) once a turn has ended, or `-` before
/// the session's first turn / for a row that predates this field. `age` is
/// computed from `ended_at` via a plain epoch-seconds subtraction (the same
/// RFC 3339 shape the hub always emits — see `holler_hub::roster::
/// parse_rfc3339`'s own doc for why there is only one parser for it).
fn last_turn_display(last_turn: Option<&serde_json::Value>) -> String {
    let Some(lt) = last_turn.filter(|v| !v.is_null()) else {
        return "-".to_string();
    };
    let state = lt.get("state").and_then(|v| v.as_str()).unwrap_or("-");
    let stop_reason = lt.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("-");
    let ended_at = lt.get("ended_at").and_then(|v| v.as_str());
    let age = ended_at
        .and_then(holler_hub::roster::parse_rfc3339_secs_since)
        .map(|secs| if secs < 60 { format!("{secs}s ago") } else { format!("{}m ago", secs / 60) })
        .unwrap_or_else(|| "-".to_string());
    format!("{state} {stop_reason} {age}")
}

/// The `HOLD` column (issues #443, #460): `-` for a session that is not held,
/// else `held <age> ago` (`held (default) <age> ago` for a session that joined
/// held) and the reason when there is one, and `[+default]` when a default hold
/// sits under an operator hold. The row's `hold*` fields are absent unless
/// held; `--json` carries the exact since-time.
fn hold_display(row: &serde_json::Value) -> String {
    if row.get("hold").and_then(|v| v.as_bool()) != Some(true) {
        return "-".to_string();
    }
    let age = row
        .get("held_since")
        .and_then(|v| v.as_str())
        .and_then(holler_hub::roster::parse_rfc3339_secs_since)
        .map(|secs| if secs < 60 { format!("{secs}s ago") } else { format!("{}m ago", secs / 60) })
        .unwrap_or_else(|| "-".to_string());
    let kind = if row.get("hold_kind").and_then(|v| v.as_str()) == Some("default") { " (default)" } else { "" };
    let under = if row.get("hold_default").and_then(|v| v.as_bool()) == Some(true) { " [+default]" } else { "" };
    match row.get("hold_reason").and_then(|v| v.as_str()) {
        Some(reason) => format!("held{kind} {age}{under}: {reason}"),
        None => format!("held{kind} {age}{under}"),
    }
}
