# Handoff-T-red: #844 oc-tmux-utf8

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Brief / wireframe reviewed:** docs/handoffs/844-brief.md (amended); no wireframe (no UI)

## A precondition
Confirmed: A returned PASS on the plan (second pass, `docs/handoffs/844/handoff-A.md`).

## Tests authored
| Test | Pins | Tier |
|---|---|---|
| `tui_test.rs::ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane` (extended) | AC 1: `tmux_command` argv is `flag ++ ["-u"] ++ call` for `Path`, `Name` and `Default` | unit, pure argv |
| `utf8_locale_test.rs::the_query_reads_a_live_pane_in_a_process_with_no_utf8_locale` (new) | AC 2: against a real private tmux server, the adapter's own call (`tmux_command` over `query_args`, read by `parse_query`) returns `Query::Live` with title `OC \| ses_x` under `LANG=C`, under `LC_ALL=C`, and with no locale variable; `LC_CTYPE` removed in each. Env set on the child only. Drop guard kills the server and removes the tempdir. Skips with `eprintln!` if no tmux on PATH. | real tmux, cheapest tier that can see the `_`-for-TAB behaviour |
| `fixtures/fake-tmux` (+ contract comments in it and `support/fake_tmux.rs`) | AC 5: requires `-u` right after `-S <path>` (exit 64 otherwise), does not record it. Pins every hermetic call site in `attach_test.rs`, which is otherwise unchanged. | fixture |
| `pane_integration/guards.rs::the_binary_env_is_built_from_nothing_inherited`, `scratch/mod.rs::binary_env` | AC 3: `LANG=C.UTF-8` removed from the binary env; name set drops `LANG`; asserts no `LANG` and no `LC_*` name. The #649 scenario's binary now runs with no locale. | test support |

Not run here: the opt-in #649 real-OpenCode scenario (`pane_integration` skips without real binaries; its 4 ignored tests were not enabled).

## RED confirmation
`cargo test -p holler-adapter-opencode --no-fail-fast`:
- `tui_test`: `ac11c_...` FAILED, `assertion left == right failed: Path("/x/sock")  left: ["-S", "/x/sock", "display-message", "-p"]  right: ["-S", "/x/sock", "-u", "display-message", "-p"]` (the feature assertion; the other 12 pass).
- `utf8_locale_test`: FAILED, `LANG=C, LC_ALL unset: the pane is live, but the query read NoPane` (tmux 3.7c prints `_` for TAB, `parse_query` finds one field). Not a setup error: `new-session`, `select-pane` and the query's exit status all succeeded before the assertion.
- `attach_test`: 12 of 23 FAILED, each `Unavailable { what: "tmux failed for pane w9:p1: fake-tmux: expected -u right after -S <path> (#844)" }` because the adapter does not yet pass `-u`. This is the fixture doing its job; the 11 that do not reach tmux pass.
- `pane_integration` guard test: passes already (a consistency edit with its helper, not a RED).

Sanity (GREEN reachable): a temporary one-line `.arg("-u")` in `tmux_command` made all of `cargo test -p holler-adapter-opencode` pass (agent 11, attach 23, tui 13, utf8 1, others green). The production file was then restored with `git checkout`; no production change is in the tree.

Also: `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings` clean; `pane_integration` 71 passed / 4 ignored on the current tree; `scripts/lint.sh` shows only size warnings for other files. Touched files formatted with `rustfmt` individually, not workspace-wide.

Files (uncommitted; the caller stages by explicit path):
`crates/holler-adapter-opencode/tests/utf8_locale_test.rs` (new), `.../tests/tui_test.rs`, `.../tests/fixtures/fake-tmux`, `.../tests/support/fake_tmux.rs`, `crates/holler-cli/tests/pane_integration/guards.rs`, `.../scratch/mod.rs`.

## Ready for F
Confirmed RED is valid; F may implement against these tests. F's change: `.arg("-u")` after the socket flag in `tui.rs::tmux_command` (AC 1, 4), its doc comment and the module-doc "UTF-8 output" rule, the CHANGELOG bullet under `### Bug Fixes`, and the optional ADR-0021 sentence. Leave `lib.rs:234` (`env -u`) alone.
