//! `FakeHerdr`, the in-memory `HerdrPort` that pane-control tests run against instead
//! of a Herdr server, and `HerdrOp`, the port's methods as its faults and its call log
//! name them (slice d of #638, #683).
//!
//! The fake models what the Herdr spike found (`docs/research/herdr-api-spike.md`)
//! where a verb relies on it, in `GridPos` cells only: the adapter (#640) is the only
//! code that knows Herdr's own order and base (ADR-0021 section 10). It passes the
//! `HerdrPort` conformance suite ([`crate::conformance::herdr`]) in both placements.
//! What only the fake has is a size per workspace, the split-only placement, the
//! selectable version, fault injection, a pane whose shell exits or whose program
//! prints, and the record of what was typed into a pane.
//!
//! Where the fake departs from real Herdr, or decides what the spike left open, an
//! `ASSUMPTION (#640)` comment says so at the code concerned.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use holler_pane::error::RefusalCode;
use holler_pane::{
    GridPos, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId,
};

use crate::fault::{FaultSwitch, PortOp};

// ASSUMPTION (#640): only protocol 22 was tested. How other versions differ is
// unverified (spike section 13: no second build was compared), so `UNSUPPORTED_VERSION`
// is invented.
/// The version string of the one Herdr build the spike tested (protocol 22).
pub const PROTOCOL_22_VERSION: &str = "0.9.1-preview.2026-09-21-0ff0f27e2226";

// ASSUMPTION (#640): provisional test vocabulary, like `GRID_UNREACHABLE` below.
/// What a `herdr-version-unsupported` message names as supported (spike section 13).
/// Provisional test vocabulary: a verb test compares against this constant, never the
/// literal.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";

/// The version string of the fake's unsupported build. Invented: no second build was
/// compared.
pub const UNSUPPORTED_VERSION: &str = "99.0.0-fake";

// ASSUMPTION (#640): `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` are the fake's own
// values. No verb or adapter raises or names them yet, and no suite case tests them, so
// they do not bind #640 today. A merged code is never renamed (ADR-0021 section 9), so a
// verb test (#644 and later) compares against the constants, never the literals. #640
// either declares the same values in its own file and asserts in its dev-tests that they
// equal the test kit's, or the fake takes #640's values before any verb story pins them.
/// The open code that split-only placement refuses an unreachable cell with: a refusal
/// (`ErrorClass::Refusal`, exit 3). Provisional test vocabulary: a verb test compares
/// against `GRID_UNREACHABLE.as_str()`, never the literal.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");

/// A method of the `HerdrPort`, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HerdrOp {
    EnsurePane,
    SendText,
    SendKeys,
    Read,
    Close,
    Snapshot,
    Version,
}

impl PortOp for HerdrOp {
    fn as_str(self) -> &'static str {
        match self {
            HerdrOp::EnsurePane => "herdr.ensure_pane",
            HerdrOp::SendText => "herdr.send_text",
            HerdrOp::SendKeys => "herdr.send_keys",
            HerdrOp::Read => "herdr.read",
            HerdrOp::Close => "herdr.close",
            HerdrOp::Snapshot => "herdr.snapshot",
            HerdrOp::Version => "herdr.version",
        }
    }
}

/// How `ensure_pane` may place a new pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Any free cell inside the workspace (the default).
    #[default]
    Absolute,
    /// Only a cell that a split reaches, as in real Herdr (spike section 7): the root
    /// `r1c1` of an empty workspace, or a free cell right of a pane (a `right` split) or
    /// below one (a `down` split). Any other free cell is refused with
    /// [`GRID_UNREACHABLE`].
    SplitOnly,
}

/// Which Herdr build `version()` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HerdrVersion {
    /// Protocol 22: `version()` is `Ok(PROTOCOL_22_VERSION)` (the default).
    #[default]
    Protocol22,
    /// A build the adapter does not know: `version()` is `herdr-version-unsupported`.
    Unsupported,
}

/// One `send_text` or `send_keys` that reached a pane: its payload. A failed attempt
/// (an unknown or closed pane, or one a fault stopped) is not here. Every attempt is in
/// `faults().calls()` as [`HerdrOp::SendText`] or [`HerdrOp::SendKeys`], and that log is
/// the check for I4's "no keystroke".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    Text { pane: PaneId, text: String },
    Keys { pane: PaneId, keys: Vec<Key> },
}

/// An in-memory `HerdrPort` serving one Herdr session.
///
/// A test declares each workspace with a size ([`FakeHerdr::with_workspace`]).
/// `ensure_pane` checks, in this order: the session and the workspace (one the fake
/// does not serve is `unavailable`), the range (a cell outside the workspace is
/// `grid-out-of-range`), an occupant (an occupied cell answers its pane and changes
/// nothing, in either placement), the placement (see [`Placement`]), and then it mints
/// the next id. An id is `w<N>:p<M>`: the workspace's number, then its pane counter in
/// base 36 (spike section 5). The counter only goes up, so an id is never reused after
/// a close or a vanish. `close` and [`FakeHerdr::vanish`] free a pane's cell, and no
/// other pane is ever moved, replaced or renumbered.
///
/// A pane's screen shows what is typed into it (`enter` breaks a line) and what
/// [`FakeHerdr::print`] adds, and `read` returns its last lines. An unknown, closed or
/// vanished id is `pane-not-found` everywhere. Every `send_text` and `send_keys` that
/// reaches a pane is in [`FakeHerdr::sent`].
///
/// Every port method first passes [`FakeHerdr::faults`], and when that fails, it
/// returns the error and changes nothing. The call log there holds every attempt, and
/// it is where a test checks I4's "no keystroke". `vanish` and `print` are the outside
/// world acting on a pane: they bypass the faults and the call log.
///
/// It is not `Clone`: share it behind an `Arc`, because a copy would split the session.
pub struct FakeHerdr {
    state: Mutex<State>,
    faults: FaultSwitch<HerdrOp>,
}

/// What the fake holds, behind its lock.
struct State {
    /// The one session the fake serves.
    session: String,
    /// The workspaces, in declaration order.
    workspaces: Vec<Workspace>,
    placement: Placement,
    version: HerdrVersion,
    /// What reached a pane, oldest first.
    sent: Vec<Sent>,
}

/// One declared workspace and its panes.
struct Workspace {
    name: String,
    /// The `N` of its pane ids: 1, 2, ... in declaration order.
    number: usize,
    rows: u16,
    cols: u16,
    /// The pane counter: the `M` of the last id minted here, 0 before the first.
    minted: u64,
    /// The panes by `(row, col)`, so they list by row and then by column.
    panes: BTreeMap<(u16, u16), LivePane>,
}

/// One pane that exists: its id and the text its screen shows.
struct LivePane {
    id: PaneId,
    screen: String,
}

impl FakeHerdr {
    /// Serves the session `session`, with no workspace, [`Placement::Absolute`] and
    /// [`HerdrVersion::Protocol22`].
    pub fn new(session: &str) -> Self {
        Self {
            state: Mutex::new(State {
                session: session.to_owned(),
                workspaces: Vec::new(),
                placement: Placement::default(),
                version: HerdrVersion::default(),
                sent: Vec::new(),
            }),
            faults: FaultSwitch::new(),
        }
    }

    // ASSUMPTION (#640): Herdr has no grid (spike section 2), so the fake is told a
    // workspace's size. How the real adapter learns it (the profile's extent,
    // configuration, the tree walk) is #640's.
    // ASSUMPTION (#640): a real workspace is created with a root pane (spike section 4,
    // `workspace.create`). The fake's starts empty, so a verb test sees an empty Herdr.
    /// Declare a workspace of `rows` by `cols` cells, with no pane. Workspaces are
    /// numbered 1, 2, ... in declaration order. A name already declared is `usage`.
    pub fn with_workspace(self, name: &str, rows: u16, cols: u16) -> Result<Self, PaneError> {
        self.lock().declare(name, rows, cols)?;
        Ok(self)
    }

    /// How `ensure_pane` places a new pane from now on.
    pub fn set_placement(&self, placement: Placement) {
        self.lock().placement = placement;
    }

    /// Which build `version()` reports from now on.
    pub fn set_version(&self, version: HerdrVersion) {
        self.lock().version = version;
    }

    /// The fault switch of every port method and the log of the calls made through the
    /// port.
    pub fn faults(&self) -> &FaultSwitch<HerdrOp> {
        &self.faults
    }

    /// The payloads of every `send_text` and `send_keys` that reached a pane, oldest
    /// first. It omits a failed send: to assert I4's "no keystroke", check that
    /// `faults().calls()` holds no [`HerdrOp::SendText`] or [`HerdrOp::SendKeys`], not
    /// that this is empty (a verb that tried to type and failed would pass that).
    pub fn sent(&self) -> Vec<Sent> {
        self.lock().sent.clone()
    }

    /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is
    /// free, its id is never reused, and every later call naming it is
    /// `pane-not-found`. It bypasses the faults and the call log. `pane-not-found` when
    /// no such pane exists.
    pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock().remove(pane)
    }

    /// The pane's program prints `text`, which `read` then shows. It bypasses the faults
    /// and the call log. `pane-not-found` when no such pane exists.
    pub fn print(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.lock().pane_mut(pane)?.screen.push_str(text);
        Ok(())
    }

    /// The state, locked. A poisoned lock is taken over: a test that panicked while
    /// holding it must not wedge every later call.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HerdrPort for FakeHerdr {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        self.faults.enter(HerdrOp::EnsurePane)?;
        let mut state = self.lock();
        let placement = state.placement;
        let pane_id = state.workspace_mut(spec)?.place(spec.grid, placement)?;
        Ok(HerdrPane {
            session: spec.session.clone(),
            workspace: spec.workspace.clone(),
            pane_id,
            grid: spec.grid,
        })
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.faults.enter(HerdrOp::SendText)?;
        let mut state = self.lock();
        state.pane_mut(pane)?.screen.push_str(text);
        state.sent.push(Sent::Text {
            pane: pane.clone(),
            text: text.to_owned(),
        });
        Ok(())
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        self.faults.enter(HerdrOp::SendKeys)?;
        // ASSUMPTION (#640): the port's doc names keys `Enter` and `C-c`, but Herdr's own
        // names are `enter` and `ctrl+c` (spike section 4). The fake breaks a line for
        // `enter` in any case, as a shell echoes it, and shows nothing for any other key.
        let enters = keys
            .iter()
            .filter(|key| key.as_str().eq_ignore_ascii_case("enter"))
            .count();
        let mut state = self.lock();
        state.pane_mut(pane)?.screen.push_str(&"\n".repeat(enters));
        state.sent.push(Sent::Keys {
            pane: pane.clone(),
            keys: keys.to_vec(),
        });
        Ok(())
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        self.faults.enter(HerdrOp::Read)?;
        Ok(last_lines(&self.lock().pane_mut(pane)?.screen, max_lines))
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.faults.enter(HerdrOp::Close)?;
        // ASSUMPTION (#640): real Herdr gives a closed pane's space to its sibling (spike
        // section 7), so the cell the tree walk reads for a surviving pane can change. The
        // fake keeps every pane in its cell, and the suite asserts ids, not positions,
        // after a close.
        self.lock().remove(pane)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        self.faults.enter(HerdrOp::Snapshot)?;
        let state = self.lock();
        let panes = state
            .workspaces
            .iter()
            .flat_map(|workspace| workspace.listed(&state.session))
            .collect();
        Ok(HerdrSnapshot { panes })
    }

    fn version(&self) -> Result<String, PaneError> {
        self.faults.enter(HerdrOp::Version)?;
        let version = self.lock().version;
        match version {
            HerdrVersion::Protocol22 => Ok(PROTOCOL_22_VERSION.to_owned()),
            // ASSUMPTION (#640): only `version()` refuses an unsupported build. Whether
            // the adapter also refuses every other call after a failed version check is
            // #640's.
            HerdrVersion::Unsupported => Err(PaneError::HerdrVersionUnsupported {
                message: format!(
                    "Herdr reports version {UNSUPPORTED_VERSION}; the supported one is \
                     {SUPPORTED_VERSIONS}"
                ),
            }),
        }
    }
}

impl State {
    /// Add the workspace `name`, numbered after the ones declared before it.
    fn declare(&mut self, name: &str, rows: u16, cols: u16) -> Result<(), PaneError> {
        let taken = self
            .workspaces
            .iter()
            .any(|workspace| workspace.name == name);
        if taken {
            return Err(PaneError::Usage {
                message: format!("the fake Herdr workspace {name:?} is already declared"),
            });
        }
        let number = self.workspaces.len() + 1;
        self.workspaces.push(Workspace {
            name: name.to_owned(),
            number,
            rows,
            cols,
            minted: 0,
            panes: BTreeMap::new(),
        });
        Ok(())
    }

    /// Step 1 of `ensure_pane`: the workspace `spec` names, in the session the fake
    /// serves.
    fn workspace_mut(&mut self, spec: &HerdrSpec) -> Result<&mut Workspace, PaneError> {
        // ASSUMPTION (#640): a session or a workspace the fake does not serve is
        // `unavailable`. Whether the adapter creates a missing workspace
        // (`workspace.create`) is #640's.
        let served = spec.session == self.session;
        let session = &self.session;
        self.workspaces
            .iter_mut()
            .find(|workspace| served && workspace.name == spec.workspace)
            .ok_or_else(|| PaneError::Unavailable {
                what: format!(
                    "Herdr session {:?}, workspace {:?}: the fake serves only session {:?} \
                     and the workspaces declared in it",
                    spec.session, spec.workspace, session
                ),
            })
    }

    /// The pane `id`, in whichever workspace holds it: `pane-not-found` when none does
    /// (it was never minted, or it closed or vanished).
    fn pane_mut(&mut self, id: &PaneId) -> Result<&mut LivePane, PaneError> {
        self.workspaces
            .iter_mut()
            .flat_map(|workspace| workspace.panes.values_mut())
            .find(|pane| pane.id == *id)
            .ok_or_else(|| not_found(id))
    }

    /// Remove the pane `id`, leaving every other pane in its cell with its id:
    /// `pane-not-found` when no workspace holds it.
    fn remove(&mut self, id: &PaneId) -> Result<(), PaneError> {
        for workspace in &mut self.workspaces {
            let before = workspace.panes.len();
            workspace.panes.retain(|_, pane| pane.id != *id);
            if workspace.panes.len() < before {
                return Ok(());
            }
        }
        Err(not_found(id))
    }
}

impl Workspace {
    /// Steps 2 to 5 of `ensure_pane` in this workspace: the id of the pane at `grid`,
    /// made when the cell is free and `placement` reaches it.
    fn place(&mut self, grid: GridPos, placement: Placement) -> Result<PaneId, PaneError> {
        self.check_range(grid)?;
        if let Some(occupant) = self.occupant(grid) {
            return Ok(occupant.clone());
        }
        if placement == Placement::SplitOnly {
            self.check_split(grid)?;
        }
        self.mint(grid)
    }

    /// Step 2: `grid` must be one of the workspace's cells, and a zero is not one.
    fn check_range(&self, grid: GridPos) -> Result<(), PaneError> {
        if (1..=self.rows).contains(&grid.row) && (1..=self.cols).contains(&grid.col) {
            return Ok(());
        }
        Err(PaneError::GridOutOfRange {
            what: format!(
                "{grid} is outside workspace {:?}, which is {} by {}",
                self.name,
                count(self.rows, "row"),
                count(self.cols, "column")
            ),
        })
    }

    /// Step 3: the id of the pane at `grid`, if one is there.
    fn occupant(&self, grid: GridPos) -> Option<&PaneId> {
        self.panes.get(&(grid.row, grid.col)).map(|pane| &pane.id)
    }

    /// Step 4, split-only placement: a free cell is reachable as the root of an empty
    /// workspace (`r1c1`), or by a `right` split of the pane on its left or a `down`
    /// split of the pane above it. No call places a pane at a cell (spike section 7).
    fn check_split(&self, grid: GridPos) -> Result<(), PaneError> {
        // Row 0 and column 0 hold no pane, so nothing is left of the first column or
        // above the first row.
        let holds = |row: u16, col: u16| self.panes.contains_key(&(row, col));
        let root = self.panes.is_empty() && (grid.row, grid.col) == (1, 1);
        let left = holds(grid.row, grid.col.saturating_sub(1));
        let above = holds(grid.row.saturating_sub(1), grid.col);
        if root || left || above {
            return Ok(());
        }
        Err(PaneError::Refused {
            code: GRID_UNREACHABLE,
            message: format!(
                "{grid} cannot be reached in workspace {:?}: Herdr places a pane only by a \
                 right or down split of an existing one (an empty workspace starts at r1c1), \
                 and no pane sits left of {grid} or above it",
                self.name
            ),
        })
    }

    /// Step 5: a new pane at `grid` with an empty screen, under the next id. The counter
    /// only goes up, so no id is ever reused; an overflow is `unavailable` and creates
    /// nothing.
    fn mint(&mut self, grid: GridPos) -> Result<PaneId, PaneError> {
        let counter = self
            .minted
            .checked_add(1)
            .ok_or_else(|| PaneError::Unavailable {
                what: format!("the pane counter of workspace {:?} overflowed", self.name),
            })?;
        let id = PaneId::new(format!("w{}:p{}", self.number, base36(counter)));
        self.minted = counter;
        self.panes.insert(
            (grid.row, grid.col),
            LivePane {
                id: id.clone(),
                screen: String::new(),
            },
        );
        Ok(id)
    }

    /// Every pane of the workspace, by row and then by column, as `snapshot` lists it.
    fn listed<'a>(&'a self, session: &'a str) -> impl Iterator<Item = HerdrPane> + 'a {
        self.panes.iter().map(move |(&(row, col), pane)| HerdrPane {
            session: session.to_owned(),
            workspace: self.name.clone(),
            pane_id: pane.id.clone(),
            grid: GridPos { row, col },
        })
    }
}

/// `pane-not-found` for `id`, as Herdr answers for a pane it does not have.
fn not_found(id: &PaneId) -> PaneError {
    PaneError::PaneNotFound {
        what: id.as_str().to_owned(),
    }
}

/// `n` `noun`s, in the singular for one: "2 rows", "1 column".
fn count(n: u16, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// `n` in base 36 with upper-case letters, as Herdr numbers its panes (spike section
/// 5): `1` to `9`, `A` to `Z`, then `10`.
fn base36(n: u64) -> String {
    let lowest_first: String =
        std::iter::successors(Some(n), |rest| (*rest >= 36).then_some(rest / 36))
            .filter_map(|rest| char::from_digit(u32::try_from(rest % 36).ok()?, 36))
            .collect();
    lowest_first
        .chars()
        .rev()
        .map(|digit| digit.to_ascii_uppercase())
        .collect()
}

/// The last `max_lines` lines of `screen` (as `str::lines` splits it), joined with
/// `"\n"`, with no newline after the last: `""` for none.
fn last_lines(screen: &str, max_lines: usize) -> String {
    let skip = screen.lines().count().saturating_sub(max_lines);
    screen.lines().skip(skip).collect::<Vec<_>>().join("\n")
}
