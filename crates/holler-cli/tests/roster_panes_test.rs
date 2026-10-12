#![allow(clippy::unwrap_used)] // #648
#![allow(clippy::expect_used)] // #648
#![allow(clippy::panic)] // #648
#![allow(clippy::unreachable)] // #648
//! `holler roster` with the pane columns (story #648, epic #633): PANE, POS, PROFILE, HEALTH,
//! SHOWN and DRIVEN after HOLD, the summary line when a pane's SHOWN and DRIVEN differ,
//! `--profile`, and `--format=json`'s envelope.
//!
//! In-process (the brief's R8): every case hands `roster_cmd::render` a literal hub reply
//! (`{"rows": [...]}`, in the hub's `Row` serde form) and the registry as the test kit's fakes,
//! and captures both streams. Only `hubless` calls `roster_cmd::run`, which makes the hub call,
//! and it does so under a state-dir lock with `HOLLER_STATE_DIR` on an empty scratch dir (A's
//! W-6). The process-level cases (the real wiring, `--server`, clap's usage errors) are in
//! `roster_cli_test.rs`.
//!
//! The fixture rows carry no wall-clock cell (`last_turn` is null and no row is held), so the
//! golden files under `tests/fixtures/roster_panes/` are stable.

#[path = "roster_panes/hubless.rs"]
mod hubless;
#[path = "roster_panes/join.rs"]
mod join;
#[path = "roster_panes/profile.rs"]
mod profile;
#[path = "roster_panes/registry.rs"]
mod registry;

use std::sync::Arc;

use clap::{CommandFactory, Parser};
use holler_cli::output::{resolve_format, FormatChoice, Sink};
use holler_cli::roster_cmd::{self, Registry, PANE_COLUMNS};
use holler_cli::{Cli, Command};
use holler_hub::roster::Row;
use holler_pane::pane::{Health, LastObserved};
use holler_pane::{Actor, GridPos, Pane, Ports, Profile, ProfileName};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::{sample_pane, sample_profile};
use holler_pane_testkit::harness::FakeHarness;
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::host::FakeHost;
use holler_pane_testkit::pane_store::FakePaneStore;
use holler_pane_testkit::prober::FakeProber;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;
use holler_proto::docs::{PendingItem, PendingKind};
use holler_proto::SessionHold;
use serde_json::{json, Value};

// --- the harness -------------------------------------------------------------------

/// The three output modes of the roster (A's W-4): text, a bare `--json` (today's own
/// shape) and an explicit `--format=json` (the envelope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Text,
    Legacy,
    Envelope,
}

impl Mode {
    pub(crate) const ALL: [Mode; 3] = [Mode::Text, Mode::Legacy, Mode::Envelope];

    fn flags(self) -> &'static [&'static str] {
        match self {
            Mode::Text => &[],
            Mode::Legacy => &["--json"],
            Mode::Envelope => &["--format=json"],
        }
    }
}

/// What one roster run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Run {
    pub code: i32,
    pub out: String,
    pub err: String,
}

/// `holler roster <args> <mode's flags>`, parsed and resolved as `main.rs` does.
pub(crate) fn parse(args: &[&str], mode: Mode) -> (Cli, FormatChoice) {
    let mut argv = vec!["holler", "roster"];
    argv.extend_from_slice(args);
    argv.extend_from_slice(mode.flags());
    let cli = Cli::try_parse_from(&argv).unwrap_or_else(|e| panic!("{argv:?} must parse: {e}"));
    let choice = resolve_format(cli.json, cli.format).expect("the mode's flags resolve");
    (cli, choice)
}

/// Capture what `body` writes to a sink, with its exit code.
pub(crate) fn capture(body: impl FnOnce(&mut Sink<'_>) -> i32) -> Run {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = body(&mut Sink {
        out: &mut out,
        err: &mut err,
    });
    Run {
        code,
        out: String::from_utf8(out).expect("out is UTF-8"),
        err: String::from_utf8(err).expect("err is UTF-8"),
    }
}

/// Run the roster's pure half over a hub reply and a registry.
pub(crate) fn render(reply: &Value, registry: Registry<'_>, args: &[&str], mode: Mode) -> Run {
    let (cli, choice) = parse(args, mode);
    let Command::Roster(roster) = &cli.command else {
        unreachable!("parsed `holler roster`")
    };
    capture(|sink| roster_cmd::render(reply, registry, roster, choice, sink))
}

/// The registry's fakes and the adapter fakes, whose call logs prove the roster observes
/// nothing (acceptance 7).
pub(crate) struct Rig {
    pub panes: Arc<FakePaneStore>,
    pub profiles: Arc<FakeProfileStore>,
    pub scope: FakeProfileScope,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
}

impl Rig {
    /// A store holding `panes` and the profile `fleet` (whose members are the panes whose
    /// record names it).
    pub fn new(panes: impl IntoIterator<Item = Pane>) -> Self {
        let actor = Actor::parse("test").unwrap();
        let panes = Arc::new(FakePaneStore::seeded(panes).unwrap());
        let fleet: Profile = sample_profile("fleet", &["demo-c1r1", "zz-c2r2"]).unwrap();
        let profiles = Arc::new(FakeProfileStore::seeded([fleet], &actor).unwrap());
        let scope = FakeProfileScope::new(profiles.clone(), panes.clone(), actor);
        Self {
            panes,
            profiles,
            scope,
            herdr: FakeHerdr::new("scratch"),
            host: FakeHost::new(),
            harness: FakeHarness::new(),
            prober: FakeProber::new(),
        }
    }

    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: self.panes.as_ref(),
            profile_store: self.profiles.as_ref(),
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &self.scope,
            prober: &self.prober,
        }
    }

    /// [`render`] over this rig's ports.
    pub fn render(&self, reply: &Value, args: &[&str], mode: Mode) -> Run {
        render(reply, Registry::Ports(self.ports()), args, mode)
    }
}

// --- the fixtures ------------------------------------------------------------------

/// One hub row named `name`: the hub's own `Row`, serialized as `control/roster` sends it
/// (`last_turn` is `None` and the row is not held, so no cell reads the wall clock).
/// `io/alpha` differs in every text column, and `hj-c1r2/build` holds one permission.
pub(crate) fn hub_row(name: &str) -> Value {
    let io = name == "io/alpha";
    let pending = (name == "hj-c1r2/build").then(|| {
        vec![PendingItem {
            id: "perm-1".to_owned(),
            kind: PendingKind::Permission,
            prompt: "allow?".to_owned(),
            options: vec!["once".to_owned()],
        }]
    });
    let row = Row {
        name: name.to_owned(),
        harness: if io { "claude" } else { "opencode" }.to_owned(),
        mode: if io { "attach" } else { "spawn" }.to_owned(),
        state: if io { "working" } else { "idle" }.to_owned(),
        conn_state: if io { "reconnecting" } else { "connected" },
        token_id: "tok-1".to_owned(),
        client_id: "client-1".to_owned(),
        hostname: if io { "host-b" } else { "host-a" }.to_owned(),
        harness_session_id: None,
        first_seen: 1,
        last_seen: 2,
        last_update_at: None,
        pending,
        turn_id: None,
        last_turn: None,
        hold: SessionHold::default(),
    };
    serde_json::to_value(row).unwrap()
}

/// A hub reply whose rows are `names`, in that order.
pub(crate) fn reply(names: &[&str]) -> Value {
    json!({ "rows": names.iter().map(|n| hub_row(n)).collect::<Vec<_>>() })
}

/// A pane record at `r{row}c{col}` with the given profile, health and recorded pair.
pub(crate) fn pane(
    name: &str,
    (row, col): (u16, u16),
    profile: Option<&str>,
    health: Health,
    (shown, driven): (Option<&str>, Option<&str>),
) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    pane.herdr.grid = GridPos { row, col };
    pane.profile = profile.map(|p| ProfileName::parse(p).unwrap());
    pane.harness.health = health;
    pane.last_observed = LastObserved {
        shown: shown.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 1_000,
    };
    pane
}

/// The hub rows of acceptance 1, in the hub's order (by name): two agents with no pane
/// (`alpha`, and `io/alpha`, whose label names no pane) and three rows joined to a pane.
pub(crate) const BASE_ROWS: [&str; 5] = [
    "alpha",
    "demo-c1r1/main",
    "hj-c1r2/build",
    "io/alpha",
    "zz-c2r2/work",
];

/// The records of acceptance 1: one pane per HEALTH value, with and without a profile, and
/// none a mismatch. `demo-c1r1`'s session of record differs from its DRIVEN, and
/// `hj-c1r2`'s from its SHOWN while its DRIVEN is absent (acceptance 7, round 3).
/// `spare-c3r3` has no hub row, so it is never listed.
pub(crate) fn base_panes() -> Vec<Pane> {
    let mut demo = pane(
        "demo-c1r1",
        (1, 1),
        Some("fleet"),
        Health::Healthy,
        (Some("ses-a"), Some("ses-a")),
    );
    demo.session_of_record = Some("ses-record".to_owned());
    let unhealthy = Health::Unhealthy("port closed".to_owned());
    let mut hj = pane("hj-c1r2", (2, 1), None, unhealthy, (Some("ses-b"), None));
    hj.session_of_record = Some("ses-r2".to_owned());
    let zz = pane(
        "zz-c2r2",
        (2, 2),
        Some("fleet"),
        Health::Unknown,
        (None, Some("ses-z")),
    );
    let spare = pane(
        "spare-c3r3",
        (3, 3),
        None,
        Health::Healthy,
        (Some("x"), Some("y")),
    );
    vec![demo, hj, zz, spare]
}

/// The records of acceptance 2's fixture with two mismatches: `demo-c1r1` and `hj-c1r2`
/// differ, and `zz-c2r2` shows nothing while its DRIVEN is known (never named).
pub(crate) fn two_mismatch_panes() -> Vec<Pane> {
    let unhealthy = Health::Unhealthy("port closed".to_owned());
    vec![
        pane(
            "demo-c1r1",
            (1, 1),
            Some("fleet"),
            Health::Healthy,
            (Some("ses-a"), Some("ses-x")),
        ),
        pane(
            "hj-c1r2",
            (2, 1),
            None,
            unhealthy,
            (Some("ses-b"), Some("ses-y")),
        ),
        pane(
            "zz-c2r2",
            (2, 2),
            Some("fleet"),
            Health::Unknown,
            (None, Some("ses-z")),
        ),
    ]
}

/// Acceptance 2's rows, deliberately not in the store's name order: the summary line follows
/// the order the roster prints, which is the reply's.
pub(crate) const TWO_ROWS: [&str; 3] = ["hj-c1r2/build", "demo-c1r1/main", "zz-c2r2/work"];

/// Acceptance 9 and 10's example: two rows carry one mismatched pane.
pub(crate) const ONE_ROWS: [&str; 3] = ["alpha", "hj-c1r2/build", "hj-c1r2/review"];

pub(crate) fn one_mismatch_panes() -> Vec<Pane> {
    let unhealthy = Health::Unhealthy("port closed".to_owned());
    vec![pane(
        "hj-c1r2",
        (2, 1),
        None,
        unhealthy,
        (Some("ses-b"), Some("ses-c")),
    )]
}

pub(crate) const GOLDEN_BASE: &str = include_str!("fixtures/roster_panes/base.txt");
pub(crate) const GOLDEN_NO_REGISTRY: &str = include_str!("fixtures/roster_panes/no_registry.txt");
const GOLDEN_TWO: &str = include_str!("fixtures/roster_panes/two_mismatches.txt");
const GOLDEN_ONE: &str = include_str!("fixtures/roster_panes/one_mismatch.txt");

// --- reading the output ------------------------------------------------------------

/// The summary line's fixed words (scope item 3): only N and the names vary.
pub(crate) const SUMMARY: &str = " pane(s) show a different session than the hub drives: ";

/// The table's lines after the optional summary line: the header, then one per row.
pub(crate) fn table(out: &str) -> Vec<&str> {
    out.lines().skip_while(|l| l.contains(SUMMARY)).collect()
}

/// The first whitespace-separated cell of each row line (the SESSION cell).
pub(crate) fn sessions(out: &str) -> Vec<&str> {
    table(out)
        .iter()
        .skip(1)
        .map(|l| l.split_whitespace().next().unwrap())
        .collect()
}

/// The six trailing cells of a line (a fixture's cells hold no space).
pub(crate) fn last_six(line: &str) -> Vec<&str> {
    let cells: Vec<&str> = line.split_whitespace().collect();
    cells[cells.len() - 6..].to_vec()
}

/// The `rows` of a `--json` document.
pub(crate) fn json_rows(out: &str) -> Vec<Value> {
    let doc: Value =
        serde_json::from_str(out).unwrap_or_else(|e| panic!("--json parses: {e}: {out}"));
    doc["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("--json has rows: {doc}"))
        .clone()
}

/// The `data` of a successful `--format=json` envelope, checked with the test kit's helper.
pub(crate) fn envelope_data(run: &Run) -> Value {
    let envelope = check_envelope(&run.out, run.code).unwrap_or_else(|f| panic!("{f:?}: {run:?}"));
    assert!(envelope.ok, "{run:?}");
    envelope.data
}

/// The six new JSON keys (scope item 6).
pub(crate) const NEW_KEYS: [&str; 6] = ["pane", "pos", "profile", "health", "shown", "driven"];

/// Today's header and row line through the HOLD cell, frozen from `roster_cmd.rs:71-92`
/// before #648. A fixture row has no `last_turn` and no hold, so both cells are `-`.
const TODAY_HEADER: &str = "SESSION              HARNESS    MODE     STATE      CONN       HOSTNAME PENDING  LAST TURN            HOLD";

fn today_line(row: &Value) -> String {
    let s = |key: &str| row[key].as_str().unwrap_or("-").to_owned();
    let pending = row["pending"]
        .as_array()
        .map_or("-".to_owned(), |p| p.len().to_string());
    format!(
        "{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} {:<20} {}",
        s("name"),
        s("harness"),
        s("mode"),
        s("state"),
        s("conn_state"),
        s("hostname"),
        pending,
        "-",
        "-"
    )
}

// --- acceptance 1, 2, 5 and 9: the text table ---------------------------------------

/// Acceptance 1 (and 2's fixture with none, and 5): every pre-existing column is today's,
/// byte for byte, and the six new columns follow HOLD; no summary line; `hj-c1r2` sits at
/// `r2c1`.
#[test]
fn old_columns_are_unchanged_and_the_six_new_ones_follow_hold() {
    let rig = Rig::new(base_panes());
    let doc = reply(&BASE_ROWS);
    let run = rig.render(&doc, &[], Mode::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    assert_eq!(run.out, GOLDEN_BASE, "the golden table");

    let lines: Vec<&str> = run.out.lines().collect();
    assert!(
        lines[0].starts_with(TODAY_HEADER),
        "line 1 is today's header, with no summary line before it"
    );
    assert_eq!(
        last_six(lines[0]),
        PANE_COLUMNS,
        "the new columns, in order, after HOLD"
    );
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(
        lines.len(),
        rows.len() + 1,
        "one line per hub row, no legend: {lines:?}"
    );
    for (line, row) in lines[1..].iter().zip(rows) {
        let today = today_line(row);
        assert!(
            line.starts_with(&today),
            "today's cells through HOLD, unchanged:\n{line}\n{today}"
        );
        assert_eq!(
            line.as_bytes()[today.len()],
            b' ',
            "the new columns follow HOLD: {line}"
        );
    }
    assert!(
        !run.out.replace("hj-c1r2", "").contains("c1r2"),
        "no position is written c<col>r<row>"
    );
}

/// Acceptance 2: two mismatched panes, named in the order the roster prints its rows (the
/// reply's, not the store's), on one summary line before the table. A pane whose SHOWN is
/// `-` is never named, whatever its DRIVEN.
#[test]
fn two_mismatches_lead_with_one_summary_line_in_roster_order() {
    let rig = Rig::new(two_mismatch_panes());
    let run = rig.render(&reply(&TWO_ROWS), &[], Mode::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    assert_eq!(run.out, GOLDEN_TWO, "the golden table");
}

/// Acceptance 9 and 10: one mismatched pane carried by two rows is counted and named once,
/// and the line says `pane(s)` for N = 1.
#[test]
fn one_mismatch_is_named_once_however_many_rows_carry_it() {
    let rig = Rig::new(one_mismatch_panes());
    let run = rig.render(&reply(&ONE_ROWS), &[], Mode::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    assert_eq!(run.out, GOLDEN_ONE, "the golden table");
}

// --- acceptance 3 and 4: JSON ------------------------------------------------------

/// Acceptance 3: `--json` keeps today's document, `{"rows": [...]}`, with every key of every
/// row and its value, and adds the six keys: `null` for a row with no pane, the record's
/// values otherwise (`pos` in `GridPos`'s form, `health` in `pane list`'s).
#[test]
fn json_keeps_every_key_and_adds_the_six() {
    let rig = Rig::new(base_panes());
    let doc = reply(&BASE_ROWS);
    let run = rig.render(&doc, &[], Mode::Legacy);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    let parsed: Value = serde_json::from_str(&run.out).unwrap();
    let top: Vec<&String> = parsed.as_object().unwrap().keys().collect();
    assert_eq!(
        top,
        ["rows"],
        "`--json` stays `{{\"rows\": [...]}}`, with no `mismatches`"
    );

    let rows = json_rows(&run.out);
    assert_eq!(rows.len(), BASE_ROWS.len());
    for (row, before) in rows.iter().zip(doc["rows"].as_array().unwrap()) {
        for (key, value) in before.as_object().unwrap() {
            assert_eq!(&row[key], value, "`{key}` is unchanged: {row}");
            // The fixture is the hub's own `Row`, so a hub field added later under one of the
            // six names fails here instead of being overwritten (A's W-10).
            assert!(
                !NEW_KEYS.contains(&key.as_str()),
                "the hub's row has a key `{key}` of its own"
            );
        }
        for key in NEW_KEYS {
            assert!(
                row.get(key).is_some(),
                "`{key}` is present (never left out): {row}"
            );
        }
    }
    let six = |i: usize| NEW_KEYS.map(|k| rows[i][k].clone());
    let nulls = [(); 6].map(|()| Value::Null);
    assert_eq!(six(0), nulls, "an agent with no pane: six nulls");
    assert_eq!(six(3), nulls, "a label that names no pane: six nulls");
    let pos =
        |row: u16, col: u16| json!({ "row": row, "col": col, "pos": format!("r{row}c{col}") });
    assert_eq!(
        six(1),
        [
            json!("demo-c1r1"),
            pos(1, 1),
            json!("fleet"),
            json!("healthy"),
            json!("ses-a"),
            json!("ses-a")
        ]
    );
    assert_eq!(
        six(2),
        [
            json!("hj-c1r2"),
            pos(2, 1),
            Value::Null,
            json!({ "unhealthy": "port closed" }),
            json!("ses-b"),
            Value::Null
        ],
        "DRIVEN is `last_observed.driven` (null), never the session of record"
    );
    assert_eq!(
        six(4),
        [
            json!("zz-c2r2"),
            pos(2, 2),
            json!("fleet"),
            json!("unknown"),
            Value::Null,
            json!("ses-z")
        ]
    );
}

/// Acceptance 4: `--format=json` is one envelope whose `data` is exactly `{rows, mismatches}`:
/// `rows` equal to `--json`'s, `mismatches` the summary line's names in roster order.
#[test]
fn format_json_is_one_envelope_of_rows_and_mismatches() {
    let cases: [(&[&str], Vec<Pane>, Vec<&str>); 3] = [
        (&BASE_ROWS, base_panes(), vec![]),
        (
            &TWO_ROWS,
            two_mismatch_panes(),
            vec!["hj-c1r2", "demo-c1r1"],
        ),
        (&ONE_ROWS, one_mismatch_panes(), vec!["hj-c1r2"]),
    ];
    for (rows, panes, mismatches) in cases {
        let rig = Rig::new(panes);
        let doc = reply(rows);
        let run = rig.render(&doc, &[], Mode::Envelope);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{rows:?}: {run:?}");
        let data = envelope_data(&run);
        let keys: Vec<&String> = data.as_object().unwrap().keys().collect();
        assert_eq!(keys.len(), 2, "data has exactly two keys: {data}");
        let legacy = rig.render(&doc, &[], Mode::Legacy);
        assert_eq!(
            data["rows"],
            Value::Array(json_rows(&legacy.out)),
            "data.rows is --json's rows"
        );
        assert_eq!(data["mismatches"], json!(mismatches), "{rows:?}");
    }
}

// --- scope item 2: the help ---------------------------------------------------------

/// Scope item 2 (and A's W-7): `holler roster --help` names the six columns and says that a
/// pane named `hj-c1r2` sits at `r2c1`. The legend is help text, never a line of output
/// (acceptance 1 counts the lines).
#[test]
fn help_names_the_columns_and_that_a_pane_name_is_not_its_position() {
    let mut cli = Cli::command();
    let help = cli
        .find_subcommand_mut("roster")
        .unwrap()
        .render_long_help()
        .to_string();
    for word in PANE_COLUMNS.iter().chain(&["hj-c1r2", "r2c1"]) {
        assert!(
            help.contains(word),
            "`holler roster --help` names `{word}`:\n{help}"
        );
    }
}
