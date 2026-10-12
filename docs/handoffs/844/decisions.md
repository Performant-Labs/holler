
## A (up-front plan review) — 2026-10-12
- **Decided:** BLOCK. The `-u` in `tmux_command` is the right place and shape. The brief must cover `pane_integration/guards.rs`, which pins `LANG=C.UTF-8` (`:109`, `:128`), and must correct "ambient `LANG`" (the rig inherits nothing). Four warns (LC_CTYPE removal, helper rationale, server cleanup, fixture require-vs-skip).
- **Assumed:** tmux picks UTF-8 from `LC_ALL`, then `LC_CTYPE`, then `LANG` (from memory of tmux's client code; not re-verified here).
- **Hedged:** whether finding 1 is a block or a warn; chosen block because an always-run test in AC 3's own command fails as the brief is written.
- **Evidence:** read the brief, `tui.rs`, `fake-tmux`, `fake_tmux.rs`, `tui_test.rs:190-230`, `rig.rs`, `guards.rs:100-128`, `scratch/mod.rs:234-260`, workspace `Cargo.toml` lints, `scripts/lint.sh`; grep for `tmux_command`, `C.UTF-8`.
