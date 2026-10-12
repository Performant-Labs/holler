#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! `plan_splits`: the right/down splits that reach a set of cells, and the refusals when
//! Herdr cannot reach one without nesting (#640 part 1, AC 7-13). Existing maps are read
//! from trees by `grid_of`; the planner never sees a tree itself.

mod common;

use common::{cell, chain, pane, split};
use holler_adapter_herdr::layout::{grid_of, Direction, GridMap, LayoutNode};
use holler_adapter_herdr::plan::{plan_splits, Extent, Step, Target, GRID_UNREACHABLE};
use holler_pane::error::{class_of, ErrorClass};
use holler_pane::{GridPos, HerdrPort, HerdrSpec, PaneError};
use holler_pane_testkit::herdr::{FakeHerdr, Placement};

const TOLERANCE: f64 = 1e-9;

/// A rows-of-columns tree with `counts[r]` panes in row `r + 1`.
fn rows_tree(counts: &[usize]) -> LayoutNode {
    let mut n = 0;
    let rows = counts
        .iter()
        .map(|&count| {
            let slots = (0..count)
                .map(|_| {
                    n += 1;
                    pane(&format!("w1:p{n}"))
                })
                .collect();
            chain(Direction::Right, slots, &vec![0.5; count - 1])
        })
        .collect();
    chain(Direction::Down, rows, &vec![0.5; counts.len() - 1])
}

fn map_of(counts: &[usize]) -> GridMap {
    grid_of(&rows_tree(counts))
}

fn target(rows: u16, cols: u16, cells: &[(u16, u16)]) -> Target {
    Target {
        extent: Extent { rows, cols },
        cells: cells.iter().map(|&(r, c)| cell(r, c)).collect(),
    }
}

fn down(from: (u16, u16), ratio: f64, creates: (u16, u16)) -> Step {
    step(from, Direction::Down, ratio, creates)
}

fn right(from: (u16, u16), ratio: f64, creates: (u16, u16)) -> Step {
    step(from, Direction::Right, ratio, creates)
}

fn step(from: (u16, u16), direction: Direction, ratio: f64, creates: (u16, u16)) -> Step {
    Step::Split {
        from: cell(from.0, from.1),
        direction,
        ratio,
        creates: cell(creates.0, creates.1),
    }
}

/// Equal step for step, with ratios compared within `TOLERANCE`.
fn assert_plan(actual: Result<Vec<Step>, PaneError>, expected: &[Step]) {
    let actual = actual.expect("a plan");
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (got, want) in actual.iter().zip(expected) {
        match (got, want) {
            (Step::CreateRoot, Step::CreateRoot) => {}
            (
                Step::Split {
                    from,
                    direction,
                    ratio,
                    creates,
                },
                Step::Split {
                    from: f,
                    direction: d,
                    ratio: r,
                    creates: c,
                },
            ) => {
                assert_eq!((from, direction, creates), (f, d, c), "{actual:?}");
                assert!(
                    (ratio - r).abs() < TOLERANCE,
                    "{ratio} != {r} in {actual:?}"
                );
            }
            _ => panic!("{got:?} != {want:?} in {actual:?}"),
        }
    }
}

/// The refusal's message, after checking it is `grid-unreachable`, a refusal, one line
/// and names the cell.
fn unreachable_naming(result: Result<Vec<Step>, PaneError>, named: GridPos) -> String {
    let Err(PaneError::Refused { code, message }) = result else {
        panic!("expected grid-unreachable, got {result:?}");
    };
    assert_eq!(code, GRID_UNREACHABLE);
    assert_eq!(class_of(code.as_str()), ErrorClass::Refusal);
    assert!(message.contains(&named.to_string()), "{message}");
    assert!(!message.contains('\n'), "{message}");
    message
}

#[test]
fn ac7_an_empty_workspace_to_2x4_is_the_spikes_verified_recipe() {
    let all: Vec<(u16, u16)> = (1..=2).flat_map(|r| (1..=4).map(move |c| (r, c))).collect();
    let expected = [
        Step::CreateRoot,
        down((1, 1), 1.0 / 2.0, (2, 1)),
        right((1, 1), 1.0 / 4.0, (1, 2)),
        right((1, 2), 1.0 / 3.0, (1, 3)),
        right((1, 3), 1.0 / 2.0, (1, 4)),
        right((2, 1), 1.0 / 4.0, (2, 2)),
        right((2, 2), 1.0 / 3.0, (2, 3)),
        right((2, 3), 1.0 / 2.0, (2, 4)),
    ];

    assert_plan(
        plan_splits(&GridMap::default(), &target(2, 4, &all)),
        &expected,
    );

    let reversed: Vec<(u16, u16)> = all.iter().rev().copied().collect();
    assert_plan(
        plan_splits(&GridMap::default(), &target(2, 4, &reversed)),
        &expected,
    );
}

#[test]
fn ac7_three_rows_split_the_row_above_with_a_third_then_a_half() {
    let expected = [
        Step::CreateRoot,
        down((1, 1), 1.0 / 3.0, (2, 1)),
        down((2, 1), 1.0 / 2.0, (3, 1)),
    ];

    assert_plan(
        plan_splits(
            &GridMap::default(),
            &target(3, 1, &[(1, 1), (2, 1), (3, 1)]),
        ),
        &expected,
    );
}

#[test]
fn ac8_one_more_cell_is_one_split() {
    let existing = map_of(&[1, 1]);

    assert_plan(
        plan_splits(&existing, &target(2, 2, &[(1, 1), (2, 1), (1, 2)])),
        &[right((1, 1), 1.0 / 2.0, (1, 2))],
    );
}

#[test]
fn ac8_rows_are_planned_before_columns() {
    let existing = map_of(&[1]);

    assert_plan(
        plan_splits(&existing, &target(2, 2, &[(1, 2), (2, 1)])),
        &[
            down((1, 1), 1.0 / 2.0, (2, 1)),
            right((1, 1), 1.0 / 2.0, (1, 2)),
        ],
    );
}

#[test]
fn ac9_a_target_whose_cells_all_exist_is_no_steps() {
    let existing = map_of(&[2, 1]);

    let plan = plan_splits(&existing, &target(2, 2, &[(1, 1), (1, 2), (2, 1)]));

    assert_eq!(plan, Ok(vec![]));
}

#[test]
fn ac9_existing_cells_are_answered_even_with_an_unplaced_pane() {
    let nested = chain(
        Direction::Right,
        vec![
            pane("A"),
            split(Direction::Down, 0.5, pane("B"), pane("C")),
            pane("D"),
        ],
        &[0.5, 0.5],
    );
    let existing = grid_of(&nested);

    assert_eq!(
        plan_splits(&existing, &target(1, 3, &[(1, 1), (1, 3)])),
        Ok(vec![])
    );
}

#[test]
fn ac9_a_duplicate_cell_in_the_target_is_planned_once() {
    let existing = map_of(&[1]);

    assert_plan(
        plan_splits(&existing, &target(1, 2, &[(1, 2), (1, 2), (1, 1), (1, 1)])),
        &[right((1, 1), 1.0 / 2.0, (1, 2))],
    );
}

#[test]
fn ac10_a_cell_outside_the_extent_is_grid_out_of_range_naming_cell_and_extent() {
    for outside in [cell(1, 2), cell(3, 1), cell(0, 1), cell(1, 0)] {
        let wanted = Target {
            extent: Extent { rows: 2, cols: 1 },
            cells: vec![cell(1, 1), outside],
        };

        let Err(PaneError::GridOutOfRange { what }) = plan_splits(&map_of(&[1]), &wanted) else {
            panic!("{outside} is outside 2 rows by 1 column");
        };

        assert!(what.contains(&outside.to_string()), "{what}");
        assert!(what.contains("2 rows by 1 column"), "{what}");
        assert!(!what.contains('\n'), "{what}");
    }
}

#[test]
fn ac10_the_range_is_checked_before_reachability() {
    // r1c2 is outside 2x1 and also unreachable (an empty workspace starts at r1c1).
    let result = plan_splits(&GridMap::default(), &target(2, 1, &[(1, 2)]));

    assert!(
        matches!(result, Err(PaneError::GridOutOfRange { .. })),
        "{result:?}"
    );
}

#[test]
fn ac11a_an_empty_workspace_starts_at_r1c1() {
    for first in [(1, 2), (2, 1)] {
        let result = plan_splits(&GridMap::default(), &target(2, 2, &[first]));
        let named = cell(first.0, first.1);

        unreachable_naming(result, named);
    }
}

#[test]
fn ac12_an_empty_workspace_accepts_r1c1_alone() {
    assert_plan(
        plan_splits(&GridMap::default(), &target(2, 2, &[(1, 1)])),
        &[Step::CreateRoot],
    );
}

#[test]
fn ac11b_a_gap_in_a_row_or_a_column_is_unreachable() {
    let existing = map_of(&[1]);

    unreachable_naming(plan_splits(&existing, &target(1, 3, &[(1, 3)])), cell(1, 3));
    unreachable_naming(plan_splits(&existing, &target(3, 1, &[(3, 1)])), cell(3, 1));
}

#[test]
fn ac11b_no_partial_plan_when_one_cell_of_the_target_is_unreachable() {
    let existing = map_of(&[1]);

    // r1c2 alone is reachable; r1c4 is not (no r1c3), so the whole call is refused.
    unreachable_naming(
        plan_splits(&existing, &target(1, 4, &[(1, 2), (1, 4)])),
        cell(1, 4),
    );
}

#[test]
fn ac11c_a_new_row_below_a_row_of_two_is_unreachable() {
    let existing = map_of(&[2]);

    for wanted in [(2, 1), (2, 2)] {
        let message = unreachable_naming(
            plan_splits(&existing, &target(2, 2, &[wanted])),
            cell(wanted.0, wanted.1),
        );
        assert!(message.to_lowercase().contains("one pane"), "{message}");
    }
}

#[test]
fn ac11c_a_new_row_below_a_row_of_one_is_reachable() {
    assert_plan(
        plan_splits(&map_of(&[1]), &target(2, 2, &[(2, 1)])),
        &[down((1, 1), 1.0 / 2.0, (2, 1))],
    );
}

#[test]
fn ac11d_no_new_cell_while_any_pane_is_unplaced() {
    // Row 1 is `A | (B over C) | D`: r1c4 would be a plain right split of D, but the
    // workspace is not a grid, so it is refused. A cell that exists is still answered.
    let nested = chain(
        Direction::Right,
        vec![
            pane("A"),
            split(Direction::Down, 0.5, pane("B"), pane("C")),
            pane("D"),
        ],
        &[0.5, 0.5],
    );
    let existing = grid_of(&nested);

    unreachable_naming(plan_splits(&existing, &target(1, 4, &[(1, 4)])), cell(1, 4));
    assert_eq!(plan_splits(&existing, &target(1, 4, &[(1, 1)])), Ok(vec![]));
}

#[test]
fn ac11d_a_workspace_of_only_unplaced_panes_is_never_created_again() {
    // Columns first: panes exist but none is placed. r1c1 must not become `CreateRoot`,
    // which would try to create a workspace that already exists.
    let columns_first = split(
        Direction::Right,
        0.5,
        split(Direction::Down, 0.5, pane("A"), pane("C")),
        split(Direction::Down, 0.5, pane("B"), pane("D")),
    );
    let existing = grid_of(&columns_first);

    unreachable_naming(plan_splits(&existing, &target(2, 2, &[(1, 1)])), cell(1, 1));
}

#[test]
fn ac12_the_fakes_sequence_stops_where_real_herdr_nests() {
    // The sequence `FakeHerdr`'s split-only placement accepted before #853: r1c1, r1c2,
    // r2c2, r2c1. Real Herdr nests the r2c2 that a `down` split of r1c2 makes, so the
    // adapter refuses it (Decision 6), and since #853 the fake refuses it too (the
    // cross-check below).
    assert_plan(
        plan_splits(&map_of(&[1]), &target(2, 2, &[(1, 2)])),
        &[right((1, 1), 1.0 / 2.0, (1, 2))],
    );
    unreachable_naming(
        plan_splits(&map_of(&[2]), &target(2, 2, &[(2, 2)])),
        cell(2, 2),
    );
}

#[test]
fn ac13_the_refusal_code_is_the_test_kits() {
    assert_eq!(
        GRID_UNREACHABLE.as_str(),
        holler_pane_testkit::herdr::GRID_UNREACHABLE.as_str()
    );
}

/// The widths (panes per row) of every grid a split-only workspace of at most `rows` rows
/// of at most `cols` panes can hold with no hole (each built by successive splits), the
/// empty workspace first.
fn grids_up_to(rows: usize, cols: usize) -> Vec<Vec<usize>> {
    let mut grids = vec![vec![]];
    let mut last = vec![vec![]];
    for _ in 0..rows {
        last = last
            .iter()
            .flat_map(|widths: &Vec<usize>| {
                (1..=cols).map(move |width| {
                    let mut grown = widths.clone();
                    grown.push(width);
                    grown
                })
            })
            .collect();
        grids.extend(last.iter().cloned());
    }
    grids
}

/// A fresh `FakeHerdr` with one workspace of `extent` holding the grid `widths`, placed
/// absolutely, then switched to split-only placement.
fn split_only_fake(extent: u16, widths: &[usize]) -> FakeHerdr {
    let herdr = FakeHerdr::new("scratch")
        .with_workspace("w", extent, extent)
        .unwrap();
    for (row, &width) in (1..).zip(widths) {
        for col in (1..).take(width) {
            herdr.ensure_pane(&fake_spec(row, col)).unwrap();
        }
    }
    herdr.set_placement(Placement::SplitOnly);
    herdr
}

fn fake_spec(row: u16, col: u16) -> HerdrSpec {
    HerdrSpec {
        session: "scratch".to_owned(),
        workspace: "w".to_owned(),
        grid: cell(row, col),
    }
}

/// #853 (ADR-0021 section 10): `FakeHerdr`'s split-only placement accepts a free cell
/// exactly when `plan_splits` plans it in one step, and refuses it with the planner's code
/// otherwise, so a test of `profile apply` on the fake meets the adapter's rule. Every grid
/// with no hole of up to 3 rows of up to 3 panes (and the empty workspace), every free cell of
/// a 3-by-3 extent and the ring just outside it. A fresh fake per probe: an accepted probe
/// adds a pane.
#[test]
fn the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it() {
    const EXTENT: u16 = 3;
    for widths in grids_up_to(3, 3) {
        let existing = if widths.is_empty() {
            GridMap::default()
        } else {
            map_of(&widths)
        };
        for row in 1..=EXTENT + 1 {
            for col in 1..=EXTENT + 1 {
                let occupied = usize::from(row)
                    .checked_sub(1)
                    .and_then(|r| widths.get(r))
                    .is_some_and(|&width| usize::from(col) <= width);
                if occupied {
                    continue;
                }
                let planned = plan_splits(&existing, &target(EXTENT, EXTENT, &[(row, col)]));
                let placed = split_only_fake(EXTENT, &widths).ensure_pane(&fake_spec(row, col));
                let at = format!("r{row}c{col} on the grid {widths:?}");
                match (&planned, &placed) {
                    (Ok(steps), Ok(_)) => assert_eq!(steps.len(), 1, "{at}: {steps:?}"),
                    (Err(plan), Err(fake)) => {
                        assert_eq!(plan.code(), fake.code(), "{at}: {plan} / {fake}")
                    }
                    _ => panic!(
                        "{at}: the planner answered {planned:?}, the fake {placed:?}"
                    ),
                }
            }
        }
    }
}
