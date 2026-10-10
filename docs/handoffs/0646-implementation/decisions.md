# Decision journal — run 0646-implementation (issue #646, parts 2 and 3)

Append-only. Format per pipeline-conventions §1.

---

## 2026-10-10 — Phase 0/1: pre-flight, survey, brief (O)

- Pre-flight PASS 15/15 (worktree `<run-worktree>` provisioned, branch `issue-0646-implementation`,
  port 9046; rigor in-session; dual-review gates declared N/A; F on claude-cli `claude-opus-5-5`
  xhigh, all other stages passive opencode-agent on the session model).
- **Scope finding**: #646 part 1 (`pane park`/`unpark`) already merged as #711. This run owns
  part 2 (`pane close`) and part 3 (routed `say`/`interrupt`/`answer` + the `send_prompt`
  pane-state gate). Issue stays open until all parts land.
- Survey written (reuse map: `switch.rs`/`tx_switch` as close's structural analog but the engine
  stays in `close.rs` per ruling 2; `ProfileScope::edit_spec` for the I8 transaction;
  `Registry::with_holds`/`HoldGate` as the gate precedent; part 1's `park/rig.rs` as the test
  rig pattern).
- Brief written with 4 open points routed to A: (1) the two one-line radius readings
  (`circuit.rs` gate construction, `serve.rs` handle attachment) vs the CLI-side-exception
  fallback; (2) the gate's wire carrier code (proposal: `session_held` + `hold_kind: "pane"` +
  `data.reason` = the pane code, since `holler_proto::Code` is a frozen 17-variant table);
  (3) routed-refusal exit codes (proposal: ADR-0021 §9 exit 3 with the stable code in the
  message); (4) `close --spec-only` with no spec for the pane → `pane-not-in-profile`.
- Open codes proposed (ruling 3, consts in the owning files): `pane-unhealthy`,
  `pane-shown-driven-mismatch`, `pane-parked` (gate); `session-not-found` reused (closed code)
  for a pane with no session of record.
- Delegated `pipeline_advance` tool call returned empty output → shell fallback used
  (`<cache release>/runner/pipeline-cli.ts`), per the dispatch rule. Recorded as tooling
  anomaly #1 for the run report.
- D and U declared N/A (no UI surface).

## 2026-10-10 — Phase A attempt 1: BLOCK (one finding), brief amended to v2

- A BLOCKED on Finding 1: the routed `say --queue` "returns at once" AC named no mechanism —
  hub-side early return would need `talk.rs`/`control_server.rs` (outside radius). Remedy
  adopted: CLI-side short fixed acceptance wait (~2 s const), deadline-after-acceptance →
  `queued <session>` exit 0, pure decision/outcome function for T; bare `say --queue`
  unchanged.
- A's four open-point rulings ACCEPTED as bindings: (1) the two one-line radius touches
  accepted (serve.rs reorder + `.with_panes`; `HoldGate` drops `Copy`); (2) gate carrier
  `session_held` + `hold_kind: "pane"` + `data.reason` = pane code, pane arm before `is_held`
  in say/interrupt, `answer` ungated, no `data.since`, `release` does not lift it; (3) exit 3
  for every pane-routed refusal (ADR-0003 say row amended), session-side codes unchanged;
  (4) `close --spec-only`/`--profile` pre-check spec presence in P (not membership) and refuse
  `pane-not-in-profile` before `edit_spec` (its Remove no-ops but the CAS still bumps the
  generation).
- NOTE rulings 6-11 folded: CLI parked check in the engine; gate matches bare session name vs
  `session_of_record` (one sync list(), fail-closed on any match; twin predicates
  cross-referenced); reconcile-step only after the first live call, never double-appended;
  `--spec-only` needs no pane record; health read from the record; `stub.rs` also deletes
  `PANE_FORM_REFUSAL`.
- brief.md rewritten as v2; re-presenting to A (attempt 2).

## 2026-10-10 — Phase A attempts 2-3: PASS recorded; worktree build contamination found and fixed

- A round 2 PASS; but the verdict line's parenthetical (`Verdict (round 2, operative …): PASS`)
  defeated the recorder's fail-closed parser, so attempt 1 recorded BLOCK. Round 3 was a
  mechanical format fix of the two verdict lines (substance untouched); attempt 2 recorded
  PASS. Lesson for all later handoffs this run: the verdict line is exactly `**Verdict:** PASS`.
- **Odd thing #2 (bigger)**: the advance into t-red ran the suite and recorded RED [exit 101]
  on a tree with no story code — `ProfileSpec` appeared to have an `opencode_agent` field the
  sources do not have. Root cause: the pipeline's worktree `linkDirs` symlinked this worktree's
  `target/` to `<primary>/target`, which the parallel #700 run (its own symlinked, in-flight
  tree) had built an `opencode_agent`-flavored `holler-pane` rlib into; cargo judged our older
  sources fresh against those artifacts (the exact hazard `scripts/seed-target-dir.sh`'s header
  warns about). Fix: replaced the symlink with a private `target/` seeded by that script
  (workspace crates cleaned, deps reused). Baseline re-verified GREEN across the workspace.
  The t-red entry RED recorded from the contaminated run is superseded by T's own stage-run
  verdict; the shared-target setup remains for other concurrent runs (reported to the operator
  in the run report — not this run's to change).

## 2026-10-10 — Phase T-red: 33 tests authored; gate block applied by O

- T authored 27 applied tests (14 close in `close.rs` + new `close/rig.rs`; 7 routing/queue/
  pane-arm in `target_flags.rs`; 6 process-level in `process/legacy_verbs.rs` rewritten after
  ruling 9 deleted `PANE_FORM_REFUSAL`) + 6 hub gate tests it could not write: T's write scope
  (`crates/*/tests/**`, `docs/handoffs/**`) denies `src/**`, and a denial is policy. T delivered
  the gate tests verbatim in `gate-tests-dispatch.md`.
- **O applied T's block verbatim** into `circuit/dispatch.rs`'s `mod hold_tests` (three
  `panes: None` edits + the appended block) — mechanical application of T's authored artifact
  (authorship T's; the hash-pinned handoff carries the bytes), recorded here. Gate red verified:
  E0560 ×7 (no `panes` field), E0599 (`with_panes`), and T's `panic!("{other:?}")` pins
  `Debug` on `SendPromptError` (F derives it).
- Red shape across the suite: 1 CLI compile error (four missing `say_cmd` items — the
  engine/queue/pane-arm contract), 14 staged close failures (13 missing-positional parse
  contract + the stub's exit 1 vs wanted usage 2), 3 process assertion failures (the #646 stub
  still printed), hub lib compile error (the gate contract, above). Pure-signature contract for
  F (T-owned): `resolve_pane_target`, `QueueWait`/`queue_outcome`, `pane_hold_refusal`, the
  three open-code consts; hub: `HoldGate.panes`, `Registry::with_panes`/`panes()`.
- T contract decisions worth S's eye: PANE positional optional with verb-level usage;
  `--profile` without `--pane` on the prompt verbs is usage exit 2; close wording set
  (`closed <pane>` / `(removed from profile "<P>")` / `removed <pane> from profile "<P>"`);
  bare close call-set pinned to exactly `[StopOwned]`+`[Close]` (no snapshot observe — F adds
  one only as a contract conversation); not pinned: close of an already-gone Herdr pane, close
  of a parked pane.


## 2026-10-10 — Phase 4 (t-red): T authors the failing suite; one scope block

- **Decided (T, the pure-signature contract)**: `say_cmd::resolve_pane_target(ports: Ports<'_>,
  pane: &str, profile: Option<&str>) -> Result<String, PaneError>` (chain order: `PaneName::parse`
  → `scope.resolve` → `pane_store.get` → `session_of_record` → parked → health → SHOWN≠DRIVEN);
  `say_cmd::QueueWait { Reply(Value), Deadline }` + `say_cmd::queue_outcome(session, wait, json)
  -> SayResult`; `say_cmd::pane_hold_refusal(&WireError, json) -> Option<SayResult>` (the pane
  arm, before `is_held`); the three open-code consts via `RefusalCode::from_static` in
  `say_cmd.rs`.
- **Decided (T, wording within the brief's envelope contract)**: close text lines
  `closed <pane>` / `closed <pane> (removed from profile "<P>")` / `removed <pane> from profile
  "<P>"` (`--spec-only`); JSON data `{"pane"}` and `{"pane", "generation"}` when P was written;
  after-live-call failure messages end with `reconcile_step`'s exact string (built by the one
  builder, asserted by calling it), never appended twice.
- **Decided (T)**: `close`'s PANE positional follows park's pattern (optional in clap, the verb
  refuses `usage` itself) so the in-process harness can run it; per-port exact call sets pinned
  (host `[StopOwned]`, herdr `[Close]`, pane writes `[Delete]` — no `ensure_pane`, no snapshot
  observe); `--profile` without `--pane` on the prompt verbs is usage exit 2 naming `--pane`
  (ADR-0021 §3 "still need a pane name"); the gate's fail-closed refusal names the first bad
  pane in name order.
- **Assumed**: `legacy_verbs.rs` had to be rewritten with `stub.rs`'s ruling-9 edit — deleting
  `PANE_FORM_REFUSAL` removes its import, and its cases assert exactly the refusal this story
  replaces. Test-side, inside the story's blast radius; flagged to O.
- **Blocked (needs O)**: the dispatch.rs gate unit tests could not be applied — T's write scope
  (`crates/*/tests/**`, `docs/handoffs/**`) denies `crates/holler-hub/src/**`. Authored in full
  in `gate-tests-dispatch.md` for O (or F under O's instruction) to paste; without it the
  hub-gate piece of the RED contract is missing.
- **Evidence**: staged runs on the private target dir — close (routing import set aside):
  14/14 FAILED for the right reasons (13 × missing-PANE parse, 1 × stub-exit-1 assertion);
  final pane_verbs: one compile error naming the four missing `say_cmd` items; pane_cli_process:
  3 FAILED (routing replaced the stub), 33 passed; hub lib 77 passed (gate tests pending paste).
  rustfmt clean and clippy clean on every file T touched.

