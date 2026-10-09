# Handoff-T-green: Phase 7 - #639 the hub pane registry

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Issue:** #639
**Handoff-F reviewed:** docs/handoffs/639/handoff-F.md
**Handoff-T-red:** docs/handoffs/639/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-hub --no-fail-fast --test pane_registry_test --test pane_feed_test --test pane_handlers_test --test pane_dispatch_test`:
14/14, 7/7, 6/6 and 10/10 passed. F flagged no test as wrong and edited none; I changed no test.

Spot-check that the tests pin behavior (mutations applied to `panes/store.rs`, then reverted with `git checkout`; the tree is clean):
- Removed `self.changed.notify_all()`: `a_waiting_watch_wakes_on_the_next_write` FAILED.
- Ignored the save result (`let _ = self.save(..)`): `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` FAILED.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the skip CI applies) | PASS, 964 passed, 0 failed |
| `bash scripts/lint.sh` | PASS (warnings name only pre-existing files) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo machete` | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |

F's reported numbers (964 passed, 0 failed) match mine.

Re-verified in a second, independent pass on the committed tree (clean `git status`): the four pane binaries 10/7/6/14 green; clippy, `lint.sh`, `changelog-check.sh`, `cargo machete` clean; workspace 964 passed, 0 failed; 15 repeat runs of the three new pane binaries, 0 failures. A third mutation (removing the generation check in `Store::delete`) was killed by `delete_checks_missing_before_generation` and `bad_params_answer_their_code`; reverted.

## Tier 2 results
- Test quality: each test names one behavior; no `thread::sleep`; waits are bounded `recv_timeout` or the 100 ms `watch_wait`. Races are asserted as invariants (exactly one winner, no lost update). PASS.
- Files under 900 lines (tests max 523, `panes/` max 340); every `#[allow]` carries a `// #NNN` link. PASS.
- Fail-closed: six AC 9 cases plus the unreadable and no-secret-echo cases pass, and the file is never rewritten. PASS.
- Protocol/golden: `holler-proto` and `holler-pane` untouched, so no golden or `v2.md` change is needed. PASS.
- AC 27: `grep -n "check_membership(" crates/holler-hub/src/panes/` matches `handlers.rs:111`. PASS.
- Radius: the diff touches `panes/**`, `CHANGELOG.md`, the five pane test files and handoffs only. `pane_feed_test.rs` and `pane_support/mod.rs` are outside the brief's list; O must record the W-9 approval in decisions.md.
- Evidence appendix: `evidence.md` has 12 entries from F; none of my tests rely on further unchanged-code facts. PASS.
- Flakiness: F's 100 repeat runs reported 0 failures; I ran the suite twice more without failure.

## Acceptance criteria status
AC 1-26 and 28: PASS, each backed by the test named in handoff-T-red.md. AC 27: PASS by grep (no behavioral test by design). AC 29: PASS (Tier 1 above). AC 30: PASS pending O's W-9 note.

## Blocking issues
None.

## Advisory notes
- Tombstones stay in `entries` with `pane: None`; behavior is correct per tests (list filters them).
- Idle reply for `since = 0` against an all-deleted registry returns `cursor: head`, not `since`; the draft ADR-0021 sentence says `since`. Both resume correctly; S or O may reconcile the ADR text.

## Re-verification at HEAD 95b7724 (after the S hold: ADR-0021 amendment, merge of origin/main, new edge-case test)
No test was wrong and I changed none. Commits since the previous pass: 1d67ef3 (merge), 639f130 (ADR/brief), 4a75b54 (new test), 3b5e7ee (doc comments in `feed.rs`, `mod.rs`, `persist.rs`), 95b7724 (journal).
- New test `an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head` (`pane_handlers_test.rs:308`): handler tier, no sleep, asserts `{"events": [], "cursor": 2}` for the idle window; O's recorded mutation (idle window answering `since`) fails it alone. It pins AC 25 as amended and S's edge case. PASS.
- Pane binaries: `pane_dispatch_test` 10, `pane_feed_test` 7, `pane_handlers_test` 7, `pane_registry_test` 14, all passed. 15 repeat runs of the four binaries: 0 failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS. `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 1068 passed, 0 failed (the count includes the merged-in origin/main tests). `scripts/lint.sh`, `scripts/changelog-check.sh`, `cargo machete`, `docs_cli_test`, `wire_selftest`: all exit 0.
- Files: tests max 523 lines, `panes/` max 340. Every `#[allow]` carries a `// #NNN` link. AC 27: `check_membership(` still matches `handlers.rs:111`. AC 30: the `pane_feed_test.rs` approval is journalled in decisions.md (O entry).
- The advisory note above about the ADR sentence is resolved: ADR-0021 §6 now says the head.

Blocking issues: None. Verdict: PASS.
