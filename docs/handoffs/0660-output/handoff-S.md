# Handoff-S: spec audit — #660 conformance suite (test-only continuation)

**Date:** 2026-10-09, 11:45 PM MDT (audit concluded Oct 9 ~11:55 PM MDT)
**Branch / worktree:** `issue-0660-output` / `.claude/worktrees/0660-output` (base `519947a`, opener `2f122b9`; the run's diff is uncommitted working-tree changes)
**Issue:** #660 (epic #633, wave 3) · **Rigor:** in-session · **UI surface:** no (browser/visual-diff preconditions N/A)
**Handoff-T reviewed:** `handoff-T-green.md` (GREEN, zero blocking) — under it `handoff-T-red.md` (GREEN-on-contact)
**Handoff-A reviewed:** `handoff-A.md` (PASS, up-front) and `handoff-A-dup.md` (PASS, anti-duplication)
**Handoff-F reviewed:** `handoff-F.md` (honest no-op, operator-recorded)
**Diff audited:** `git diff 519947a` — exactly two test files, +321/−9, zero production-code change (verified first-hand: `git diff 519947a --name-only` → `crates/holler-cli/tests/pane_verbs/output_api.rs`, `crates/holler-cli/tests/pane_verbs/process/stub.rs`)

VERDICT: PASS

## Preconditions

- **A precondition: CONFIRMED.** Up-front plan review PASS (`handoff-A.md:6`); anti-duplication gate PASS (`handoff-A-dup.md:11`). A's findings 1–3 were applied, not waved: no new top-level test target (both edits are inside the declared `pane_verbs` / `pane_cli_process` targets), the expected files were held to (verified: the diff touches only the two named files), and the checker is the conformance authority with `one_envelope` untouched (`git diff 519947a` contains no `verb_harness/` hunk; the one `ALL_CODES` use of `one_envelope` was **replaced** by `check_envelope`, diff −247..−249 → +260..+266).
- **T precondition: CONFIRMED.** `handoff-T-green.md:9` VERDICT: GREEN, "Blocking issues: None" (:127-129). Narrow counts 134/0 (`pane_verbs`) and 35/0 (`pane_cli_process`) identical across T-red, F and T-green; plugin-owned workspace GREEN at the t-green crossing (below).
- **Browser / visual-diff preconditions: N/A** — no UI surface (brief:3; plugin state `uiSurface false`).

## Spec / preview sanity check

Brief checked against the repo's binding records before the diff walk: the exit-code resolution (brief:31-34) matches ADR-0021 §9 verbatim in substance (`docs/adr/ADR-0021.md:365-375`: "0 ok, 1 runtime failure, 2 usage, 3 refusal … This **amends the epic's output contract**, which said refusals exit 1") and matches landed code (`output.rs:201-202`, checker rule 13 `envelope.rs:33`/`:345`). The GridPos row-first demand matches `grid.rs:129-137` (`serialize_field` order row, col, pos). No defect in the source of truth; no ADVISORY-HOLD.

## Acceptance-checkbox walk (brief:17-29)

| # | Checkbox (brief) | Test(s) (file:line) | Assertions that satisfy it | Evidence | Verdict |
|---|---|---|---|---|---|
| 1 | Goldens — success, refusal, usage, both formats, through `check_envelope` | `golden_success_in_both_formats_through_the_checker` (`output_api.rs:499-527`); `golden_refusal_in_both_formats_through_the_checker` (:532-565); `golden_usage_error_in_both_formats_through_the_checker` (:570-593) | Each outcome has a text leg (exact raw: `"3 items\n"` :506; `error: …\n` :543, :576; empty `out`/`err` per routing) and a JSON leg through `check_envelope` at the outcome's exit (0 :520; 3 :556; 2 :582) **plus** checker-returned `data`/`error.code`/`error.message` asserts (:521-522, :557-560, :583-585) **plus** the exact compact golden line (`schema_version` first) (:523-526, :561-564, :586-592) | T-red narrow 134/0 incl. all three (:88-95); T-green re-verify 134/0; individually green in F's filtered run | **PASS** |
| 2 | NDJSON (`emit_stream`, JSON mode): each line parses on its own through `check_ndjson` | `emit_stream_in_json_mode_is_valid_ndjson_through_the_checker` (:598-616); failure leg `emit_stream_ending_in_a_refusal_is_valid_ndjson_at_exit_3` (:622-641) | 3 ok items → `check_ndjson(&out, 0)` (:609), all lines `ok` (:610), per-line `data` sequence asserted (:611-615). Failure leg: ok-then-refusal at exit 3 → `check_ndjson(&out, 3)` (:632), 2 envelopes (:633), first ok / last the failure with code+message (:634-640) — `NotLastFailure` held positively. `check_ndjson` re-checks every line alone with `check_envelope` (`envelope.rs:35-40`) — "each line parses on its own" is the checker's own semantics | Same runs as above; both tests individually green (T-red :91-92) | **PASS** |
| 3 | Forced diagnostic in JSON mode; stdout still ONE envelope (process level legitimate) | `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` (`process/stub.rs:223-243`) | Real binary (`holler()`, :230) under `--debug noisy --format=json`: exit 1 (:231); the banner `logging_started level=noisy` asserted **on stderr** (:232-235); `check_envelope(&out.stdout, out.code)` ok (:236-237) with `not-implemented` (:238-242). `noisy` is a valid level (`cli.rs:31` none\|quiet\|noisy) so dispatch is reached — the pre-`--debug bogus` refusal path (:199-215) never does. Banner source verified: `holler-proto/src/log.rs:332` prints `logging_started level={}` | T-red `pane_cli_process` 35/0 incl. this test (:99-102); T-green 35/0 | **PASS** (interpretation named below) |
| 4 | Table-driven exit-code parity both formats; `ALL_CODES` extended with checker assertions | `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats` (extended in place, :234-268) | Iterates `ALL_CODES` (:235), expected `class_of(code).exit_code()` (:236); text = json = expected (:253-255); JSON mode writes nothing to err (:256-259); **the extension**: old `one_envelope` + 3 shape asserts (diff −247..−249) replaced by `check_envelope(&out, json)` (:260-261) + the envelope carries the code (:262-266). Rule 13 re-derives class↔exit (`envelope.rs:33`, `:345`) — the checker adds framing, exact key set, `schema_version`==1, one-line message, class-vs-exit | Green in every narrow run (T-red :95, F, T-green) | **PASS** |
| 5 | `GridPos` golden `{"row":R,"col":C,"pos":"rRcC"}`, row first | `an_envelope_carrying_a_grid_pos_serializes_row_col_pos_row_first` (:648-675) | `emit(Format::Json, Ok(GridPos{row:2,col:3}))` at exit 0 (:657); `check_envelope` (:660); `data == {"row":2,"col":3,"pos":"r2c3"}` (:661); exact raw line (:662-665); **byte-position asserts** `row < col < pos` (:668-674) — order cannot pass by coincidence. Matches `grid.rs:129-137` (epic decision 7, #637) | Green in every narrow run | **PASS** |
| 6 | Compile pins: `emit`/`emit_stream`/`emit_usage_error` + fixed types | `the_fixed_signatures_and_types_compile_unchanged` (:697-753) + aliases `EmitU8` (:681), `EmitStreamU8` (:685-686), `EmitUsageError` (:689) | Aliases restate the declared signatures (`output.rs:207-212`, :225-230, :247-249) at concrete types; binding + one call each (:698-717) proves they are the module's fns — any parameter/return/bound change breaks the build. Fixed types used as declared: `Format` two variants + **`Copy` pinned by a real move-and-use** (:723-726, the t-green repair — `let again = resolved;` + read of `resolved` is E0382 without `Copy`); `Envelope` four members, `success`/`failure`, `SCHEMA_VERSION` (:729-743); `ErrorBody` two members, `ErrorCode::new`/`as_str` (:747-751); `GridPos` public fields + `rRcC` display (:752) | Green post-repair (T-green 134/0); repair proven on scratch files (decisions ~11:30 PM) | **PASS** |
| 7 | fmt clean, clippy clean, no new `unsafe`, full CI command green | The two touched files | **fmt:** verified first-hand — `rustfmt --check --edition 2021` exit 0 on both, under this machine's rustfmt 1.9.0-stable (the version the journal names). **no new unsafe:** verified first-hand — 0 added `unsafe` lines in `git diff 519947a` (and the diff is test-only). **clippy:** triple-recorded zero-warning on both targets (T-red :104-107, F :47-48, T-green :109). **full CI:** the plugin-owned workspace run (`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`) measured **GREEN, exit 0** at the t-green crossing (state update 2026-10-10T05:24:10.610Z = 11:24 PM MDT, rework 1/5); F's full-crate `cargo test -p holler-cli` 619/0 included both flaky targets | Plugin state (read this audit); F self-check; T-green Tier 2 | **PASS for this diff's scope** — external pre-existing conditions stated below, not buried |

## The AC7 judgment, stated precisely

Two conditions sit outside this diff while touching the checkbox's letter:

1. **Crate-wide fmt drift.** `cargo fmt --check -p holler-cli` reports ~2,916 hunks on the **committed** tree under this machine's rustfmt 1.9.0 (O-confirmed, decisions ~10:45 PM). Corroborated this audit: untouched committed files are fmt-dirty under 1.9.0 (e.g. `src/cli.rs` → exit 1) while both files this diff touches are clean — the drift predates the branch point and cannot have come from this diff. A repo-wide fmt gate is red today **independent of this story**; reconciling it belongs to whoever owns repo hygiene (parallel c3r1 / operator), and must happen before any fmt-gated CI on this branch.
2. **Workspace-suite flakes under load.** Two plugin-owned workspace REDs, both diagnosed and both external: `remote_admin_test` at the t-red crossing (state 2026-10-10T05:03:41Z = 11:03 PM MDT; non-reproducing — 21/21 in isolation and in sequence after `pane_cli_process`) and `body_run_test` at the first t-green attempt (05:18:56Z = 11:18 PM MDT; 10/10 in isolation 2 minutes later; machine load 31+ with 6 sibling cargos). Zero production code changed since `519947a`; both targets pass on the clean crossings — the t-green crossing GREEN (11:24 PM) is the recorded measurement, and F's 619/0 full-crate run included both targets.

**Judgment:** within this diff's scope — the two files it touches, which are the only things it can make (un)clean — the checkbox is satisfied: both files fmt-clean, clippy-clean, zero added `unsafe`, and the full CI command measured GREEN at the plugin-owned crossing. The two conditions above are pre-existing/external, disclosed in the record since T-red, and correctly not waived silently.

## Exit-code resolution honored (brief:31-34)

The brief records ADR-0021 §9 as the winner over the issue's older "1 refused or failed" line, with **parity between formats** as the tested property. What the tests actually assert:

- **Parity, per code, both formats:** `output_api.rs:253-255` (text = json = `class_of(code).exit_code()` over all of `ALL_CODES`).
- **0/1/2/3 itself:** checker rule 13 re-derives `class_of(code).exit_code() == exit` on every JSON leg (`envelope.rs:345`); the goldens pin 0/3/2 at their outcomes (:505/:517, :541/:553, :574/:579); the pre-existing plain-numbers test `a_refusal_exits_3_and_a_failure_exits_1_in_both_formats` (:273-283) pins 3/1/2 independently of `class_of`, unchanged by this diff.

No test asserts the issue's superseded "refusal exits 1" wording. Honored.

## Deviations and interpretations (named, none blocking)

1. **Test-only continuation / F no-op (the run's one deviation).** The brief's RED policy (brief:36-41) says a green-on-contact suite stops the run for the operator. The record shows exactly that: T's GREEN-on-contact verdict with evidence (`handoff-T-red.md:7`, counts :76-102); the stop and the three options (a) test-only continuation (b) extend the brief (c) end the run (decisions ~10:45 PM); the operator's choice of (a) standing on the green-on-contact fact (~11:05 PM); F executing a no-op contract with zero `src/**` change (verified by T-green and by this audit). **The record is coherent, not convenient:** the inconvenient facts are all disclosed in it — the t-red crossing's plugin-owned RED (11:03 PM, the `remote_admin_test` flake, non-reproducing, outside F's boundary and outside this story's contract), the second flake at 11:18 PM, and the note that no waiver was consumed. The timeline also checks out: operator decision journaled ~11:05 PM, implement stage started 11:06 PM (state 05:06:38Z), t-green stage 11:29 PM, a-dup 11:34 PM. The green-on-contact fact the continuation stands on is quadruple-evidenced (T-red, F, T-green narrow runs; plugin workspace GREEN at the crossing).
2. **`--debug noisy` as the reading of "diagnostic" (brief:21-22).** The brief's own parenthetical — "banner/diagnostic on stderr, envelope intact on stdout — process level is legitimate" — equates the banner with the diagnostic, so the reading follows the contract's letter rather than inventing one. It is nonetheless recorded as an assumption in three places (decisions ~10:50 PM; the test's doc comment `stub.rs:217-221`; F note 3) and is the one interpretation the operator may still want to confirm before merge. Named here for that purpose; it does not block — the contract text answers it.
3. **The STUBS-riding diagnostic test** (`stub.rs:226-229`) rides the first pane stub in `STUBS` and self-expires when the last pane stub goes live — hedged in the `.expect` message itself (:229) and in the journal. Advisory, ages with the stub table.
4. **T-red's two production advisories** (blank-message `one_line()` edge; zero-item `emit_stream` → `EmptyStream`) remain deliberately unimplemented and unpinned — outside #660's acceptance bullets; the operator's option (b) was not taken. Correctly left alone; a speculative production edit would have violated the run's own logic.

## Boundaries (brief:43-50)

**HELD — verified first-hand.** `git diff 519947a --name-only` → exactly the two permitted files, both in the brief's expected set (`pane_verbs/output_api.rs`, `pane_verbs/process/*`). No `Cargo.toml`, no `src/**`, no `holler-pane/**`, no `holler-pane-testkit/**`, no `cli.rs`/`main.rs`/verb files, no `verb_harness/` change, no new files (so no new top-level target under `autotests=false`). The only untracked addition is this handoffs directory. Zero added `unsafe`. Nothing committed (all edits left in the working tree as instructed).

## Quality audit

| Area | Result | Notes |
|------|--------|-------|
| API consistency | N/A | Test-only diff; no API surface added or changed |
| Error handling | PASS | Refusal, usage-error, stream-failure, diagnostic paths all pinned (goldens, NDJSON failure leg, forced diagnostic, `ALL_CODES`) |
| UI/UX match to spec | N/A | No UI surface |
| Accessibility | N/A | No UI surface |
| Architecture gate | PASS | A PASS (up-front + a-dup); findings 1-3 verifiably applied; no conflict between this audit and A's verdict |
| Code organization | PASS | The #660 tail is self-documenting (module doc :11-14, section banner :488-494, per-test AC-numbered doc comments). Advisory: 753 lines, over the 600-warn / under the 900-fail gate — the conformance tail is the blessed submodule split when next touched |
| Security | N/A | No secrets, no inputs, no fleet/pane/Herdr contact; nothing printed or committed |
| Performance | N/A | Test-only; no production path touched |
| Visual regression | N/A | No UI surface |
| Naming consistency | PASS | Test names name behaviors; `AC n` doc tags map one-to-one to the brief's checkboxes; terminology matches the brief and ADR-0021 (envelope, NDJSON, refusal/failure/usage) |
| Test quality (rubric §7) | PASS | See below |

**Test-quality detail:** every new test names one behavior and asserts **behavior, not implementation** (output bytes + exit codes + checker verdicts; the `|_| panic!("JSON mode never renders text")` renderers pin "the renderer is never called in JSON mode" — a real property at zero cost). Tiers are cheapest-sufficient: unit tier with an in-process `Sink` for everything the module can prove alone; process tier only for the banner test, which only the real binary can produce (and the brief itself blesses that tier). Each test fails in isolation for the right reason (exact raw goldens fault on any framing/order/key change; checker calls fault on any of the 13 rules; compile pins break the build). Suite is proportionate: 8 new + 1 extended against seven checkboxes, no padding, no assertion-free/tautological/mock-shaped/snapshot tests. **No "delete or merge" findings** — the checker + raw-golden pairing is complementary, not duplicate-signal: the raw assertions land exactly on the two dimensions the checker deliberately does not pin (key order, compactness — `envelope.rs:21-23`), as A-dup verified. The one weak pin F found (the `Copy` claim) was repaired at t-green into a genuine move-and-use pin (:723-726) — verified present in the diff.

## Scope check

Delivered: the conformance layer the brief's objective names — checker wiring (7 `check_envelope`/`check_ndjson` call sites in `output_api.rs` + 1 in `stub.rs`), the three outcome goldens, the NDJSON legs, the forced-diagnostic process test, the `ALL_CODES` checker extension, the GridPos row-first golden, the signature/type compile pins. Not delivered, by recorded operator decision: any production change (there was no RED to fix — option (a) continuation). No over-delivery (no file outside the two permitted); no under-delivery (every checkbox has a passing test).

## Verdict

**PASS** — all seven acceptance checkboxes are satisfied by tests that are green on quadruple-recorded evidence; the exit-code resolution is honored as recorded (ADR-0021 §9, parity tested); the run's one deviation (test-only, F no-op) is the operator-recorded outcome of the brief's own green-on-contact policy and the record around it is coherent and complete; boundaries held. Ready for O to commit.

**For O to decide before merge (none blocking):**

1. Confirm the `--debug noisy` reading of "diagnostic" (brief:21-22's own parenthetical already equates banner/diagnostic — a yes is a formality, but it is the run's one standing assumption).
2. AC7's external residue: who reconciles the pre-existing ~2,916-hunk crate-wide fmt drift (red under rustfmt 1.9.0 today, independent of this branch), and whether the workspace-suite flake-under-load gets its own story (two diagnosed episodes, both non-reproducing in isolation).
3. The standing advisories: `output_api.rs` at 753 lines (split the #660 tail when next touched); the STUBS-riding diagnostic test expires with the last pane stub; T-red's two production advisories remain open for a future brief (option (b)).

## Evidence list

1. `git diff 519947a --stat` / `--name-only` — exactly two test files, +321/−9; 0 added `unsafe` lines (grep on the diff); no `src/**` hunk. Run by S this audit.
2. `rustfmt --check --edition 2021` on `output_api.rs` and `process/stub.rs` — exit 0 both (S, rustfmt 1.9.0-stable 48a229ceae, matching the journal's version); untouched `src/cli.rs` → exit 1 (drift corroborated as pre-existing/external).
3. Plugin state (read via the release CLI this audit): phase spec-audit; pre-flight pass; t-red crossing RED 11:03 PM MDT (`remote_admin_test` episode); t-green RED 11:18 PM then **GREEN 11:24 PM MDT** (workspace command, exit 0, rework 1/5); stage attempts recorded with hashed artifacts (A PASS, t-red, implement, t-green, a-dup PASS).
4. T-red narrow evidence: `pane_verbs` 134/0, `pane_cli_process` 35/0, per-test green list (`handoff-T-red.md:76-102`); F self-check adds full-crate 619/0 incl. both flaky targets (`handoff-F.md:31-52`); T-green re-verify 134/0 + 35/0 post-repair (`handoff-T-green.md:16-30`).
5. Source-of-truth verifications by S: checker API and 13 rules incl. rule 13 (`holler-pane-testkit/src/envelope.rs:12-47`, `:202`, `:241`, `:345`); `output.rs` signatures `:207-212`/`:225-230`/`:247-249`, `Format: Copy` `:41`, `Envelope`/`ErrorBody`/`Sink` public shapes `:154-188`, `SCHEMA_VERSION` `:34`; GridPos serde order `grid.rs:129-137`; ADR-0021 §9 exit codes and the "amends the epic" clause `docs/adr/ADR-0021.md:365-375`; `--debug` levels `cli.rs:31`, banner emission `main.rs:211`, banner format `holler-proto/src/log.rs:332`.
6. Hygiene/environment note: one sibling `cargo test --workspace` was mid-run during this audit (load ~4.5, down from the journal's 31+); per the one-cargo-at-a-time rule S ran no cargo, relying on the recorded and plugin-owned evidence above — which is corroborated across four independent recorders.

## Advisory notes (non-blocking)

- When the #660 conformance tail next grows, split it into the `pane_verbs/` submodule A blessed (finding 1) — the 600-line warn is already crossed.
- The four pre-existing `one_envelope` call sites (`output_api.rs:139, :172, :326, :425`) can migrate to the checker whenever a future story touches them (A-dup finding 2) — no action for #660.
- If `pane watch`'s JSON mode ever goes through `check_ndjson`, its nothing-owed silence will meet `EmptyStream` — that contract collision belongs to watch's story (T-red advisory, seconded by F and T-green).
