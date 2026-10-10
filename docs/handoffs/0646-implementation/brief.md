# Brief — issue #646 parts 2 and 3: `holler pane close`, and `say`/`interrupt`/`answer` routed by pane

Run: `0646-implementation` · branch `issue-0646-implementation` · **rigor: in-session** ·
uiSurface: false (D and U declared N/A). Part 1 (`park`/`unpark`) merged as #711; this run
delivers the rest of the story. The GitHub issue is the spec; the epic (#633) contract and
ADR-0021 are fixed.

**v2 — amended after A's BLOCK (attempt 1)**: Finding 1's mechanism added (§ Queue below); A's
open-point rulings 1-4 and NOTE rulings 2-11 folded in as bindings. handoff-A.md is the authority
for their exact wording; this brief carries the plan against them.

## Objective

1. **`holler pane close PANE [--profile NAME] [--spec-only]`** (part 2): stop the pane's owned
   processes (`HostPort::stop_owned`), close its Herdr pane (`HerdrPort::close`), and remove its
   record (`PaneStore::delete` at the read generation, ruling 9) — plan → act → observe → record
   (I3), never moving a pane's Herdr position. With `--profile P` it is spec-editing: through
   `ProfileScope::edit_spec(P, pane, SpecEdit::Remove, act)` the spec removal and the live close
   are ONE transaction (I8); a failed close leaves P's specs unchanged (the generation may move
   by two, ADR-0021 decision 1); a successful close bumps P's generation once. `--spec-only`
   removes the spec and closes nothing. Without `--profile` no profile is written.
2. **`say`/`interrupt`/`answer --pane NAME`/`--profile P` routing** (part 3): replace the
   `not implemented (story #646)` refusal in `say_cmd.rs`/`interrupt_cmd.rs`/`answer_cmd.rs`.
   `--pane NAME` addresses the pane's **session of record**; the verbs refuse when the record
   says the pane is parked, unhealthy, or has SHOWN ≠ DRIVEN (the record is the source —
   `harness.health`, `last_observed`; not a live probe). `--profile P` beside `--pane X` refuses
   a pane outside P. A pane with no session of record refuses (`session-not-found`, naming the
   pane). `say --queue` (routed form) returns at once (§ Queue).
3. **The pane-state prompt gate at `send_prompt`** (part 3, the issue's amendment): a prompt
   refusal derived from pane state must sit at `send_prompt` (`circuit/dispatch.rs`), the hub's
   one enforcement point, following the holds precedent: the `Registry` (live.rs) carries a
   pane-state handle (`Arc<panes::PaneState>` via a `with_panes` builder mirroring `with_holds`)
   that the gate checks — a prompt to any session that is a pane's session of record is refused
   while that pane is parked, unhealthy, or has SHOWN ≠ DRIVEN (both observed, differing). This
   closes the bypass a bare `say <session-of-record>` would otherwise have; CLI-side routing is
   name resolution plus the same refusals for a clean single-verb failure (and one fewer hub
   round trip).

## A's rulings, folded in as bindings (attempt 1)

1. **Radius**: the two touches accepted — `circuit.rs`'s single `HoldGate { .. }` construction
   (line ~739) gains the handle; `serve.rs`'s `build_shared_state` gets a two-statement reorder
   (`PaneDeps::load` above the registry chain) plus `.with_panes(Arc::clone(&pane_deps.panes))`.
   `HoldGate` drops `Copy` (keeps `Clone`); the three `hold_tests` constructions gain
   `panes: None`. Both touches named verbatim in ADR-0021 "as built".
2. **Gate wire carrier**: `Code::SessionHeld` + `data.hold_kind = "pane"` + `data.reason` =
   the pane kebab code; message names pane and code. `say_cmd.rs` recognizes
   `hold_kind == "pane"` **before** the generic `is_held` arm (same arm where
   `interrupt_cmd.rs` checks `is_held`; `answer` needs none — `session/answer` is not gated) and
   renders a one-line refusal with the right remedy (`holler pane unpark` / `holler pane
   doctor`), exit 3 — never the generic release tail or `HELD_EXIT_CODE` 4. Omit `data.since`.
   ADR records: the carrier, the `hold_kind` vocabulary extension (`operator|default|pane`), and
   that `holler release` does NOT lift a pane-state refusal.
3. **Exit codes**: exit 3 with the stable code in the message for EVERY pane-routed refusal —
   closed (`pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found`)
   and open (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) — including the
   gate's refusal rendered CLI-side. Session-side codes unchanged (busy 1, ambiguous 2, held 4,
   invalid-grant 5, connection 1). ADR-0003's say row is amended to record the exit-3 extension.
4. **`close --spec-only` / `--profile` spec pre-check**: close.rs pre-checks `profile_store
   .get(P)` for a spec naming the pane and refuses `pane-not-in-profile` FIRST — in both the
   `--spec-only` and live `--profile` paths (`with_edit`'s `Remove` no-ops on a missing spec but
   `edit_spec` still CAS-writes: a silent generation bump). Never teach `edit_spec` to refuse
   (frozen trait, pinned conformance). Spec presence, not membership, is the key.
5. **CLI parked check**: the routing engine checks `hold == Parked` too (open code `pane-parked`,
   const in say_cmd.rs, exit 3) — the record was just read; no hub round trip to learn it.
6. **Gate matching**: match the prompted **bare session name** against each pane's
   `session_of_record` — one synchronous `PaneState::list()` read, no `await` (mirrors
   `holds.admit`'s total-ordering argument); refuse if ANY matching pane is
   parked/unhealthy/SHOWN≠DRIVEN (fail-closed across same-named sessions on different bodies).
   The pane-state predicate exists twice (CLI engine, hub gate) because `holler-pane/**` is
   frozen; keep each tiny and cross-reference the twin in both doc comments. Not a parallel path.
7. **Reconcile-step discipline** (close): the doctor step follows only failures **after the
   first live call** (`stop_owned` onwards); plan refusals (`pane-not-found`, the first-CAS
   `generation-conflict`, `--spec-only`'s `pane-not-in-profile`) carry none; never double-append
   (`edit_spec`'s own messages already carry `reconcile_step`); build via the existing
   `reconcile_step`/`doctor_command` builders and record the form in the ADR row.
8. **`close --spec-only` needs no record**: the record requirement belongs to the live close
   only; with `--spec-only` the spec-presence pre-check is the sole gate (a detached spec for a
   dead pane is exactly what it cleans up).
9. **stub.rs**: drop the `("pane", "close", 646)` row AND delete the `PANE_FORM_REFUSAL` const
   (keep the `// #646` comment line) per that file's own instructions.

## Queue — the routed `say --queue` early-return mechanism (Finding 1's remedy)

The routed (`--pane`/`--profile`) `--queue` path in `say_cmd.rs` sends the prompt with
`queue: true` and a **short fixed acceptance wait** (a named `const`, ~2 s) as the *client's*
wait — the prompt's own `timeout_ms` stays the user's `--timeout`, so the hub keeps waiting for
the turn. A deadline expiry after acceptance (control `Io` timeout at the deadline the CLI
itself chose) is reported as `queued <session>` on stdout, exit 0. Any refusal renders per
rulings 2/3. If the queued turn happens to finish inside the wait, the reply prints as today.
The deadline decision + outcome mapping is a **pure function** so T can test it in-process (the
wire wait itself is untestable without a hub, as with the rest of say). The bare
`say --queue SESSION` form is UNCHANGED (#190's wait-for-reply contract); the ADR-0021 "as
built" note records the difference.

## Acceptance criteria (from the issue; checkboxes for S)

- [ ] Park/unpark round-trip (part 1, merged #711 — S verifies presence, not re-test).
- [ ] `close` leaves no owned process (fake host's call log shows `stop_owned`; herdr fake shows
      `close`; the record is gone from the store).
- [ ] `say --pane` to a pane with SHOWN ≠ DRIVEN refuses with the named error (exit 3,
      `pane-shown-driven-mismatch`).
- [ ] `say --pane` to an unhealthy pane refuses (`pane-unhealthy`); to a parked pane
      (`pane-parked`); both CLI-side and via the hub gate.
- [ ] `say --queue` (routed form) returns without waiting for a reply (the § Queue mechanism;
      the pure decision function is tested; the acceptance outcome is `queued <session>`, 0).
- [ ] `close --profile P` removes the spec and bumps P's generation once; a failed close leaves
      P's specs unchanged; `close --spec-only` changes P and nothing live; without `--profile`
      no profile changes; `close --spec-only` on a spec-less pane refuses
      `pane-not-in-profile`.
- [ ] `say --profile P --pane X` with X outside P refuses (`pane-not-in-profile`).
- [ ] JSON passes the envelope helper (`check_envelope`) for park/unpark/close; exit codes equal
      across formats.
- [ ] The hub gate: a prompt to a parked/unhealthy/SHOWN≠DRIVEN pane's session of record is
      refused at `send_prompt` (unit tests in dispatch.rs's test module, mirroring `hold_tests`);
      a healthy unparked matching pane's prompt goes through; the refusal wire error carries
      `hold_kind: "pane"` + the pane code in `data.reason`.
- [ ] ADR-0003 rows (incl. the say-row exit-3 extension), `cli-surface.txt` lines, ADR-0021
      "as built" sections + code-table row, CHANGELOG entry; `process/stub.rs` per ruling 9.
- [ ] `cargo fmt` (new/edited files rustfmt-clean, edition 2021), clippy clean, no new `unsafe`,
      no new dependency, tests use only #638's fakes and envelope helper.

## Blast radius (the issue's, plus A-accepted readings)

Code: `crates/holler-cli/src/pane/close.rs` (the real verb; own clap `Args`: `close PANE`),
`crates/holler-cli/src/say_cmd.rs` (routing engine + queue mechanism + pane arm),
`interrupt_cmd.rs`, `answer_cmd.rs` (call the engine; interrupt gets the pane arm),
`crates/holler-hub/src/live.rs` and `crates/holler-hub/src/circuit/dispatch.rs` (the pane-state
gate only), plus the two A-accepted one-liners: `circuit.rs`'s `HoldGate` construction and
`serve.rs`'s `build_shared_state` reorder + `.with_panes`. Tests:
`crates/holler-cli/tests/pane_verbs/close.rs` (own file; may add `close/` for rig/failures like
park), `pane_verbs/target_flags.rs` (its refusal cases assert what this story replaces),
`pane_verbs/process/stub.rs` (ruling 9), plus gate unit tests inside `circuit/dispatch.rs`'s
existing test module (radius file). Docs: ADR-0003, ADR-0021, `tests/fixtures/cli-surface.txt`,
CHANGELOG.

## Design decisions (A-affirmed)

- **Close order**: with `--profile`/`--spec-only` — spec-presence pre-check (ruling 4), then
  `edit_spec(P, pane, Remove, act)` where `act` = the live close (read record `pane-not-found`
  → `stop_owned` → herdr `close` → observe) and the record removal (`delete`) follows the act;
  without `--profile` the same live close with no profile write. Failure messages per ruling 7.
  If the live close succeeded but `delete` failed (generation-conflict), the message says so
  and points at doctor.
- **Close output**: `{"pane": <name>}` data (plus the written profile's generation when P was
  written); text one line, e.g. `closed demo-c1r1` / with profile `... (removed from profile
  "demo")`. Exact wording is F/T's within the envelope contract.
- **Routing engine** (`say_cmd.rs`): `resolve_pane_target(ports, pane, profile)` →
  `PaneName::parse` → `--profile` membership via `scope.resolve` (so `profile-not-found` /
  `pane-not-in-profile` come from the helper, exactly as park) → `pane_store.get`
  (`pane-not-found`) → `session_of_record` (`session-not-found`, message names the pane) →
  parked (`pane-parked`) / health (`pane-unhealthy`, reason in the message) / SHOWN≠DRIVEN
  (`pane-shown-driven-mismatch`). Open codes as consts in `say_cmd.rs` via
  `RefusalCode::from_static` (ruling 3). interrupt/answer call the same engine then run their
  existing session flow with the resolved session. `prompt_target.rs` is NOT edited (frozen to
  #670); `route()` stays `pub` though all three verbs move off it.
- **Hub gate**: as Objective 3 + rulings 1/2/6.

## Non-goals

Mutation testing (operator instruction this run). No live fleet, running pane, real Herdr
session, or real OpenCode session; tests use #638's fakes, kit rigs and temp dirs; no pane name
that could name a real session. No migrations (n/a). No edits to frozen files (`pane/mod.rs`,
`prompt_target.rs`, `holler-pane` types, `holler-proto` code table) beyond the A-accepted
touches. Never print or commit a credential or secret.

## Handoffs (this run)

`docs/handoffs/0646-implementation/`: survey.md, brief.md (this), decisions.md (journal),
handoff-A.md, handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, handoff-S.md.
Public-repo rule: these files carry neutral placeholders only — `<primary>`, `<run-worktree>`,
`<cache>`; no host or user names, no fleet ids.

## Pipeline notes

Test-first: T authors `pane_verbs/close.rs` (+rig), updates `target_flags.rs` routing cases,
adds dispatch.rs gate unit tests, confirms RED. F implements; T confirms GREEN (suite:
`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`).
First commit lands right after T's tests are staged (operator git rule), rebase on `origin/main`
before the PR, Conventional Commit subject naming #646, `Co-Authored-By` trailer. On S PASS the
pane self-merges per the in-session rule (verify CI + gates, squash, delete branch, add the
AI-disclosure to the PR body per CONTRIBUTING if missing, close the loop on #646 — parts 2 and 3
complete the story if every AC is met).
