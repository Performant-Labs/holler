//! Reserved by issue #669 (epic #633); deliberately empty.
//!
//! The pane verbs run CLI-side, against the ports, and the hub is the store only: it
//! registers no adapters (epic #633, ruling 0). The adapters are wired in
//! `holler-cli`'s `pane/wiring.rs` (#649), so this module has no job. It stays
//! declared in `lib.rs` as an empty placeholder.
