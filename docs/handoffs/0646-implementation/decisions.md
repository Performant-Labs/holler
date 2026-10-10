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

## 2026-10-10 — Phase F: accepted on attempt 3 (attempts 1-2 refused mechanically)

- **Attempt 1** (22.2 min, 162 turns): full implementation landed in-radius (8 code files + 2
  ADRs + handoff-F.md) but refused on the write-boundary check — every offender was `target/**`.
  Root cause: the plugin symlinks `target/` out of the worktree precisely so cargo's writes stay
  outside the boundary watcher; this run's de-contaminated *private in-worktree* `target/`
  made cargo's 18k build writes look like F's out-of-scope writes. Fixed by moving the private
  target dir to `<cache>/646-target` and symlinking `target -> <cache>/646-target` (keeps the
  de-contamination AND the boundary contract).
- **Attempt 2** (6.7 min): refused `git-metadata` / category `runner-input` ("git metadata
  changed", offender list not computed). No commit was made in the window; HEAD never moved;
  operator instructed attempt 3.
- **Attempt 3** (10.05 min, 46 turns): **STAGE OK**, 1 artifact (handoff-F.md) accepted.
- F's substance (handoff-F.md is the authority): the hub gate (`HoldGate.panes`,
  `pane_state_refusal`, `pane_gate` — one sync `list()`, first-bad-pane-in-name-order,
  `SessionHeld` + `hold_kind:"pane"`, no `data.since`, after `holds.admit`), `Registry::
  with_panes`, the two binding-1 one-liners (circuit.rs 898 lines — under the gate), the CLI
  routing engine (`resolve_pane_target`, `route_target`, three open-code consts, 2 s
  `QUEUE_ACCEPT_WAIT` on `ControlCall.timeout` only), close as a real verb with the
  edit_spec-precheck transaction, ADR-0003/0021 rows drafted. clippy clean (lib+bins);
  file gates pass; no unsafe, no new deps.
- **F found four defects in T's red suite** (3 compile classes in `target_flags.rs`, 1
  self-contradicting `assert_untouched` in `close.rs:232`) + 1 clippy lint in the applied gate
  tests + the denied-by-scope fixture/CHANGELOG edits (drafted in the handoff for T/O). F
  verified everything else green in a throwaway copy OUTSIDE the worktree (233/234 pane_verbs,
  all other binaries green) without ever touching the run's test files — the right call under
  the write scope. F's one contract deviation: `pane_hold_refusal(session, e, json)` takes the
  session first, matching `hold_cmd::held_refusal`, because T's own JSON expectation needs
  `session` in the object — T rules on it at green.
- F's flagged staleness (cli.rs help strings, prompt_target.rs module doc saying "until #646")
  is in files frozen to #637/#670 — correctly left; journal as follow-up story material, and S
  should expect the ADR text to carry the truth instead.

## 2026-10-10 — Phase T-green: GREEN (plugin-owned), repairs by T, two O items applied

- The advance to t-green ran the suite plugin-side: RED exit 101 — the four test-side defects
  F had flagged, exactly as its handoff predicted (the plugin's boilerplate said "dispatch F",
  but `tests/**` is denied to F; the repair was T's by role scope — journal records why T).
- T accepted F's 3-arg `pane_hold_refusal(session, e, json)` (T's own pinned JSON needs the
  session; mirrors `hold_cmd::held_refusal` per binding 2), fixed the three compile classes,
  resolved the `close.rs:232` self-contradiction (explicit no-pane-store-write assert; "nothing
  live" still pinned by the exact-empty call asserts), added the four `# #646` cli-surface
  fixture lines, fixed two lints in its own file.
- O applied the two write-scope-denied edits verbatim: `clippy::unreachable` added to the
  hold_tests `#[allow]` (#442, #646) — F/T's exact diff — and F's CHANGELOG entries for parts 2
  and 3 after the park/unpark entry.
- Verification: T ran the workspace suite (1779 passed / 0 failed, exit 0) and clippy on its
  files; O's allow-line application verified `cargo clippy --workspace --all-targets` exit 0;
  the plugin's own advance verdict: **GREEN [exit 0]** (206 s). t-green stage recorded (STAGE OK,
  1 artifact).
- Committed `ff32f8d` (feat(cli): #646 pane close + routed prompt verbs; 17 files, +1327/−69;
  gitleaks + scrub grep clean).

## 2026-10-10 — Phase A-dup: PASS (0 blocks, 4 advisories); U N/A

- A re-reviewed the full run diff against the reuse map: drift PASS on all 11 bindings (the two
  bound one-liners are the only out-of-radius code; frozen files zero-diff); duplication check
  clean — F extended (close engine over `edit_spec` verbatim, routing over frozen
  `prompt_target`, `with_panes` mirroring `with_holds`), never paralleled. Queue verified at
  wire level (`ControlCall::say_with` puts `--timeout` into `timeout_ms`; the 2 s wait replaces
  only `call.timeout`).
- Advisories (recorded, none blocking): legacy_verbs.rs out-of-radius rewrite doubly forced by
  ruling 9; `SendPromptError` Debug derive (test-enabling); frozen-file staleness (cli.rs help,
  `route()` now production-unused) left for #670 follow-up; commit journals before merge.

## 2026-10-10 — Phase S: PASS (3 advisories); decision gate -> merge

- S audited the story against the issue + brief v2 at source level (not T's table alone): every
  AC backed by a named passing test or verified in committed docs; the load-bearing behaviors
  re-verified in the diff (gate fail-closed order after `holds.admit`, exit 3 + stable codes,
  `queued <session>` exit 0, close transaction rollback with reconcile step, pre-check before
  writes, `--spec-only` inert); all five deviations recorded with owners; scope exact; 0 new
  unsafe/deps/debug prints.
- Advisories: commit journals before merge (doing now); #670 staleness follow-up (recorded);
  F-decision-3 interrupt-cancel edge unpinned — note for when `interrupt.rs` reopens.
- Both gates PASS -> S decision gate: commit, PR, record prNumber, CI + pr-agent, then merge.


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

## 2026-10-10 — Phase T-green: the four test defects repaired; suite GREEN

- **Decided (T, ruling on F's contract change — defect 3)**: ACCEPT
  `pane_hold_refusal(session: &str, e: &WireError, json: bool)`. T's own pinned JSON
  (`{"error":"session_held","session":"<s>","reason":"<code>","hold_kind":"pane"}`) requires the
  session in the object; `WireError`/`ErrorData` carries none (only `hold_kind`, `reason`,
  `since`), so only the caller can supply it. The 3-arg shape also mirrors
  `hold_cmd::held_refusal(session, e, json)` — the held form binding 2 says to mirror. Rejecting
  would mean weakening the pinned JSON (dropping `session`) and diverging from the held form.
  Session `"ses-demo-c1r1"` passed at the four call sites.
- **Decided (T, defect 4)**: `close_spec_only_…_touches_nothing_live` now pins "no pane-store
  write" instead of `assert_untouched` (zero writes everywhere). The original contradicted the
  test's own generation-2 pin two lines above: a spec removal IS one profile `CasPut` (the log
  is `[Get, Get, CasPut]`), and `ProfileStoreOp` has no other write op — no production code
  could ever satisfy both assertions. "Nothing live" stays pinned by the four exact-empty
  asserts (herdr/host/harness/probes) plus the record-stays assert.
- **Decided (T, defects 1/2)**: mechanical compile repairs — `*code` at target_flags.rs:471
  (`&str` vs the loop's `&&str`), `.unwrap_err()` at 493/502/511 (the cases expect usage /
  `pane-not-in-profile` / `session-not-found` refusals; `.unwrap()` on the `Ok` `String` both
  fails to compile and contradicts each case's own code assertion). Also applied F's
  `option_map_unit_fn` rewrite (`.map(|d| …)` → `if let`) in the fall-through test — two
  CI-gate clippy lints in T's file.
- **Decided (T, fixture)**: the four `# #646` cli-surface lines added (close with the PANE
  positional ×3, `say --pane --profile … --queue`), grouped verb-wise; `cli_surface_test` 3/3.
- **Blocked (needs O, one line)**: the `clippy::unreachable` deny hit by T's gate block at
  `dispatch.rs:650` (`let … else { unreachable!() }`) — the fix is adding `clippy::unreachable`
  to the module's existing `#[allow]` line (drafted in handoff-T-green.md); T's write scope
  denies `src/**`. CI's clippy gate (`--all-targets -D warnings`) fails on exactly this one
  error until it lands; the suite is unaffected. Also O's: the CHANGELOG entry F drafted.
- **Evidence**: `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`
  → exit 0, 140 binaries, 1779 passed / 0 failed (identical to F's scratch copy).
  pane_verbs 234, pane_cli_process 36, hub lib 83 (gate tests in), cli_surface 3.
  `cargo clippy --workspace --all-targets` → exactly ONE error, the `unreachable!` above;
  `cargo clippy -p holler-cli --all-targets` (T's files) → clean. `scripts/lint.sh` → exit 0.
  rustfmt --check --edition 2021 clean on both Rust files T touched.


## Chain Summary (run closed as merged, PR #724, 2026-10-10)

**Story:** #646 parts 2 and 3 (`pane close`; routed `say`/`interrupt`/`answer` + the
`send_prompt` pane gate). Part 1 (park/unpark) had merged as #711. Run: `0646-implementation`,
rigor in-session, no UI surface (D/U N/A, declared). Merged as `3c3b5cf` (12:54 MDT) after
CI green and pr-agent 92 / no flags; issue auto-closed.

**The chain that held:** survey → brief v2 (A BLOCK round 1 → all 11 bindings folded) →
T-red (33 tests; the hub gate tests applied by O verbatim after T's write scope denied
`src/**`) → F (Opus 5.5 xhigh) → T-green (T repaired its own four defects, accepted F's
3-arg `pane_hold_refusal`) → A-dup PASS → S PASS → merge. The suite's authority was the
plugin's own verdicts: RED from T's tests, GREEN [exit 0], merge verification GREEN.

**What the outside caught that the room missed:** pr-agent's spec-drift Major was real — the
pane hold is a third `hold_kind` on the wire and `docs/protocol/v2.md` §8/§10 still said
operator/default. Fixed in `f38682d`; its follow-up (`42a70c1`) repaired the two inline
`holler release` mentions the docs command extractor (correctly) tried to parse. Score
80-with-flags → 92-clean. The other Major (the gate's `list()` error path returning Ok)
was F's recorded decision 1, mirrors `holds.rs`, was accepted by A-dup and S, and stands —
answered on the PR with the record.

**Operational lessons (for future runs, all journal'd in place):**
1. The plugin symlinks `target/` out of the worktree on purpose: build writes then stay
   outside the F write-boundary. This run's (necessary) de-contamination put a private
   in-worktree `target/` there instead and F's 22-minute attempt 1 was refused on 18k
   `target/**` "violations". The fix: private target dir *outside* the worktree, symlinked
   back. Attempt 2 then hit a one-off `git-metadata` refusal; attempt 3 recorded clean.
2. The delegated (dir-routed) pipeline tool calls answer empty from this seat — the shell
   CLI (`pipeline-cli.ts`) is the reliable path, as AGENTS.md says.
3. Verdict lines must be exactly `**Verdict:** PASS`; A's round-2 parenthetical broke the
   fail-closed parser and recorded BLOCK until a mechanical round 3.
4. When the plugin's RED boilerplate says "dispatch F", read whose fault the red actually
   is: four of the five red causes here were T-side (compile errors + one self-contradicting
   assertion) and the repair belonged to T.

**Follow-ups recorded (not this run's to do):** `cli.rs` help strings and
`prompt_target.rs`'s module doc still say "until #646" (files frozen to #637/#670 — the
follow-up story should carry them); the interrupt-cancel edge under a gated session
(F decision 3) is unpinned and deserves a case when `interrupt.rs` reopens; routed forms
still answer `not implemented` until #649 wires the stores into the binary.

**Stage wall times:** survey+brief+A ≈ 75 min (A both rounds) · t-red ≈ 41 min (T) ·
F ≈ 22.2 + 6.7 + 10.0 min (attempts 1-3; 1 and 2 refused mechanically, no re-work) ·
T-green ≈ 26 min (T) · A-dup ≈ 11 min · S ≈ 14 min · suite verdicts ≈ 3.3 min each
(t-green advance 206 s, merge verification 198 s).
