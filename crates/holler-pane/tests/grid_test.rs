#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! `GridPos` (epic #633 decision 7, #637 AC 2): the one grid-position type, its one
//! parser, the `rRcC` display and the `{"row","col","pos"}` serde form.

use holler_pane::GridPos;

/// What `GridPos::parse` must say for an input: the cell, or the refusal code.
type Cell = Result<(u16, u16), String>;

/// `Ok((row, col))` or `Err(code)`: what `GridPos::parse` must say for an input.
fn parsed(input: &str) -> Result<(u16, u16), String> {
    GridPos::parse(input)
        .map(|g| (g.row, g.col))
        .map_err(|e| e.code().to_string())
}

#[test]
fn grid_parse_table() {
    let ok = |row, col| Ok((row, col));
    let ambiguous = || Err("grid-ambiguous".to_string());
    let out_of_range = || Err("grid-out-of-range".to_string());

    #[rustfmt::skip]
    let table: Vec<(&str, Cell)> = vec![
        // The three accepted spellings give the same cell.
        ("r2c1", ok(2, 1)),
        ("c1r2", ok(2, 1)),
        ("2,1", ok(2, 1)),
        // A bare pair is row,col and is never read as col,row.
        ("1,2", ok(1, 2)),
        ("3,12", ok(3, 12)),
        // Upper case and surrounding ASCII whitespace are accepted.
        ("R2C1", ok(2, 1)),
        ("C1R2", ok(2, 1)),
        ("r2C1", ok(2, 1)),
        (" r2c1 ", ok(2, 1)),
        ("\tc1r2\n", ok(2, 1)),
        ("  2,1  ", ok(2, 1)),
        // The ends of the u16 range.
        ("r1c1", ok(1, 1)),
        ("r65535c65535", ok(65535, 65535)),
        ("65535,1", ok(65535, 1)),
        // Ambiguous: a missing half, a repeated label, a mix, a bare number, nothing.
        ("r2", ambiguous()),
        ("c1", ambiguous()),
        ("r2c1c3", ambiguous()),
        ("2,1,3", ambiguous()),
        ("21", ambiguous()),
        ("c1r2c3", ambiguous()),
        ("", ambiguous()),
        ("r2r3", ambiguous()),
        ("c1c2", ambiguous()),
        // Zero is out of range (the grid is 1-based), in every spelling.
        ("r0c1", out_of_range()),
        ("c0r1", out_of_range()),
        ("0,1", out_of_range()),
        ("r2c0", out_of_range()),
        ("2,0", out_of_range()),
        // Above u16::MAX is out of range, however large the number.
        ("r65536c1", out_of_range()),
        ("r1c65536", out_of_range()),
        ("65536,1", out_of_range()),
        ("1,65536", out_of_range()),
        ("c99999999999r1", out_of_range()),
        ("r1c99999999999999999999999999999999", out_of_range()),
    ];

    for (input, want) in table {
        assert_eq!(parsed(input), want, "GridPos::parse({input:?})");
    }
}

#[test]
fn grid_refuses_interior_whitespace_and_junk() {
    for input in [
        "r2 c1",
        "r 2c1",
        "2, 1",
        "2 ,1",
        "2 1",
        "c1 r2",
        "r2c1 x",
        "abc",
        "r2c",
        "r,c",
        "2,",
        ",1",
        "r-1c1",
        "r1.5c1",
        "r2c1;",
        "r\u{0661}c1",
    ] {
        assert!(GridPos::parse(input).is_err(), "{input:?} must be refused");
    }
}

#[test]
fn grid_display_is_row_first_and_round_trips_every_cell_of_a_12_by_12_grid() {
    assert_eq!(GridPos { row: 2, col: 1 }.to_string(), "r2c1");
    for row in 1..=12u16 {
        for col in 1..=12u16 {
            let pos = GridPos { row, col };
            let text = pos.to_string();
            assert_eq!(text, format!("r{row}c{col}"));
            let back = GridPos::parse(&text).expect("a printed position parses");
            assert_eq!((back.row, back.col), (row, col), "parse(format({text}))");
        }
    }
}

#[test]
fn grid_serde_writes_row_col_pos_in_that_order_and_reads_it_back() {
    let pos = GridPos { row: 2, col: 1 };
    let text = serde_json::to_string(&pos).unwrap();
    assert_eq!(text, r#"{"row":2,"col":1,"pos":"r2c1"}"#);

    let back: GridPos = serde_json::from_str(&text).unwrap();
    assert_eq!((back.row, back.col), (2, 1));

    // The same cell with a wide label still prints row first.
    let wide = serde_json::to_string(&GridPos { row: 12, col: 3 }).unwrap();
    assert_eq!(wide, r#"{"row":12,"col":3,"pos":"r12c3"}"#);
}

#[test]
fn grid_serde_refuses_a_pos_that_disagrees_and_a_zero_cell() {
    // `pos` is derived; a transposed or stale label must not be trusted silently.
    for bad in [
        r#"{"row":2,"col":1,"pos":"r1c2"}"#,
        r#"{"row":2,"col":1,"pos":"r3c1"}"#,
        r#"{"row":2,"col":1,"pos":"c2r1"}"#,
        r#"{"row":0,"col":1,"pos":"r0c1"}"#,
        r#"{"row":1,"col":0,"pos":"r1c0"}"#,
    ] {
        assert!(serde_json::from_str::<GridPos>(bad).is_err(), "{bad}");
    }
}
