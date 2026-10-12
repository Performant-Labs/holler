# Handoff-F: Phase 6 - OpenCode adapter tmux client forces UTF-8 (#844)

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Issue:** #844

## What was done
- `crates/holler-adapter-opencode/src/tui.rs`: `tmux_command` adds `-u` after the socket flag and before the call's
  arguments (`tmux [-S p | -L n] -u <args...>`); its doc comment states the new argv and why; the module-doc rule list
  gains a "UTF-8 output" rule.
- `CHANGELOG.md`: a bullet under `## [Unreleased]` / `### Bug Fixes` linking #633 and #844.
- `docs/adr/ADR-0021.md`: one sentence in "`HarnessPort` as built (#642)", item 1 (SHOWN through tmux).

## Design decisions
- `-u`, not parsing `_` as TAB (brief decision 1): a client flag, no change to `QUERY_FORMAT`, `parse_query` or any
  reply; `_` parsing would be ambiguous with a title holding `_`.
- Placement after the socket flag (brief decision 2): keeps `$1 == -S` for the fake tmux; for `TmuxSocket::Default`
  `-u` is the first argument. Added with `.arg("-u")` in the existing builder chain, before `.args(args)`.
- No locale variable is set or removed on the child (brief decision 4).

## Reuse / extend-vs-new
Extended the single builder, `tui::tmux_command`, per the brief's Reuse map. No new object, no second tmux builder.

## Architecture notes for A
One line in the one tmux builder plus docs. No public signature change, no new dependency, no protocol or state change.
`lib.rs:234` (`env -u`) left alone, per A's warn 2.

## Deviations from spec / wireframe
None. A's warn 1 (say why the real-tmux test is not `#[ignore]`) concerns T's test and the brief; I did not touch
`docs/testing.md`, and T's decisions entry records the reason (a pin that must fail CI).

## Tier 1 self-check (incl. tests now GREEN)
- `rustfmt --edition 2021 --check crates/holler-adapter-opencode/src/tui.rs`: clean.
- `cargo test -p holler-adapter-opencode`: agent 11, attach 23, hermetic 33, prompt 12, tui 13, utf8_locale 1 passed;
  real_opencode 12 ignored (opt-in). The utf8 test ran (tmux 3.7c on PATH; no skip line with `--nocapture`).
- `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode --no-deps`: clean (new intra-doc links resolve).
- `cargo test -p holler-cli --test pane_integration`: 71 passed, 4 ignored (opt-in #649 real-OpenCode scenario not run).
- `bash scripts/changelog-check.sh`: ok. `bash scripts/lint.sh`: exit 0, one size warn for an untouched file
  (`holler-pane/src/error.rs`).
- AC 4: `grep -n '"-u"' crates/holler-adapter-opencode/src/tui.rs` -> `135:        .arg("-u")`, the one place.

## Evidence appendix
`docs/handoffs/844/evidence.md`

## Tests that look wrong (for T)
None.

## Known issues
None.

## Files changed
- `crates/holler-adapter-opencode/src/tui.rs`
- `CHANGELOG.md`
- `docs/adr/ADR-0021.md`
