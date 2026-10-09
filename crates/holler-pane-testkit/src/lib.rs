//! `holler_pane_testkit` — fakes of every `holler_pane` port and the conformance
//! suite each implementation must pass, so no test touches a real Herdr, tmux or
//! OpenCode (epic #633, story #638).
//!
//! It depends on `holler-pane` and `serde_json` (for the envelope checker) alone and
//! must not depend on `holler-cli`: the hub and the CLI take it as a dev-dependency, so
//! a normal dependency back would make a cycle (ADR-0021 section 5). Every item is
//! reached by its module path. There are no flat re-exports, so a later slice never
//! edits this file.
//!
//! The modules, and the slice of #638 that fills each:
//!
//! - [`fault`] — the fault switch every fake shares: a wedged or failing port, one-shot
//!   errors, a slow call and the call log (slice a, #638).
//! - `feed` (crate-private) — the change feed every fake store shares: the cursors, the
//!   watch from `Cursor(0)` and the idle wait (slice a, #638).
//! - [`fixture`] — `sample_pane`, a valid and harmless `Pane` (slice a, #638).
//! - [`pane_store`] — `FakePaneStore`, the in-memory `PaneStore` (slice a, #638).
//! - [`conformance`] — the suites: `pane_store` (slice a, #638); `profile_store` and
//!   `profile_scope` (slice c, #682); `herdr` (slice d, #683); `host` and `harness`
//!   (slice e, #684).
//! - [`envelope`] — the JSON-envelope checker (slice b, #681).
//! - [`profile_store`] and [`profile_scope`] — `FakeProfileStore` and
//!   `FakeProfileScope` (slice c, #682).
//! - [`herdr`] and [`prober`] — `FakeHerdr` and `FakeProber` (slice d, #683).
//! - [`host`] and [`harness`] — `FakeHost` and `FakeHarness` (slice e, #684).
//!
//! The modules of slices b to e are empty stubs, declared here so that no two slices
//! edit this file.

pub mod conformance;
pub mod envelope;
pub mod fault;
mod feed;
pub mod fixture;
pub mod harness;
pub mod herdr;
pub mod host;
pub mod pane_store;
pub mod prober;
pub mod profile_scope;
pub mod profile_store;
