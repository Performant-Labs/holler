//! Tree builders shared by the layout, planner and protocol tests (#640 part 1). Each
//! test file uses a few of them, so the rest would trip `dead_code`.
#![allow(dead_code)] // #640

use holler_adapter_herdr::layout::{Direction, LayoutNode};
use holler_pane::{GridPos, PaneId};

/// A pane id, verbatim.
pub fn id(text: &str) -> PaneId {
    PaneId::new(text)
}

/// A cell, row first and counted from 1.
pub fn cell(row: u16, col: u16) -> GridPos {
    GridPos { row, col }
}

/// A leaf pane.
pub fn pane(text: &str) -> LayoutNode {
    LayoutNode::Pane { pane_id: id(text) }
}

/// A split with `first` and `second`.
pub fn split(
    direction: Direction,
    ratio: f64,
    first: LayoutNode,
    second: LayoutNode,
) -> LayoutNode {
    LayoutNode::Split {
        direction,
        ratio,
        first: Box::new(first),
        second: Box::new(second),
    }
}

/// `nodes` as a chain of splits that each leave the new node on the `second` side, the
/// way splitting the newest pane builds one (`ratios` has one entry per split).
pub fn chain(direction: Direction, nodes: Vec<LayoutNode>, ratios: &[f64]) -> LayoutNode {
    let mut nodes = nodes.into_iter().rev();
    let mut node = nodes.next().expect("a chain needs a node");
    for (first, ratio) in nodes.zip(ratios.iter().rev()) {
        node = split(direction, *ratio, first, node);
    }
    node
}

/// The spike's row: four slots built with `right` 0.25, 0.3333 and 0.5.
pub fn spike_row(slots: Vec<LayoutNode>) -> LayoutNode {
    chain(Direction::Right, slots, &[0.25, 0.3333, 0.5])
}

/// The spike's verified 2x4 build (`herdr-grid.sh`, spike section 6). Row 1 is
/// `w1:p1 w1:p3 w1:p4 w1:p5` and row 2 is `w1:p2 w1:p6 w1:p7 w1:p8`; `r1c2` replaces the
/// slot of `w1:p3`, so that a test can nest it.
pub fn spike_2x4_with(r1c2: LayoutNode) -> LayoutNode {
    let row1 = spike_row(vec![pane("w1:p1"), r1c2, pane("w1:p4"), pane("w1:p5")]);
    let row2 = spike_row(vec![
        pane("w1:p2"),
        pane("w1:p6"),
        pane("w1:p7"),
        pane("w1:p8"),
    ]);
    split(Direction::Down, 0.5, row1, row2)
}

/// The spike's 2x4 tree.
pub fn spike_2x4() -> LayoutNode {
    spike_2x4_with(pane("w1:p3"))
}

/// The spike's 2x4 cells, row first, with the ids of `spike_2x4`.
pub fn spike_cells() -> Vec<(GridPos, PaneId)> {
    let ids = [
        ["w1:p1", "w1:p3", "w1:p4", "w1:p5"],
        ["w1:p2", "w1:p6", "w1:p7", "w1:p8"],
    ];
    let mut cells = Vec::new();
    for (r, row) in ids.iter().enumerate() {
        for (c, pane_id) in row.iter().enumerate() {
            cells.push((cell(r as u16 + 1, c as u16 + 1), id(pane_id)));
        }
    }
    cells
}
