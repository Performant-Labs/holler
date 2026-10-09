//! `GridPos`: the one grid-position type of pane control (epic #633, decision 7).
//!
//! Grid notation is ROWCOL and 1-based. The type, its parser, the `rRcC` display
//! and the serde form are defined here once; the Herdr adapter (#640) is the only
//! place that converts a `GridPos` to Herdr's own order and base.
//!
//! - **Input** ([`GridPos::parse`]): `r<row>c<col>` or `c<col>r<row>` (labelled, in
//!   either order) or a bare `<row>,<col>`, which is always row first and never
//!   col,row. Upper case and surrounding ASCII whitespace are accepted; interior
//!   whitespace is refused. A missing half, a repeated label, a mix such as `r2c1c3`
//!   or anything unreadable is `grid-ambiguous`; a zero, or a number above
//!   `u16::MAX`, is `grid-out-of-range`.
//! - **Output**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use crate::error::{excerpt, PaneError};

/// What every refusal of a grid position tells the user to write instead.
const FORMS: &str = "write r<row>c<col>, c<col>r<row> or <row>,<col>";

/// What every out-of-range refusal says about the bounds.
const BOUNDS: &str = "rows and columns run from 1 to 65535";

/// A cell of the layout grid, 1-based: `row` counts down, `col` counts across.
///
/// The fields are public so a cell can be written `GridPos { row: 2, col: 1 }`; a
/// zero there is not a cell, and [`GridPos::parse`] and the serde reader both
/// refuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    /// The row, from 1.
    pub row: u16,
    /// The column, from 1.
    pub col: u16,
}

impl GridPos {
    /// Parse `r2c1`, `c1r2` or `2,1` (see the module docs for the exact rules).
    pub fn parse(text: &str) -> Result<GridPos, PaneError> {
        let lower = text.trim_ascii().to_ascii_lowercase();
        let (row, col) = split_cells(&lower).ok_or_else(|| PaneError::GridAmbiguous {
            what: format!("{} ({FORMS})", excerpt(text)),
        })?;
        match (coordinate(row), coordinate(col)) {
            (Some(row), Some(col)) => Ok(GridPos { row, col }),
            _ => Err(PaneError::GridOutOfRange {
                what: format!("{} ({BOUNDS})", excerpt(text)),
            }),
        }
    }

    /// A cell from its two numbers, refusing a zero.
    fn from_cells(row: u16, col: u16) -> Result<GridPos, PaneError> {
        if row == 0 || col == 0 {
            return Err(PaneError::GridOutOfRange {
                what: format!("row {row}, column {col} ({BOUNDS})"),
            });
        }
        Ok(GridPos { row, col })
    }
}

impl fmt::Display for GridPos {
    /// The `rRcC` form, row first: `r2c1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}c{}", self.row, self.col)
    }
}

/// The digit strings of the row and the column of `text` (already trimmed and
/// lower-cased), or `None` when its structure is wrong.
fn split_cells(text: &str) -> Option<(&str, &str)> {
    if text.contains(',') {
        split_pair(text)
    } else {
        split_labelled(text)
    }
}

/// `<row>,<col>`: exactly two digit runs around one comma, row first.
fn split_pair(text: &str) -> Option<(&str, &str)> {
    let (row, col) = text.split_once(',')?;
    (is_number(row) && is_number(col)).then_some((row, col))
}

/// `r<row>c<col>` or `c<col>r<row>`: two labelled cells with different labels.
fn split_labelled(text: &str) -> Option<(&str, &str)> {
    let (first_label, first, rest) = take_cell(text)?;
    let (second_label, second, tail) = take_cell(rest)?;
    if !tail.is_empty() || first_label == second_label {
        return None;
    }
    if first_label == 'r' {
        Some((first, second))
    } else {
        Some((second, first))
    }
}

/// One labelled cell at the start of `text`: its label (`r` or `c`), its digits and
/// what follows.
fn take_cell(text: &str) -> Option<(char, &str, &str)> {
    let label = text.chars().next().filter(|c| matches!(c, 'r' | 'c'))?;
    let body = text.strip_prefix(label)?;
    let end = body
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(body.len());
    let (digits, rest) = body.split_at(end);
    (!digits.is_empty()).then_some((label, digits, rest))
}

/// A non-empty run of ASCII digits (so no sign, no space, no other script).
fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The coordinate `digits` spells: `None` for zero or a number above `u16::MAX`,
/// however many digits it has.
fn coordinate(digits: &str) -> Option<u16> {
    digits.parse::<u16>().ok().filter(|n| *n > 0)
}

impl Serialize for GridPos {
    /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("GridPos", 3)?;
        out.serialize_field("row", &self.row)?;
        out.serialize_field("col", &self.col)?;
        out.serialize_field("pos", &self.to_string())?;
        out.end()
    }
}

/// The serde form before it is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGridPos {
    row: u16,
    col: u16,
    /// Derived from `row` and `col`; optional on input, and refused when it
    /// disagrees (a stale or transposed label must not be trusted silently).
    #[serde(default)]
    pos: Option<String>,
}

impl<'de> Deserialize<'de> for GridPos {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawGridPos::deserialize(deserializer)?;
        let cell = GridPos::from_cells(raw.row, raw.col)
            .map_err(|e| de::Error::custom(e.coded_message()))?;
        if let Some(label) = raw.pos {
            if GridPos::parse(&label).ok() != Some(cell) {
                let err = PaneError::GridAmbiguous {
                    what: format!(
                        "pos {} disagrees with row {} and column {}",
                        excerpt(&label),
                        raw.row,
                        raw.col
                    ),
                };
                return Err(de::Error::custom(err.coded_message()));
            }
        }
        Ok(cell)
    }
}
