//! `holler pane list` (story #643, epic #633): the pane registry as a table, one row per
//! pane, and the view code the three read verbs share.
//!
//! The shared items are `pub` and live here because the frozen `pane/mod.rs` admits no new
//! module. [`PaneRow`] is one pane in summary: a `list` row, and the `pane` of a `watch`
//! put. [`SessionSync`] is the one SHOWN/DRIVEN rule. [`text_value`] and [`json_text`] make
//! a stored string safe to print on a terminal, and [`observed_at`] prints a record
//! timestamp. `get.rs` and `watch.rs` use them through `super::list`, and a later consumer
//! (the roster, #648) can use them through `crate::pane::list`.
//!
//! The read verbs read the record and nothing else. They call `PaneStore::{get, list,
//! watch}`, `ProfileStore::get` and `ProfileScope::resolve`, never an adapter or a probe,
//! and they write nothing. SHOWN, DRIVEN and health are what reconcile last recorded.

use clap::Args;
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::{GridPos, Pane, PaneError, PaneName, Ports, ProfileName};
use serde::Serialize;

use super::args::ProfileOpt;
use crate::output::{emit, ErrorBody, VerbCtx};
use crate::time_fmt::format_epoch;

/// The table's columns, in order. A `pane watch` line prints the same cells after the
/// pane's name, each as `key=value` with the column name in lower case.
pub const COLUMNS: [&str; 9] = [
    "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
];

/// What a text cell or field holds when the record has no value.
pub const NO_VALUE: &str = "-";

/// What separates two columns of the table.
const GAP: &str = "  ";

/// List panes, one row per pane, sorted by name.
///
/// Text is a table for people; `--format=json` is for scripts. Every value is the pane
/// registry's: SHOWN, DRIVEN and HEALTH are what reconcile last recorded, and the verb
/// observes nothing itself.
///
/// The columns: PANE is the pane's name. POS is its grid cell, row first (`r2c1` is row 2,
/// column 1). PROFILE is the profile it belongs to. PROJECT is the directory it works in.
/// HEALTH is `healthy`, `unhealthy` or `unknown`, as the harness server last reported it
/// (`pane get` shows the reason). SHOWN is the session the pane's TUI shows, and DRIVEN is
/// the session the hub drives. SYNC is `ok` when they are the same session, MISMATCH when
/// they differ, and `-` while either one is unobserved (nothing has recorded it yet, or
/// reconcile could not tell). HOLD is `none`, `parked` or `drained`.
///
/// A `-` is an empty value. A stored value that is empty or `-`, or that holds a space, a
/// quote, a backslash, an `=` or a character a terminal could act on, is printed quoted and
/// escaped, so it stays one cell.
///
/// With PANE, only that pane is listed (the table is empty when it has no record). With
/// `--profile`, only the panes of that profile are listed, and a named pane must belong to
/// it.
///
/// `--format=json` prints one envelope whose data is `{"panes": [ROW, ...]}`, sorted by
/// name. A ROW has the keys "name", "pos" (`{"row": 2, "col": 1, "pos": "r2c1"}`),
/// "profile", "project", "health" (`"healthy"`, `"unknown"` or `{"unhealthy": REASON}`),
/// "shown", "driven", "sync" (`"ok"`, `"mismatch"` or `"unobserved"`) and "hold" (`"none"`,
/// `"drained"` or `{"parked": {...}}`); an empty value is null.
#[derive(Args, Debug)]
pub struct PaneList {
    /// List only this pane.
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane list`: read the panes in scope and print them sorted by name.
pub fn run(args: &PaneList, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = rows_in_scope(args, ctx.ports).map_err(|e| ErrorBody::from(&e));
    emit(&mut ctx.sink, ctx.format, result, render_table)
}

/// The data of `pane list --format=json`: an object, so that a field can be added later
/// without a schema change (ADR-0021 section 9).
#[derive(Debug, Serialize)]
struct PaneTable {
    panes: Vec<PaneRow>,
}

/// The panes in scope, as rows sorted by name. The names are typed here, so a bad one is
/// `usage` (exit 2) in both formats. With `--profile` the scope decides (`profile-not-found`,
/// `pane-not-in-profile`); without it a named pane that has no record is an empty table.
fn rows_in_scope(args: &PaneList, ports: Ports<'_>) -> Result<PaneTable, PaneError> {
    let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
    let mut panes = match (profile_name(&args.profile)?, pane) {
        (Some(profile), pane) => ports.scope.resolve(&profile, pane.as_ref())?.panes,
        (None, Some(pane)) => ports.pane_store.get(&pane)?.into_iter().collect(),
        (None, None) => ports.pane_store.list()?,
    };
    panes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(PaneTable {
        panes: panes.iter().map(PaneRow::from).collect(),
    })
}

/// The table: the header, then one line per row. Each column is left-aligned to its widest
/// cell (counted in characters) and two spaces from the next; the last one is not padded,
/// so no line ends in a space. With no pane, the header alone.
fn render_table(table: &PaneTable) -> String {
    let rows: Vec<[String; COLUMNS.len()]> = table.panes.iter().map(PaneRow::cells).collect();
    let mut widths = COLUMNS.map(|column| column.chars().count());
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let header = COLUMNS.map(str::to_owned);
    std::iter::once(&header)
        .chain(&rows)
        .map(|cells| table_line(cells, &widths))
        .collect()
}

/// One line of the table, ending in a newline.
fn table_line(cells: &[String], widths: &[usize]) -> String {
    let mut line = String::new();
    if let Some((last, padded)) = cells.split_last() {
        for (cell, width) in padded.iter().zip(widths.iter().copied()) {
            line.push_str(&format!("{cell:<width$}{GAP}"));
        }
        line.push_str(last);
    }
    line.push('\n');
    line
}

/// The `--profile` a verb is scoped to, typed: `usage` (exit 2) when it is not a valid
/// profile name.
pub fn profile_name(opt: &ProfileOpt) -> Result<Option<ProfileName>, PaneError> {
    opt.profile.as_deref().map(ProfileName::parse).transpose()
}

/// One pane in summary: a row of `pane list`, and the `pane` of a `pane watch` put.
///
/// Its JSON keys are its fields, in this order. An empty value is `null`, never left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneRow {
    pub name: PaneName,
    /// The grid cell, row first, in `GridPos`'s own serde form.
    pub pos: GridPos,
    pub profile: Option<ProfileName>,
    /// The directory the pane works in (`host.cwd`).
    pub project: String,
    /// What the harness server last reported, in `Health`'s serde form.
    pub health: Health,
    /// The session the pane's TUI shows, as reconcile last recorded it.
    pub shown: Option<String>,
    /// The session the hub drives, as reconcile last recorded it.
    pub driven: Option<String>,
    pub sync: SessionSync,
    /// In `Hold`'s serde form.
    pub hold: Hold,
}

impl From<&Pane> for PaneRow {
    fn from(pane: &Pane) -> Self {
        Self {
            name: pane.name.clone(),
            pos: pane.herdr.grid,
            profile: pane.profile.clone(),
            project: pane.host.cwd.clone(),
            health: pane.harness.health.clone(),
            shown: pane.last_observed.shown.clone(),
            driven: pane.last_observed.driven.clone(),
            sync: SessionSync::of(&pane.last_observed),
            hold: pane.hold.clone(),
        }
    }
}

impl PaneRow {
    /// The row's text cells, one per entry of [`COLUMNS`]. Every stored string goes
    /// through [`text_value`], an empty value is `-`, and health, sync and hold are the
    /// verb's own words.
    pub fn cells(&self) -> [String; COLUMNS.len()] {
        [
            text_value(self.name.as_str()),
            self.pos.to_string(),
            optional_text(self.profile.as_ref().map(ProfileName::as_str)),
            text_value(&self.project),
            health_word(&self.health).to_owned(),
            optional_text(self.shown.as_deref()),
            optional_text(self.driven.as_deref()),
            self.sync.text().to_owned(),
            hold_word(&self.hold).to_owned(),
        ]
    }
}

/// Whether the session a pane's TUI shows is the session the hub drives, as reconcile last
/// recorded both (`last_observed`). This is the one copy of the rule the read verbs print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSync {
    /// Both were observed, and they are the same session.
    Ok,
    /// Both were observed, and they differ: the pane shows one session while the hub
    /// drives another.
    Mismatch,
    /// One of them was not observed, so there is nothing to compare. This is not flagged:
    /// reconcile records `None` when it cannot tell, and flagging every such pane would
    /// make the flag noise.
    Unobserved,
}

impl SessionSync {
    /// The rule: [`Ok`](Self::Ok) when `shown` and `driven` are both set and equal,
    /// [`Mismatch`](Self::Mismatch) when both are set and differ, and
    /// [`Unobserved`](Self::Unobserved) when either is unset.
    pub fn of(observed: &LastObserved) -> Self {
        match (&observed.shown, &observed.driven) {
            (Some(shown), Some(driven)) if shown == driven => Self::Ok,
            (Some(_), Some(_)) => Self::Mismatch,
            _ => Self::Unobserved,
        }
    }

    /// The word text mode prints: `ok`, `MISMATCH` (loud on purpose) or `-`.
    pub fn text(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Mismatch => "MISMATCH",
            Self::Unobserved => NO_VALUE,
        }
    }
}

/// The health word of a table cell or a watch line. The reason is `pane get`'s.
pub fn health_word(health: &Health) -> &'static str {
    match health {
        Health::Healthy => "healthy",
        Health::Unhealthy(_) => "unhealthy",
        Health::Unknown => "unknown",
    }
}

/// The hold word of a table cell or a watch line. The details are `pane get`'s.
pub fn hold_word(hold: &Hold) -> &'static str {
    match hold {
        Hold::None => "none",
        Hold::Parked { .. } => "parked",
        Hold::Drained => "drained",
    }
}

/// A stored string, safe to print in a text line: unchanged when it is plain, otherwise
/// quoted and escaped in Rust's `{:?}` form (an ESC becomes the six characters `\u{1b}`, a
/// newline the two characters `\n`).
///
/// A value is plain when it is not empty, is not `-` (the empty value), and has no
/// whitespace, no `"`, `\` or `=`, and no character that could act on a terminal (a
/// control character, or one Rust's `{:?}` does not print as itself, such as a
/// bidirectional override). So no stored value can emit an escape sequence, a carriage
/// return or a line of its own, and a table cell or a `key=value` field stays one word.
/// Every stored string the read verbs print in text mode goes through here.
pub fn text_value(value: &str) -> String {
    let plain = !value.is_empty()
        && value != NO_VALUE
        && !value
            .chars()
            .any(|c| acts_on_terminal(c) || c.is_whitespace() || matches!(c, '"' | '\\' | '='));
    if plain {
        value.to_owned()
    } else {
        format!("{value:?}")
    }
}

/// [`text_value`] of `value`, or `-` when there is none.
pub fn optional_text(value: Option<&str>) -> String {
    value.map_or_else(|| NO_VALUE.to_owned(), text_value)
}

/// `value` as compact JSON for a text line (an argv, a list of strings, a spec). It is
/// `serde_json`'s form, except that every character that could act on a terminal (as for
/// [`text_value`]) is written as a JSON `\u` escape: `serde_json` escapes only the C0
/// controls, and leaves DEL, the C1 controls and the format characters raw. The line stays
/// valid JSON for the same value.
pub fn json_text<T: Serialize + ?Sized>(value: &T) -> String {
    // `serde_json` fails only for a map whose keys are not strings, or for a `Serialize`
    // that errors. The argv, string lists and specs printed here have neither.
    let json = serde_json::to_string(value).unwrap_or_default();
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        if acts_on_terminal(c) {
            for unit in c.encode_utf16(&mut [0; 2]) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether `c` could act on a terminal or hide inside a line: a control character, or a
/// character that Rust's `{:?}` writes as a `\u{..}` escape because it is not printed as
/// itself (a bidirectional override, a zero-width or combining character, a line
/// separator).
fn acts_on_terminal(c: char) -> bool {
    c.is_control() || c.escape_debug().nth(1) == Some('u')
}

/// A record timestamp (milliseconds since the Unix epoch) in text: `never` for zero or
/// less, otherwise the UTC date and time, as `2026-10-09 17:30:00 UTC`.
pub fn observed_at(ms: i64) -> String {
    match u64::try_from(ms) {
        Ok(ms) if ms > 0 => format!("{} UTC", format_epoch(ms / 1000)),
        _ => "never".to_owned(),
    }
}
