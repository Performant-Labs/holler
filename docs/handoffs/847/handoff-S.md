# Handoff-S: #847 remote-admin last_seen flake (spec audit)

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Issue:** #847
**Handoffs reviewed:** docs/handoffs/847-brief.md; 847/handoff-A.md, handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, decisions.md; diff `origin/main...HEAD`.

## A precondition
Met. handoff-A.md and handoff-A-dup.md both show PASS.

## T precondition
Met. handoff-T-green.md lists zero blocking issues; RED then GREEN confirmed (old test 3/3 red under `HOLLER_HEARTBEAT_INTERVAL_MS=300`; rewritten test green, red 3/3 against the mutant).

## Acceptance criteria
| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | No `thread::sleep`, no `Body::start` in the test; silent hand-rolled body | `remote_admin_test.rs` `remote_admin_traffic_never_moves_the_bodys_last_seen`: `live_socket(.., "body")` plus one `presence_frame`, bound as `body_ws` to the end. I re-ran the grep: `Body::start` hits are only lines 98, 802, 808 (other tests); no `thread::sleep` in the file. | Met |
| 2 | Invariant asserted: `last_seen` equals baseline, `conn_state == connected`, no reliance on elapsed time | Same test: asserts `conn_state == "connected"` and `last_seen == baseline` after `wait_past_second` and two rounds of `roster --server` and `hub status --server`. The boundary is a `wait_for` condition, not a sleep. | Met |
| 3 | 30 consecutive loaded runs, zero failures | handoff-T-green: 30/30 under a looped `--test-threads=16` target plus a CPU burn; 10/10 at `HOLLER_HEARTBEAT_INTERVAL_MS=300`. Evidence is a recorded run, which I did not repeat (T's job). | Met |
| 4 | Mutant (`roster.touch` in `admin::run`) makes the test fail | handoff-T-red and T-green: red 3/3 at `remote_admin_test.rs:392`, hub restored. The test asserts on `last_seen` itself, so it fails without the invariant. | Met |
| 5 | Only the test file, optionally `remote_admin_rig.rs`, plus `docs/handoffs/` | `git diff origin/main --stat`: two test files and handoffs only; `crates/holler-hub` is untouched. | Met |
| 6 | `remote_admin_test` 21/21 and `cargo clippy -p holler-cli --all-targets -- -D warnings` clean | 21/21 recorded. Clippy is clean for the touched targets. `--all-targets` fails at `token_load_test.rs:60`, a file absent from the diff, so it fails on main as well. | Met for the touched targets; the workspace clause is a pre-existing, unrelated failure (see advisories) |

## Spec compliance
- D1 (silent body is the controlled variable): implemented.
- D2 (condition wait on an observable): implemented as `wait_past_second` on the test process's whole-second clock, placed BEFORE the admin calls (A's W-1). T confirmed `hub status` exposes no `now`, so the brief's stated fallback applies. The helper reads `holler_proto::now_secs`, which A-dup confirmed is the hub's clock source.
- D3 (no production hook, no new env): holds. The test sets the hub's existing `HOLLER_HUB_LIVENESS_TIMEOUT_MS`. This is a justified, recorded deviation from the brief's claim that the test stays green at a 300 ms heartbeat. The brief's claim was wrong because the hub derives liveness from that variable, and T-red documented it.
- D4 (process level): kept.
- D5 (other tests untouched): the other 20 tests are unchanged apart from `remote_admin_hello_must_not_create_a_roster_row_from_presence`, which uses the extracted `presence_frame` with identical JSON. A's W-3 asked for this and AC 5 allows it. The `last_seen` grep was re-run and the pre-existing flake in `remote_say_wait_and_hub_query_json_match_local_after_masking` was logged as a follow-up.

## Quality audit
- Correctness: the baseline is read after the row exists. Any later hub write to that row would be stamped in a strictly later second than the baseline, so the equality assertion can fail on a real touch. The silent body cannot move the row itself, because its liveness timeout is pinned to 120 s. Longest observed run was 2.09 s.
- Build guards: the diff adds no `unwrap`/`expect`/`panic` outside test code, no `#[allow]` and no dead code. The helpers are used: `joined_rig` by `rig_with_env` and the target test, `presence_frame` by two tests, `wait_past_second` by the target test. `wc -l`: `remote_admin_test.rs` is 827 and `remote_admin_rig.rs` is 114, both under the 900 limit.
- Protocol: no change, so no golden or `docs/protocol/v2.md` update is needed.
- Privacy: grep of the diff for tailnet names, IPs and personal names returned nothing.
- Commits: Conventional Commit subjects. The pipeline's chore commits carry `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link (see advisories).
- Documentation: no `CHANGELOG.md` entry. The change is test-only with no user-visible behavior, no new log event, CLI surface or protocol field, and AC 5 limits the diff to test files and handoffs, so none is required.

## Scope check
Delivered exactly the brief's scope. F made no production change, which was correct; T-red authored the test and helpers. The `rig_with_env` refactor to `joined_rig` is behavior-preserving and was anticipated by A. No over-delivery.

## Verdict
PASS

## Advisory notes
- Workspace clippy `--all-targets` fails on main at `crates/holler-cli/tests/token_load_test.rs:60` (`unneeded cloning of iterator items`). It needs its own issue.
- Pre-existing flake in `remote_say_wait_and_hub_query_json_match_local_after_masking` (1 of 14 on unmodified files). It needs its own issue.
- The brief's claim that the test is green at a 300 ms heartbeat without further change was incorrect and was corrected during T-red. This is a note on the brief, not on F's work.
- `remote_admin_test.rs` is 827 lines. Add any future tests to a sibling target.
- Commit trailers lack a session link. The hook that adds them is outside this story. Check the PR body against `CONTRIBUTING.md` for the AI disclosure after the PR opens.
