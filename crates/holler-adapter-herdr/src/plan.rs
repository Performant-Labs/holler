//! The split planner (epic #633; spike section 7): the `right` and `down` splits that
//! take a workspace to one holding every cell asked for, or the reason Herdr cannot.
//!
//! Herdr places no pane at a cell. A pane is made by splitting one, and a split divides
//! only the split pane's own cell (the nesting rule). So the plan is built the way the
//! spike built its 2x4 grid (`scripts/spikes/herdr-grid.sh`), one new cell per step and
//! never a pane that was not asked for:
//!
//! 1. An empty workspace starts at `r1c1`, the root pane of `workspace.create`.
//! 2. New rows come first, top to bottom. Row `r` is a `down` split of `r(r-1)c1`, the
//!    first pane of the last row, and that row must hold one pane: a `down` split of a
//!    pane in a wider row nests inside its cell, which is not a grid.
//! 3. Then each row's new columns, left to right. `rRcC` is a `right` split of
//!    `rRc(C-1)`, which must be the last pane of its row.
//!
//! A split's `ratio` is the share of its cell that the split pane keeps (Herdr's
//! `first` child): `1/(rows - r + 2)` when it makes row `r`, and `1/(cols - c + 2)` when
//! it makes column `c`, where `rows` and `cols` are the extent's. A whole extent built
//! this way has equal rows and equal columns (the spike's recipe). Ratios never change a
//! position (`crate::layout`).
//!
//! The checks, in order:
//!
//! - A cell outside the extent is `grid-out-of-range`, before anything else.
//! - A cell that exists is answered with no step, even in a workspace that is not a grid.
//!   A cell asked for twice is planned once.
//! - Any other cell is [`GRID_UNREACHABLE`] when the workspace is not a grid (it has an
//!   unplaced pane), or when no single split makes it: it needs a pane that is neither
//!   there nor asked for, or it is in a new row below a row of more than one pane.
//!
//! A refusal returns no plan at all, never the steps before it.

use holler_pane::error::RefusalCode;
use holler_pane::{GridPos, PaneError};

use crate::layout::{Direction, GridMap};

/// `grid-unreachable`: Herdr cannot reach the cell by splits without nesting.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");

/// Where an empty workspace starts: its root pane.
const ROOT: GridPos = GridPos { row: 1, col: 1 };

/// A workspace's size, as the caller configures it (Herdr has no grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub rows: u16,
    pub cols: u16,
}

/// The cells wanted, inside an extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub extent: Extent,
    pub cells: Vec<GridPos>,
}

/// One Herdr call of a plan.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `workspace.create`: its root pane is r1c1.
    CreateRoot,
    /// Split the pane at `from`; the new pane lands at `creates`.
    Split {
        from: GridPos,
        direction: Direction,
        ratio: f64,
        creates: GridPos,
    },
}

/// The steps that take `existing` to a map holding every cell of `target`: the cells
/// that exist need none, and each step makes one new cell, rows first. The input order
/// of `target.cells` does not change the plan.
pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError> {
    let extent = target.extent;
    let mut cells = target.cells.clone();
    cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
    cells.dedup();
    if let Some(&outside) = cells.iter().find(|cell| !contains(extent, **cell)) {
        return Err(out_of_range(outside, extent));
    }
    cells.retain(|cell| existing.at(*cell).is_none());
    let Some(&first) = cells.first() else {
        return Ok(Vec::new());
    };
    let unplaced = existing.unplaced().len();
    if unplaced > 0 {
        return Err(refuse(
            first,
            &format!(
                "the workspace is not a grid ({} in a split nested inside one cell), and no \
                 pane is added to a workspace that is not one",
                count(unplaced, "pane")
            ),
        ));
    }
    // A new cell in column 1 is a new row; the rest are new columns. Both stay in order.
    let (rows, cols): (Vec<GridPos>, Vec<GridPos>) =
        cells.into_iter().partition(|cell| cell.col == 1);
    let mut grid = Widths::of(existing);
    rows.into_iter()
        .chain(cols)
        .map(|cell| grid.add(cell, extent))
        .collect()
}

/// The grid a plan builds on, as the width of each row: the existing panes, then each
/// step planned so far. Every slot of it holds a pane (a map with an unplaced pane is
/// refused before one is built).
struct Widths(Vec<u16>);

impl Widths {
    /// The grid of `map`.
    fn of(map: &GridMap) -> Self {
        Self((1..=map.rows()).map(|row| map.cols_in(row)).collect())
    }

    /// The step that makes `cell`, recorded in the grid.
    fn add(&mut self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
        let step = self.step_to(cell, extent)?;
        match usize::from(cell.row)
            .checked_sub(1)
            .and_then(|row| self.0.get_mut(row))
        {
            Some(width) => *width = cell.col,
            None => self.0.push(1),
        }
        Ok(step)
    }

    /// The one split that makes `cell`, a cell that is not in the grid yet.
    fn step_to(&self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
        if self.0.is_empty() {
            if cell == ROOT {
                return Ok(Step::CreateRoot);
            }
            return Err(refuse(
                cell,
                "an empty workspace starts at r1c1, and Herdr makes every other pane by \
                 splitting one",
            ));
        }
        match self.width(cell.row) {
            Some(width) => Self::right_of(cell, width, extent),
            None => self.below(cell, extent),
        }
    }

    /// A new cell of an existing row of `width` panes: a `right` split of the pane on its
    /// left, which must be the last of the row.
    fn right_of(cell: GridPos, width: u16, extent: Extent) -> Result<Step, PaneError> {
        let left = GridPos {
            row: cell.row,
            col: cell.col.saturating_sub(1),
        };
        if left.col != width {
            return Err(gap(cell, left));
        }
        Ok(Step::Split {
            from: left,
            direction: Direction::Right,
            ratio: share(extent.cols, cell.col),
            creates: cell,
        })
    }

    /// A cell of a row that does not exist yet. Only the first cell of the next row is
    /// one split away: a `down` split of the first pane of the last row, which must hold
    /// one pane.
    fn below(&self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
        let last = self.rows();
        let above = GridPos {
            row: cell.row.saturating_sub(1),
            col: 1,
        };
        if above.row != last {
            return Err(gap(cell, above));
        }
        let width = self.width(last).unwrap_or(0);
        if width != 1 {
            return Err(refuse(
                cell,
                &format!(
                    "a row can be added only below a row of one pane, and row {last} has {} \
                     (a down split there nests inside one cell, which is not a grid); place \
                     rows before columns",
                    count(usize::from(width), "pane")
                ),
            ));
        }
        if cell.col != 1 {
            let left = GridPos {
                row: cell.row,
                col: cell.col.saturating_sub(1),
            };
            return Err(gap(cell, left));
        }
        Ok(Step::Split {
            from: above,
            direction: Direction::Down,
            ratio: share(extent.rows, cell.row),
            creates: cell,
        })
    }

    /// The number of rows.
    fn rows(&self) -> u16 {
        u16::try_from(self.0.len()).unwrap_or(u16::MAX)
    }

    /// The number of panes in `row`; `None` for a row that does not exist.
    fn width(&self, row: u16) -> Option<u16> {
        self.0.get(usize::from(row).checked_sub(1)?).copied()
    }
}

/// Whether `cell` is one of the extent's cells: a zero is none.
fn contains(extent: Extent, cell: GridPos) -> bool {
    (1..=extent.rows).contains(&cell.row) && (1..=extent.cols).contains(&cell.col)
}

/// The share of its cell that the split pane keeps when it makes row or column
/// `number` of `size`: it spans the `size - number + 2` equal parts from its own to the
/// last, and keeps one.
fn share(size: u16, number: u16) -> f64 {
    1.0 / (f64::from(size) - f64::from(number) + 2.0)
}

/// `grid-out-of-range` for `cell`, naming the extent.
fn out_of_range(cell: GridPos, extent: Extent) -> PaneError {
    PaneError::GridOutOfRange {
        what: format!(
            "{cell} is outside the workspace, which is {} by {}",
            count(usize::from(extent.rows), "row"),
            count(usize::from(extent.cols), "column")
        ),
    }
}

/// `grid-unreachable` for `cell`, because `needed` is neither there nor asked for.
fn gap(cell: GridPos, needed: GridPos) -> PaneError {
    refuse(
        cell,
        &format!(
            "Herdr makes a pane only by a right split of the pane on its left or a down \
             split of the first pane of the row above, and {needed} is neither there nor \
             asked for"
        ),
    )
}

/// `grid-unreachable` for `cell`, saying `why`.
fn refuse(cell: GridPos, why: &str) -> PaneError {
    PaneError::Refused {
        code: GRID_UNREACHABLE,
        message: format!("{cell} cannot be reached: {why}"),
    }
}

/// `n` `noun`s, in the singular for one: "2 rows", "1 column".
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
