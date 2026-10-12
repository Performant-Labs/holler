# Handoff-A-dup: Phase 7 - OpenCode adapter tmux client forces UTF-8 (#844)  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Diff base:** origin/main   **Diff head:** ba17a04
**Reuse map:** docs/handoffs/844-brief.md (Files / Reuse map)
**Verdict:** PASS

## Summary
F extended the single builder the map named, `tui::tmux_command`, with one `.arg("-u")`; no second tmux builder exists
(`Command::new` appears once in the adapter's `src/`). The only near-copies are the two small helpers in the new
`utf8_locale_test.rs`, which the brief justified in writing (the rig cannot be included under `dead_code = "deny"`).

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-adapter-opencode/tests/utf8_locale_test.rs:30,40,52` | `on_path`, the private-socket `tmux` runner and `Guard` re-write `real_opencode/rig.rs`. Justified in the brief and in the file's module doc; accepted as a deliberate decision, not drift. | Optional follow-up: lift the shared tmux test helpers into `tests/support/` if a third test file needs them. |

No duplication in production code; the extension is clean. Fixture, `ac11c`, `guards.rs` and `scratch/mod.rs` edits extend
the existing objects in place. No layering, protocol, state or size drift (`tui.rs` is 326 lines).

## Notes for F
None.
