# Handoff-S: #845 switch-after-relaunch (option B)

**Date:** 2026-10-12
**Branch:** issue-845-implementation
**Issue:** #845
**Handoffs reviewed:** brief (docs/handoffs/845-brief.md), handoff-A, handoff-T-red, handoff-F, handoff-T-green, handoff-A-dup, decisions, evidence; the diff `origin/main...HEAD`.

## A precondition
handoff-A.md: PASS. handoff-A-dup.md: PASS. Met.

## T precondition
handoff-T-red.md: RED confirmed for the right reason (ac29f/g/h, assertion failures, not compile errors). handoff-T-green.md: GREEN, zero blocking issues, 8 stable repeats, three mutations each caught. Met.

## Acceptance criteria
| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Title turns after N select POSTs; `Ok(())`; 2+ POSTs | `attach_test.rs::ac29f_a_switch_the_tui_takes_only_on_a_later_request_is_asked_for_again` (turns on the 3rd POST; asserts `Ok`, exactly 3 POSTs, 1 GET before the first send, body on each). RED before (one POST then `Timeout`), mutation "re-send removed" fails it. | PASS |
| 2 | ac29a/b/c/e unchanged and green | Not edited by the diff; attach_test 27/27 | PASS |
| 3 | ac29d bound unchanged | ac29d green; mutation "settle cap removed" fails it; `settle`/`call` untouched (`lib.rs` not in the diff) | PASS |
| 4 | 404 on a re-send is `SessionNotFound`; dead/missing TUI between sends is `unavailable`, no further send | `ac29g_a_repeated_switch_answered_404_is_session_not_found` (2 POSTs); `ac29h_a_tui_gone_between_two_requests_is_unavailable_and_gets_no_third` (dead and no-pane cases, exactly 2 POSTs, no secret echoed). Mutation "404 swallowed" fails ac29g. | PASS |
| 5 | T's pin of risk 2 and measurement of risk 1 in T's handoff before F | handoff-T-red.md: real-OpenCode probe on 1.18.35 (race not reproduced, no hung request, readiness within ~0.3 s); hung-request bound pinned by `ac29i` (passes today, valid regression pin). Method and figures recorded. Neither stop condition (a) nor (b) was triggered. See advisory 1: the 1.19.0 window itself is unmeasured. | PASS (with advisory) |
| 6 | `switch_once_the_tui_takes_it` deleted; strict `scratch.json(&["pane","switch",C1,&target], 0)`; `eprintln` gone; grep empty | Diff of `scenario.rs`; `grep -rn "TUI-readiness race\|switch_once_the_tui_takes_it" crates` returns nothing (re-run by S). Compiles; default run green (71 passed, 4 opt-in ignored). The opt-in run was not performed and its result is not recorded. | PASS statically; live run outstanding (advisory 2) |
| 7 | ADR-0021 and `pane-control.md` state the rule once each; CHANGELOG `[Unreleased]` entry links #845 | ADR-0021 "Switch and reset as built (#645)" sentence; `pane-control.md` `switch` entry; CHANGELOG entry links #845 and #633; adapter module doc and `select_session` doc updated; `scripts/changelog-check.sh` ok | PASS |
| 8 | Crate tests, clippy, lint green | handoff-F and handoff-T-green: adapter 27/27 + other binaries, holler-pane 141, pane_verbs 347, pane_integration 71 (4 ignored), clippy on touched crates clean, `scripts/lint.sh` exit 0. The `token_load_test.rs:60` clippy failure is in a file unchanged from origin/main. | PASS |

## Spec compliance
- Option B implemented as decided: the re-send is inside the one `select_session` call; `tx_switch`, `relaunch`, `shown_session`, `HarnessPort`, the Claude adapter, fakes and `holler-proto` are untouched (diff file list confirms).
- `known` (one GET) stays once, before the first send. `true` stays non-evidence: success is only the title (`Seen::Shown`).
- Reuse as the brief and A required: no second poll; `Tui::watch` gained a per-poll `unseen` action (private fn), `attach_tui` passes a no-op; one private `select` helper carries the reply rules for the first send and each re-send. `Call::send_by` is the existing method (`lib.rs:513`) and bounds a re-send by `min(request, poll end, call deadline)`, so `settle` and `call` hold. The first send passes the call deadline, which equals the old `call.send` (`send` is `send_by(.., self.deadline)`).
- Step order query-then-re-send means a dead or missing TUI ends the call with no further request. A failed switch still records nothing; the mismatch/never-shown outcome stays `timeout` as built, and `unavailable` is kept for dead/non-`true`, so the re-send does not blur them.
- `settle`/`call` were not raised; nothing added under "Needs operator decision".

## Quality audit
- Correctness and failure handling: error paths covered (404, dead, no pane, hung request, never shown, non-`true`/HTML via the shared `select`). No persistence or concurrency change.
- Build guards: no `unwrap`/`expect`/`panic` in production code (the only hits in the diff are in test files). The two new `#[allow(dead_code)]` carry `// #642`, matching their neighbours in `stub.rs`. Line counts: `attach.rs` 268, `attach_test.rs` 841, `stub.rs` 390, `scenario.rs` 269; all under 900. No dead code.
- Protocol: none changed (no wire, error-code table, golden file, or `docs/protocol/v2.md` change needed).
- Tests: hermetic stub plus fake tmux; the title is turned by a stub callback with an atomic rename, so no fixed sleep as synchronization; `fake_tmux.rs` and the fixture untouched (#844 clash avoided). Positive tests use a wide `settle` and end when the title shows. Bound pins use SLACK. RED-first evidence is in handoff-T-red.md.
- Documentation: CHANGELOG, ADR-0021, `pane-control.md`, module doc each carry the rule once.
- Public-repository privacy: grep of the diff for tailnet names, `.ts.net`, personal or account names, home paths and private IPs found nothing.
- Commit hygiene: pipeline commits are Conventional Commit style with a `Co-Authored-By` trailer. The pipeline-phase commits carry `Co-Authored-By: Claude <noreply@anthropic.com>` without a session link (the role doc asks for one). The PR is not yet open; its AI-disclosure per `CONTRIBUTING.md` is for the run's agent to add after it opens (CLAUDE.md).

## Scope check
Exactly the brief's scope. Production change is `attach.rs` only; tests are `attach_test.rs`, `stub.rs` (additive helpers `hang`, `on_nth`) and `scenario.rs`; docs are the three named. No over- or under-delivery beyond the outstanding live run in advisory 2.

## Verdict
**PASS**

## Advisory notes (non-blocking)
1. The real cause of the 1.19.0 timeouts is not established by a real run. T's probe on 1.18.35 never reproduced the race, so B is verified against the hermetic model (200 `true`, title unchanged) only. Risk 1 (is 5 s `settle` enough on 1.19.0) is unmeasured. If the opt-in run shows a start-up window over 5 s, or a hung request, that goes to the operator under "Needs operator decision if measured" (a)/(b); the pipeline must not raise `settle`.
2. Criterion 6's result must be recorded: run the #649 scenario once with `HOLLER_TEST_PANE_INTEGRATION=1` on a scratch Herdr with OpenCode 1.19.0 before or at merge, since the retry that hid the race is now gone and a failure there would be the signal for advisory 1.
3. `attach_test.rs` is at 841 lines (soft limit about 800); the next change to it should move the #845 block into its own test file. After #844 (PR #870) merges, fold an atomic `reply` into `Fake` and have `Rig::on_select` call it (A-dup warns 1 and 2).
4. Re-send cadence is one POST per 100 ms poll, at most about 50 per call during a real wedge; idempotent, accepted by A.
5. Pipeline-phase commit trailers lack a session link; the PR body needs the AI disclosure after the PR opens. Textual conflicts with #844 are likely in ADR-0021 and CHANGELOG only; whoever merges second rebases.
