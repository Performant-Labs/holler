# Brief: #845 switch-after-relaunch

Repo: Performant-Labs/holler. Issue: #845 (part of epic #633). Rigor: in-session. UI surface: no. Kind: bug.

**STATUS: BLOCKED ON ONE OPERATOR DECISION (section "Open decision").** Everything else is written. The sections that depend on the
choice give both branches; O/MO resolves the choice, deletes the losing branch, and only then does T start. No code is written until then.

## Problem
A `holler pane switch` sent right after `holler pane relaunch` of an OpenCode pane ends `timeout` in `harness.select_session` until the
relaunched TUI takes `/tui/select-session`. Seen 3 of 3 in the #649 scenario and 2 of 3 in a probe (OpenCode 1.19.0); the same switch sent 8 s
later always succeeded. `relaunch` returns once the TUI's title shows the session of record (which is also when `attach_tui` returns), not once
the TUI accepts a select-session call. A failed switch records nothing, so the operator sees a spurious `timeout` on a healthy pane.
The #649 scenario hides it with a 20 s retry loop (`switch_once_the_tui_takes_it`).

## Is it settled by an existing decision? No.
Checked: `docs/adr/ADR-0021.md` (sections 2, 5 "Switch and reset as built (#645)", "The health wait moved into the engine", 12), `docs/protocol/v2.md`
(no pane verbs; its only "retry" text is the body/hub reconnect rules, §3/§8, unrelated), `docs/pane-control.md` (`relaunch` and `switch` entries,
lines 231-262), `docs/handoffs/649/decisions.md` (lines 286-295: the finding, "the scenario works around (2) with a bounded retry", filed as #845).
What they say and do not say:
- ADR-0021 "Switch and reset as built (#645)": the act is `select_session`; "a mismatch observed after the act is the closed failure `unavailable`".
  It says nothing about a TUI that is not yet ready, and nothing about retrying or about relaunch waiting for the TUI.
- The OpenCode adapter header (attach.rs:1-19) pins: one deadline per method; "an attach or a switch is trusted only once seen (I3)"; `select-session`
  "answers `true` with no TUI at all (opencode-pane-spike.md:117-124), so its `true` proves nothing"; the pane is polled every `SETTLE_POLL` within
  `settle`, else `timeout`. This pins that a `true` reply is not evidence, not what to do when the title does not change.
- ADR-0021 section 12 / I5: each port call returns within its bound (10 s default) or `timeout`; each verb has its budget. `relaunch` is under the
  launch engine's 10 s budget (`DEFAULT_BUDGET`, `TxOptions.budget`), `switch` in place makes at most eight port calls and has no budget of its own.
- So choosing relaunch-waits versus switch-retries is a new behaviour choice. Per the task rules it is not decided here.

## Evidence (verbatim, from `origin/main` at 6eff9cc)

The act and its watch, `crates/holler-adapter-opencode/src/attach.rs:58-90`:
```
pub(crate) fn select_session(
    harness: &OpenCodeHarness,
    pane: &PaneId,
    session: &str,
) -> Result<(), PaneError> {
    let config = &harness.config;
    let deadline = deadline_after(config.timeouts.call);
    let tui = Tui::of(harness, pane, OP_SELECT_SESSION, deadline)?;
    let port = match tui.query(deadline)? {
        Query::Live { start_command, .. } => tui::attach_port(&start_command),
        Query::Dead(_) | Query::NoPane => None,
    }
    .ok_or_else(|| tui.no_tui())?;
    let call = harness.call_until(port, OP_SELECT_SESSION, deadline);
    known(&call, &session_path(session), session)?;
    let reply = call.send(SELECT, SELECT.label, Some(&json!({ "sessionID": session })))?;
    if reply.status == 404 {
        return Err(PaneError::SessionNotFound {
            what: session.to_owned(),
        });
    }
    if json_of(&reply) != Some(Value::Bool(true)) {
        return Err(call.unexpected(SELECT, &reply, "true"));
    }
    match tui.watch(port, session, config.timeouts.settle)? {
        Seen::Shown => Ok(()),
        Seen::Dead(_) | Seen::Gone => Err(tui.no_tui()),
    }
}
```
The select request is sent exactly once. The `timeout` comes from `tui.watch` (title never shows the session within `settle`), or from the request
itself if the not-ready TUI never answers (5 s `request`). **Which of the two it is, is not established by any file in the repo** (the finding's probes
were removed); T must establish it against the fake (below) and, if the operator wants, F against the opt-in real OpenCode, before choosing a fix shape.

The poll, `crates/holler-adapter-opencode/src/lib.rs:650-671`:
```
fn poll<T>(
    settle: Duration,
    deadline: Instant,
    op: &str,
    mut step: impl FnMut(Instant) -> Result<Option<T>, PaneError>,
) -> Result<T, PaneError> {
    let until = deadline_after(settle).min(deadline);
    loop {
        if let Some(seen) = step(until)? {
            return Ok(seen);
        }
        if Instant::now() + SETTLE_POLL >= until {
            return Err(PaneError::Timeout { op: op.to_owned() });
        }
        std::thread::sleep(SETTLE_POLL);
    }
}
```
The bounds, `lib.rs:252-271`: `call` 10 s (every port method), `request` 5 s, `health` 2 s, `settle` 5 s ("how long a switch, an attach or an abort may
take to be observed"); `SETTLE_POLL` is 100 ms (`lib.rs:171`).

Why relaunch returns early, `crates/holler-pane/src/tx_launch/server.rs:77-90` (`bring_up`): `ports.harness.attach_tui(&herdr.pane_id, &at, &session)?;` then
`shown_session` once, compared with the session of record. `attach_tui` (attach.rs:35-52) returns `Ok` when the title shows the session, which happens before
the TUI takes a select-session call.

The caller, `crates/holler-pane/src/tx_switch.rs:244` (inside `switch_in_place`): `.select_session(&record.herdr.pane_id, &target).map_err(acted)?;` then
`observe`, then the record's `cas_put`. Any failure from `select_session` on is `acted` (ends with the doctor `--fix` reconcile step).

The workaround to remove, `crates/holler-cli/tests/pane_integration/scenario.rs:42-67` (`switch_once_the_tui_takes_it`: `wait_for(20 s)`, retries on error
code `timeout`, `eprintln!("... the TUI-readiness race after relaunch")`), called at `scenario.rs:102`.

Existing tests the change must keep green or deliberately amend, `crates/holler-adapter-opencode/tests/attach_test.rs`:
- `ac29a` (L293): select checks the session, POSTs once, confirms by title; asserts the request lines are exactly `GET /session/ses_B`, `POST /tui/select-session`.
- `ac29d` (L375): "a switch the title never shows times out within settle": `Err(Timeout{op: select_session})` and `took < quick().settle + SLACK`.
- `ac29b`, `ac29c`, `ac29e` (404s, no TUI, HTML reply). `ac29c` asserts no request at all when there is no TUI.
- The stub (`tests/support/stub.rs`, `fake_tmux.rs`) already scripts replies and the title (`rig.showing("OC | ses_B")`); a title that turns later needs a small extension (see Test plan).

Spec pins: ADR-0021 "Switch and reset as built (#645)" (the act, one CAS, mismatch is `unavailable`); `docs/pane-control.md:247-258` (`switch`: "A TUI that does not
show the session afterwards is `unavailable` (1) and records nothing"); the spike `opencode-pane-spike.md:117-124` (`true` with no TUI).

## Open decision (operator), with a recommendation

**Option B, recommended: `select_session` re-sends the select request while the title does not show the session, within its existing bounds.**
Adapter-internal (`crates/holler-adapter-opencode/src/attach.rs`). Contract: **unchanged**. `HarnessPort::select_session` keeps its signature, its
`Ok`-only-once-seen rule (I3) and its failure set; it just tolerates the TUI's start-up window. `relaunch` is untouched. Needs only a prose edit to ADR-0021
(a sentence in "Switch and reset as built (#645)" and the adapter's module doc: the select request is repeated every poll until seen) and the
`pane-control.md` `switch` entry. Idempotent: selecting a session the TUI already shows or will show is harmless, and it is the same HTTP call a human operator
could repeat. Cost: a switch to a not-yet-ready TUI can take up to `settle`/`call` (5 s/10 s) instead of failing at once; **if the measured start-up window
exceeds 5 s (the finding saw success only 8 s after relaunch returned), the default `settle` must also be raised, which is a second, visible choice (see Risks).**
Also fixes `reset` and `reset --first`, `pane doctor --fix`, and `profile apply`'s relaunch-then-select paths, which share `select_session`.

**Option A: `relaunch` waits until the TUI accepts select-session before it returns.**
Contract: **changes**. A new post-condition for `relaunch`/`launch` (the TUI is ready, not merely showing), which needs either a new `HarnessPort` method
(for example a `tui_ready`) or a stronger `attach_tui`; the port is "final" and amended only amend-first, in its own ADR PR, with `holler-pane-testkit` fakes and the
conformance suite (`conformance/harness.rs`) updated, and the Claude adapter (`holler-adapter-claude`) given an answer too. It also has no clean readiness
signal (there is no observable but the select call itself, so the wait would be a select-then-poll anyway), it must fit in the launch engine's 10 s budget together
with the server boot (a measured launch near the budget is already a recorded risk, ADR-0021 line 1013), and it slows every relaunch and launch, including those no
switch follows. It would not help a switch sent to a pane that was relaunched by someone else or earlier-but-still-warming, nor a TUI restarted by `attach_tui` alone.

Recommendation: **B**. One adapter, no contract change, no cost on launches, covers every select caller. State both in the PR if the operator picks A.
Nothing below depends on the choice except the lines marked [A] or [B].

## Acceptance criteria (each observable)
1. [B] `crates/holler-adapter-opencode/tests/attach_test.rs`: a new test `ac29f_...` where the title shows the old session for the first N polls and the target after,
   and the stub receives more than one `POST /tui/select-session`: `select_session` returns `Ok(())` and the record-side call count of POSTs is 2 or more.
   [A] a new test where a relaunch against a fake whose select-session is refused for the first window returns only after it is accepted, and `relaunch` still ends `timeout`
   at the budget when it never is.
2. `ac29a` still passes unchanged (one `GET`, one `POST` when the title shows at once), `ac29b`, `ac29c`, `ac29e` pass unchanged.
3. `ac29d` still passes: a TUI that never takes the switch ends `Err(Timeout{op: select_session})` no later than `settle` + `SLACK` ([B] unchanged bound; if `settle` is raised,
   the test follows `quick().settle`, and the new default is named in the PR).
4. A 404 on a re-sent select still ends `SessionNotFound`, and a dead or missing TUI between sends still ends `unavailable` ("no TUI in pane ..."), without further sends.
5. The #649 scenario no longer needs its retry: `switch_once_the_tui_takes_it` is deleted and `scenario.rs:102` calls `scratch.json(&["pane","switch", ...], 0)` (strict), with the
   `eprintln` race note gone. `grep -rn "TUI-readiness race\|switch_once_the_tui_takes_it" crates` returns nothing. (This test is opt-in; it is run by the operator or an
   opted-in F, `HOLLER_TEST_PANE_INTEGRATION=1`, never in default CI.)
6. ADR-0021 and `docs/pane-control.md` state the chosen rule in one place each; CHANGELOG `## [Unreleased]` has an entry linking #845.
7. `cargo test -p holler-adapter-opencode`, `cargo test -p holler-pane`, `cargo test -p holler-cli --test pane_verbs`, and `cargo clippy` over the touched crates `-- -D warnings` are green
   ([A] adds `-p holler-pane-testkit`, `-p holler-adapter-claude`); `bash scripts/lint.sh` exits 0. No workspace-wide `cargo fmt`.

## Files
Production: [B] `crates/holler-adapter-opencode/src/attach.rs` (the re-send loop; reuse `poll`/`SETTLE_POLL`, add no second poll); possibly `lib.rs` (`Timeouts::settle`
default and its doc) if the measured window needs it. [A] `crates/holler-pane/src/ports.rs`, `ports/router.rs`, `tx_launch/server.rs`, the opencode and claude adapters,
`holler-pane-testkit`.
Tests: `crates/holler-adapter-opencode/tests/attach_test.rs` (+ a small `support/` extension so the fake tmux title can change after N queries); `crates/holler-cli/tests/pane_integration/scenario.rs`.
Docs: `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.
Reuse map (extend, do not duplicate): `poll`, `Tui::watch`, `Tui::query`, `deadline_after`, `known`; the existing stub/fake-tmux rig; `wait_healthy`/`wait_until` for [A] only.

## Decisions already made (MO)
- Rigor in-session, no UI, so D and U never run.
- No change to what a failed switch records: nothing (ADR-0021, #645). A retry is inside the one `select_session` call; `tx_switch` is not changed and does not retry itself
  ([B]); it must not become a verb-level retry that spans port calls.
- The `true` reply stays non-evidence (I3, spike): success is only ever the title.
- The branch to take: **pending the operator** (Open decision).

## Out of scope
- The tmux UTF-8 locale bug (#844), the `LinkedHost` fold (#846), #847, #848, #696.
- Any change to Claude Code panes (a process relaunch, no TUI to select).
- Any change to `pane doctor`, to the reconcile step text, or to `shown_session`.
- Measuring OpenCode's start-up time beyond what the opt-in real test shows; no live Herdr or pane is touched by the brief or by T's default-run tests.

## Test plan (test-first)
RED, in the default run (hermetic, no tmux, no OpenCode): extend the fake-tmux rig so the title query answers the old title for the first N queries, then the target's; the stub
counts `POST /tui/select-session` and, to model "not ready", answers the first N posts `200 true` with no effect on the title. T first writes a characterisation test that shows
the current code stops after one POST and ends `Timeout` (so which of "request hangs" or "title never changes" the not-ready TUI does is pinned in a test, not assumed; if it is
the request that hangs, the test scripts a delayed stub reply and the fix shape changes accordingly, and T reports that to O before F starts).
- [B] RED: `ac29f` (above) fails today with `Timeout`, one POST. GREEN: `select_session` re-sends each poll and returns `Ok`. Keep `ac29a`/`ac29d` as the regression pins; add a test that a
  404 on the second send is `SessionNotFound` and a dead pane on the second query is `unavailable` with no third send.
- [A] RED: the equivalent against `relaunch` plus a conformance-suite case; GREEN after the port change.
- Opt-in: the strict scenario of acceptance 5 is run by the operator (F may run it only with `HOLLER_TEST_PANE_INTEGRATION=1` on a scratch Herdr, as in #649), and its result recorded.
- Mutation check for the retry bound: remove the `settle` cap and see `ac29d` fail.

## Blast radius
- [B] `HarnessPort::select_session` of the OpenCode adapter only. Its callers: `tx_switch::switch_in_place` (switch, reset), doctor's repair (`reconcile/observe.rs`), `profile apply`
  (via the same engine), and the `holler-cli` wiring (`wiring.rs`, `wiring/bind.rs`). No signature changes, no wire, no record, no exit-code change, `holler-proto` untouched. Observable change:
  a switch to a warming TUI succeeds instead of `timeout` (exit 1); a TUI that never takes it still ends `timeout` at the same bound. The Claude adapter and the fakes are untouched.
- [A] `HarnessPort` (a port amendment, ADR amend-first), `holler-pane-testkit` fakes and conformance, both adapters, `tx_launch`, and the budget math for every launch and relaunch.
- Docs: ADR-0021, `pane-control.md`, CHANGELOG. Tests: one adapter test file and one opt-in scenario file.

## Clashes (siblings from the #649 run)
- **#844** (OpenCode adapter tmux UTF-8): same crate, different files. #844 changes how the adapter's tmux client runs/parses (`tui.rs`/`exec.rs`, the `query_args` path) and adds a `LANG=C` test.
  #845 [B] edits `attach.rs::select_session` and uses `Tui::query`. A merge-time conflict is possible only if #844 also touches `attach.rs` or the shared `tests/support/fake_tmux.rs`.
  Whoever merges second rebases; #845's new fake-title helper must not change the query argv #844 pins. Merge order: either; prefer #844 first (it is a prerequisite for any real-tmux run).
  Independent in behaviour.
- **#846** (fold `LinkedHost` fakes): touches `holler-cli` test rigs (`profile_verbs`, `pane_verbs`, `pane_integration`). #845 touches `pane_integration/scenario.rs` (deletes one helper and its call).
  Likely a small textual conflict in `scenario.rs` if #846 edits that file; trivially resolved. #845 adds no `LinkedHost`, so it creates no new duplicate for #846 to fold.
- **#847** (flaky `remote_admin_test`): no overlap (hub test). Do not read a #847 failure in the workspace run as a #845 regression; re-run it alone first.
- **#848** (wizard rename scan skips `docs/handoffs/`): no code overlap, but this brief and any handoff under `docs/handoffs/` quote the old skill path only if they mention it; this brief does not.
  Until #848 merges, keep the old wizard path out of every file under `docs/handoffs/845*`.
- **#696** (shared bounded subprocess runner): it moves the adapters' private `exec` onto a `holler-pane` runner. #845 [B] calls `Tui::run` -> `exec::capture` only through the existing path and adds
  no subprocess code, so the two do not overlap in logic; but both edit the OpenCode adapter's tmux plumbing neighbourhood, so keep #845's diff to `attach.rs` (and tests) and do not touch `exec.rs`.
  The re-send adds no new subprocess kind, only more `query` calls inside the same deadline.

## Risks
- The settle window may be shorter than the start-up window (`settle` 5 s versus up to about 8 s seen). If T/F's measurement shows that, [B] alone still fails sometimes. Then a second choice is
  needed (raise the `settle` default, which also lengthens the failure of a real wedge from 5 s to N s, or accept it): flag it to the operator, do not decide it in F. The call bound (10 s) caps any such raise.
- Which of "title never changes" or "request hangs" the not-ready TUI does is unverified; the fix shape depends on it (see Test plan). A hung request is capped by the 5 s `request` bound and could
  not be re-sent within `settle` without a shorter per-send bound.
- A re-send during a real wedge sends several selects; harmless (idempotent) but visible in the TUI's logs.
- The mismatch after the act stays the closed failure `unavailable`; the re-send must not blur it with `timeout`.
- Risk that A's budget (10 s) is exceeded by a slow boot plus the readiness wait; the reason the recommendation is B.
