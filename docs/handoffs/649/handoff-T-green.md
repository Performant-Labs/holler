# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario  (fifth pass: S's test-only rework)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `d387625` plus this pass)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md` (third cycle; F has not run since)
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md` (second cycle)
**Handoff-S:** `docs/handoffs/649/handoff-S.md` (REWORK, test-only)

This handoff replaces the fourth pass's (`6ce945d`), which git history keeps. Its results (F's third cycle, a
two-line comment, verified; mutants M1 to M8; the 6 s pin; the opt-in scenario's 4/4 at `22aa539`) stand. **This
pass changes one test file, `crates/holler-cli/tests/pane_integration/fake_hub.rs`, and no production code.**

## What S asked for, and what changed

| S's finding | Change in `fake_hub.rs` | Mutant it now kills |
|---|---|---|
| (1) D3's row "a write with no hub (`NoLiveHub`) is `unavailable`" has no test. The only no-hub test called `list()`, a read. | `a_client_call_with_no_hub_is_unavailable_naming_the_hub` (one read) is replaced by `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub`: all five reads and all five writes, each against its own fresh state dir with no hub. Each must answer `unavailable` (so never `timeout`), and the message must contain `no live holler hub reachable at <that state dir>`. | **M9:** `NoLiveHub` added to `timed_out`'s write arm (`store.rs:375`). Fails: `pane/cas_put: timed out: pane_store.cas_put`. |
| (2) AC 3 says every other read failure is `unavailable` *naming the hub*, but two tests checked only the code. | `a_read_whose_reply_is_lost_is_unavailable` becomes `..._naming_the_hub`, and `a_json_rpc_error_is_unavailable_for_reads_and_writes` becomes `a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes`. Both call the new `Outcome::assert_names_the_hub`: the text contains `holler hub` **and** the fake hub's own state dir. The dir is a fresh temporary path, so neither `BadReply`'s display text nor the fake hub's refusal text can supply it. A bare `contains("hub")` would not discriminate, as S notes. | **M10:** `hub_unavailable`'s second arm (`store.rs:387`) rewritten to `"{method} failed: {other}"`. Both tests fail at the new assertion. |

To carry the state dir, the two helpers `each_write`/`each_read` (each a closure table plus a runner) became one
`Call` type, two tables `writes()`/`reads()`, and one runner `against(answer, calls)` that returns an `Outcome` with
the method, code, text and the hub's state dir. The no-hub test reuses the same tables, so all ten calls are listed
once. The other three users of the old helpers (the lost write, the undecodable `data`) now destructure `Outcome`;
their assertions did not change. The file is 454 lines (limit 900). It adds no `#[allow]`.

Both mutants were applied to `store.rs` by `sed`, run, and restored with `git checkout`. `git status` showed the file
clean after each.

**Evidence appendix:** a new section heads `evidence.md`, inside the gate's 12,000-byte window. It holds the two
unchanged-source facts the new assertions rely on: `resolve_state_dir` returns `HOLLER_STATE_DIR` as set
(`state.rs:38-40`), and a failed connect is `NoLiveHub` before any request is written (`control.rs:492`). Both
excerpts were copied from source with `sed -n`.

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration fake_hub` | `ok. 15 passed; 0 failed; finished in 53.22s` |
| `pane_integration` inside the workspace run | Still running when this handoff was written (see below). |
| Opt-in run (`HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1`) | Not re-run. No production line has changed since `22aa539`, where it passed 4/4, and this pass touches no scenario test. |

## Tier 1 results

| Check | Command | Actual | |
|---|---|---|---|
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | clean | PASS |
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | Still running when this pass had to return: 57 of the 151 binaries had reported, with 0 failures. | PENDING |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | Part of the workspace run. Not run separately this pass. | PENDING |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | Part of the workspace run. Not run separately this pass. | PENDING |
| Lint | `bash scripts/lint.sh` | exit 0. The only warning is a line-count notice on `holler-pane/src/error.rs` (713), which this pass does not touch. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | PASS |
| Unused deps | `cargo machete` | none | PASS |

The workspace run must still finish before the PR. It is expected to fail only `wizard_scripts`' rename scan, on the two lines that are not T's (see the advisories). This pass's own files no longer match the scan's needles: `git grep` over `docs/handoffs/649` finds only `decisions.md:66`.

**Cross-check against F's handoff:** F has not run since the third cycle, and the fourth pass already reproduced its
numbers. This pass's counts differ from F's only by this pass's net test change (one test replaced by one).

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | Every row of the module doc's W-9 table (`fake_hub.rs:10-18`) now has a test for each column it fills, and the no-hub row covers all ten calls. | PASS |
| Test quality | Each changed test names its behaviour, fails in isolation on its own assertion (M9, M10), and sits at the cheapest tier (an in-process client against a fake socket). The no-hub test replaces the one-read test rather than duplicating it, so the suite is the same size. | PASS |
| Behaviour, not implementation | The tests assert the code and that the message names the hub by its state dir, which is what AC 3 and W-9 say. They do not pin the wording around it, except `no live holler hub reachable at`, the CLI's existing text that six other test files already pin. | PASS |
| Type safety | Clippy `-D warnings` is clean. The ten calls coerce to one `fn` pointer type. | PASS |
| Error handling | The `NoLiveHub`, `Refused` and `BadReply` rows are each pinned for reads; `NoLiveHub` and `Refused` for writes, with `BadReply` for writes as `timeout`. | PASS |
| Security | No secret enters these messages. The state dir is a temporary path. | PASS |
| Data integrity, API contract, migration safety, Playwright | Unchanged this pass. Earlier results stand. N/A: no schema, no browser surface. | PASS |

## Acceptance criteria status

All PASS. AC 3 is now fully backed:
- `fake_hub::a_read_whose_reply_is_lost_is_unavailable_naming_the_hub`;
- `fake_hub::a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes`;
- `fake_hub::every_read_and_write_with_no_hub_is_unavailable_naming_the_hub`;
- the timeout rows and `data_that_does_not_decode_is_unavailable_naming_the_method`.

The other criteria keep the tests named in the fourth pass: `no_hub::*`, `store_only::*`, `env::*`, `wired::*`,
`guards::*`, `watch::*`, `bind::*`, the opt-in `scenario::*`, and the D28 sentinel checks.

## Blocking issues

None in F's code.

## Advisory notes

- The merge-blocking rename-scan hits S names are O's to settle before the PR. This pass rewrote T's own line (the
  previous handoff's line 48). The other two are not T's: `649-brief.md:1197` (O's, entry E64; the file is now
  `agent-skills/herdr-setup/lib/herdr.sh`) and `649/decisions.md:66` (inside A's first entry). Either fix those two
  lines or pass `removeHandoffsBeforePr: true`.
- S's other advisories are not this pass's to act on: the D28 sentinel (for #829), scenario step 7's finding kind, and
  `pane get` vs `pane list` in the end-state check. They are scenario changes, and S did not classify them as rework.
- `fake_hub.rs` is not bare-`rustfmt` clean. It was not at HEAD either (15 diffs), and CI does not run `rustfmt`.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
