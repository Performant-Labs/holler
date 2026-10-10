# Handoff-A: architecture-review — issue #646 parts 2 and 3 (up-front plan review)

**Date:** 2026-10-10
**Branch:** `issue-0646-implementation`
**Brief reviewed:** `docs/handoffs/0646-implementation/brief.md` · **Reuse map:** `docs/handoffs/0646-implementation/survey.md` · **Wireframe:** N/A
**Round-1 verdict (superseded):** BLOCK — see Round 2 below.

## Summary

The plan is architecturally sound and faithful to the issue's blast radius, the epic's nine rulings,
and ADR-0021: `close` extends its own file with a private engine (no frozen-file edits), reuses
`ProfileScope::edit_spec`, `PaneStore::delete` (ruling 9's declared consumer), `HostPort::stop_owned`
and `HerdrPort::close` verbatim; the routing engine in `say_cmd.rs` shared by `interrupt`/`answer`
mirrors part 1's park/unpark engine-sharing precedent; the hub gate extends the `with_holds` pattern
exactly as the issue names. The one BLOCK: the brief asserts the acceptance criterion
"`say --queue` (routed form) returns without waiting for a reply" but names **no mechanism**, and no
in-radius mechanism exists by default — the control exchange for a queued say blocks until the turn
ends (`talk.rs` `send_turn`; `ControlCall::say_with` sets the client deadline to the say timeout
+ 5 s). T cannot write a red test for an unnamed behavior. Remedy in Finding 1.

## Open-point decisions

1. **Radius reading — ACCEPT** the two touches; no CLI-side exception. `circuit.rs:739` is the single
   `HoldGate` construction and `serve.rs:363-366` is the Registry construction; the gate cannot exist
   without them, so they are inside the issue's "only for the pane-state gate at `send_prompt`".
   Precision (binding): the serve.rs edit is a two-statement reorder — `PaneDeps::load` (line 366)
   must move above the registry chain (line 363) so `.with_panes(Arc::clone(&pane_deps.panes))` can
   be appended — still confined to `build_shared_state`, nothing else in the file changes. Note
   `HoldGate` is `#[derive(Clone, Copy)]` (`dispatch.rs:58`); carrying `Option<Arc<PaneState>>` drops
   `Copy` (keep `Clone`), and the three `hold_tests` constructions gain `panes: None` — all inside
   radius files. Record both touches in ADR-0021's "as built". The fallback was correctly judged
   worse: the frozen `Hold` doc (`holler-pane/src/pane.rs:161-165`) itself says a pane-state prompt
   refusal "belongs at `send_prompt`", and CLI-side-only leaves the bare
   `say <session-of-record>` bypass open.

2. **Gate wire carrier — ACCEPT** `Code::SessionHeld` + `data.hold_kind = "pane"` +
   `data.reason = <pane kebab code>` (`pane-parked` / `pane-unhealthy` / `pane-shown-driven-mismatch`),
   message naming pane and code. `Code::InvalidRequest` is REJECTED: `-32600` is a JSON-RPC framing
   code (malformed request); riding a policy refusal on it conflates worse than "held from new work"
   does. Binding conditions: (a) `say_cmd.rs` recognizes `hold_kind == "pane"` **before** the generic
   `is_held` arm and renders its own one-line refusal naming the pane code with the right remedy
   (`holler pane unpark` / `holler pane doctor`), exit 3 — never the generic
   "; ask whoever holds it to release it" tail (`hold_cmd.rs:163-167` points at `hold release`, the
   wrong remedy, and `HELD_EXIT_CODE` 4, the wrong exit); the same pane arm goes where
   `interrupt_cmd.rs`'s `is_held` check lives (its redirect passes `send_prompt`); `answer` needs no
   arm (`session/answer` is not a prompt and is not gated). (b) Omit `data.since` (its documented
   form is RFC 3339; the park's `since` is epoch ms — don't invent a conversion) unless F renders a
   conversion and records it. (c) ADR-0021 "as built" records: the carrier, the `hold_kind`
   vocabulary extension (`operator` | `default` | `pane`), and that `holler release` does NOT lift a
   pane-state refusal (there is no hold entry; the remedy is unpark/doctor). Note: `protocol/v2.md`
   §10 documents the held vocabulary and is not in this story's radius — the ADR row carries the
   record; a later docs pass can move it.

3. **Exit codes — ACCEPT** exit 3 with the stable code in the message for every pane-routed refusal,
   closed (`pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found`) and
   open (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`), matching `class_of`'s table
   (all Refusal(3)) and ADR-0021 §9 decision 5, and for the hub gate's refusal rendered CLI-side
   (same codes, exit 3, per decision 2). Session-side codes are unchanged: busy 1, ambiguous 2, held
   4 (`HELD_EXIT_CODE`), invalid-grant 5, connection 1. ADR-0003's say row is amended by this story
   to record that exit 3 on the prompt verbs now also covers pane-routed policy refusals (it already
   covers a malformed `--timeout`/`--parts-file`); the message's stable code disambiguates. Tests
   assert exit codes equal across formats — automatic on the SayResult path (message + exit are
   format-independent).

4. **`close --spec-only` with no spec — ACCEPT** `pane-not-in-profile`, with a correction the brief
   must absorb: the check is **spec presence in P**, not scope membership, and it runs in `close.rs`
   **before** `edit_spec`. Reason (verified): `with_edit`'s `Remove` "changes nothing when there is
   none" (`profile_scope.rs:248-266`) yet `edit_spec` still performs the CAS (`profile_scope.rs:190-195`)
   — a no-spec Remove through `edit_spec` would bump P's generation and log an update while removing
   nothing. So close.rs pre-checks `profile_store.get(P)` (Ports carries it, `ports.rs:233`) for a
   spec naming the pane and refuses `pane-not-in-profile` first — in **both** the `--spec-only` and
   the live `--profile` paths (the same silent no-op would let a live close "succeed" while removing
   no spec). Do NOT teach `edit_spec` to refuse: the trait is frozen, the conformance suite pins the
   no-op Remove, and §8 step 1 keeps "a detached spec stays removable". Membership is the wrong key
   either way: a member can lack a spec (after an earlier `close --spec-only`) and a detached spec
   can name a non-member — spec presence is the exact condition.

## Findings

| # | Severity | Anchor | Finding (and remedy where NOTE is a ruling) |
|---|---|---|---|
| 1 | BLOCK | brief.md:22-23, brief.md:40 (AC); `crates/holler-hub/src/talk.rs:308-345`; `crates/holler-hub/src/control.rs:83-95`; `crates/holler-hub/src/control_server.rs:287-305` | The AC "`say --queue` (routed form) returns without waiting for a reply" has no named mechanism, and none exists by default: `send_turn` awaits `handle.say(.., timeout)` (the reply arrives when the queued turn ends), and `ControlCall::say_with`'s client deadline is the say timeout + 5 s — the control call blocks for the whole turn. The only hub-side early return would edit `control_server.rs`/`talk.rs`, both outside this story's radius. **Exact remedy (brief amendment, before T writes tests):** the routed `--pane … --queue` path in `say_cmd.rs` sends the prompt with `queue: true` and a short fixed acceptance wait (a `const`, e.g. 2 s, replacing the *client's* wait — the prompt's own `timeout_ms` stays the user's `--timeout` so the hub keeps waiting for the turn); a deadline expiry after acceptance (control `Io` timeout at the deadline the CLI itself chose) is reported as `queued <session>` on stdout, exit 0; any refusal renders per decisions 2/3; if the queued turn happens to finish inside the wait, the reply prints as today. The deadline decision + outcome mapping must be a pure function so T can test it in-process (the wire wait itself is untestable without a hub, as with the rest of say). The bare `say --queue SESSION` form is UNCHANGED — its wait-for-reply contract is #190's — and the ADR-0021 "as built" note records the difference. |
| 2 | NOTE | `circuit.rs:739`; `serve.rs:363-366`; `dispatch.rs:58-64`; `live.rs:453-461` | Open point 1 accepted with the precision recorded above (serve.rs is a two-statement reorder inside `build_shared_state`; `HoldGate` drops `Copy`; `Registry::with_panes` mirrors `with_holds`, `Default` stays valid since the field is `Option`). The ADR "as built" names both out-of-radius touches verbatim so the drift record is exact. |
| 3 | NOTE | `holler-proto/src/error.rs:240-258`; `hold_cmd.rs:130-169`; `say_cmd.rs:155-158` | Open point 2 accepted with its binding conditions (decision 2 above): the pane arm must precede `is_held` in say_cmd.rs (and interrupt's held arm), or a bare `say <session-of-record>` to a parked pane renders the wrong remedy ("ask whoever holds it to release it") and exits 4 instead of 3. |
| 4 | NOTE | ADR-0021 §9 table + "Decisions taken" 5; ADR-0003 say row; `holler-pane/src/error.rs:266-301` | Open point 3 accepted (decision 3 above); the ADR-0003 say-row edit this story already owes is where the exit-3 extension is recorded. |
| 5 | NOTE | `profile_scope.rs:190-195, 248-266`; `ports.rs:233` | Open point 4 accepted with the spec-presence correction (decision 4 above). Without the pre-check the no-spec path silently bumps P's generation and logs a no-op update. |
| 6 | NOTE | brief.md:86-91 (engine chain) | The routing engine omits the **parked** check CLI-side. "CLI-side routing is name resolution plus the same refusals for a clean single-verb failure" (brief's own objective 3) reads as all three pane-state conditions; as listed, `say --pane parked-X` pays a hub round trip to learn what the record it just read already says. Ruling: add the parked check to the engine (open code `pane-parked`, `RefusalCode::from_static` const in say_cmd.rs, exit 3) and list it beside `pane-unhealthy`/`pane-shown-driven-mismatch` in the ADR-0021 code-table row this story edits. The hub gate remains the authority either way. |
| 7 | NOTE | `dispatch.rs:81-103`; `panes/mod.rs:111-152` | Gate matching detail to record in the ADR "as built": match the prompted **bare session name** against each pane's `session_of_record` (one synchronous `PaneState::list()` read, no `await`, mirroring `holds.admit`'s total-ordering argument); refuse if ANY matching pane is parked/unhealthy/SHOWN≠DRIVEN — fail-closed across same-named sessions on different bodies. The pane-state predicate exists twice (CLI engine, hub gate) because `holler-pane/**` is frozen to #637 and the hub cannot depend on the CLI crate; keep each predicate tiny and cross-reference the twin in both doc comments so a later story can fold them into `holler-pane`. This forced twin is not a parallel path. |
| 8 | NOTE | ADR-0021 §8 step 6 + "Switch and reset as built"; `profile_scope.rs:49-56` | Reconcile-step discipline for close: the step follows only failures **after the first live call** (`stop_owned` onwards); the plan refusals (`pane-not-found`, the first-CAS `generation-conflict`, `--spec-only`'s `pane-not-in-profile`) carry none; never double-append — `edit_spec`'s profile-conflict / restore-failure / first-write-timeout messages already carry `reconcile_step`, and §8 step 6's rule is "appends it only to an error that does not carry it". Build via `reconcile_step`/`doctor_command` (the one builders) and record the chosen form in the ADR row. |
| 9 | NOTE | brief.md:76-77 (close order) | The brief's order "read record (`pane-not-found`)" runs unconditionally, which would make `close --spec-only` refuse a pane with **no record** — yet a detached spec for a dead pane is exactly what `--spec-only` exists to clean up (C2, §8 step 1's "a detached spec stays removable"). Ruling: the record requirement belongs to the live close only; with `--spec-only` the spec-presence pre-check (decision 4) is the sole gate. |
| 10 | NOTE | issue #646 scope; ADR-0021 §8 "Switch and reset as built" | For the routed verbs, health is read from the **record** (`harness.health`), per the issue's own words ("refuse when the record says the pane is unhealthy") — unlike `tx_switch`'s live-observed `server-unhealthy`, which acts on the server. No conflict, but the ADR row should say "the record says" so S's audit anchors to the right source. |
| 11 | NOTE | `tests/pane_verbs/process/stub.rs:18-40` | Besides dropping the `("pane", "close", 646)` row, this story deletes the `PANE_FORM_REFUSAL` const per that file's own instructions (keep the `// #646` comment line) — the brief's "drops the close row" under-names it; T will see it, but the brief should say both. |

## Reuse-map check

Verified against the map, item by item: the close engine lives in `close.rs` (no `tx_close` in the
frozen `holler-pane/**`, correctly mirroring `park.rs`'s own engine); `edit_spec` reused verbatim
(never re-implemented); `PaneStore::delete` is ruling 9's declared consumer; `stop_owned`/`close`
ports called in the issue's order; the routing engine in `say_cmd.rs` called by
`interrupt_cmd.rs`/`answer_cmd.rs` follows the park/unpark `super::park` precedent (the three cmd
files are top-level modules, so `crate::say_cmd::…` resolves); `prompt_target.rs` stays untouched
with `route()` left `pub` (dead but frozen — correct); the gate extends `Registry::with_holds`/
`HoldGate`/`hold_tests`, not a parallel registry; tests copy `park/rig.rs` and use the kit fakes and
`check_envelope`. Frozen files confirmed untouched by the plan: `pane/mod.rs` (`PaneClose` is already
mounted at `pane/mod.rs:46,63`; adding the `PANE` positional edits only `close.rs`), `prompt_target.rs`,
`holler-pane` types (open codes as `RefusalCode::from_static` consts in `say_cmd.rs` — ruling 3's
"constants in each verb's own file" applied to the cmd file that owns the routing), `holler-proto`'s
`Code` table (17 variants, verified closed). Exit codes via `class_of`/`output::emit` for close;
`--json --format=text` and envelope parity inherited from the shared output module. No new
dependency; no `send_text`/`send_keys` use (I4 clean). The one duplication risk — the twin
pane-state predicates — is forced by the frozen crate boundary and is recorded (Finding 7), not a
parallel path.

## Notes for O

Amend the brief with Finding 1's mechanism (required — the BLOCK), and fold in the NOTE rulings you
accept (2-11 are written as bindings F can follow directly; at minimum absorb 5, 6 and 9, which
correct the engine chain and the close order). Then re-present for A's re-review; no other change is
needed — the plan's shape is right.

## Patterns referenced

- `crates/holler-cli/src/pane/park.rs` (part 1's engine-sharing + rig precedent)
- `crates/holler-cli/src/pane/profile_scope.rs` (`edit_spec` write order, no-op Remove, `reconcile_step`)
- `crates/holler-hub/src/circuit/dispatch.rs` (`HoldGate`, `send_prompt`, `hold_tests`)
- `crates/holler-hub/src/live.rs` (`Registry::with_holds` precedent)
- `docs/adr/ADR-0021.md` §8 (I8 order, tx_switch as-built), §9 (codes, exits), and ADR-0003's rows

## Round 2 — re-check of brief v2

**Date:** 2026-10-10 · **Brief re-reviewed:** `docs/handoffs/0646-implementation/brief.md` (v2)

Brief v2 carries the remedy and every ruling faithfully. § Queue (brief.md:83-94) restates Finding
1's mechanism exactly — the named ~2 s `const` replaces the *client's* wait only (the prompt's
`timeout_ms` stays the user's `--timeout`, so the hub keeps waiting for the turn), deadline after
acceptance maps to `queued <session>` on stdout with exit 0, refusals render per bindings 2/3, a
turn finishing inside the wait prints the reply as today, the deadline decision + outcome mapping
is a pure function for T, the bare `say --queue SESSION` form is unchanged, and the ADR-0021 "as
built" note records the difference — and the AC (brief.md:105-106) now names the mechanism and the
tested pure function, so T has a named behavior to write a red test against. All ten NOTE rulings
are folded in as bindings 1-9 plus objective 2's record-source wording, with nothing dropped or
distorted; the one compression (binding 2's flat "Omit `data.since`" versus my "unless F renders
and records a conversion") is stricter, not looser, and the brief declares this handoff the
authority for exact wording. One reading nit, recorded here and not a finding: the close-order
design decision brackets the record read inside `act` (the live close), so objective 1
("`--spec-only` … closes nothing") and binding 8 govern the `--spec-only` reading.

Carry-map (finding → brief v2): F1 → § Queue + AC brief.md:105-106 · F2 → binding 1 + blast
radius · F3 → binding 2 · F4 → binding 3 · F5 → binding 4 · F6 → binding 5 + engine chain
brief.md:152 · F7 → binding 6 · F8 → binding 7 · F9 → binding 8 · F10 → objective 2
brief.md:24-26 · F11 → binding 9 + AC brief.md:118-119. Zero block findings remain.

Round 2 is the operative verdict; it supersedes round 1.
**Verdict:** PASS
