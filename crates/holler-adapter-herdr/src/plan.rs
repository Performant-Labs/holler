//! The split planner: the right/down splits that reach a set of cells.
//!
//! stub (#640 part 1): T-red holds only the pinned signatures. F fills the bodies.

use holler_pane::error::RefusalCode;
use holler_pane::{GridPos, PaneError};

use crate::layout::{Direction, GridMap};

/// `grid-unreachable`: Herdr cannot reach the cell by splits without nesting.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");

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

/// The steps that take `existing` to a map holding every cell of `target`.
pub fn plan_splits(_existing: &GridMap, _target: &Target) -> Result<Vec<Step>, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}
