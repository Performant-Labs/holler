# Handoff-A: #844 oc-tmux-utf8  (up-front plan review, second pass)

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Brief reviewed:** docs/handoffs/844-brief.md (amended in a66de75)   **Reuse map:** the brief's "Reuse map" paragraph   **Wireframe:** N/A (no UI)
**Verdict:** PASS

## Summary
The amended brief closes the one block of the first pass. `guards.rs` and the `scratch/mod.rs:234` doc line are now in Files,
AC 3 says the binary runs with no locale at all (verified: `binary_env` inherits nothing), the `LC_CTYPE` removals, the helper
rationale, the Drop guard and the require-`-u` fixture are all in. The production change stays one line in the single
builder, `tmux_command`, with no wire, state, secret or concurrency impact. Three non-blocking warnings remain.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | New `utf8_locale_test.rs` runs real tmux in the default `cargo test` (skip when absent) | pattern consistency | The repo's convention for real-tmux tests is `#[ignore]`: `holler-adapter-host/tests/real_tmux_test.rs` ("needs tmux; run with --ignored") and `docs/testing.md:447`. The brief's choice is defensible (a regression pin that is `#[ignore]`d never fails CI, and the test uses a private socket and kills it), but it departs from that convention without saying so. | Add one sentence to the brief (and `docs/testing.md` if F touches it) giving the reason: the pin must run in CI, so it skips on a missing `tmux` instead of being ignored. Keep the socket path short (`/tmp`, as `rig.rs:368` does with `hlr642r-`) so macOS's `sun_path` limit is not hit. |
| 2 | warn | AC 4: `grep -n '"-u"' crates/holler-adapter-opencode/src/tui.rs` | naming / verifiability | The grep is scoped to `tui.rs`, and passes. An unscoped grep of `src/` also hits `lib.rs:234` (`env -u <var>`, the server launcher's, not a tmux flag). A reader could mistake it for a second tmux `-u`. | Keep the AC scoped to `tui.rs` as written; F should not "tidy" `lib.rs:234`. |
| 3 | warn | CHANGELOG bullet "under `[Unreleased]` (Fixes)" | naming | The heading in `[Unreleased]` is `### Bug Fixes` (CHANGELOG.md:600), not "Fixes". | Add the bullet under `### Bug Fixes`. |

## Checked and consistent
- **Choke point:** `tmux_command` is the only tmux builder in the crate (`attach.rs:217` is its sole caller; no other `Command::new` of tmux in `src/`). Adding `-u` there covers `attach_tui`, `select_session` and `shown_session` in every delivery variant.
- **Placement:** `-u` after the socket flag and before the call's arguments keeps `$1 == -S` for the fixture; `ac11c`'s `flag ++ ["-u"] ++ call` is the right extension, and `TmuxSocket::Default` yields `-u` first.
- **Parser and wire:** no change to `QUERY_FORMAT`, `parse_query` or any protocol file; the host and Claude adapters are space-joined and out of scope. Rejecting `_`-as-TAB parsing is right (ambiguous with a title containing `_`).
- **Size:** `tui.rs` is 319 lines; `attach_test.rs` 706, `guards.rs` 245, `scratch/mod.rs` 553, all under the 800 threshold.
- **Test duplication (Phase 7 candidate):** the two local helpers in `utf8_locale_test.rs` are justified in writing (`rig.rs` cannot be included: `dead_code = "deny"` at `Cargo.toml:33`, `lint.sh` forbids an unlinked `allow(dead_code)`, and `rig::binaries()` panics without opencode). Phase 7 should hold them to a PATH lookup and a private-socket runner only, with the call under test built by `tui::{tmux_command, query_args, parse_query}`, never a hand-built argv.
- **Fixture:** requiring `-u` and not recording it keeps `attach_test.rs` unchanged and pins every hermetic call site; POSIX sh and `cat` only still holds. Both contract comments (`fake-tmux:5`, `support/fake_tmux.rs:7`) are named for update.
- **Clashes:** #845 and #696 touch `attach.rs` and possibly `tui.rs`; the edit here is one function plus docs, so a merge is mechanical.

## Notes for O
None required (PASS). Findings 1 to 3 are one-line brief or implementation tweaks and do not need a re-review.

## Patterns referenced
- `crates/holler-adapter-opencode/src/tui.rs:5-30,114-133`, `src/attach.rs:217`, `src/lib.rs:234`
- `crates/holler-adapter-opencode/tests/tui_test.rs:197-224`, `tests/fixtures/fake-tmux`, `tests/support/fake_tmux.rs`
- `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:89-165,368`
- `crates/holler-adapter-host/tests/real_tmux_test.rs`, `docs/testing.md:447`
- `crates/holler-cli/tests/pane_integration/guards.rs:100-128`, `scratch/mod.rs:234-257`; `CHANGELOG.md:8,600`
