#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! The conversion from Herdr's split tree to a `GridPos` (#640 part 1, AC 1-6). This is
//! the only place a tree becomes a row and a column, so a transposed or off-by-one
//! walk must fail here.

mod common;

use common::{cell, chain, id, pane, spike_2x4, spike_2x4_with, spike_cells, split};
use holler_adapter_herdr::layout::{grid_of, Direction, GridMap};

#[test]
fn ac1_the_spike_2x4_tree_puts_the_marker_at_row_2_column_3() {
    let map = grid_of(&spike_2x4());

    let marker = map.position_of(&id("w1:p7"));
    assert_eq!(marker, Some(cell(2, 3)));
    assert_eq!(marker.unwrap().to_string(), "r2c3");
    assert_eq!(map.cells(), spike_cells(), "all eight, row first");
    assert_eq!((map.rows(), map.cols_in(1), map.cols_in(2)), (2, 4, 4));
    assert_eq!(
        map.cols_in(3),
        0,
        "a row that does not exist has no columns"
    );
    assert!(map.unplaced().is_empty());
    assert!(!map.is_empty());
}

#[test]
fn ac2_row_and_column_are_not_swapped_and_count_from_one() {
    let map = grid_of(&spike_2x4());

    // The transposition guard: the pane at the end of the down split is row 2 column 1,
    // and the first pane of the right chain is row 1 column 2.
    assert_eq!(map.position_of(&id("w1:p2")), Some(cell(2, 1)));
    assert_eq!(map.position_of(&id("w1:p3")), Some(cell(1, 2)));
    assert_eq!(map.at(cell(1, 2)), Some(&id("w1:p3")));
    assert_eq!(map.at(cell(2, 1)), Some(&id("w1:p2")));
    // The off-by-one guard: nothing past the edges, and nothing at 0.
    assert_eq!(map.at(cell(3, 1)), None);
    assert_eq!(map.at(cell(1, 5)), None);
    assert_eq!(map.at(cell(0, 1)), None);
    assert_eq!(map.at(cell(1, 0)), None);
    for (placed, _) in map.cells() {
        assert!(placed.row >= 1 && placed.col >= 1, "{placed} counts from 0");
    }
}

#[test]
fn ac3_a_down_chain_nested_on_either_side_flattens_in_order() {
    let right_nested = split(
        Direction::Down,
        0.5,
        pane("A"),
        split(Direction::Down, 0.5, pane("B"), pane("C")),
    );
    let left_nested = split(
        Direction::Down,
        0.5,
        split(Direction::Down, 0.5, pane("A"), pane("B")),
        pane("C"),
    );
    let expected = vec![
        (cell(1, 1), id("A")),
        (cell(2, 1), id("B")),
        (cell(3, 1), id("C")),
    ];

    assert_eq!(grid_of(&right_nested).cells(), expected);
    assert_eq!(grid_of(&left_nested).cells(), expected);
}

#[test]
fn ac3_a_right_chain_nested_on_either_side_flattens_in_order() {
    let right_nested = split(
        Direction::Right,
        0.5,
        pane("A"),
        split(Direction::Right, 0.5, pane("B"), pane("C")),
    );
    let left_nested = split(
        Direction::Right,
        0.5,
        split(Direction::Right, 0.5, pane("A"), pane("B")),
        pane("C"),
    );
    let expected = vec![
        (cell(1, 1), id("A")),
        (cell(1, 2), id("B")),
        (cell(1, 3), id("C")),
    ];

    assert_eq!(grid_of(&right_nested).cells(), expected);
    assert_eq!(grid_of(&left_nested).cells(), expected);
}

#[test]
fn ac4_unaligned_rows_are_ranked_by_the_tree_and_ratios_change_nothing() {
    let rows = |r1: f64, r2: f64| {
        split(
            Direction::Down,
            0.5,
            split(Direction::Right, r1, pane("a"), pane("b")),
            split(Direction::Right, r2, pane("c"), pane("d")),
        )
    };
    let expected = vec![
        (cell(1, 1), id("a")),
        (cell(1, 2), id("b")),
        (cell(2, 1), id("c")),
        (cell(2, 2), id("d")),
    ];

    assert_eq!(
        grid_of(&rows(0.5, 0.3)).cells(),
        expected,
        "spike section 6"
    );
    assert_eq!(grid_of(&rows(0.01, 0.99)).cells(), expected);
}

#[test]
fn ac5_a_nested_slot_is_unplaced_and_later_columns_keep_their_slot_numbers() {
    let nested = split(Direction::Down, 0.5, pane("w1:p3"), pane("w1:p9"));
    let map = grid_of(&spike_2x4_with(nested));

    assert_eq!(map.unplaced(), [id("w1:p3"), id("w1:p9")]);
    assert_eq!(map.at(cell(1, 2)), None);
    assert_eq!(map.position_of(&id("w1:p3")), None);
    assert_eq!(map.position_of(&id("w1:p9")), None);
    assert_eq!(
        map.at(cell(1, 3)),
        Some(&id("w1:p4")),
        "slot numbers are kept"
    );
    assert_eq!(map.position_of(&id("w1:p5")), Some(cell(1, 4)));
    assert_eq!((map.rows(), map.cols_in(1), map.cols_in(2)), (2, 4, 4));
    let row_2: Vec<_> = spike_cells()
        .into_iter()
        .filter(|(at, _)| at.row == 2)
        .collect();
    let placed_row_2: Vec<_> = map
        .cells()
        .into_iter()
        .filter(|(at, _)| at.row == 2)
        .collect();
    assert_eq!(placed_row_2, row_2, "row 2 is unchanged");
    assert_eq!(map.cells().len(), 7);
}

#[test]
fn ac6_a_columns_first_tree_places_nothing_but_is_not_empty() {
    let columns_first = split(
        Direction::Right,
        0.5,
        split(Direction::Down, 0.5, pane("A"), pane("C")),
        split(Direction::Down, 0.5, pane("B"), pane("D")),
    );
    let map = grid_of(&columns_first);

    assert!(map.cells().is_empty());
    assert_eq!(map.unplaced(), [id("A"), id("C"), id("B"), id("D")]);
    assert!(
        !map.is_empty(),
        "panes exist, so a workspace must not be created"
    );
}

#[test]
fn ac6_the_default_map_is_an_empty_workspace() {
    let map = GridMap::default();

    assert!(map.is_empty());
    assert_eq!(map.rows(), 0);
    assert_eq!(map.cols_in(1), 0);
    assert!(map.cells().is_empty() && map.unplaced().is_empty());
    assert_eq!(map.at(cell(1, 1)), None);
}

#[test]
fn a_lone_pane_is_row_1_column_1() {
    let map = grid_of(&pane("only"));

    assert_eq!(map.cells(), vec![(cell(1, 1), id("only"))]);
    assert_eq!((map.rows(), map.cols_in(1)), (1, 1));
    assert!(!map.is_empty());
}

#[test]
fn a_chain_helper_tree_agrees_with_the_hand_built_one() {
    // The planner tests build their maps with `chain`; pin that it makes the shape the
    // spike's walk expects, so a bug in the helper cannot hide a bug in `grid_of`.
    let by_chain = chain(
        Direction::Down,
        vec![pane("A"), pane("B"), pane("C")],
        &[0.5, 0.5],
    );

    assert_eq!(
        grid_of(&by_chain).cells(),
        vec![
            (cell(1, 1), id("A")),
            (cell(2, 1), id("B")),
            (cell(3, 1), id("C"))
        ]
    );
}
