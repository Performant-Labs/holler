# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario  (fourth pass: verify F's third cycle, the production half of the round-2 mixed rework)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `4d7f58a`)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md` (third cycle)
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md` (second cycle)

This handoff replaces the third pass's (`13010f4`), which git history keeps. That pass's test repairs (the D8 test's
comment and T's four B-1 evidence entries) are unchanged. So are the earlier spot-checks (M1 to M8, the W-T row, the
6 s pin) and the acceptance table. **F's third cycle changed one production file, `wiring.rs`, by a two-line comment
and nothing else**, and reordered `evidence.md`. No test changed, and this phase changes no test.

## What F's third cycle changed, and how I checked it

| Change | Check | |
|---|---|---|
| `wiring.rs:139-140`: a comment above `LazyHerdr::new(move \|\| connect_herdr(&herdr_config))`. | `git show 4d7f58a -- crates/` is two added comment lines. No code line differs. | PASS |
| B-2 answered as "refuted by the compiler" | Build and clippy `-D warnings` are clean at HEAD. I repeated F's mutation myself: with `move` removed, `cargo check -p holler-cli` fails with `E0373` ("closure may outlive the current function, but it borrows `herdr_config`"). The file was restored with `git checkout`, and `git status` is clean. | PASS |
| New B-2 evidence entry (`adapter.rs:73-74`, `error.rs:402-403`) | I checked both excerpts against source with `sed -n`. They match byte for byte. | PASS |
| `evidence.md` reordered newest first | A sorted-line diff (`LC_ALL=C sort`, `comm`) of `13010f4` against HEAD: only the old title and intro are gone. Every other line is still there, and the only lines added are the new title, intro, section headings and the B-2 entry. | PASS |
| The gate's 12,000-byte window | Byte offsets: T's B-1 section at 397, F's B-2 section at 3,776, round 1's section at 4,903. The cut falls in round 1's conformance entry, as F says. | PASS |

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration` (inside the workspace run) | `ok. 61 passed; 0 failed; 4 ignored; finished in 119.25s` |
| Opt-in run (`HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1`) | Not re-run. No code line has changed since the second cycle's 4/4 at `22aa539`. This cycle adds a comment. |

The tests still fail when the behaviour is removed: nothing in this cycle touches a test or a code path, so the first
cycle's M1 to M7 and the second cycle's M8 still hold.

## Tier 1 results

| Check | Command | Actual | |
|---|---|---|---|
| Build | `cargo build -p holler-cli --all-targets` | clean | PASS |
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | clean | PASS |
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | 151 binaries: 2312 passed, 1 failed, 32 ignored. The failure is `wizard_scripts`' rename scan (`skilltext_w5_rename`). | PASS for this story |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed | PASS |
| Lint | `bash scripts/lint.sh` | exit 0. Warnings are line-count notices on files this story does not touch. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | PASS |
| Unused deps | `cargo machete` | none | PASS |

The rename scan's three hits are all under `docs/handoffs/` (`649-brief.md`, `649/decisions.md`, and the previous
`649/handoff-T-green.md`). `git grep -l agent-skills/setup-wizard -- ':!docs/handoffs'` finds nothing. The scan
passes once `docs/handoffs/` is removed before the push.

**Cross-check against F's handoff:** 61/0/4, 2312/1/32, `E0373` on the mutation, and clean build, clippy, lint,
changelog and machete all reproduce. There is no discrepancy.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | No change. Every criterion keeps its test (below). | PASS |
| Test quality | No test changed this cycle. The suite stays proportionate: no new test is warranted, because a comment has no behaviour to pin and the `move` is pinned by the compiler. | PASS |
| Type safety | Build and clippy `-D warnings` are clean. The mutation confirms the closure needs `move` and has it. | PASS |
| Evidence appendix | The B-2 entry is verbatim. No entry was lost in the reorder. T's B-1 entries now sit inside the gate's window. | PASS |
| Error handling, data integrity, API contract, security | No behaviour changed this cycle. The earlier results stand. | PASS |
| Migration safety, Playwright | N/A: no schema, and no browser surface. | N/A |

## Acceptance criteria status

All PASS, backed by the same tests as before:
- `no_hub::*`, `store_only::*` and `env::malformed_adapter_settings_…`;
- `fake_hub::*` (timeout, unavailable, the lost write, the 6 s pin);
- `wired::*` (the relative socket and the non-UTF-8 socket), `guards::*` and `watch::*`;
- `bind::launch_resolves_attach_tuis_pane_…` (`attach_tui` resolves before the record exists);
- the opt-in `scenario::*` (the scenario, the profile scenario, the grid, and the server argv under tmux);
- the D28 sentinel checks.

## Blocking issues

None.

## Advisory notes

- Earlier advisories stand: a non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace (Design decision 9, fails
  closed); the `support/` tree is not bare-`rustfmt` clean; the rename scan fails while `docs/handoffs/` is tracked.
- F's two pipeline gaps are O's to file in Aftersight: the 12,000-byte evidence cap, and the diff gate getting no
  `--handoff`. Past the cap are the rest of round 1's section and the first cycle's ten entries. Four of those back
  statements in the brief's D3, D8 and D31, so round 3 could still ask for them.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
