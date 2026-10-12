# Handoff-S: #844 oc-tmux-utf8 (spec audit)

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Issue:** #844 (epic #633)
**Handoffs reviewed:** docs/handoffs/844-brief.md (amended), 844/handoff-A.md, handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, decisions.md, evidence.md; `git diff origin/main...HEAD`.

## A precondition
Met. handoff-A (second pass) and handoff-A-dup both show PASS.

## T precondition
Met. handoff-T-green shows no blocking issues and a confirmed RED (ac11c, utf8_locale_test, 12 attach_test cases) then GREEN. The workspace clippy failure in `token_load_test.rs` is in a file the diff does not touch.

## Acceptance criteria
| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | `tmux_command` puts `-u` after the socket flag, before the call args, for Path/Name/Default | `tui_test.rs::ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane` asserts `flag ++ ["-u"] ++ call` for all three sockets; fails without `-u` (T-green spot check). `tui.rs:135` | Met |
| 2 | Under `LANG=C`, `LC_ALL=C` and no locale, against real tmux, the adapter's query reads the live pane (not NoPane) | `utf8_locale_test.rs::the_query_reads_a_live_pane_in_a_process_with_no_utf8_locale`: builds the call with `tmux_command` over `query_args`, reads it with `parse_query`, asserts `Query::Live` with title `OC \| ses_x`; `LC_CTYPE` removed in each case; RED showed `NoPane` | Met |
| 3 | #649 scenario no longer needs `LANG=C.UTF-8`; guard test updated; `pane_integration` passes | `scratch/mod.rs` entry and comment removed, doc line fixed; `guards.rs` drops `"LANG"` from the name set and asserts no `LANG` and no `LC_*` name; `pane_integration` 71 passed / 4 ignored (the opt-in real-OpenCode scenario was not run, as the brief allows) | Met |
| 4 | `"-u"` added in exactly one place in `tui.rs` | `tui.rs:135` only; `lib.rs:234` is the unrelated `env -u` | Met |
| 5 | No other behaviour change; `attach_test.rs` assertions pass | `attach_test.rs` is not in the diff; the fixture requires then shifts `-u`; 23 passed; full workspace suite exit 0 | Met |

The tests assert behavior: removing `-u` fails `ac11c`, the real-tmux test and 12 `attach_test` cases, so none passes with the change removed.

## Spec compliance
- Decision 1 (`-u`, not `_`-parsing): implemented; `QUERY_FORMAT` and `parse_query` unchanged.
- Decision 2 (placement): `.arg("-u")` after the socket match, before `.args(args)`.
- Decision 3 (fixture requires `-u`, exits 64 otherwise, does not record it): implemented as stated; the `$#` bound moved from 3 to 4. POSIX sh and `cat` only still hold. Mode 100755 is unchanged (diff shows no mode line).
- Decision 4 (no env change): the child's environment is untouched; the test sets locale on the child `Command` only.
- A's warns: 2 respected (`lib.rs` untouched), 3 respected (`### Bug Fixes`), 1 documented in the test's module doc and decisions.md.

## Quality audit
- Correctness: one flag in the sole tmux builder. No new error path, state or wire change.
- Build guards: no new `unwrap`, `expect` or `panic` in `src/`. The one `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #844` is in the new test file and carries its issue link. Sizes: `tui.rs` 326, `utf8_locale_test.rs` 162, `attach_test.rs` 706 (untouched), all under 900. No dead code.
- Protocol: no change; no golden files touched.
- Tests: a real tmux where the behavior needs it; no fixed sleeps; a Drop guard built before the server starts; the socket is under a short `/tmp` path; the test skips with an `eprintln!` when tmux is absent.
- Documentation: `CHANGELOG.md` has an `[Unreleased]` / `### Bug Fixes` entry linking #633 and #844; `tui.rs` module doc and doc comment, ADR-0021 and both fixture contract comments are updated. No new CLI surface, log event or protocol field, so no README or protocol doc change is needed.
- Public-repository privacy: grep of the added lines for personal names, tailnet or private hosts, IPs, `/home/` paths and keys found nothing.
- Commit hygiene: the pipeline commits are `chore(#844): ...` with a `Co-Authored-By` trailer, which is the convention on main. The PR body does not exist yet; its AI disclosure per CONTRIBUTING.md is for the caller to add after the PR opens.

## Scope check
Exactly the brief's scope: one production line plus docs, test edits, the fixture and the two `pane_integration` support edits. No over-delivery. No under-delivery.

## Verdict
PASS

## Advisory notes
- `docs/testing.md` does not say why `utf8_locale_test` is not `#[ignore]` (A's warn 1); the reason is in the test's module doc.
- The three trailers carry no session link; the other commits on main do not either.
- The pre-existing clippy lint in `token_load_test.rs:60` is separate from this work.
- The opt-in #649 scenario was never run with `LANG` removed; the real-tmux test covers the same condition.
