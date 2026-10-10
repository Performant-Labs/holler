# Handoff-A: Phase 3 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (up-front plan review, second pass)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `d7e0421`; base `3bdd129` = origin/main, unchanged since the first pass)
**Brief reviewed:** `docs/handoffs/663-brief.md` (as of `d7e0421`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" table under "Files" (this run has no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with no block and six warns. This pass replaces the first one, a BLOCK at `5284f91`.

The first pass's block (B-1) is resolved. AC 14 amends ADR-0021 in place at the five places it named, and each quoted place and
line range matches the ADR at `3bdd129`. Four of the first pass's six warns were taken as asked (W-1 to W-4), W-6 was taken,
and W-5 stays accepted. The code plan still extends the right objects:

- `StoreScope` implements the frozen trait in the file ADR-0021 section 5 names.
- `run_probe` fills the stub with std only.
- The suite and the fixtures are reused unchanged.

The new warns are mostly about what the ADR text will say: the "may have landed" rule (W-7), the bound section 12 restates
(W-8), and the record step inside the act (W-9). The other three are a platform guard (W-10), test placement and two near-copies
that this review pre-rules for Phase 7 (W-11), and gaps in the follow-ups (W-12). F can take most of them inside the existing
ACs.

## Findings

Numbering continues from the first pass, whose findings B-1 and W-1 to W-6 are dispositioned in the next section.

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-7 | warn | Decision 5 ("still holds the edit"), Decision 6 ("`unavailable` and the rest: the store said no"), AC 14c, AC 14d | cross-cutting (the transaction's failure semantics) | **The plan decides "may have landed" by error code, but the condition is the outcome.**<br>(1) Decision 6 says a timed-out first write "may still have been applied". Yet Decision 5's message for a timed-out *restore* (the first case of AC 2) says P "still holds the edit", as a fact. By Decision 6's own reasoning, that restore may have landed.<br>(2) Both decisions treat `unavailable` as a definite "no". That is true of the hub's own answer (ADR section 7: a failed save "changes nothing ... and the call answers `unavailable`"). It is not true of the client side:<br>- ADR section 6 (lines 214-215) and `error.rs:599-611` turn a garbled reply into `unavailable`.<br>- ADR section 9's row (line 396) says "also a garbled reply".<br>- The CLI's control path folds "a dropped socket" into `RemoteUnavailable` (`transport.rs:69-78`).<br>#649 builds the real store client, and it will follow that precedent unless told otherwise. A first write that landed and then lost its reply would return a bare `unavailable`. Decision 6 passes it through, and #644 adds the step only when `acted` (its brief, line 1705), so no step is printed. P then holds an edit that nothing live matches, silently.<br>**Why a warn:** a rerun heals it, and nothing live moved. But AC 14c would write the by-code rule into the standing spec. | (a) **Decision 5:** for `timeout`, word the clause "may still hold the edit". AC 2's substrings are unaffected.<br>(b) **AC 14c:** state the rule by outcome: "a first write whose outcome is unknown (a `timeout`) may have landed ...". Add that the store client (#649) answers `timeout`, not `unavailable`, when it loses the reply to a write it sent. That makes it a stated requirement on #649.<br>(c) **AC 14d:** for a `timeout`, "the unrestored edit" becomes "the edit, which may not have been restored". |
| W-8 | warn | AC 14b, AC 14e | ADRs (completeness) | **Two gaps in the five edits.**<br>(1) Section 12 restates the bound AC 14b narrows: "every port call is bounded by I5 (default 10 s) or ends in `timeout`" (ADR line 458). `ProfileScope` and `Prober` are ports in section 2's table (lines 94, 98). After this change, section 2 would bound `edit_spec` by a sum and `run_probe` by the deadline plus 1 s, while section 12 still says 10 s.<br>(2) AC 14e's "the scope's own errors carry the step" does not say which errors. Under Decisions 5-7, three carry it: the restore failure, the restore conflict and the first-write `timeout`. The plan-stage refusals and a first-write `generation-conflict` do not. #644's append rule depends on that list being exact: append only when `acted` and the text lacks the step (its brief, lines 1705-1708, which already names the same three). | (1) Have AC 14b's sentence say that it also qualifies section 12's per-call bound, as a cross-reference inside the section 2 sentence. Do not edit section 12: #644 (a PROPOSED note) and #647 ("Decided (#647)") both edit it, and a section 12 hunk is not one of a-e.<br>(2) Have AC 14e name the three errors and Decisions 5-7. |
| W-9 | warn | Decision 4 ("the scope writes no pane record (recording is the verb's, inside its act)"); AC 14e (the record-fence bullet, ADR lines 273-275) | ADRs; cross-cutting | **Confirming this ASSUMPTION has a consequence ADR section 8 never states.**<br>- The record step (step 4) runs inside the act. So a pane-record conflict is an act failure, and the scope restores P's specs (step 5) after a live change that worked.<br>- #644 composes its act exactly so. Its act table puts `pane_store.cas_put` inside the act, as row R, with "no rollback" (brief at `7195993`, line 1622). Line 1627 adds: "Then `edit_spec` restores P's specs".<br>- The fence bullet AC 14e edits says the verb "writes nothing more" (line 274), and #644 cites those words. With `--profile`, the restore is one more write.<br>- Decisions 5 and 7 then say "the live change failed (<failure>)" when only the record failed. The scope cannot see the difference, and the codes are right.<br>**Why a warn:** the gap predates this story. #697 made the ASSUMPTION, and #663 confirms it. | In AC 14e's edit of the fence bullet (the same hunk), add one clause: with `--profile` the record step runs inside the act, so a record conflict also restores P's specs (step 5), and the verb prints the reconcile step. Cite it `(#663)`, confirming #638's ASSUMPTION.<br>Optionally, word Decisions 5 and 7 as "after the act failed (<failure>)". AC 2 and AC 3 assert neither phrase. |
| W-10 | warn | Decision 13 ("Unix only, as the rest of the workspace (`holler-hub` uses `std::os::unix` unconditionally)") | pattern consistency (platform) | **The case Decision 13 cites is the exception.**<br>- In production `src`, `std::os::unix` is guarded with `#[cfg(unix)]` at seven sites: `holler-body` `instance_lock.rs:84` and `x25519_identity.rs:206`; `holler-hub` `identity.rs:196` and `serve.rs:247, 448`; `holler-proto` `atomic_file.rs:135, 156`.<br>- It is unguarded at one site: `holler-hub/src/control.rs:8`.<br>- Both existing `process_group(0)` calls guard it: `holler-cli/tests/support/mod.rs:782` and `holler-load-test/src/hub.rs:327`.<br>- ADR 0002 keeps Windows off the CI matrix but "tracked as a deferred story" (line 26). It is not retired.<br>`holler-pane` is the crate every other pane crate depends on, and it has no platform code today. An unguarded call would make it, the test kit and the adapters Unix-only at compile time.<br>**Why a warn:** the shipping binary is already Unix-only through `control.rs:8`. | Guard the `process_group(0)` call and the `kill` spawn with `#[cfg(unix)]`, as `own_process_group` does. On other platforms only `Child::kill` runs, the same path as Decision 16's missing-`kill` fallback.<br>Correct Decision 13's rationale.<br>Keep the test module's own `#[cfg(test)]` line as it is, since AC 9's `sed` range anchors on it. AC 9's count of two `Command::new` is unchanged. |
| W-11 | warn | Decision 21 (inline probe tests; the scratch-dir guard); AC 8f and 8g (the bounded `ps` poll) | file structure; duplication (Phase 7 candidates) | **(1) Test placement.** `holler-pane` has no inline test module. Its tests are ten files under `crates/holler-pane/tests/` plus `tests/common/`, and the stub's own test is `tests/ports_test.rs:512`. `probe::tests` would be the crate's first inline module. This is not a block: the workspace mixes the two (`holler-body` and `holler-cli` use inline modules, e.g. `backoff.rs:43`), and the blast radius justifies the choice.<br>**(2) Two near-copies.** The probe tests' guard and AC 8f's poll copy `StateDir` (`holler-cli/tests/support/mod.rs:69`: a temp dir named from the pid and a counter, removed on `Drop`) and `wait_for` (`:151`: a deadline-bounded poll). Both are on this stack's Phase 7 rejection list. Neither can be reused: `holler-pane` has no dev-dependency and this story adds none, and `tests/support` is another crate's test module. The Reuse map does not name either. | Add one Reuse-map row: "`StateDir`, `wait_for` (`holler-cli/tests/support`): not reachable from `holler-pane`'s lib tests; a minimal private guard and a counted poll in `probe::tests`". Have Decision 21 say that inline is a deliberate departure from the crate's `tests/` pattern.<br>**Pre-ruled for Phase 7:** both copies pass if they stay private to `probe::tests` and minimal: create, path and `Drop` for the guard, and a counted loop for the poll, with no `hub()` or `body()` and no use outside the module. |
| W-12 | warn | F2, F4, AC 14b ("cites F4") | follow-ups (completeness) | **Three gaps in the follow-ups.**<br>(1) F4 misses the crate docs that call the runner a stub: `holler-pane/src/lib.rs:26` ("[`probe`] — [`ProbeResult`] and the [`run_probe`] stub") and line 7 ("no behaviour behind a stub"). F4 already covers the "no I/O" wording on line 7.<br>(2) Follow-ups are filed by O in Phase 11 (the pipeline doc's decision-journal section), after F writes the ADR. So AC 14b's sentence cannot cite F4 by number.<br>(3) F2 hoists the append helper "beside `code()`". Its natural neighbour is `detail()` (`error.rs:535`), the crate-private exhaustive read of the same one-string payload, with `from_closed` (`error.rs:565`) as the rebuild. Placed there, the hoisted helper is a short method built on them, not a fourth walk of all 23 variants. | (1) Add `lib.rs:7` and `lib.rs:26` to F4.<br>(2) In AC 14b, write "a follow-up amends the frozen trait docs" with no number, and link the issue from the PR body or the Phase 11 summary. Alternatively, O files F4 before the run.<br>(3) Reword F2 to "beside `detail()`, built on it and `from_closed`". |

### The first pass's findings

- **B-1, resolved.** AC 14 (a) to (e), Files, Blast radius, AC 12 and Decision 22 now carry the ADR edit, and F3 is withdrawn.
  The places a-e name match the ADR at `3bdd129`: lines 78-80, 86-88, 273-275, 292-293, 296-298 and 299-301. AC 14's
  merge-hygiene rule and its checks are sound.
- **W-1, resolved.** Decision 8 adds `pub const RECONCILE_STEP_UNSCOPED` beside `reconcile_step`, AC 5 pins it, and F5 hands the
  change to #644. The #644 lines F5 lists (1554, 1557-1558, 1707, 1954, 2031 and 2157 at `7195993`) are accurate. The path
  `super::profile_scope::...` resolves from `pane/launch.rs`, `relaunch.rs` and `close.rs`, all in `pane/` (`pane/mod.rs:22`).
- **W-2, resolved.** F1 names routes (a) and (b).
- **W-3, resolved.** Decision 5 requires an exhaustive match with no `_` arm, a rustdoc note on `Timeout.op`, and the F2 hoist.
  W-12 (3) refines where the hoist goes. Decision 5's variant lists are complete: 23 variants, five of them payload-less
  (`error.rs:403-491`).
- **W-4, resolved.** Decision 20 keeps the mechanics apart from the verdict and lists three open points for #696. #641's brief has
  moved to `0f18b80`, but it still matches all three: `exec.rs` drains stdout and stderr on threads, it sends TERM and then KILL
  after a grace, and its kill binary is a seam (`with_kill_binary`).
- **W-5, stands.** It is accepted for Phase 7, and the brief needs no change.
- **W-6, resolved.** F4 takes `ports.rs:204-207` and `profile.rs:377-379`.

### Also checked (no finding)

- **No missed reuse candidate.**
  - No production shell-quoting helper exists. The only one is the test-local `sh_quote` in `multiword_command_test.rs:63`.
  - No bounded runner exists in a library crate.
  - `holler-load-test`'s `kill_tree` (`main.rs:556`) is a group kill, but it is a binary-only harness that uses `libc` and
    `unsafe`, so it is precedent only. `holler-pane` cannot depend on it, and AC 11 rules out its route.
- **The CLI's inline test module is forced.** `holler-cli` has `autotests = false`. `tests/pane_verbs/main.rs` is #670's frozen
  file, with one `mod` per verb and no slot for the scope. `[lib]` has no `test = false`, so `cargo test -p holler-cli --lib`
  runs the module.
- **AC 1's closure type-checks.** The suite's `build` is `FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S`
  (`conformance/profile_scope.rs:193-196`), and those `Arc`s coerce into `StoreScope::new`'s `Arc<dyn ...>` parameters.
- **Decision 2's texts match the fake.** The fake writes `profile.to_string()` and `"{name} is not in profile {:?}"`
  (`testkit/src/profile_scope.rs:97-126`).
- **AC 14 can merge cleanly.** #644 adds a paragraph after ADR line 305, #647 edits only section 12 and "Deferred", and #662 edits
  only sections 3 and 9. All three branches are still brief-only, so no ADR edit has been committed. AC 14's hunks are separated
  from theirs by unchanged lines.
- **ADR record.** Issue #22 still holds the placeholder body, so the markdown is the working record. In-place `(#NNN)` edits are
  the practice (#639 `2a6f349`, #692 `316b8e3`, #697 `c76bbed`).
- **Wire and persistence.** `ProbeResult`'s shape is unchanged, so no golden file or `docs/protocol/v2.md` change follows.
  Decision 14 changes only what `Pane.probe.last` holds. Nothing new is persisted.

## Notes for O

PASS, so no amendment is required before T runs.

**Can be taken at F time, with no brief change**, because each stays within the existing ACs and the hunks AC 14 names:

- W-7: (a) and (c), and the outcome wording in (b).
- W-8: (1), as a cross-reference inside the section 2 sentence, and (2).
- W-9: the clause in the fence-bullet edit.
- W-10: the `#[cfg(unix)]` guards.

F should record each choice in `handoff-F.md`.

**For O:**

- W-7 (b)'s requirement on #649's store client goes to #649's brief. It belongs beside F5, as one more cross-story note.
- W-11's Reuse-map row can wait for the Phase 11 summary. This handoff already pre-rules it for Phase 7.
- W-12's follow-up edits are O's at Phase 11.

**What Phase 7 will check:**

- `PaneInOtherProfile` is produced in `crates/holler-cli/src` only by the scope's `Set`-only helper, and it uses the hub's text
  shape (W-5).
- `profile_scope.rs` holds exactly one formatter (`reconcile_step`), one const (`RECONCILE_STEP_UNSCOPED`), one quoting function
  and one payload-append helper. That helper is an exhaustive match with no `_` arm. There is no copy of the fake's helpers
  beyond the deliberate re-implementation the Reuse map names.
- `probe.rs` adds no public item, and its process mechanics are apart from the verdict mapping. It spawns only `argv[0]` and
  `kill`, and uses no `libc` and no `unsafe` (AC 9, AC 11).
- `probe::tests` has its own guard and poll, private and minimal (W-11), and no other test-support copy.
- The ADR diff touches only the places a-e name, with no table row and no heading changed.

## Patterns referenced

- `docs/adr/ADR-0021.md`: lines 78-80, 86-98, 176-192, 214-215, 273-305, 396 and 457-460, plus "Decisions taken". Also
  `docs/adr/README.md` and ADR 0002, line 26.
- `crates/holler-pane/src/error.rs`: lines 399-401 and 494-611 (`code`, `classify`, `detail`, `from_closed`, `from_wire`).
  `crates/holler-pane-testkit/src/profile_scope.rs`, the reference behaviour.
- The platform and test-support precedents: `crates/holler-cli/tests/support/mod.rs:69, 151, 782` (`StateDir`, `wait_for`,
  `make_own_process_group`), `crates/holler-load-test/src/hub.rs:327` and `crates/holler-hub/src/control.rs:8`.
- `crates/holler-cli/src/transport.rs:69-78`, the control path's dropped-socket mapping.
- The consumers: `origin/issue-644-implementation:docs/handoffs/644-brief.md` at `7195993` (lines 1596-1627 and 1705-1708) and
  `origin/issue-641-implementation:docs/handoffs/641-brief.md` at `0f18b80` (lines 524-526, 564 and 611-624).
