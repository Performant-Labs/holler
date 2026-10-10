# Handoff-S: Phase 8 - #645 part 2, `--first` on reset and the activity refusals

**Date:** 2026-10-10
**Branch:** `issue-0645-switch-reset` (T-red at `560a6c0`, on `3d95aec`; F's implementation + T's fixture row uncommitted on top — O commits per the standing rules)
**Issue:** #645 (part 2)
**Handoff-T reviewed:** `handoff-T-red.md`, `handoff-T-green.md`
**Handoff-A reviewed:** `handoff-A.md`, `handoff-A-dup.md`
**Handoff-F reviewed:** `handoff-F.md`
**Operator-facing report:** N/A (no UI surface; non-visual audit, markdown handoff only)

## VERDICT

**PASS** — all eight brief checkboxes are met by the diff and the recorded evidence; the
operator carve-out is coherent, journaled, and nowhere silently exceeded (exactly 9 non-handoff
files, all in the amended boundary). One **merge-time action for O** (not a checkbox failure):
the known half-stale clause at `docs/adr/ADR-0021.md:534`. Nothing requires F rework.

Method note: per S's role, Tier 1/2 were not re-run; this audit walks the full run diff
(`git diff 3d95aec`) hunk by hunk against the brief, the issue, and the recorded evidence
(T-red's RED set, T-green's counts, a-dup's independent reproduction, the plugin-owned
workspace crossing). Static checks S did re-verify itself: file-set enumeration, hygiene greps,
runner-guarantee locations, ADR line state, worktree `target` symlink isolation.

## Preconditions

| Precondition | Status |
|---|---|
| A up-front | Confirmed PASS (`handoff-A.md`; W-1 applied by O before F, N-1..N-5 carried) |
| A a-dup | Confirmed PASS (`handoff-A-dup.md`; 1 warn = the :534 merge-time item, 0 blocks) |
| T green | Confirmed GREEN, zero blocking issues (`handoff-T-green.md`) |
| Browser / visual-diff | N/A — no UI surface (brief, D and U skipped) |

## Acceptance checkboxes — one by one

| # | Brief checkbox (summary) | Status | Evidence (S's own reading of the diff unless noted) |
|---|---|---|---|
| 1 | `--first` queued to the NEW session via `send_prompt` after the record write, in-transaction, no keystroke; absent → identical to part 1 | **PASS** | Engine: the prompt step is strictly after `cas_put` inside `switch()` (`tx_switch.rs:216-224`, `send_prompt(record.harness.port, &target, …)` at :218), target already the session of record. Tests: `reset_with_first_queues_the_message_to_the_new_session_only` (one prompt; new session ≠ old, ≠ Q's; P's port; `then_of_record == new` ordering pin, `reset.rs:368`; part 1's op sequence `[Health, CreateSession, SelectSession, ShownSession]`) and `reset_without_first_queues_no_prompt` (empty prompt log, part 1's op sequence + `[Get, CasPut]` + `assert_recorded`). I4 via the runner's no-Herdr/no-host assertion (`switch.rs:145-146`). The "(no port call added)" clause read as `send_prompt`-only — see Advisory 2. |
| 2 | The wrong-session incident as a test | **PASS** | The same incident case: P and Q seeded, old session present, shared data dir; exactly one prompt asserted (count + port + session + text + ordering), never the old session, never Q's; zero Herdr/host calls via the runner. |
| 3 | Prompt failure after a successful record reported honestly | **PASS** | `reset_first_prompt_failure_after_the_record_is_honest`: `failed(…, 1, "unavailable")` (exit parity both formats, the port's own stable closed code); message contains the brief's phrase "first message did not land" (`tx_switch.rs:170`), asserts NO "to reconcile" and NO "is not recorded"; prompt attempted; record stands (`assert_recorded`). Engine: third decoration `unprompted` (`tx_switch.rs:140`) set only with `acted: false, created: None` (:219-223); reconcile clause gated on `acted`, not-recorded clause on `created` — neither fires; exit stays `class_of`'s (per ADR-0021 §9). |
| 4 | Plan-phase activity refusals; no-session not refused; switch unaffected | **PASS** | `reset_refuses_a_busy_or_questioning_conversation`: table `Busy`→exit 3 `session-busy`, `HoldingQuestion`→exit 3 `session-holds-question` (consts `tx_switch.rs:69,72`); `failed_before_the_act` (exit parity); no `CreateSession`, no `CasPut`, `assert_unchanged`; exactly one query on the CURRENT session of record. Non-vacuous proof: `reset_skips_the_activity_gate_without_a_session_of_record` (wrapper answers Busy for ANY query; run succeeds; activity log EMPTY — skip by `None`, engine `tx_switch.rs:313-317`). Switch: `switch_is_unaffected_by_the_activity_seam` (busy-for-any wrapper; records target; no query, no prompt); engine `plan()` `Some` arm unchanged, `None => check_idle` (:243-247), after `refuse_orchestrator` and `check_health` — A's approved slot. |
| 5 | Permissive defaults; no existing impl changes; testkit/adapter untouched | **PASS** | `ports.rs:213-229` exactly the two default-armed methods; `PROMPT_UNSUPPORTED` const (:234); closed `Activity { Idle, Busy, HoldingQuestion }` (:238, minimal derives, no Serde, no root re-export — `lib.rs` not in the diff). `seam_defaults_are_permissive` pins default→`Idle` and default refusal code string `prompt-unsupported`. No-impl-change: all 214 pre-existing `pane_verbs` cases green inside 221; whole `holler-cli` 738/0/3 (T-green); a-dup confirmed all nine impls compile untouched; testkit/adapter files absent from the diff (S's file-set check). |
| 6 | Every new case rides the shared runner (envelope, one line, exit parity) | **PASS** (with a note) | All seven verb cases ride `both_with` (via `reset_over_seam` or directly) → `both`, which asserts exit parity across formats (`switch.rs:132`), `check_envelope` on JSON (:136), one text line, and I4 (:145). The `help_names_the_arguments` flip is parse-tier (part 1's own precedent). `seam_defaults_are_permissive` is port-level and rides no runner — deliberate, cheapest sufficient tier; see Advisory 1. |
| 7 | ADR-0003 row + cli-surface fixture row (+ W-1's ADR-0021 spots) | **PASS** | `ADR-0003.md:52` one row gains `[--first TEXT]`. Fixture: one line under `# #645` after `--as-operator`, parse-proven by `cli_surface_test` 3/0 (parses + leaf set still equals clap's; T's amended boundary after F's attempt-1 denial — journaled, not silent). ADR-0021's three W-1 spots all rewritten in the diff: §8 as-built incl. the refusals, prompt-after-record, "at most eight port calls" (:382) and the park-state sentence; the new seam paragraph (:399-410) carrying N-3's `prompt-unsupported`-vs-`not-implemented` rationale and the two 645b departures; §9 codes row as-built (:473); the deferred item decided (:686-691). |
| 8 | Hygiene: fmt-clean new content, clippy -D warnings, zero unsafe, no Cargo.toml, narrow green | **PASS** | T-green Tier 1: rustfmt `--check` clean on all six touched `.rs`; clippy exit 0 on `-p holler-cli --test pane_verbs -p holler-pane`; narrow suites 221/0 and 12/0. S re-verified statically: zero `unsafe`/`#[allow]`/`Cargo.toml` occurrences in the code diff (only handoff prose mentions them); `git diff 3d95aec --name-only` = exactly 9 non-handoff files. |

## The issue's own test-kit bullets (the spec above the brief)

- "No call types into a TUI" — I4 runner assertion, inherited by every new case. **Met.**
- "Refusals are named" — stable kebab codes (`session-busy`, `session-holds-question`,
  `prompt-unsupported`), consts per ruling 3, closed `PaneError` set untouched (`error.rs`
  absent from the diff). **Met.**
- "The first message lands in the new session and nowhere else (the incident, as a test)" —
  checkbox 2. **Met.**
- Envelope + exit parity for every refusal and success — the runner. **Met.**
- Part-1 bullets (deleted-session switch, orchestrator pane, server-unhealthy, profile scoping)
  — outside this run's delta and still green inside the 221 (survey verified them test-by-test
  at run open). **Standing.**

## The deviation record (carve-out vs issue blast radius) — judged coherent

The issue's blast radius names only `switch.rs`, `reset.rs`, `tx_switch.rs`. The run's delta
adds `ports.rs` (two default-armed seam methods + `Activity` + `PROMPT_UNSUPPORTED`), the two
ADR rows, the fixture row, and the two test files. This is exactly the operator's surgical
carve-out: **decided and journaled before F ran** (`decisions.md`, threshold escalation, #700
precedent), including the flagged-and-kept scope reading (refusals included, "one run closes
#645 complete"), reviewed by A up-front, extent verified independently by a-dup (9 files), and
**re-verified by S**: `ports.rs` holds exactly the two methods, the const, the enum, the one
import and one doc sentence — the carve-out's whole extent as amended (N-1). Nothing from the
Never list appears in the diff (testkit, adapter, `wiring.rs`, `doctor/**`, `launch/**`,
`cli.rs`/`main.rs`, `error.rs`, no `Cargo.toml`). The one mid-run boundary amendment
(fixture row F→T) has its cause (F's hard denial on `tests/**`) and recovery journaled at
~11:05 AM. **No silent exceedance anywhere.**

## Quality audit

| Area | Result | Notes |
|------|--------|-------|
| API consistency | PASS | Codes kebab-case stable; consts in owning files (ruling 3); `--first` in reset's own `Args` (ruling 2, `reset.rs:36-37`); `SwitchRequest.first` per the survey's name. |
| Error handling | PASS | Exit 1 honest failure / exit 3 refusals / exit 2 usage — all under the runner's parity; the third decoration's clause arithmetic verified in `message()` (`tx_switch.rs:161-176`). |
| UI/UX match to spec | N/A | No UI surface. |
| Accessibility | N/A | No UI surface. |
| Architecture gate | PASS | A PASS + a-dup PASS; S found no quality finding that conflicts with either. |
| Code organization | PASS | Verb-owns-its-flag split kept; engine owns the Activity→code mapping, wrapper only replays answers (a-dup item 2, confirmed in the test diff). |
| Security | PASS | Fakes + temp dirs only; no fleet/pane/session touched; no credentials in scope. |
| Performance | PASS | "At most eight port calls" (I5-bounded) stated and now true; one activity query per reset-with-record; no loops added. |
| Visual regression | N/A | No UI surface. |
| Naming consistency | PASS | `session-busy`/`session-holds-question` from ADR-0021's own 645b wording; `first`/`unprompted` match survey/A handoffs. |
| Test quality (rubric §7) | PASS | Each case names one behavior; assertions all outward-facing (codes, exits, records, logs, envelope); two mechanistically independent ordering pins (pane-store snapshot vs fake-log sequence); N-5 case non-vacuous by construction; suite proportionate (7 cases + flip for this delta) — no padding, no tautology, nothing to delete or merge. Compile-level RED honest by design (N-4), six seam-named errors recorded verbatim. |

## Scope check

F delivered exactly the contract — no over-delivery (park check and `--first ""` validation
deliberately NOT added, each recorded as #646/#649 and #642-pt-3 follow-ups in the ADR seam
paragraph and F's notes), no under-delivery (every T-red contract item has its implementation
hunk, verified hunk-by-hunk by T-green and spot-verified by S against the diff).

## The :534 residual — merge-time, not a checkbox failure

`docs/adr/ADR-0021.md:534` (§9 class table, "any other well-formed code" row) still reads
"#640's `grid-unreachable`, merged; #645's and #646's, planned". S confirms: it is genuinely
**outside** the three amended W-1 spots (:371/:382, :399-410, :473, :686-691), F correctly
left it inside the boundary discipline, and it is already flagged three times (F item 2,
T-green advisory 1, a-dup finding 1). It becomes half-stale the moment this PR merges. **For O
at merge**: the one-clause edit ("#645's, merged; #646's, planned") or an explicit decision to
leave it — do not let it reach the PR unnoted.

## Advisory notes (non-blocking)

1. **The default-refusal surface is unpinned at verb level.** `seam_defaults_are_permissive`
   pins the defaults at port level; no case runs `reset --first` through an UNWIRED port to
   assert the resulting exit-3 `prompt-unsupported` envelope end-to-end. The code string and
   the open-code→3 class are each pinned separately, so the residual risk is thin. The natural
   home for that case is #684 (the fake's seam recording), which will need exactly it.
2. **AC 1's "(no port call added)" is `send_prompt`-only** — journaled at Phase 4 (O+T):
   AC 4's gate queries activity on every reset with a session of record, `--first` or not
   (the issue's own refusal demand is unqualified by `--first`), and on unwired ports the
   query is the trait default — a pure function that never enters any port's call log, so
   outward behavior is byte-identical to part 1 (pinned by the absent-case test). Coherent,
   recorded; noted for the record, not a failure.
3. **Park state + `--first`** (a parked pane's reset still queues) and **`--first ""`** —
   recorded deliberate deferrals (ADR seam paragraph; #646/#649, #642 part 3). Agreed.
4. **Fixture flag-coverage unenforced** (header asks every flag once; no test asserts it) —
   future fixture strengthening, out of scope.

## For O before merge

1. The `ADR-0021.md:534` one-clause edit (or explicit acceptance, noted here).
2. The standing close-out: commit (implementation + fixture row + handoffs, placeholders
   already verified by a-dup), rebase on `origin/main`, then the in-session self-merge; PR CI
   on clean runners remains the final arbiter as journaled.
