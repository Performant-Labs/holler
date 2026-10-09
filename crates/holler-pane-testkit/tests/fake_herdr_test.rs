#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #683
//! `FakeHerdr`'s own mechanisms: workspace bounds, id minting, the split-only mode,
//! the selectable versions, fault injection, the vanished pane and the record of
//! everything typed. The generic port rules (the grid, idempotence, never reused
//! ids) are the conformance suite's job; this file pins what only the fake has
//! (#683, slice d).

use std::time::{Duration, Instant};

use holler_pane::error::{class_of, ErrorClass};
use holler_pane::{GridPos, HerdrPane, HerdrPort, HerdrSpec, Key, PaneError, PaneId};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::herdr::{
    FakeHerdr, HerdrOp, HerdrVersion, Placement, Sent, GRID_UNREACHABLE, PROTOCOL_22_VERSION,
    SUPPORTED_VERSIONS, UNSUPPORTED_VERSION,
};

const SESSION: &str = "scratch";

fn fake(rows: u16, cols: u16) -> FakeHerdr {
    FakeHerdr::new(SESSION)
        .with_workspace("scratch", rows, cols)
        .unwrap()
}

fn spec_in(workspace: &str, row: u16, col: u16) -> HerdrSpec {
    HerdrSpec {
        session: SESSION.to_owned(),
        workspace: workspace.to_owned(),
        grid: GridPos { row, col },
    }
}

fn spec(row: u16, col: u16) -> HerdrSpec {
    spec_in("scratch", row, col)
}

fn ensure(herdr: &FakeHerdr, row: u16, col: u16) -> HerdrPane {
    herdr.ensure_pane(&spec(row, col)).unwrap()
}

fn id(text: &str) -> PaneId {
    PaneId::new(text)
}

fn ids(herdr: &FakeHerdr) -> Vec<String> {
    herdr
        .snapshot()
        .unwrap()
        .panes
        .iter()
        .map(|p| p.pane_id.as_str().to_owned())
        .collect()
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn enter() -> Key {
    Key::new("Enter")
}

// --- the workspace bounds ensure_pane ---

#[test]
fn a_workspace_bounds_ensure_pane() {
    let herdr = fake(2, 3);
    assert_eq!(ensure(&herdr, 2, 3).grid, GridPos { row: 2, col: 3 });

    for (row, col, cell) in [
        (3, 1, "r3c1"),
        (1, 4, "r1c4"),
        (0, 1, "r0c1"),
        (1, 0, "r1c0"),
    ] {
        match herdr.ensure_pane(&spec(row, col)) {
            Err(PaneError::GridOutOfRange { what }) => {
                assert!(what.contains(cell), "the message names {cell}, got: {what}");
            }
            other => panic!("{cell}: expected grid-out-of-range, got {other:?}"),
        }
    }
    assert_eq!(ids(&herdr).len(), 1, "a refused cell creates nothing");
}

#[test]
fn an_undeclared_workspace_or_another_session_is_unavailable() {
    let herdr = fake(2, 2);
    let other_workspace = herdr.ensure_pane(&spec_in("elsewhere", 1, 1));
    assert!(matches!(
        other_workspace,
        Err(PaneError::Unavailable { .. })
    ));

    let mut other_session = spec(1, 1);
    other_session.session = "live".to_owned();
    assert!(matches!(
        herdr.ensure_pane(&other_session),
        Err(PaneError::Unavailable { .. })
    ));
    assert!(ids(&herdr).is_empty());
}

#[test]
fn declaring_a_workspace_twice_is_usage() {
    let err = fake(2, 2).with_workspace("scratch", 1, 1).err().unwrap();
    assert!(matches!(err, PaneError::Usage { .. }), "got {err:?}");
}

// --- ids ---

#[test]
fn ids_are_minted_per_workspace_in_base_36_and_never_reused() {
    let herdr = FakeHerdr::new(SESSION)
        .with_workspace("scratch", 1, 12)
        .unwrap()
        .with_workspace("second", 1, 1)
        .unwrap();
    let minted: Vec<String> = (1..=12)
        .map(|col| ensure(&herdr, 1, col).pane_id.as_str().to_owned())
        .collect();
    assert_eq!(
        minted,
        [
            "w1:p1", "w1:p2", "w1:p3", "w1:p4", "w1:p5", "w1:p6", "w1:p7", "w1:p8", "w1:p9",
            "w1:pA", "w1:pB", "w1:pC"
        ]
    );

    herdr.close(&id("w1:p3")).unwrap();
    herdr.vanish(&id("w1:pA")).unwrap();
    assert_eq!(
        herdr.ensure_pane(&spec(1, 3)).unwrap().pane_id,
        id("w1:pD"),
        "a closed id is not reused"
    );
    assert_eq!(
        herdr.ensure_pane(&spec(1, 10)).unwrap().pane_id,
        id("w1:pE"),
        "a vanished id is not reused"
    );
    assert_eq!(
        herdr.ensure_pane(&spec_in("second", 1, 1)).unwrap().pane_id,
        id("w2:p1"),
        "the first pane of the second workspace"
    );
}

#[test]
fn the_pane_counter_carries_into_two_digits_in_base_36() {
    let herdr = fake(1, 36);
    let last = (1..=36).map(|col| ensure(&herdr, 1, col)).last().unwrap();
    assert_eq!(last.pane_id, id("w1:p10"));
}

#[test]
fn ensure_on_an_occupied_cell_returns_the_occupant() {
    let herdr = fake(2, 2);
    let a = ensure(&herdr, 1, 1);
    let b = ensure(&herdr, 2, 1);
    let before = herdr.snapshot().unwrap();

    assert_eq!(ensure(&herdr, 1, 1), a);
    assert_eq!(ensure(&herdr, 2, 1), b);
    assert_eq!(
        herdr.snapshot().unwrap(),
        before,
        "no pane was added or moved"
    );
    assert_eq!(
        ensure(&herdr, 1, 2).pane_id,
        id("w1:p3"),
        "an occupied ensure does not spend an id"
    );
}

#[test]
fn closing_a_pane_leaves_its_siblings_ids_and_cells() {
    let herdr = fake(2, 2);
    let a = ensure(&herdr, 1, 1);
    let b = ensure(&herdr, 1, 2);
    let c = ensure(&herdr, 2, 1);
    herdr.close(&b.pane_id).unwrap();
    assert_eq!(herdr.snapshot().unwrap().panes, vec![a, c]);
}

// --- split-only placement ---

#[test]
fn split_only_mode_refuses_an_absolute_placement() {
    let herdr = fake(2, 2);
    herdr.set_placement(Placement::SplitOnly);

    match herdr.ensure_pane(&spec(2, 2)) {
        Err(PaneError::Refused { code, message }) => {
            assert_eq!(code.as_str(), GRID_UNREACHABLE.as_str());
            assert_eq!(class_of(code.as_str()), ErrorClass::Refusal);
            assert!(message.contains("r2c2"), "names the cell, got: {message}");
        }
        other => panic!("expected grid-unreachable, got {other:?}"),
    }
    assert!(ids(&herdr).is_empty(), "a refusal creates nothing");

    ensure(&herdr, 1, 1);
    assert_eq!(
        herdr
            .ensure_pane(&spec(2, 2))
            .err()
            .map(|e| e.code().to_owned()),
        Some(GRID_UNREACHABLE.as_str().to_owned()),
        "r2c2 has no pane to its left or above yet"
    );
    ensure(&herdr, 1, 2);
    ensure(&herdr, 2, 2);
    ensure(&herdr, 2, 1);
    assert_eq!(ids(&herdr).len(), 4);
}

#[test]
fn split_only_mode_starts_an_empty_workspace_at_r1c1() {
    let herdr = fake(2, 2);
    herdr.set_placement(Placement::SplitOnly);
    for (row, col) in [(1, 2), (2, 1)] {
        let err = herdr.ensure_pane(&spec(row, col)).err().unwrap();
        assert_eq!(err.code(), GRID_UNREACHABLE.as_str(), "r{row}c{col}");
    }
    assert!(ids(&herdr).is_empty());
    ensure(&herdr, 1, 1);
}

#[test]
fn split_only_mode_returns_an_occupant_without_a_split() {
    let herdr = fake(2, 2);
    let a = ensure(&herdr, 1, 1);
    herdr.set_placement(Placement::SplitOnly);
    assert_eq!(ensure(&herdr, 1, 1), a);
}

#[test]
fn split_only_mode_checks_the_range_first() {
    let herdr = fake(2, 1);
    herdr.set_placement(Placement::SplitOnly);
    let err = herdr.ensure_pane(&spec(3, 1)).err().unwrap();
    assert!(
        matches!(err, PaneError::GridOutOfRange { .. }),
        "got {err:?}"
    );
}

// --- versions ---

#[test]
fn each_selectable_version_is_observable() {
    let herdr = fake(1, 1);
    assert_eq!(herdr.version().unwrap(), PROTOCOL_22_VERSION);

    herdr.set_version(HerdrVersion::Unsupported);
    match herdr.version() {
        Err(PaneError::HerdrVersionUnsupported { message }) => {
            assert!(message.contains(SUPPORTED_VERSIONS), "got: {message}");
            assert!(message.contains(UNSUPPORTED_VERSION), "got: {message}");
        }
        other => panic!("expected herdr-version-unsupported, got {other:?}"),
    }
    ensure(&herdr, 1, 1);
    assert_eq!(
        herdr.snapshot().unwrap().panes.len(),
        1,
        "the version gates version() only"
    );

    herdr.set_version(HerdrVersion::Protocol22);
    assert_eq!(herdr.version().unwrap(), PROTOCOL_22_VERSION);
    assert_eq!(herdr.snapshot().unwrap().panes.len(), 1);
}

// --- faults ---

#[test]
fn a_wedged_herdr_times_out_every_method() {
    let herdr = fake(1, 1);
    herdr.faults().set(Some(Fault::Wedged));
    let a = id("w1:p1");

    assert_eq!(
        herdr.ensure_pane(&spec(1, 1)).err(),
        Some(timeout("herdr.ensure_pane"))
    );
    assert_eq!(
        herdr.send_text(&a, "x").err(),
        Some(timeout("herdr.send_text"))
    );
    assert_eq!(
        herdr.send_keys(&a, &[enter()]).err(),
        Some(timeout("herdr.send_keys"))
    );
    assert_eq!(herdr.read(&a, 1).err(), Some(timeout("herdr.read")));
    assert_eq!(herdr.close(&a).err(), Some(timeout("herdr.close")));
    assert_eq!(herdr.snapshot().err(), Some(timeout("herdr.snapshot")));
    assert_eq!(herdr.version().err(), Some(timeout("herdr.version")));

    herdr.faults().set(None);
    assert!(
        herdr.snapshot().unwrap().panes.is_empty(),
        "the wedged ensure_pane created nothing"
    );
}

#[test]
fn a_failed_ensure_creates_nothing() {
    let herdr = fake(1, 1);
    let error = PaneError::Unavailable {
        what: "herdr".to_owned(),
    };
    herdr.faults().fail_next(HerdrOp::EnsurePane, error.clone());
    assert_eq!(herdr.ensure_pane(&spec(1, 1)).err(), Some(error));
    assert!(herdr.snapshot().unwrap().panes.is_empty());
    assert_eq!(
        ensure(&herdr, 1, 1).pane_id,
        id("w1:p1"),
        "the failed call spent no id"
    );
}

#[test]
fn a_slow_call_takes_at_least_the_delay() {
    let herdr = fake(1, 1);
    let delay = Duration::from_millis(80);
    herdr.faults().set_delay(Some(delay));
    let started = Instant::now();
    herdr.version().unwrap();
    assert!(started.elapsed() >= delay, "took {:?}", started.elapsed());
}

#[test]
fn a_vanished_pane_is_pane_not_found_everywhere() {
    let herdr = fake(2, 1);
    let a = ensure(&herdr, 1, 1);
    let b = ensure(&herdr, 2, 1);
    let calls_before = herdr.faults().calls();

    herdr.vanish(&b.pane_id).unwrap();
    assert_eq!(
        herdr.faults().calls(),
        calls_before,
        "vanish is not a port call"
    );

    for (call, result) in [
        ("read", herdr.read(&b.pane_id, 1).err()),
        ("send_text", herdr.send_text(&b.pane_id, "x").err()),
        ("send_keys", herdr.send_keys(&b.pane_id, &[enter()]).err()),
        ("close", herdr.close(&b.pane_id).err()),
    ] {
        let code = result.map(|e| e.code().to_owned());
        assert_eq!(code.as_deref(), Some("pane-not-found"), "{call}");
    }
    assert_eq!(herdr.snapshot().unwrap().panes, vec![a.clone()]);

    let again = ensure(&herdr, 2, 1);
    assert_ne!(again.pane_id, b.pane_id, "a vanished id is never reused");
    assert_eq!(
        herdr
            .vanish(&id("w9:p9"))
            .err()
            .map(|e| e.code().to_owned())
            .as_deref(),
        Some("pane-not-found")
    );
}

// --- the record of what was typed ---

#[test]
fn send_text_and_send_keys_are_recorded_in_order() {
    let herdr = fake(1, 1);
    let a = ensure(&herdr, 1, 1);
    let unknown = id("w1:p99");
    assert!(herdr.sent().is_empty());

    herdr.send_text(&a.pane_id, "ls").unwrap();
    herdr
        .send_keys(&a.pane_id, &[enter(), Key::new("C-c")])
        .unwrap();
    assert!(herdr.send_text(&unknown, "x").is_err());
    herdr.faults().fail_next(
        HerdrOp::SendKeys,
        PaneError::Unavailable {
            what: "herdr".to_owned(),
        },
    );
    assert!(herdr.send_keys(&a.pane_id, &[enter()]).is_err());
    herdr.print(&a.pane_id, "out\n").unwrap();
    herdr.read(&a.pane_id, 5).unwrap();
    ensure(&herdr, 1, 1);

    assert_eq!(
        herdr.sent(),
        vec![
            Sent::Text {
                pane: a.pane_id.clone(),
                text: "ls".to_owned()
            },
            Sent::Keys {
                pane: a.pane_id,
                keys: vec![enter(), Key::new("C-c")]
            },
        ],
        "failed sends, ensure_pane, read and print are not in the record"
    );
    let ops = herdr.faults().calls();
    let count = |op: HerdrOp| ops.iter().filter(|o| **o == op).count();
    assert_eq!(
        count(HerdrOp::SendText),
        2,
        "the attempt on the unknown pane is in the log"
    );
    assert_eq!(
        count(HerdrOp::SendKeys),
        2,
        "the attempt a fault stopped is in the log"
    );
}

#[test]
fn read_returns_the_last_lines_of_the_screen() {
    let herdr = fake(1, 1);
    let a = ensure(&herdr, 1, 1).pane_id;
    assert_eq!(
        herdr.read(&a, 3).unwrap(),
        "",
        "a new pane has an empty screen"
    );

    herdr.print(&a, "a\nb\nc\n").unwrap();
    assert_eq!(herdr.read(&a, 2).unwrap(), "b\nc");
    assert_eq!(herdr.read(&a, 10).unwrap(), "a\nb\nc");
    assert_eq!(herdr.read(&a, 0).unwrap(), "");

    herdr.send_text(&a, "ls").unwrap();
    herdr.send_keys(&a, &[enter()]).unwrap();
    assert_eq!(herdr.read(&a, 1).unwrap(), "ls");
    herdr.send_keys(&a, &[Key::new("enter")]).unwrap();
    herdr.send_keys(&a, &[Key::new("C-c")]).unwrap();
    assert_eq!(
        herdr.read(&a, 10).unwrap(),
        "a\nb\nc\nls\n",
        "only enter breaks a line"
    );
}

#[test]
fn print_to_an_unknown_pane_is_pane_not_found() {
    let herdr = fake(1, 1);
    let err = herdr.print(&id("w1:p1"), "x").err().unwrap();
    assert_eq!(err.code(), "pane-not-found");
}

#[test]
fn snapshot_lists_panes_by_workspace_then_row_then_col() {
    let herdr = FakeHerdr::new(SESSION)
        .with_workspace("first", 2, 2)
        .unwrap()
        .with_workspace("second", 2, 2)
        .unwrap();
    for (workspace, row, col) in [
        ("second", 1, 1),
        ("first", 2, 1),
        ("first", 1, 2),
        ("first", 1, 1),
        ("second", 2, 2),
        ("second", 1, 2),
    ] {
        herdr.ensure_pane(&spec_in(workspace, row, col)).unwrap();
    }
    let order: Vec<(String, GridPos)> = herdr
        .snapshot()
        .unwrap()
        .panes
        .into_iter()
        .map(|p| {
            assert_eq!(p.session, SESSION);
            (p.workspace, p.grid)
        })
        .collect();
    let cell = |row, col| GridPos { row, col };
    assert_eq!(
        order,
        vec![
            ("first".to_owned(), cell(1, 1)),
            ("first".to_owned(), cell(1, 2)),
            ("first".to_owned(), cell(2, 1)),
            ("second".to_owned(), cell(1, 1)),
            ("second".to_owned(), cell(1, 2)),
            ("second".to_owned(), cell(2, 2)),
        ]
    );
}

// --- naming and thread safety ---

#[test]
fn port_op_names_are_herdr_dot_method() {
    let names: Vec<&str> = [
        HerdrOp::EnsurePane,
        HerdrOp::SendText,
        HerdrOp::SendKeys,
        HerdrOp::Read,
        HerdrOp::Close,
        HerdrOp::Snapshot,
        HerdrOp::Version,
    ]
    .into_iter()
    .map(PortOp::as_str)
    .collect();
    assert_eq!(
        names,
        [
            "herdr.ensure_pane",
            "herdr.send_text",
            "herdr.send_keys",
            "herdr.read",
            "herdr.close",
            "herdr.snapshot",
            "herdr.version"
        ]
    );
}

#[test]
fn the_fake_is_send_and_sync_and_a_dyn_herdr_port() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FakeHerdr>();
    let herdr = fake(1, 1);
    let port: &dyn HerdrPort = &herdr;
    assert_eq!(port.ensure_pane(&spec(1, 1)).unwrap().pane_id, id("w1:p1"));
}
