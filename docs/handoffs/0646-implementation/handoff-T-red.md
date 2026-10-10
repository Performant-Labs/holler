# Handoff-T-red: Phase 4 — the failing suite for #646 parts 2 and 3

**Date:** 2026-10-10
**Branch:** `issue-0646-implementation`
**Brief / wireframe reviewed:** `docs/handoffs/0646-implementation/brief.md` (v2),
`docs/handoffs/0646-implementation/handoff-A.md` (round-2 PASS, rulings 1-11), issue
#646, ADR-0021 (§8, §9). No wireframe (uiSurface false).

## A precondition

Confirmed: A round 2 PASS on brief v2 (handoff-A.md), every ruling folded in as a
binding. The plan T wrote tests against is the approved one.

## ⚠ One blocker for O before F starts

**The dispatch.rs gate tests are authored but not applied.** T's session write scope
is `crates/*/tests/**` and `docs/handoffs/**`; the instructed edit inside
`crates/holler-hub/src/circuit/dispatch.rs`'s `#[cfg(test)]` module was **denied by
the permission layer**, and a denial is policy, not a puzzle. The complete,
paste-ready block (6 tests + the three `panes: None` edits to the existing
`HoldGate` constructions + the production shape it pins) is
[`gate-tests-dispatch.md`](gate-tests-dispatch.md), beside this handoff. **O must
apply it (or re-scope T) before F starts**, or the hub-gate piece (Objective 3,
bindings 1/2/6, the `send_prompt` AC) is implemented against no red test.

## Files touched (all test-side, plus the two handoff files)

| File | Change |
|---|---|
| `crates/holler-cli/tests/pane_verbs/close.rs` | replaced the #670 stub case with 14 close tests |
| `crates/holler-cli/tests/pane_verbs/close/rig.rs` | new: the close rig (fakes + live seeding + call-log span) |
| `crates/holler-cli/tests/pane_verbs/target_flags.rs` | + 7 tests: the routing engine, the queued wait, the pane arm |
| `crates/holler-cli/tests/pane_verbs/process/stub.rs` | ruling 9: dropped the `("pane","close",646)` row, deleted `PANE_FORM_REFUSAL` (both `// #646` lines kept) |
| `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs` | rewritten (forced: ruling 9 deleted the const it imported); its cases now pin the *routed* behaviour |
| `docs/handoffs/0646-implementation/gate-tests-dispatch.md` | new: the blocked gate tests, paste-ready |

27 tests authored and applied (14 close + 7 routing/queue/arm + 6 rewritten
process-level, of which 3 fail today), plus 6 authored-but-blocked gate tests in the
snippet. Every file rustfmt-clean (edition 2021) and clippy-clean.

## The pure-signature contract F implements (T owns these shapes)

In `crates/holler-cli/src/say_cmd.rs`:

```rust
/// The three open pane-state codes (ruling 3: consts in the owning cmd file).
pub const PANE_PARKED: RefusalCode = RefusalCode::from_static("pane-parked");
pub const PANE_UNHEALTHY: RefusalCode = RefusalCode::from_static("pane-unhealthy");
pub const PANE_SHOWN_DRIVEN_MISMATCH: RefusalCode =
    RefusalCode::from_static("pane-shown-driven-mismatch");

/// Resolve `--pane NAME [--profile P]` to the pane's session of record.
/// Chain (the brief's design decision, in order): `PaneName::parse` (usage),
/// `scope.resolve(P, Some(name))` when `--profile` (so `profile-not-found` and
/// `pane-not-in-profile` come from the helper, exactly as park), `pane_store.get`
/// (`pane-not-found`), `session_of_record` (`session-not-found`, message names the
/// pane), then the pane-state checks from the record: `hold == Parked`
/// (`pane-parked`), `Health::Unhealthy(reason)` (`pane-unhealthy`, reason in the
/// message), SHOWN and DRIVEN both `Some` and differing
/// (`pane-shown-driven-mismatch`). Every refusal is a refusal (exit 3); the three
/// open codes appear in their messages.
pub fn resolve_pane_target(
    ports: Ports<'_>,
    pane: &str,
    profile: Option<&str>,
) -> Result<String, PaneError>;

/// What the routed `say --pane … --queue` acceptance wait ended with.
pub enum QueueWait {
    /// The queued turn finished inside the wait; the control call answered this.
    Reply(serde_json::Value),
    /// The CLI's own acceptance deadline expired after the prompt was accepted.
    Deadline,
}

/// Map the wait's end to what `say` prints: `Reply` prints exactly the bare form's
/// reply (text: the `text` member; JSON: the document); `Deadline` prints
/// `queued <session>` (JSON: `{"queued":true,"session":"…"}`) on stdout, exit 0 —
/// the prompt was accepted, so returning is not a failure.
pub fn queue_outcome(session: &str, wait: QueueWait, json: bool) -> SayResult;

/// Render a `-32011 session_held` whose `data.hold_kind == "pane"` (binding 2): one
/// line naming the pane code with the right remedy — `holler pane unpark` for
/// `pane-parked`, `holler pane doctor` otherwise — exit 3, never the generic
/// release tail and never exit 4. JSON: the held form's object shape with
/// `hold_kind: "pane"`, on stdout. `None` for every other `-32011` (the caller
/// falls through to the generic `is_held` arm). `interrupt_cmd.rs` calls the same
/// helper and maps the `SayResult` into its own result; `answer` needs no arm.
pub fn pane_hold_refusal(e: &holler_proto::WireError, json: bool) -> Option<SayResult>;
```

On the hub (the shape the blocked gate tests pin — see `gate-tests-dispatch.md`):
`HoldGate` gains `panes: Option<Arc<crate::panes::PaneState>>` (drops `Copy`);
`Registry::with_panes(..)` + a `panes()` getter; `send_prompt` checks after
`holds.admit` with one synchronous `PaneState::list()`, matching the prompted bare
session name, refusing fail-closed (first bad pane in name order) via
`WireError::new(Code::SessionHeld, message-naming-pane-and-code, Some(code))`
`.with_hold_kind("pane")` — no `data.since`.

## Tests authored (each names the AC/binding it pins)

**close (`close.rs`, 14)** — `close_takes_one_pane_positional` (the clap contract);
`close_stops_the_processes_closes_the_herdr_pane_and_deletes_the_record` (the close
AC: host log exactly `[StopOwned]`, Herdr exactly `[Close]` — no `ensure_pane`, so a
position never moves; pane writes exactly `[Delete]`; `assert_gone`: record, Herdr
pane and owned processes all gone); `close_json_is_one_envelope_and_the_exits_agree`
(envelope helper, `{"pane"}` data, format parity); `close_with_a_profile_removes_…`
(spec gone, generation bumped **once** to 2, one `Updated` log entry, same live
calls); `a_failed_live_close_restores_the_profile_specs` (specs unchanged, generation
3 — "may move by two", 3 log entries, record kept, message ends with the profile
reconcile step); `close_spec_only_removes_the_spec_and_touches_nothing_live` (no
adapter call, no delete, P edited); `close_spec_only_removes_a_detached_spec_…`
(binding 8: no record needed); `close_refuses_a_spec_less_pane_before_any_profile_write`
(binding 4: `pane-not-in-profile` in both the live and `--spec-only` paths, generation
still 1, nothing live); `close_refuses_a_pane_with_no_record` (exit 3, no step);
`close_needs_a_pane` (usage exit 2, park's optional-positional pattern);
`close_without_a_profile_never_touches_a_profile` (not even a read);
`a_live_failure_after_stop_owned_names_the_reconcile_step` (ruling 7 both ways: a
failed `stop_owned` sends nothing to Herdr yet carries the step; a failed Herdr close
keeps the record and carries it); `a_record_conflict_after_the_act_fails_loudly`
(`generation-conflict` + step, bare and with `--profile`, where P is already
restored); `a_store_failure_fails_the_run` (exit 1, nothing live).

**routing / queue / pane arm (`target_flags.rs`, 7)** —
`the_engine_answers_the_session_of_record_of_a_healthy_pane` (healthy, driven-absent
and shown-absent both fine, `--profile` beside `--pane`);
`the_engine_refuses_every_routed_case_with_exit_3` (all seven codes table-driven,
`class_of` == Refusal, message names pane/profile/reason as applicable);
`the_engine_checks_the_name_the_scope_the_record_then_the_session` (bad name is
usage; the scope refuses before the record; the session before the pane-state
checks); `a_deadline_expiry_after_acceptance_is_queued_exit_0`;
`a_reply_inside_the_wait_prints_as_the_bare_form_does`;
`a_pane_hold_renders_one_line_exit_3_with_the_right_remedy` (unpark for parked,
doctor otherwise, never the release tail, never exit 4);
`every_other_held_error_falls_through_to_the_generic_arm`.

**process level (`process/legacy_verbs.rs`, 6 — 3 red, 3 kept)** —
`the_pane_forms_route_and_never_print_the_646_stub` and
`the_pane_forms_stay_plain_text_under_every_format` (the stub refusal is gone; plain
text, no envelope, exit 1 in this no-hub environment);
`a_profile_without_a_pane_is_a_usage_error` (exit 2, message names `--pane` —
ADR-0021 §3's "still need a pane name"); the roster, malformed-form and
session-forms cases kept as they were (minus the deleted const).

**gate (`gate-tests-dispatch.md`, 6 — authored, blocked from application)** — parked
refused in every variant with the full wire shape (`-32011`, `hold_kind: "pane"`,
`data.reason` = the code, no `data.since`, one line naming pane and code, nothing
sent); unhealthy and SHOWN≠DRIVEN the same; a healthy matching pane and an unmatched
session go through; fail-closed across same-named sessions (first bad pane in name
order); an operator hold refuses first with its own kind; `with_panes` carries the
handle.

## RED confirmation

Run on the private, freshly seeded `target/` (the de-contaminated one), staged so
each piece's red is attributable:

1. **close, with the routing import set aside** (staged run, then restored — the
   final tree is the full one):
   `cargo test -p holler-cli --test pane_verbs close::` →
   `test result: FAILED. 0 passed; 14 failed` — 13 fail on
   `["holler", "pane", "close", …] must parse: error: unexpected argument '<PANE>'
   found` (the missing positional is the contract those tests stand on), and
   `close_needs_a_pane` fails as the assertion
   `left: 1 (error: not implemented (story #646)), right: 2` — the stub's exit 1
   against the wanted usage 2. All 14 for the right reason; no setup or import error.
2. **final tree, the whole suite** (the runner's command):
   `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`
   → exit 101, one compile error, exactly the missing-API red:
   `error[E0432]: unresolved imports holler_cli::say_cmd::{pane_hold_refusal,
   queue_outcome, resolve_pane_target, QueueWait}` (`target_flags.rs:18`). This is
   the engine/queue/pane-arm contract; cargo stops the workspace here, so the close
   and process reds above are the staged evidence for those pieces.
3. **process target**: `cargo test -p holler-cli --test pane_cli_process` →
   `33 passed; 3 failed` — `a_profile_without_a_pane_is_a_usage_error` (got exit 1 +
   the 646 stub line), `the_pane_forms_route_and_never_print_the_646_stub` and
   `the_pane_forms_stay_plain_text_under_every_format` (the binary still prints
   `error: not implemented (story #646)`). All three name the refusal this story
   replaces. The stub-table tests still pass (the ruling-9 deletions are clean).
4. **hub lib**: `cargo test -p holler-hub --lib` → `77 passed; 0 failed` — unchanged,
   because the gate tests are the blocked piece (see the blocker above).

Compile-fail vs assert-fail split: **1 compile error** (four missing `say_cmd` items,
intentional — that is the red for the engine, the queue mechanism and the pane arm);
**14 test failures** staged for close (13 parse-contract + 1 assertion); **3
assertion failures** at process level; **6 authored** gate tests unapplied.

## Contract decisions T made (all within the brief's "F/T's within the contract")

1. **Signatures** as spelled out above, including `pane: &str` (the engine owns
   `PaneName::parse`) and `profile: Option<&str>`.
2. **close's clap `Args`**: PANE is an *optional* positional the verb itself requires
   (park's pattern), so `pane close` with no PANE is the verb's `usage` message, not
   clap's — pinned by `close_needs_a_pane` running through the in-process harness.
3. **Wording**: `closed <pane>`; `closed <pane> (removed from profile "<P>")`;
   `removed <pane> from profile "<P>"` for `--spec-only`; JSON data `{"pane": <name>}`
   and `{"pane": <name>, "generation": <g>}` when P was written; the JSON queued line
   `{"queued":true,"session":"<s>"}`; the pane-arm JSON
   `{"error":"session_held","session":"<s>","reason":"<code>","hold_kind":"pane"}`.
4. **Call-set pins**: the bare close's host log is exactly `[StopOwned]` and Herdr's
   exactly `[Close]` — no `ensure_pane` (a close never moves a Herdr position), no
   keystroke, and **no snapshot observe** (if F wants an observe step, that is a
   contract conversation at GREEN, not a silent addition). Pairwise ordering is pinned
   by fault injection (a failed `stop_owned` sends nothing to Herdr; a failed Herdr
   close deletes nothing).
5. **`--profile` without `--pane`** on `say`/`interrupt`/`answer` is usage, exit 2,
   message naming `--pane` (ADR-0021 §3's "still need a pane name"); pinned at
   process level.
6. **The gate's fail-closed determinism**: when several panes share a session of
   record, the refusal names the first bad pane in name order (`PaneState::list()` is
   name-sorted).
7. **`legacy_verbs.rs`** was rewritten alongside ruling 9 (deleting
   `PANE_FORM_REFUSAL` removes its import; its cases assert exactly the refusal this
   story replaces). Test-side, within the story's test blast radius; flagged to O.
8. **Not pinned** (deliberately): close on an *already-gone* Herdr pane (the fake
   answers `pane-not-found`; whether close treats that as success or loud failure is
   F's to decide and S's to audit), close on a parked pane (allowed — park is a
   record hold, close is teardown), and `say`'s wire-wait behaviour beyond the pure
   functions (untestable in-process, as with the rest of say).

## Ready for F

RED is valid on every piece T could apply: each new test fails for the reason the
feature will fix — the missing `PANE` positional and stub exits for close, the four
missing `say_cmd` items for routing/queue/arm, the still-present #646 stub at process
level. **Condition on O:** apply `gate-tests-dispatch.md` (or re-scope T to do it)
before F starts, so the hub gate is red-tested too. F implements against these tests
and the signatures above.
