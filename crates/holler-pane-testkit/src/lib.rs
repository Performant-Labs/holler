//! `holler_pane_testkit` — fakes of every `holler_pane` port and the conformance
//! suite each adapter must pass, so no test touches a real Herdr, tmux or OpenCode
//! (epic #633).
//!
//! Empty skeleton (story #637); story #638 fills it. It must not depend on
//! `holler-cli`: the hub and the CLI take it as a dev-dependency.
