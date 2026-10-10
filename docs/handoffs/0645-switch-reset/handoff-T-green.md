# Handoff-T-green: Phase 6 - #645 part 2, `--first` on reset and the activity refusals

**Date:** 2026-10-10
**Branch:** `issue-0645-switch-reset` (T-red commit `560a6c0`; F's implementation uncommitted on top — verified in place, nothing committed by T)
**Issue:** #645 (part 2)
**Handoff-F reviewed:** `docs/handoffs/0645-switch-reset/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/0645-switch-reset/handoff-T-red.md`

**VERDICT: GREEN — all 8 contract cases pass, the whole `holler-cli` crate is green, Tier 1/Tier 2 clean, the fixture row is in and parses. No blocking issues.**

## GREEN confirmation

All runs through the worktree's `target` symlink (the isolated `<cache>/tmp/opencode/0645-target`;
`CARGO_TARGET_DIR` unset — no stale shared-cache involvement), one cargo at a time.

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs` | **221 passed, 0 failed** — the 214 base (F's corrected count; my T-red 219 was the stale-cache artifact, odd thing 3) + exactly the 7 new cases. |
| same, filtered to the contract names | **8 passed, 0 failed**: `reset_with_first_queues_the_message_to_the_new_session_only`, `reset_without_first_queues_no_prompt`, `reset_first_prompt_failure_after_the_record_is_honest`, `reset_refuses_a_busy_or_questioning_conversation`, `reset_skips_the_activity_gate_without_a_session_of_record`, `switch_is_unaffected_by_the_activity_seam`, `seam_defaults_are_permissive`, `help_names_the_arguments` (flipped pin) — each `ok` by name. |
| `cargo test -p holler-pane --lib` | **12 passed, 0 failed** (matches F). |
| `cargo test -p holler-cli` (whole crate) | **exit 0**: 59 test binaries, every one `ok` — **738 passed, 0 failed, 3 ignored** (221 of them `pane_verbs`). |

**Behavior, not implementation:** mutation testing is forbidden by the brief, so the pinning
evidence is the compile-level RED (the binary could not compile without the seam; every case
faults with the seam absent) plus assertions on outward behavior only — codes, exits, records,
fake-log op sequences, prompt log, envelope. No assertion reads F's internals; the two ordering
pins are mechanistically independent (pane-store snapshot at prompt time vs fake-log op sequence).

## The fixture row (amended T boundary, this phase)

`crates/holler-cli/tests/fixtures/cli-surface.txt`, under `# #645`, after the `--as-operator` row
(one line, F's proposed argv, which F verified runs in the green incident case):

```text
pane reset | demo-c1r1 --first "ship the fix" --format=json
```

Proof it parses: `cargo test -p holler-cli --test cli_surface_test` → **3 passed, 0 failed**
(`every_surface_line_parses` — the row parses with `Cli::try_parse_from`, which also proves
`--first` is real clap surface; `fixture_leaf_set_equals_clap_leaf_set` — leaf set still equal;
`every_pending_line_does_not_parse_yet` — pending file untouched). This closes AC 7's
`cli-surface.txt` half, which F is permission-denied from writing.

## Diff spot-check (F's uncommitted `git diff` vs the T-red contract)

Every claim in handoff-F verified against the diff hunk by hunk:

- **`ports.rs` (permissive defaults, AC 5):** exactly the carve-out — `send_prompt(port, session,
  text)` default → `Refused { PROMPT_UNSUPPORTED, .. }`; `session_activity(port, session)` default
  → `Ok(Activity::Idle)`; `pub const PROMPT_UNSUPPORTED = "prompt-unsupported"`; closed
  `Activity { Idle, Busy, HoldingQuestion }`, derives `Debug, Clone, Copy, PartialEq, Eq`, no
  Serde, no root re-export. Nothing else in the file.
- **`tx_switch.rs` gate ordering (AC 4):** `plan()` = `read` → `refuse_orchestrator` →
  `check_health` → `match existing`: `Some` keeps `check_listed`/`check_unclaimed` (switch
  untouched); `None` (exactly `Target::Fresh`) runs `check_idle`. `check_idle` returns `Ok` with
  **no port call** when `session_of_record` is `None` (N-5's skip-by-`None`), else exactly one
  `session_activity` query on the current session of record; `Busy`/`HoldingQuestion` refuse with
  the consts `SESSION_BUSY`/`SESSION_HOLDS_QUESTION` (ruling 3), message names session and pane.
- **Prompt after record (AC 1, AC 2):** `send_prompt(record.harness.port, &target, text)` runs
  only after `cas_put` succeeds, inside the same `switch()` transaction — which is what my
  `then_of_record` snapshot pin and the `[Health, CreateSession, SelectSession, ShownSession]`
  op-sequence pin check from the outside.
- **N-2's third decoration (AC 3):** prompt failure → `SwitchFailure { error (the port's own),
  acted: false, created: None, unprompted: Some(target) }`; `message()` appends
  `; session "<id>" was created, shown and recorded, but the first message did not land` — the
  reconcile clause fires only on `acted` (false here) and the not-recorded clause only on
  `created` (None here). Exit stays `class_of`'s.
- **CLI plumbing:** `PaneReset.first` with `#[arg(long, value_name = "TEXT")]` (ruling 2),
  threaded `run → execute → request → SwitchRequest.first`; switch passes `None`.
- **Docs:** ADR-0003's reset row gains `[--first TEXT]` (one row); ADR-0021's three W-1 spots are
  all rewritten in the diff (§8 as-built incl. "at most eight port calls" and the park-state
  sentence; §8's new seam paragraph carrying N-3's `prompt-unsupported`-vs-`not-implemented`
  rationale and the two 645b departures; §9 codes row now as-built; deferred item decided).

**Gaps — what my cases do NOT pin:** everything the brief demands is pinned (table below). Not
pinned and not brief-demanded: park-state behavior with `--first` (F recorded the gap in the ADR
seam paragraph — #646/#649 follow-up); `--first ""` validation (F noted; #642 part 3 judges it);
the exact busy/question phrasing beyond code + pane-naming (deliberate: only the brief's own
phrases are pinned); the "eight port calls" docs claim (no count test — not demanded).

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Narrow suite 1 | `cargo test -p holler-cli --test pane_verbs` | green | 221/0 | PASS |
| Narrow suite 2 | `cargo test -p holler-pane --lib` | green | 12/0 | PASS |
| Whole crate | `cargo test -p holler-cli` | exit 0 | exit 0, 738/0/3 ignored over 59 binaries | PASS |
| Fixture test | `cargo test -p holler-cli --test cli_surface_test` | green | 3/0 | PASS |
| Clippy (touched targets) | `cargo clippy -p holler-cli --test pane_verbs -p holler-pane -- -D warnings` | exit 0 | exit 0, zero warnings | PASS |
| rustfmt (touched files) | `rustfmt --edition 2021 --check` on all six touched `.rs` (F's four + T's two) | no diffs | clean | PASS |
| Existing tests | the 214 pre-existing `pane_verbs` cases | still green | inside the 221 | PASS |
| Build isolation | worktree `target` symlink → `<cache>/tmp/opencode/0645-target`, `CARGO_TARGET_DIR` unset | isolated | confirmed before every run | PASS |

Server-start / API-smoke: N/A — this story adds a CLI verb exercised through the fake-backed
verb rig; no server or HTTP surface changed. F's self-check table re-run where it overlaps mine:
no discrepancies (their 221/12/3+3+3 all reproduced; their full-workspace run is the plugin's
pre-merge business, not re-run here).

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per AC | every brief checkbox mapped to a passing case (table below) | PASS |
| Test quality | 7 new cases, each one behavior, cheapest sufficient tier (existing rig), no duplicates (two independent ordering pins; N-5 the only non-vacuous no-session proof); proportionate — nothing redundant to prune | PASS |
| Type safety | Rust: clippy `-D warnings` clean on touched targets; no `unsafe` added anywhere in the diff (grep, zero hits) | PASS |
| Error handling | exit-1 prompt-failure path, exit-3 busy/question refusals, exit-3 `prompt-unsupported` default — all under exit-parity across formats | PASS |
| Data integrity | prompt-failure-after-record leaves record/TUI exactly as success (`assert_recorded`); plan-phase refusals write nothing (no `CreateSession`, no `CasPut`) | PASS |
| API contract | codes match ADR-0021 §9's as-built row; envelope-valid JSON (`check_envelope`) + one text line + exit parity (0/1/2/3, the epic's amendment) inherited from the runner by every new case | PASS |
| Security | I4 zero-keystroke pinned via the runner; pane/profile parsing unchanged; no credentials in scope | PASS |
| Migration safety | N/A — no schema change; the record write is part 1's four-field CasPut, unchanged in the diff | PASS |
| Dependency hygiene | no `Cargo.toml` in the diff (grep, zero hits) — no new dependencies | PASS |
| Playwright / e2e | N/A — no UI surface (D and U skipped per the brief) | PASS |

## Acceptance criteria status

| Brief AC | Status | Backed by |
|---|---|---|
| `--first` queued to the NEW session via `send_prompt` after the record, in-transaction, no keystroke; absent → part 1 identical | PASS | `reset_with_first_…`, `reset_without_first_…` (runner's I4) |
| The wrong-session incident as a test | PASS | `reset_with_first_queues_the_message_to_the_new_session_only` |
| Honest prompt-failure (stable code path, brief phrase, no reconcile, record correct) | PASS | `reset_first_prompt_failure_after_the_record_is_honest` |
| Plan-phase refusals (busy, question, no-session skip, switch unaffected) | PASS | `reset_refuses_a_busy_or_questioning_…`, `reset_skips_the_activity_gate_…`, `switch_is_unaffected_…` |
| Permissive defaults (`prompt-unsupported` / idle), no impl change | PASS | `seam_defaults_are_permissive` + whole-crate green (every impl compiles/behaves) |
| Every case on the shared runner (envelope, one line, exit parity) | PASS | `both_with`/`one_case` inheritance |
| Rows updated (ADR-0003, cli-surface.txt) | PASS | diff spot-check (ADR-0003) + the fixture row and `cli_surface_test` 3/0 (this phase) |
| rustfmt/clippy/no-unsafe/narrow-green | PASS | Tier 1 table |

## Blocking issues

None.

## Advisory notes (residuals for A-dup / S / O)

1. **ADR-0021 §9 class table, line ~534** still reads "#645's and #646's, planned" about the open
   codes. Outside W-1's three spots, so F correctly left it. One-word edit at merge
   ("#645's, merged; #646's, planned") if O wants it — verified still present in the tree.
2. **Park state + `--first`:** a parked pane's reset still queues its first message (the
   carve-out goes straight to `HarnessPort::send_prompt`, not the hub's prompt gate). Recorded in
   the ADR seam paragraph; #646/#649's follow-up, not this run's defect.
3. **`--first ""`** is not validated by the verb — F's note; the real adapter (#642 part 3) is
   where an empty prompt should be judged.
4. **Success output with `--first`** is unchanged (same text line, same data) — F's note, not
   brief-pinned; the runner's envelope/one-line checks cover the shape.
5. **Flag coverage is unenforced:** the fixture header asks for every flag at least once, but no
   test asserts it (F's item 1). My row adds `--first` coverage; the fixture test enforces parse
   + leaf-set equality only. A flag-coverage assertion would be a future fixture-test
   strengthening, out of this run's scope.
6. **Baseline count:** 214 at base (F's `git grep` evidence), not T-red's 219 — the stale-cache
   artifact already journaled as odd thing 3; my 221 − 7 new = 214 confirms F's number.
7. Nothing committed by T: the fixture row and this handoff sit uncommitted on top of F's work,
   for O to commit per the standing rules.
