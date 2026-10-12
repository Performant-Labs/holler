# Handoff-T-red: #845 switch-after-relaunch (option B)

**Date:** 2026-10-12
**Branch:** issue-845-implementation
**Brief / wireframe reviewed:** docs/handoffs/845-brief.md (no wireframe, no UI); docs/handoffs/845/handoff-A.md

## A precondition
Confirmed: A returned PASS on the plan (handoff-A.md, eight warnings, none a block).

## Risk 2 and risk 1 (pinned first, before any test for F)

Method: a temporary probe test (deleted, not committed) over the repo's own opt-in real rig
(`tests/real_opencode/rig.rs`: scratch HOME, dead-end provider, ports 48100-48199, a private
tmux server on a scratch socket). No live Herdr, tmux server or pane was touched. Real
`opencode` 1.18.35 (the brief saw 1.19.0, which is not installed here). Each round: start a
server, create sessions A and B, `attach_tui(A)`, then at once switch to B.

| Probe | Result |
|---|---|
| Raw `POST /tui/select-session` repeated every 100 ms from the instant `attach_tui` returned (4 rounds) | Every POST answered `200 true` at once (6-296 ms). Title showed B after the 2nd or 3rd POST, at 135-517 ms after `attach_tui` returned. No POST hung. |
| ONE raw POST right after `attach_tui`, then the title watched for 6 s, with all 24 cores busy (10 rounds) | The one POST was enough in 10 of 10: title showed B at 70-291 ms. POST took 6-165 ms. |
| Today's `select_session` right after `attach_tui` (6 rounds) | `Ok(())` in 6 of 6, 127-381 ms. |

Findings:
- **Risk 2 (hung request or unchanged title).** In no run did the not-ready TUI hang the request:
  it answered `200 true` within 300 ms every time. That does not match (b) ("the request hangs").
  The one failure shape B can fix (the request is answered `true`, the title does not change) is
  what the hermetic tests below model; it was not seen on 1.18.35, so the real cause of the 1.19.0
  timeouts in the brief is still not established by a real run. Hermetically, today's code sends one
  POST and then ends `Timeout` at `settle` (RED output below, "1 select requests were sent"), and a
  hung POST ends `Timeout` at the `request` bound (ac29i, passes today).
- **Risk 1 (is `settle` 5 s enough).** The race did not reproduce, even under full CPU load: the TUI
  took a select within about 0.3 s of `attach_tui` returning. Nothing here shows a start-up window
  near 5 s, so (a) is not triggered by this measurement. The "8 s later" in the brief is the delay
  after which a retry was seen to work, not a measured readiness time; it cannot be checked without
  the #649 scenario on a scratch Herdr with OpenCode 1.19.0 (opt-in, `HOLLER_TEST_PANE_INTEGRATION=1`),
  which this phase did not run. If that run shows a window over 5 s, it goes under "Needs operator
  decision if measured" (a); this handoff adds nothing there.

## Tests authored

All in `crates/holler-adapter-opencode/tests/attach_test.rs`, section "#845" (after `ac29e`), tier:
unit-level over the real adapter with the fake tmux and the HTTP stub (the cheapest tier that runs
the adapter's poll, as `ac29a-e` do). The title of a "TUI that turns later" is made by the stub: on its
Nth select request it renames a new `sock.display-message.out` onto the fake tmux's data file (atomic,
so a query never reads an empty file). `fake_tmux.rs` and the fixture are not touched (#844 clash).

| Test | Pins | Today |
|---|---|---|
| `ac29f_a_switch_the_tui_takes_only_on_a_later_request_is_asked_for_again` | Criterion 1. Title shows the target only after the 3rd select POST: `Ok(())`; one `GET /session/ses_B` (before the first send); exactly 3 POSTs (none once the title shows); every POST body is `{"sessionID":"ses_B"}`. | RED |
| `ac29g_a_repeated_switch_answered_404_is_session_not_found` | Criterion 4, first half. First POST `200 true`, the second `404`: `SessionNotFound`, 2 POSTs. | RED |
| `ac29h_a_tui_gone_between_two_requests_is_unavailable_and_gets_no_third` | Criterion 4, second half. Pane turns dead (and, in the second case, missing) on the 2nd POST: `unavailable` "no TUI in pane <pane>", no secrets echoed, exactly 2 POSTs. | RED |
| `ac29i_a_select_request_the_tui_never_answers_times_out_within_the_request_bound` | Risk 2 (b), the bound that B leaves unchanged. A POST that is accepted and never answered (new per-route `Stub::hang`): `Timeout{harness.select_session}` within `request + SLACK`, one POST (a hung request is not re-sent). | passes (regression pin, valid) |
| `ac29a`, `ac29b`, `ac29c`, `ac29d`, `ac29e` | Criteria 2 and 3, unchanged, pass today and must stay green. `ac29d` is also the bound pin (title never shows: `Timeout` within `settle + SLACK`). | pass |

Helpers (test-only): `Stub::hang(method, path)` and `Stub::on_nth(method, path, nth, run)` in
`tests/support/stub.rs` (additive, `#[allow(dead_code)] // #642`, like its neighbours; `hermetic_test`
still passes, 33 tests); `Rig::on_select`, `Rig::selects`, `patient()` in `attach_test.rs`. The positive
tests use `patient()` (settle 5 s) so success does not race a loaded runner; they end as soon as the title
shows the session. `attach_test.rs` is 841 lines (lint fails at 900; `bash scripts/lint.sh` exits 0).

Not authored here: criterion 6 (the #649 scenario's retry removed, opt-in) is F's edit to
`scenario.rs`; criterion 7 is docs; the `settle` mutation check (remove the cap, see `ac29d` fail) is a
GREEN-phase check.

## RED confirmation

`cargo test -p holler-adapter-opencode --test attach_test` (24 pass, 3 fail; failing for the right reason):

```
ac29f: assertion `left == right` failed: 1 select requests were sent
  left: Err(Timeout { op: "harness.select_session" })
 right: Ok(())
ac29g: assertion `left == right` failed: 1 select requests were sent
  left: Err(Timeout { op: "harness.select_session" })
 right: Err(SessionNotFound { what: "ses_B" })
ac29h: a dead pane: expected unavailable, got Err(Timeout { op: "harness.select_session" }) after 1 requests
```

Each fails on the assertion about the missing behavior (one POST, then `Timeout`; no re-send), not on a
compile error, a missing `[[test]]` (the crate has no `autotests = false`; `attach_test` already runs),
or a harness timeout. `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings` is clean.

The suite can be satisfied and is stable: a throwaway version of `select_session` (first send as today,
then a poll that queries, and re-sends via `send_by` when the title does not show the session) turned all
27 green, `ac29` x8 repeats green, and was reverted (`git checkout`; `src/` is untouched by T).

## Ready for F
Confirmed: RED is valid. F may implement against these tests in `crates/holler-adapter-opencode/src/attach.rs`.
Open point for O: the real-OpenCode race did not reproduce on 1.18.35 and the 1.19.0 window of the brief is
unmeasured; if O wants it measured before F, it needs the opt-in #649 scenario on a scratch Herdr.
