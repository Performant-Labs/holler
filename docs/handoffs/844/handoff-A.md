# Handoff-A: #844 oc-tmux-utf8  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Brief reviewed:** docs/handoffs/844-brief.md   **Reuse map:** the brief's "Reuse map" paragraph   **Wireframe:** N/A (no UI)
**Verdict:** BLOCK

## Summary
The production design is right: `-u` added in the one builder, `tmux_command`, after the socket flag, with no new setting,
no env change, no parser change. It extends the single choke point and matches the module's "every tmux process is built
here" rule. The plan is blocked for one factual gap in the test side: removing `("LANG", "C.UTF-8")` from
`pane_integration/scratch/mod.rs` also breaks `guards.rs`, which pins that entry, and AC 3's "runs with the ambient `LANG`"
is false for that rig. Four warnings are specific enough to fold into the same amendment.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Files / AC 3: remove `("LANG","C.UTF-8")` at `scratch/mod.rs:253-257`; "the scenario runs with the ambient `LANG`" | pattern consistency (a pinned guard) | `binary_env` builds the binary's env "from nothing inherited". `crates/holler-cli/tests/pane_integration/guards.rs:100-128` (`the_binary_env_is_built_from_nothing_inherited`, not opt-in) asserts that `LANG` is in the exact name set (`:109`) and that `value(&env,"LANG") == "C.UTF-8"` (`:128`). Removing the entry fails `cargo test -p holler-cli --test pane_integration` (AC 3). The binary will then see no `LANG` at all, not an ambient one: that is the failing case of the issue, so the scenario becomes a stronger pin, but the brief's wording is wrong. The doc comment at `scratch/mod.rs:234` ("UTF-8 `LANG`") goes stale too. | Add `guards.rs` to Files (T edits it): drop `"LANG"` from the expected set, replace line 128 with an assertion that no `LANG` and no `LC_*` name is present, and reword the message to say the rig leaves the locale unset on purpose (#844). Fix `scratch/mod.rs:234`. Reword AC 3 to "the scenario runs with no locale variable". |
| 2 | warn | New `utf8_locale_test.rs` env: `.env("LANG","C").env_remove("LC_ALL")` | test validity | tmux decides UTF-8 from `LC_ALL`, then `LC_CTYPE`, then `LANG`. An ambient `LC_CTYPE=en_US.UTF-8` (common) makes the test pass before the fix, so it is not RED on that machine. | Also `env_remove("LC_CTYPE")` in the `LANG=C` case, and in the `LC_ALL=C` case remove `LANG` and `LC_CTYPE`. Say so in the test's header. |
| 3 | warn | New test's private-server helpers | duplication (Phase 7 candidate) | `rig.rs` already has `on_path` and `tmux(bin, sock, args)`. It cannot be included from a second test file: workspace `dead_code = "deny"` rejects unused `pub` items and `scripts/lint.sh` forbids an unlinked `allow(dead_code)`. Also `rig::binaries()` panics without `opencode`. So a small local helper is justified, but the brief must say so, or A will flag a near-copy at Phase 7. | Add to the brief: "own 10-line `tmux()` and PATH lookup, because `rig.rs` cannot be included without unused items (`dead_code = deny`) and needs `opencode`". Keep it to those two helpers; use `tui::{tmux_command, query_args, parse_query}` for the call under test, never a hand-built argv. |
| 4 | warn | New test: server lifetime | concurrency / cleanup | The brief does not say how the private server is stopped. A failing assert would leak a tmux server and its `sleep`. | Follow `rig.rs`: a guard value built before `new-session`, whose `Drop` runs `kill-server` on the private socket and removes the temp dir (`tempfile`, prefix as `hlr642t-`, so the socket path stays short). Never touch the default server; remove `TMUX`/`TMUX_PANE` as `rig::tmux` does. |
| 5 | warn | Fake-tmux skip (decision 3) | pattern consistency | Skipping `-u` in the fixture keeps `attach_test.rs` unchanged, which is the smaller edit and acceptable. But the contract comment (`fake-tmux:5`, `support/fake_tmux.rs:7`) says "called as `fake-tmux -S <path> <subcommand>`", and `[ "$1" = "-u" ] && shift` hides a missing `-u` from every hermetic test. | Prefer making the fixture require it: after `shift 2`, `if [ "$1" != "-u" ]; then ...exit 64; fi; shift`, so all call sites are pinned, not only `ac11c`. If the skip is kept, use an `if` (not `&&`) and update both contract comments. POSIX sh and `cat` only still holds. |

## Notes for O
Amend `docs/handoffs/844-brief.md` before a fresh run (the script stops on an unchanged brief; do not use `resumeFromRunId`):
1. Files: add `crates/holler-cli/tests/pane_integration/guards.rs` (finding 1) and the `scratch/mod.rs:234` doc line.
2. AC 3 and Test plan item 3: "no locale variable is set for the binary", not "ambient `LANG`".
3. Test plan item 2: the env removals of finding 2, the helper rationale of finding 3, the guard of finding 4.
4. Decision 3: choose between the require-`-u` fixture and the skip (finding 5), and name the contract comments to update.
Everything else (docs/CHANGELOG touch, size of `tui.rs` at 319 lines, no wire/proto/state/secret impact, clashes with #845 and
#696 limited to one function) is consistent and needs no change. `holler-adapter-host` and `-claude` are correctly out of scope.

## Patterns referenced
- `crates/holler-adapter-opencode/src/tui.rs:5-30,114-133` (single builder rule, `tmux_command`)
- `crates/holler-adapter-opencode/tests/tui_test.rs:197-224` (`ac11c`)
- `crates/holler-adapter-opencode/tests/fixtures/fake-tmux`, `tests/support/fake_tmux.rs`
- `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:153-165` (private-server helper, guard), workspace `Cargo.toml:33`
- `crates/holler-cli/tests/pane_integration/guards.rs:100-128`, `scratch/mod.rs:234-257`
