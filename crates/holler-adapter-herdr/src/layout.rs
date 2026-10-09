//! Herdr's split tree and the one conversion from it to a `GridPos` (epic #633,
//! decision 7; ADR-0021 section 10).
//!
//! Herdr has no grid. A tab's layout is a binary tree: an inner node splits one cell
//! `right` or `down`, and a leaf is a pane (spike section 7). The only positions Herdr
//! reports are terminal-cell rectangles, x (the column axis) first and counted from 0,
//! which follow the attached client's terminal (spike section 6). A `GridPos` is row
//! first and counted from 1. [`grid_of`] is the one place that reads the second off the
//! first, and it reads the tree, never the rectangles: they rank unaligned rows wrongly.
//!
//! The walk is the spike's (`scripts/spikes/herdr-grid.sh`, `derive()`), ported exactly:
//!
//! - The root's chain of `down` splits, flattened in tree order (`first` before
//!   `second`, so a chain nested on either side reads the same), gives the rows.
//! - Each row's chain of `right` splits, flattened the same way, gives its slots.
//! - A slot that is a pane sits at that row and column, both counted from 1. A slot that
//!   is a split is a `down` split nested inside one cell of the row: the workspace is not
//!   a grid there, so no pane inside it gets a position ([`GridMap::unplaced`]), and the
//!   later slots of the row keep their numbers.
//!
//! A split's ratio never changes a position. The base is applied in exactly two places,
//! `number` (a walk index to a row or column) and `index` (the reverse), and the order
//! in one, `cell_at`.

use holler_pane::{GridPos, PaneId};

/// Herdr's only two split directions (`left` and `up` are refused, spike section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// A `right` split: the new pane is the `second` child, right of the split one.
    Right,
    /// A `down` split: the new pane is the `second` child, below the split one.
    Down,
}

impl Direction {
    /// Herdr's own name: `right` or `down`.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Right => "right",
            Direction::Down => "down",
        }
    }
}

/// A node of Herdr's `layout.export` tree, reduced to what a position depends on.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// A pane.
    Pane { pane_id: PaneId },
    /// A split of one cell: `first` keeps `ratio` of it, and `second` takes the rest.
    Split {
        direction: Direction,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// One slot of a row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    /// A pane, at the slot's row and column.
    Pane(PaneId),
    /// A slot whose panes have no position: a split nested inside the row's cell, or a
    /// slot past the last row or column a `GridPos` can count. Its panes are in
    /// [`GridMap::unplaced`].
    Unplaced,
}

/// The rows of slots read off a split tree by [`grid_of`]. The default is an empty
/// workspace: no tree and no pane.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridMap {
    /// The rows in tree order. A row holds at least one slot, and a slot at least one
    /// pane, so the map holds a pane exactly when it holds a row.
    rows: Vec<Vec<Slot>>,
    /// The panes of every unplaced slot, in tree order.
    unplaced: Vec<PaneId>,
}

impl GridMap {
    /// The pane at `cell`; `None` for a free cell or an unplaced slot.
    pub fn at(&self, cell: GridPos) -> Option<&PaneId> {
        match self.rows.get(index(cell.row)?)?.get(index(cell.col)?)? {
            Slot::Pane(pane) => Some(pane),
            Slot::Unplaced => None,
        }
    }

    /// Where `pane` sits; `None` when it is unplaced or not in the map.
    pub fn position_of(&self, pane: &PaneId) -> Option<GridPos> {
        self.placed()
            .find(|(_, placed)| *placed == pane)
            .map(|(cell, _)| cell)
    }

    /// The placed panes, by row then column.
    pub fn cells(&self) -> Vec<(GridPos, PaneId)> {
        self.placed()
            .map(|(cell, pane)| (cell, pane.clone()))
            .collect()
    }

    /// The panes of every unplaced slot, in tree order.
    pub fn unplaced(&self) -> &[PaneId] {
        &self.unplaced
    }

    /// The number of rows, a row with an unplaced slot included (`u16::MAX` at most).
    pub fn rows(&self) -> u16 {
        saturate(self.rows.len())
    }

    /// The number of slots in `row`, unplaced ones included (`u16::MAX` at most); 0 for
    /// a row that does not exist.
    pub fn cols_in(&self, row: u16) -> u16 {
        index(row)
            .and_then(|row| self.rows.get(row))
            .map_or(0, |slots| saturate(slots.len()))
    }

    /// No pane at all, placed or not: an empty workspace, which only
    /// [`GridMap::default`] is.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Each placed pane with its cell, by row then column.
    fn placed(&self) -> impl Iterator<Item = (GridPos, &PaneId)> {
        self.rows.iter().enumerate().flat_map(|(row, slots)| {
            slots
                .iter()
                .enumerate()
                .filter_map(move |(col, slot)| match slot {
                    Slot::Pane(pane) => Some((cell_at(row, col)?, pane)),
                    Slot::Unplaced => None,
                })
        })
    }
}

/// Read the grid off Herdr's tree: the root's chain of `down` splits gives the rows,
/// and each row's chain of `right` splits gives its slots, both counted from 1.
pub fn grid_of(root: &LayoutNode) -> GridMap {
    let mut map = GridMap::default();
    for (row, row_node) in chain(root, Direction::Down).into_iter().enumerate() {
        let slots = chain(row_node, Direction::Right)
            .into_iter()
            .enumerate()
            .map(|(col, slot)| match slot {
                // A pane past the last row or column a `GridPos` counts has no position
                // either, so it is unplaced rather than dropped.
                LayoutNode::Pane { pane_id } if cell_at(row, col).is_some() => {
                    Slot::Pane(pane_id.clone())
                }
                _ => {
                    panes_in(slot, &mut map.unplaced);
                    Slot::Unplaced
                }
            })
            .collect();
        map.rows.push(slots);
    }
    map
}

/// The nodes of `node`'s chain of `direction` splits, in tree order: a split in that
/// direction gives its `first` chain and then its `second` chain, and any other node
/// is one link (the spike's `chain`).
fn chain(node: &LayoutNode, direction: Direction) -> Vec<&LayoutNode> {
    match node {
        LayoutNode::Split {
            direction: split,
            first,
            second,
            ..
        } if *split == direction => {
            let mut links = chain(first, direction);
            links.extend(chain(second, direction));
            links
        }
        _ => vec![node],
    }
}

/// Every pane under `node`, appended to `panes` in tree order (`first` before `second`).
fn panes_in(node: &LayoutNode, panes: &mut Vec<PaneId>) {
    match node {
        LayoutNode::Pane { pane_id } => panes.push(pane_id.clone()),
        LayoutNode::Split { first, second, .. } => {
            panes_in(first, panes);
            panes_in(second, panes);
        }
    }
}

/// The cell of the slot at the 0-based `row` and `col` of the walk: row first. `None`
/// past `u16::MAX`.
fn cell_at(row: usize, col: usize) -> Option<GridPos> {
    Some(GridPos {
        row: number(row)?,
        col: number(col)?,
    })
}

/// The row or column number of the 0-based walk index `index`: a `GridPos` counts from
/// 1. `None` past `u16::MAX`.
fn number(index: usize) -> Option<u16> {
    u16::try_from(index.checked_add(1)?).ok()
}

/// The 0-based walk index of the row or column `number`; `None` for 0, which is no row
/// or column.
fn index(number: u16) -> Option<usize> {
    usize::from(number).checked_sub(1)
}

/// `len` as a count of rows or slots, `u16::MAX` at most.
fn saturate(len: usize) -> u16 {
    u16::try_from(len).unwrap_or(u16::MAX)
}
