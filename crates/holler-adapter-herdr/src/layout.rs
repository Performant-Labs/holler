//! Herdr's split tree and the one conversion from it to a `GridPos`.
//!
//! stub (#640 part 1): T-red holds only the pinned signatures. F fills the bodies.

use holler_pane::{GridPos, PaneId};

/// Herdr's only two split directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// A `right` split: the new pane is to the right.
    Right,
    /// A `down` split: the new pane is below.
    Down,
}

impl Direction {
    /// Herdr's own name: `right` or `down`.
    pub fn as_str(self) -> &'static str {
        "" // stub (#640 part 1): F fills
    }
}

/// A node of Herdr's `layout.export` tree.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// A pane.
    Pane { pane_id: PaneId },
    /// A split of one cell into `first` and `second`.
    Split {
        direction: Direction,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// The rows of slots read off a split tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridMap {
    _stub: (), // stub (#640 part 1): F replaces this with the rows of slots
}

impl GridMap {
    /// The pane at `cell`; `None` for a free cell or a nested slot.
    pub fn at(&self, _cell: GridPos) -> Option<&PaneId> {
        None // stub (#640 part 1): F fills
    }

    /// Where `pane` sits.
    pub fn position_of(&self, _pane: &PaneId) -> Option<GridPos> {
        None // stub (#640 part 1): F fills
    }

    /// The placed panes, by row then column.
    pub fn cells(&self) -> Vec<(GridPos, PaneId)> {
        Vec::new() // stub (#640 part 1): F fills
    }

    /// The panes inside a nested slot, in tree order.
    pub fn unplaced(&self) -> &[PaneId] {
        &[] // stub (#640 part 1): F fills
    }

    /// The number of rows (slots counted, nested ones included).
    pub fn rows(&self) -> u16 {
        0 // stub (#640 part 1): F fills
    }

    /// The number of slots in `row`; 0 for a row that does not exist.
    pub fn cols_in(&self, _row: u16) -> u16 {
        0 // stub (#640 part 1): F fills
    }

    /// No pane at all, placed or not.
    pub fn is_empty(&self) -> bool {
        true // stub (#640 part 1): F fills
    }
}

/// Read the grid off Herdr's tree: the root's chain of `down` splits gives the rows,
/// and each row's chain of `right` splits gives its slots, both counted from 1.
pub fn grid_of(_root: &LayoutNode) -> GridMap {
    GridMap::default() // stub (#640 part 1): F fills
}
