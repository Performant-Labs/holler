//! The `HerdrPort` conformance suite: [`run_herdr_conformance`] runs the cases
//! [`herdr_cases`] lists against any `HerdrPort`, each against a fresh
//! [`HerdrFixture`], a scratch Herdr session whose workspace is 2 rows by 1 column
//! (slice d of #638, #683).
//!
//! The cases pin the port's rules (`holler_pane::ports`; ADR-0021 section 10; the
//! Herdr spike, `docs/research/herdr-api-spike.md`):
//!
//! - `ensure_pane` puts a pane in the cell asked for and `snapshot` reads it back there,
//!   row first, and a cell outside the workspace is `grid-out-of-range`. So an
//!   implementation that swaps rows and columns fails.
//! - `ensure_pane` of an occupied cell answers its pane, and making a pane moves no
//!   other.
//! - An id is never reused after a close, and it does not change when a sibling closes.
//! - A closed or unknown pane is `pane-not-found` for `close`, `send_text`, `send_keys`
//!   and `read`.
//! - `read` returns at most the lines asked for, and `version` reports one line.
//!
//! "The workspace's panes" in a case are the panes of `snapshot` whose session and
//! workspace are the fixture's. Every case ensures `r1c1` before any other cell and
//! never closes a workspace's last pane, so the suite holds against a Herdr that places
//! a pane only by splitting an existing one (the fake's split-only placement) and
//! against real Herdr, which creates a workspace with its root pane. After a close, a
//! case checks ids, not cells. The cases press keys by Herdr's own names (`enter`).

use holler_pane::{GridPos, HerdrPane, HerdrPort, HerdrSpec, Key, PaneId};

use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};

// The fixture workspace's two cells.
const R1C1: GridPos = GridPos { row: 1, col: 1 };
const R2C1: GridPos = GridPos { row: 2, col: 1 };
/// Outside the workspace: where an implementation that swaps rows and columns puts
/// `r2c1`.
const R1C2: GridPos = GridPos { row: 1, col: 2 };
/// Outside the workspace, below its last row.
const R3C1: GridPos = GridPos { row: 3, col: 1 };

const OUT_OF_RANGE: &str = "grid-out-of-range";
const NOT_FOUND: &str = "pane-not-found";

/// An id that no Herdr hands out to the first panes of a scratch session.
const UNKNOWN_ID: &str = "w999:p999";

// How many lines case 10 types, and how many it reads back.
const TYPED_LINES: usize = 5;
const READ_LINES: usize = 2;

// ASSUMPTION (#640): Herdr has no grid (spike section 2), so the port under test must
// know that the fixture's workspace is 2 rows by 1 column, or case 2's `r1c2` is not out
// of range. How the real adapter learns a workspace's size is #640's.
// ASSUMPTION (#640): a real workspace is created with a root pane (spike section 4), and
// the fake's starts empty. Every case holds either way.
/// What one case runs against: a port serving the scratch Herdr session `session`, in
/// which the workspace `workspace` is 2 rows by 1 column and holds no pane, or only its
/// root pane at `r1c1` (a real Herdr workspace is created with one).
pub struct HerdrFixture<H> {
    /// The implementation under test.
    pub port: H,
    /// The scratch session: never the default session or a live one.
    pub session: String,
    /// The workspace of 2 rows by 1 column that the cases place their panes in.
    pub workspace: String,
}

/// What a case sees of its fixture: the port, and the session and workspace it works
/// in.
struct Scratch<'a> {
    port: &'a dyn HerdrPort,
    session: &'a str,
    workspace: &'a str,
}

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&Scratch<'_>) -> Result<(), String>;

/// The suite, in order: the one table that the runner iterates and [`herdr_cases`]
/// lists.
const CASES: [(&str, Case); 11] = [
    (
        "ensure-r2c1-reads-back-as-r2c1",
        ensure_r2c1_reads_back_as_r2c1,
    ),
    (
        "ensure-r1c2-is-grid-out-of-range",
        ensure_r1c2_is_grid_out_of_range,
    ),
    ("ensure-is-idempotent", ensure_is_idempotent),
    (
        "ensure-never-moves-another-pane",
        ensure_never_moves_another_pane,
    ),
    ("closed-id-is-never-reused", closed_id_is_never_reused),
    (
        "ids-unique-and-stable-when-a-sibling-closes",
        ids_unique_and_stable_when_a_sibling_closes,
    ),
    (
        "close-unknown-is-pane-not-found",
        close_unknown_is_pane_not_found,
    ),
    (
        "close-twice-is-pane-not-found",
        close_twice_is_pane_not_found,
    ),
    (
        "calls-on-a-closed-pane-are-pane-not-found",
        calls_on_a_closed_pane_are_pane_not_found,
    ),
    (
        "read-returns-at-most-max-lines",
        read_returns_at_most_max_lines,
    ),
    ("version-is-reported", version_is_reported),
];

/// The ids of the cases [`run_herdr_conformance`] runs, in the order it runs them.
pub fn herdr_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`herdr_cases`], in order, each against a fresh fixture, and
/// return every case that did not hold.
///
/// `fresh` is called once per case. It returns the fixture and a guard that the suite
/// keeps alive for that case only: `()` for the fake, the scratch server's handle for
/// the adapter (#640). The fixture is dropped before its guard. How each implementation
/// runs the suite:
///
/// ```text
/// // the fake:
/// run_herdr_conformance(|| {
///     let port = FakeHerdr::new("scratch").with_workspace("scratch", 2, 1).expect("workspace");
///     (HerdrFixture { port, session: "scratch".into(), workspace: "scratch".into() }, ())
/// })
/// // the adapter (#640, opt-in, #[ignore]): a fresh scratch Herdr session per case whose
/// // workspace the adapter knows to be 2 rows by 1 column; the session's server handle
/// // as the guard; never the default or a live session.
/// ```
pub fn run_herdr_conformance<H, K, F>(fresh: F) -> Conformance
where
    H: HerdrPort,
    F: FnMut() -> (HerdrFixture<H>, K),
{
    run_cases(&CASES, fresh, |case, fixture| {
        case(&Scratch {
            port: &fixture.port,
            session: &fixture.session,
            workspace: &fixture.workspace,
        })
    })
}

// --- the cases ---

/// Case 1: a pane ensured at `r2c1` is at row 2, column 1, in the session and the
/// workspace asked for, and the snapshot lists it there, written `r2c1`.
fn ensure_r2c1_reads_back_as_r2c1(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    expect_eq("the cell ensure_pane(r2c1) returns", b.grid, R2C1)?;
    expect_eq(
        "the session and workspace ensure_pane(r2c1) returns",
        (b.session.as_str(), b.workspace.as_str()),
        (s.session, s.workspace),
    )?;
    let listed = s
        .panes()?
        .into_iter()
        .find(|pane| pane.pane_id == b.pane_id)
        .ok_or_else(|| {
            format!(
                "the snapshot does not list {}, the pane ensured at r2c1, in the workspace",
                b.pane_id.as_str()
            )
        })?;
    expect_eq(
        "the cell the snapshot lists for the pane ensured at r2c1",
        listed.grid,
        R2C1,
    )?;
    expect_eq(
        "that cell, written row first",
        listed.grid.to_string().as_str(),
        "r2c1",
    )
}

// ASSUMPTION (#640): `ensure_pane` outside the workspace is `grid-out-of-range`
// (ADR-0021 sections 9 and 10, as #683 amends them; #638 amendment 2026-10-08, grid).
// The `holler-pane` docs still describe less: the `PaneError::GridOutOfRange` doc ("a row
// or column of zero, or above `u16::MAX`") and the `HerdrPort::ensure_pane` doc ("or
// fail loudly"). #640 updates both when it finalizes `HerdrPort`. Once #640 decides where
// a workspace's extent comes from, it also updates ADR-0021 section 9's `profile apply`
// row, and any other verb's row, when that verb can ask `ensure_pane` for a cell outside
// the extent.
/// Case 2: in the workspace of 2 rows by 1 column, `r1c2` (where a transposing
/// implementation puts `r2c1`) and `r3c1` are `grid-out-of-range`, and the workspace's
/// panes are unchanged.
fn ensure_r1c2_is_grid_out_of_range(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let before = s.panes()?;
    for cell in [R1C2, R3C1] {
        let call = format!("ensure_pane({cell}) in a workspace of 2 rows by 1 column");
        expect_code(&call, s.port.ensure_pane(&s.spec(cell)), OUT_OF_RANGE)?;
        expect_eq(
            &format!("the workspace's panes after {call}"),
            &s.panes()?,
            &before,
        )?;
    }
    Ok(())
}

/// Case 3: `ensure_pane` of an occupied cell answers the pane there and makes none:
/// `r1c1` twice is one pane, the same both times.
fn ensure_is_idempotent(s: &Scratch<'_>) -> Result<(), String> {
    let first = s.ensure(R1C1)?;
    let second = s.ensure(R1C1)?;
    expect_eq("the second ensure_pane(r1c1)", &second, &first)?;
    expect_eq(
        "the workspace's panes after ensure_pane(r1c1) twice",
        s.panes()?,
        vec![first],
    )
}

/// Case 4: making a pane moves, replaces or renumbers no other. After `a` at `r1c1` and
/// `b` at `r2c1`, `ensure_pane(r1c1)` answers `a` again, and the workspace holds exactly
/// `a` and `b`, each in its cell.
fn ensure_never_moves_another_pane(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    expect_eq(
        "ensure_pane(r1c1) after r2c1 was made",
        &s.ensure(R1C1)?,
        &a,
    )?;
    expect_eq(
        "the workspace's panes, in any order",
        by_id(s.panes()?),
        by_id(vec![a, b]),
    )
}

/// Case 5: an id is never reused. After `b` at `r2c1` closes, the pane made there again
/// has an id that is neither `a`'s nor `b`'s.
fn closed_id_is_never_reused(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    s.close(&b.pane_id)?;
    let c = s.ensure(R2C1)?;
    new_id(
        "the pane made at r2c1 after its pane closed",
        &c.pane_id,
        &[&a.pane_id, &b.pane_id],
    )
}

/// Case 6: ids are unique, and an id does not change when its sibling closes. After `a`
/// at `r1c1` closes, the workspace lists `b`'s id and not `a`'s. Ids only: real Herdr
/// gives a closed pane's space to its sibling, so `b`'s cell may change.
fn ids_unique_and_stable_when_a_sibling_closes(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    new_id("the pane made at r2c1", &b.pane_id, &[&a.pane_id])?;
    s.close(&a.pane_id)?;
    let ids: Vec<PaneId> = s.panes()?.into_iter().map(|pane| pane.pane_id).collect();
    if ids.contains(&b.pane_id) && !ids.contains(&a.pane_id) {
        return Ok(());
    }
    Err(format!(
        "after closing {} (r1c1), expected the workspace's ids to include {} (r2c1) and \
         not the closed one; got {ids:?}",
        a.pane_id.as_str(),
        b.pane_id.as_str()
    ))
}

/// Case 7: closing an id that Herdr never handed out is `pane-not-found`.
fn close_unknown_is_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    expect_code(
        &format!("close({UNKNOWN_ID}), an id never handed out"),
        s.port.close(&PaneId::new(UNKNOWN_ID)),
        NOT_FOUND,
    )
}

/// Case 8: closing a pane succeeds once, and closing it again is `pane-not-found`. The
/// workspace keeps `a`, so no case empties a real workspace.
fn close_twice_is_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?.pane_id;
    s.close(&b)?;
    expect_code(
        &format!("close({}) again", b.as_str()),
        s.port.close(&b),
        NOT_FOUND,
    )
}

/// Case 9: a closed pane answers nothing: `send_text`, `send_keys` and `read` of it are
/// each `pane-not-found`.
fn calls_on_a_closed_pane_are_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?.pane_id;
    s.close(&b)?;
    let closed = b.as_str();
    expect_code(
        &format!("send_text({closed}, \"x\") after it closed"),
        s.port.send_text(&b, "x"),
        NOT_FOUND,
    )?;
    expect_code(
        &format!("send_keys({closed}, [enter]) after it closed"),
        s.port.send_keys(&b, &[enter()]),
        NOT_FOUND,
    )?;
    expect_code(
        &format!("read({closed}, 1) after it closed"),
        s.port.read(&b, 1),
        NOT_FOUND,
    )
}

/// Case 10: `read` returns at most `max_lines` lines: after five lines are typed into
/// `a`, `read(a, 2)` has at most two. This is the suite exercising the port in a scratch
/// pane, not a verb typing into a TUI (I4 binds verbs).
fn read_returns_at_most_max_lines(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?.pane_id;
    for line in 1..=TYPED_LINES {
        let text = format!("echo line{line}");
        succeeds(
            &format!("send_text({}, {text:?})", a.as_str()),
            s.port.send_text(&a, &text),
        )?;
        // ASSUMPTION (#640): the port's doc names the key `Enter`, but the suite presses
        // Herdr's own name, `enter` (spike section 4), so the real adapter needs no
        // case-folding that the port does not ask for.
        succeeds(
            &format!("send_keys({}, [enter])", a.as_str()),
            s.port.send_keys(&a, &[enter()]),
        )?;
    }
    let call = format!("read({}, {READ_LINES})", a.as_str());
    let screen = succeeds(&call, s.port.read(&a, READ_LINES))?;
    let lines = screen.lines().count();
    if lines <= READ_LINES {
        return Ok(());
    }
    Err(format!(
        "{call} after {TYPED_LINES} typed lines returned {lines} lines: {screen:?}"
    ))
}

/// Case 11: `version` reports Herdr's version: a non-empty string on one line.
fn version_is_reported(s: &Scratch<'_>) -> Result<(), String> {
    let version = succeeds("version", s.port.version())?;
    if !version.is_empty() && !version.contains(['\n', '\r']) {
        return Ok(());
    }
    Err(format!(
        "version() returned {version:?}; expected a non-empty string on one line"
    ))
}

// --- helpers ---

impl Scratch<'_> {
    /// Where `ensure_pane` puts a pane at `grid` in the fixture's workspace.
    fn spec(&self, grid: GridPos) -> HerdrSpec {
        HerdrSpec {
            session: self.session.to_owned(),
            workspace: self.workspace.to_owned(),
            grid,
        }
    }

    /// `ensure_pane` at `grid`, which must succeed.
    fn ensure(&self, grid: GridPos) -> Result<HerdrPane, String> {
        succeeds(
            &format!("ensure_pane({grid})"),
            self.port.ensure_pane(&self.spec(grid)),
        )
    }

    /// `close(pane)`, which must succeed.
    fn close(&self, pane: &PaneId) -> Result<(), String> {
        succeeds(&format!("close({})", pane.as_str()), self.port.close(pane))
    }

    /// The workspace's panes: those of `snapshot` in the fixture's session and
    /// workspace, in the order the snapshot lists them.
    fn panes(&self) -> Result<Vec<HerdrPane>, String> {
        let snapshot = succeeds("snapshot", self.port.snapshot())?;
        Ok(snapshot
            .panes
            .into_iter()
            .filter(|pane| pane.session == self.session && pane.workspace == self.workspace)
            .collect())
    }
}

/// `enter`, by Herdr's own name for the key.
fn enter() -> Key {
    Key::new("enter")
}

/// `panes` sorted by id, so two lists compare equal in any order.
fn by_id(mut panes: Vec<HerdrPane>) -> Vec<HerdrPane> {
    panes.sort_by(|x, y| x.pane_id.cmp(&y.pane_id));
    panes
}

/// `Ok` when `id`, the id of `what`, is none of the `earlier` ones.
fn new_id(what: &str, id: &PaneId, earlier: &[&PaneId]) -> Result<(), String> {
    if earlier.contains(&id) {
        return Err(format!(
            "{what} has the id {}, which an earlier pane had",
            id.as_str()
        ));
    }
    Ok(())
}
