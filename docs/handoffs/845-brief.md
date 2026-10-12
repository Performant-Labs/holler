# Brief: #845 switch-after-relaunch

Repo: Performant-Labs/holler. Issue: #845 (part of epic #633). Rigor: in-session. UI surface: no. Kind: bug.

**STATUS: DECIDED, option B.** The operator decided on 2026-10-12 (issue #845, comment of 18:20Z): `select_session` in the OpenCode adapter re-sends the select
request while the title does not show the session, within its existing bounds; no contract change. If measurement shows the 5 s `settle` window must be raised,
that returns to the operator as a separate choice (section "Needs operator decision if measured"); the pipeline stops there and does not decide it.
Rejected: A (`relaunch` waits until the TUI accepts select-session): it changes the `HarnessPort` contract, has no readiness signal but the select call itself,
and spends the launch engine's 10 s budget on every launch.

## Problem
A `holler pane switch` sent right after `holler pane relaunch` of an OpenCode pane ends `timeout` in `harness.select_session` until the
relaunched TUI takes `/tui/select-session`. Seen 3 of 3 in the #649 scenario and 2 of 3 in a probe (OpenCode 1.19.0); the same switch sent 8 s
later always succeeded. `relaunch` returns once the TUI's title shows the session of record (which is also when `attach_tui` returns), not once
the TUI accepts a select-session call. A failed switch records nothing, so the operator sees a spurious `timeout` on a healthy pane.
The #649 scenario hides it with a 20 s retry loop (`switch_once_the_tui_takes_it`).

## Is it settled by an existing decision? Now yes: the operator's.
ADR-0021 ("Switch and reset as built (#645)", line 543), `docs/protocol/v2.md`, `docs/pane-control.md` (`switch`, lines 247-258) and `docs/handoffs/649/decisions.md`
(lines 286-295) do not settle readiness or retrying; the operator did, for B. What stays pinned: the OpenCode adapter header (attach.rs:1-19) says one deadline per method,
"an attach or a switch is trusted only once seen (I3)", and `select-session` "answers `true` with no TUI at all (opencode-pane-spike.md:117-124), so its `true` proves nothing".
ADR-0021 section 12 / I5: each port call returns within its bound (10 s default) or `timeout`.

## Evidence (verbatim, from `origin/main` at 05471b9)

The act and its watch, `crates/holler-adapter-opencode/src/attach.rs:58-90` (doc comment 58-62, fn 63-90):
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
were removed); T pins it first, in a test (Work for T/F, item 2).

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

## The decided plan (B)
`select_session` (attach.rs:63-90) sends the select request again on each poll while the title does not show the session, until `settle` or the method's `call` deadline ends it.
- Adapter-internal (`crates/holler-adapter-opencode/src/attach.rs`). `HarnessPort::select_session` (`crates/holler-pane/src/ports.rs:479`) keeps its signature, its `Ok`-only-once-seen
  rule (I3) and its failure set. `relaunch`, `tx_switch`, `shown_session`, the Claude adapter and the fakes are untouched.
- The session check (`known`, one GET) stays once, before the first send. The `true` reply stays non-evidence: success is only ever the title. A re-send reuses the crate's one `poll`
  (lib.rs:655) and `Tui::query`; add no second poll.
- Idempotent: selecting a session the TUI already shows or will show is harmless. Cost: a switch to a not-yet-ready TUI takes up to `settle` instead of failing at once.
- Also fixes `reset`, `reset --first` (both through `tx_switch.rs:244`) and doctor's repair (`reconcile/observe.rs:375`), which share `select_session`. The brief earlier also named `profile apply`
  as a path; that is not re-verified (no direct call in `crates` outside the three above and `wiring/bind.rs:335`, which only forwards).
- Docs: one sentence each in ADR-0021 "Switch and reset as built (#645)" and the adapter module doc (attach.rs:1-19): the select request is repeated every poll until seen; and in the
  `pane-control.md` `switch` entry (note: it says an unshown session is `unavailable`, while the adapter ends `timeout` when the title never changes; state the rule as built, do not widen).

## Work for T and F (open risks, kept as explicit work)
1. **Is the 5 s `settle` window too short?** Success was seen about 8 s after `relaunch` returned, `settle` is 5 s (lib.rs:260, 269). T must measure it (the opt-in real OpenCode, `HOLLER_TEST_PANE_INTEGRATION=1`
   on a scratch Herdr, or the recorded #649 timings) and pin the figure in its handoff, with the method. If 5 s suffices, nothing changes. If it does not, F must NOT raise `settle`
   or `call`: stop and report under "Needs operator decision if measured".
2. **Is the timeout the title never changing, or the request hanging?** T pins this FIRST, in a test, before F writes code: one characterisation test where the not-ready TUI answers the POST
   `200 true` without the title changing (today: one POST, then `Timeout` at `settle`), and one where the stub delays or never answers the POST (today: `Timeout` from the 5 s `request` bound, or `unavailable`).
   Re-sending fixes the first; it does not fix a hung request (capped by `request`, so not re-sendable inside a shorter `settle`). If the hung-request case is the real one, B as decided does not cover it:
   T reports it to O before F starts, and that too goes under "Needs operator decision if measured" (it would need a per-send bound shorter than `request`).

## Needs operator decision if measured
Nothing is in this list yet. Entries are added only by T or F, and the pipeline stops when one is added:
- (a) `settle` (and so the failure time of a real wedge) must be raised because the measured start-up window exceeds 5 s. The 10 s `call` bound caps any raise.
- (b) the not-ready TUI hangs the request rather than leaving the title unchanged, so a per-send bound is needed.

## Acceptance criteria (each observable)
1. `crates/holler-adapter-opencode/tests/attach_test.rs`: a new test `ac29f_...` where the title shows the old session until the stub has received N select POSTs and the target after,
   `select_session` returns `Ok(())`, and the stub saw 2 or more `POST /tui/select-session`. RED today: `Timeout` after one POST.
2. `ac29a` (L293) passes unchanged (one `GET /session/ses_B`, one `POST` when the title shows at once); `ac29b`, `ac29c` (no request at all with no TUI), `ac29e` pass unchanged.
3. `ac29d` (L363) passes: a TUI that never takes the switch ends `Err(Timeout{op: select_session})` no later than `quick().settle + SLACK` (SLACK 500 ms, L44). The bound is unchanged.
4. A 404 on a re-sent select ends `SessionNotFound`; a dead or missing TUI between sends ends `unavailable` ("no TUI in pane ..."), with no further send.
5. T's pin of risk 2 and the measurement of risk 1 are in T's handoff (RED and GREEN), before F's first edit.
6. The #649 scenario no longer needs its retry: `switch_once_the_tui_takes_it` (`scenario.rs:42-67`, called at `scenario.rs:102`) is deleted and the call becomes
   `scratch.json(&["pane","switch", ...], 0)` (strict), with the `eprintln` race note gone. `grep -rn "TUI-readiness race\|switch_once_the_tui_takes_it" crates` returns nothing.
   (Opt-in; run by the operator or an opted-in F, `HOLLER_TEST_PANE_INTEGRATION=1`, never in default CI; its result is recorded.)
7. ADR-0021 and `docs/pane-control.md` state the rule in one place each; CHANGELOG `## [Unreleased]` has an entry linking #845.
8. `cargo test -p holler-adapter-opencode`, `cargo test -p holler-pane`, `cargo test -p holler-cli --test pane_verbs`, and `cargo clippy` over the touched crates `-- -D warnings` are green;
   `bash scripts/lint.sh` exits 0. No workspace-wide `cargo fmt`.

## Test plan (test-first)
RED, in the default run (hermetic, no tmux, no OpenCode):
- T first writes the two characterisation tests of risk 2 and confirms what the current code does (one POST then `Timeout`).
- Then `ac29f`. The fake tmux answers `display-message` from a static data file (`tests/support/fake_tmux.rs`, `reply`), so a title that turns later is best made WITHOUT touching the fixture or
  `fake_tmux.rs` (both changed by #844 PR #870): have the test, on the Nth `POST /tui/select-session` the stub receives, rewrite the `display-message` reply (`Rig::showing`, attach_test.rs:114) from a stub
  callback or a small helper in `attach_test.rs`/`stub.rs`. If that proves impossible, T says so before changing `fake-tmux`, and the Clashes entry for #844 applies.
- Also: 404 on the second send (`SessionNotFound`), dead pane on the second query (`unavailable`, no third send).
- GREEN: `select_session` re-sends each poll and returns `Ok`. `ac29a`/`ac29d` are the regression pins.
- Mutation check for the bound: remove the `settle` cap on the loop and see `ac29d` fail.

## Files
Production: `crates/holler-adapter-opencode/src/attach.rs` only (the re-send; reuse `poll`/`SETTLE_POLL`/`Tui::query`). `lib.rs` (`Timeouts::settle` default) is NOT touched without the operator.
Tests: `crates/holler-adapter-opencode/tests/attach_test.rs` (+ at most a small helper in `tests/support/stub.rs`); `crates/holler-cli/tests/pane_integration/scenario.rs`.
Docs: `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.
Reuse map (extend, do not duplicate): `poll`, `Tui::watch`, `Tui::query`, `deadline_after`, `known`; the existing stub/fake-tmux rig.

## Decisions already made
- Rigor in-session, no UI, so D and U never run.
- Option B (operator, 2026-10-12). The retry is inside the one `select_session` call; `tx_switch` does not change and does not retry (no verb-level retry spanning port calls).
- A failed switch still records nothing (ADR-0021, #645). The mismatch after the act stays the closed failure `unavailable`; the re-send must not blur it with `timeout`.
- The `true` reply stays non-evidence (I3, spike).

## Out of scope
- The tmux UTF-8 locale bug (#844), the `LinkedHost` fold (#846), #847, #848, #696.
- Any change to Claude Code panes, `pane doctor`, the reconcile step text, `shown_session`, `relaunch`, or the `HarnessPort` contract.
- Raising `settle` or `call` (operator's, see above). No live Herdr or pane is touched by default-run tests.

## Blast radius
`HarnessPort::select_session` of the OpenCode adapter only. Callers: `tx_switch::switch_in_place` (switch, reset; tx_switch.rs:244), doctor's repair (`reconcile/observe.rs:375`), and the
`holler-cli` forwarder (`wiring/bind.rs:335`). No signature, wire, record or exit-code change; `holler-proto` untouched. Observable change: a switch to a warming TUI succeeds instead of
`timeout` (exit 1); a TUI that never takes it still ends `timeout` at the same bound; a real wedge now receives several idempotent selects (visible in the TUI's logs). Docs: ADR-0021,
`pane-control.md`, CHANGELOG. Tests: one adapter test file and one opt-in scenario file.

## Clashes (checked against `git fetch origin`; this branch is rebased on origin/main 05471b9, with #847 and #848 merged)
- **#844, PR #870** (open): changes `tmux_command -u` in `holler-adapter-opencode/src/tui.rs`, `tests/fixtures/fake-tmux`, `tests/support/fake_tmux.rs`, `tests/tui_test.rs`, `tests/utf8_locale_test.rs`,
  `holler-cli` `pane_integration/scratch/mod.rs` and `guards.rs`, ADR-0021 and CHANGELOG. #845 does not edit `tui.rs`, the fixture or `fake_tmux.rs` (the test plan avoids them), so no code conflict is expected;
  `Tui::query` runs through the `tmux_command` that #844 changes, so a #845 test that pins the query argv would break: pin none. Textual conflicts are likely only in ADR-0021 and CHANGELOG.
  Whoever merges second rebases. Prefer #844 first (any real-tmux run needs its UTF-8 fix).
- **#846, PR #868** (open): changes `holler-cli` tests `verb_harness/linked_host.rs`, `verb_harness/mod.rs`, rigs (`pane_integration/bind.rs`, `fake_hub.rs`, `main.rs`, `no_hub.rs`, `observe.rs`, `traps.rs`, `wired.rs`,
  `wrappers.rs`, `pane_verbs/launch/rig.rs`, `profile_verbs/apply/rig.rs`, `pane_claude/rig.rs`) and scenario files. Per the file list it does not edit `pane_integration/scenario.rs`, but it edits `pane_integration/main.rs`
  and `scratch`-neighbouring modules the scenario uses; re-check `scenario.rs` and the imports of `wait_for`/`check_envelope` after it merges (they may become unused once the helper is deleted: remove them, do not leave warnings).
  #845 adds no `LinkedHost`.
- **#847, #848**: merged (ef14b1a, 05471b9); no overlap. #848 means a handoff under `docs/handoffs/` may now quote the old wizard path; this brief still does not.
- **#696** (shared bounded subprocess runner): keep the diff to `attach.rs` and tests; do not touch `exec.rs`. The re-send adds no new subprocess kind, only more `query` calls inside the same deadline.

## Risks
- The settle window and the hung-request case are Work for T/F items 1 and 2, with their stop conditions under "Needs operator decision if measured".
- A re-send during a real wedge sends several selects; harmless (idempotent).
- The re-send must not mask a dead TUI or a 404 as `timeout` (criterion 4).
