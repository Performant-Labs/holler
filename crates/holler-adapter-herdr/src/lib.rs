//! `holler_adapter_herdr` — the Herdr adapter: it implements `holler_pane::HerdrPort`
//! over Herdr's local socket, and is the only place that converts a `GridPos` to
//! Herdr's own order and base (epic #633, decision 7; ADR-0021 section 10).
//!
//! **Herdr has no grid** (the Herdr spike, `docs/research/herdr-api-spike.md`, sections
//! 6 and 7). A tab is a binary tree of splits. Each split divides one pane's cell
//! `right` (side by side) or `down` (stacked), the new pane is always the split's
//! `second` child, and no call places a pane at a cell. Herdr reports positions only as
//! terminal-cell rectangles, x (the column axis) first and counted from 0, and they
//! follow the attached client's terminal. A `GridPos` is row first and counted from 1.
//!
//! **The conversion** reads the tree, never the rectangles ([`layout::grid_of`]). The
//! root's chain of `down` splits gives the rows and each row's chain of `right` splits
//! gives its columns, both counted from 1 in tree order. A pane inside a split nested in
//! one cell has no position: the workspace is not a grid there.
//!
//! **The split model** ([`plan::plan_splits`]). An empty workspace starts at `r1c1`.
//! New rows come first, each a `down` split of the first pane of the row above, which
//! must be a row of one pane (a `down` split in a wider row nests). Then each row's new
//! columns, each a `right` split of the pane on its left. A cell that no single split
//! reaches is `grid-unreachable`. A workspace's size (its [`plan::Extent`]) is
//! configuration, since Herdr has none.
//!
//! The modules:
//!
//! - [`layout`] — Herdr's split tree and the one conversion from it to a `GridPos`.
//! - [`plan`] — the right/down splits that reach a cell, or `grid-unreachable`.
//! - [`protocol`] — Herdr's wire: requests, reply decoding, the version gate.
//! - [`transport`] — the socket: one request per connection, one deadline per call.
//! - [`adapter`] — `HerdrAdapter`, the `HerdrPort`.
//!
//! Nothing is re-exported flat: `layout::Direction` and `protocol::SessionState` would
//! share a root name with `holler_proto`'s root re-exports, so a crate that uses both
//! would see two of each. Every item is reached by its module path, as in
//! `holler_pane_testkit`.

pub mod adapter;
pub mod layout;
pub mod plan;
pub mod protocol;
pub mod transport;
