# Handoff-S: Phase 9 - #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite (spec audit)

**Date:** 2026-10-09
**Branch:** issue-683-implementation (at 4672d78; base e410e9d)
**Issue:** #683 (slice d of #638, epic #633)
**Brief:** `docs/handoffs/683-brief.md`, as amended at 9d79d0e
**Handoffs reviewed:**
- `handoff-A.md` (round 2)
- `handoff-T-red.md`
- `handoff-F.md`
- `handoff-T-green.md`
- `handoff-A-dup.md`
- `decisions.md`
- `evidence.md`

**Diff audited:** `git diff origin/main...HEAD`, in full.
- I read all six Rust files end to end: the three filled stubs and the three new test files.
- I read the CHANGELOG and ADR-0021 hunks.
- I checked each pipeline commit's file list to confirm role separation.
- I did not re-run Tier 1 or Tier 2 (T's job). The facts I checked myself use read-only commands (`git`, `grep`, `wc -l`,
  `awk`, `gh`).

## A precondition

Met.
- `handoff-A.md`, round 2: **PASS**, with 0 blocks and 3 warns. Round 1's BLOCK (ADR-0021 sections 9 and 10) was
  resolved by AC 8.
- `handoff-A-dup.md`: **PASS**, with 0 blocks and 3 warns. All three are follow-ups that Phase 3 had already accepted or
  handed to O.

## T precondition

Met.
- **RED** (`handoff-T-red.md`): `cargo test -p holler-pane-testkit --no-run` fails with only four groups of E0432
  (unresolved import), all in the three new files.
  - T also type-checked the tests against throwaway signature-only stubs, then reverted them.
  - Slice a's 22 and 10 tests still passed.
- **GREEN** (`handoff-T-green.md`):
  - The new tests pass: 22 in `fake_herdr_test`, 6 in `fake_prober_test` and 15 in `herdr_conformance_test`. Slice a's 22
    and 10 are unchanged.
  - Every Tier 1 row is PASS, and Tier 2 is PASS.
  - "Blocking issues: None".
- **Role separation**, from the commits' file lists:
  - `4bc8937` (T-red) touched only the three test files and handoffs.
  - `8196561` (F) touched only the three `src` files, `CHANGELOG.md`, `ADR-0021.md` and handoffs.
  - `bbe4d89` (T-green) and `4672d78` (A-dup) touched handoffs only.
  - So the tests were written before the code, and F did not edit them.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | The fake passes its suite in both placements; the case list; a fresh fixture per case | All in `herdr_conformance_test.rs`. `the_fake_passes_the_herdr_conformance_suite` (:53) uses the brief's `text` example. `the_fake_in_split_only_mode_passes_the_suite` (:59). In `the_suite_runs_the_documented_cases` (:69), `DOCUMENTED_CASES` matches the brief's table: 11 ids, in order (checked against the table). `the_suite_builds_a_fresh_fixture_per_case` (:137) asserts `fresh` built 11 times, 11 guards dropped, and no port call after its guard dropped. | MET |
| 2 | Mutation check: each mutant fails, on its named case | The `Mutant` wrapper is local to the test file (:201), and the fake has no mutant switch. `assert_suite_fails_on` (:385) asserts `Err`, every named case among the failures, and a non-empty `detail` on every failure. The control is `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone` (:400). There are 10 mutant tests (:406-494). The two mutants with two named cases (`Transposes`, `CloseIsIdempotent`) assert both. `an_unsupported_herdr_fails_the_version_case` also asserts that it is the only failing case. I traced each break through the case code by hand, and each one fails its named case for that case's reason. For example, under `Transposes`, `ensure_pane(r2c1)` becomes `r1c2`, which is out of range in case 1, and `r1c2` becomes `r2c1` and succeeds in case 2. | MET |
| 3 | The fake's own mechanisms | `fake_herdr_test.rs` has every one of the 18 tests the brief names, plus 3 extra: the base-36 carry to `w1:p10`, split out of the ids test; an occupant in split-only mode; `print` to an unknown pane. What they pin:<br>- ids: `w1:p1`..`w1:pC`, then `w1:pD` after a close, `w1:pE` after a vanish, and `w2:p1` in the second workspace;<br>- split-only mode: the brief's exact sequence;<br>- range before split: `r3c1` in a 2x1 workspace is `GridOutOfRange`, and would be `grid-unreachable` if the two checks were swapped;<br>- faults: a `Timeout` naming each of the seven methods, and a failed ensure spends no id;<br>- `sent()` against `faults().calls()`, for I4. | MET |
| 4 | The fake probe | `fake_prober_test.rs`: the 5 named tests, plus `an_unscripted_error_names_the_argv`. The unscripted test includes an argv one element off and a prefix-only argv. | MET |
| 5 | No other crate changes | `git diff --name-only origin/main...HEAD` lists only the brief's blast radius: the three stubs, the three new tests, `CHANGELOG.md`, `docs/adr/ADR-0021.md` and `docs/handoffs/683*`. None of these changed: any `Cargo.toml`, `Cargo.lock`, `lib.rs`, `conformance/mod.rs`, `feed.rs`, `fault.rs`, `pane_store.rs`, `fixture.rs`, `crates/holler-pane`, `docs/protocol`. Slice a's tests pass unchanged (T-green). The three-dot diff still uses base `e410e9d` after `main` moved (Advisory 1). | MET |
| 6 | CHANGELOG | `CHANGELOG.md:90-99` sits under `## [Unreleased]` (:8) and `### Enhancements` (:10), directly after the #676 entry (:82-89). It names every item AC 6 lists and links #683 and the epic #633; `changelog-check: ok`. Its one extra sentence ("ADR 0021 now records ...") is accurate. | MET |
| 7 | Guards | T-green's Tier 1 table: build, clippy `-D warnings`, `cargo test --workspace` (1152 passed, 0 failed, 5 ignored), machete, `lint.sh`, `changelog-check`, `test-hooks` and rustfmt all pass. My own checks:<br>- **Size:** `wc -l` puts the largest file at 538 lines (`fake_herdr_test.rs`; `herdr.rs` is 525), so none reaches 600.<br>- **Banned items:** a grep of `src/` for `unwrap`/`expect`/`panic!`/`unreachable!`/`assert!`/`todo!`/`allow(` finds only the `text`-fenced doc example at `conformance/herdr.rs:133`, which is not compiled.<br>- **Allows:** every `#![allow]` carries `// #683`, and there is no `dead_code` allow.<br>- **Split:** `ensure_pane` is split into a lookup (`herdr.rs:357`) and `Workspace::place` (`herdr.rs:402`), which calls `check_range` (:414), `occupant` (:429), `check_split` (:436) and `mint` (:460). | MET |
| 8 | ADR-0021 records `ensure_pane`'s `grid-out-of-range` | `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` shows exactly two items.<br>(b) The section 9 row (:377) uses the brief's wording verbatim.<br>(a) The section 10 bullet (:421-423) keeps its old text and adds the second condition in the brief's wording. It is re-wrapped to 125, 118 and 120 columns; its neighbours run from 116 to 126. It also carries A's optional sentence about #640.<br>(c) The class stays Refusal. `holler-pane` (`ALL_CODES`, `class_of` and its tests) is untouched. There is no "Decisions taken" item and no other line changed. I read (c) as two items, per A round 2, warn 1. | MET |
| Issue 1 | The fakes pass their suites and reject the mutants | `FakeHerdr`: see AC 1 and AC 2. `FakeProber` has no conformance suite. `conformance/mod.rs` declares none, and the issue's scope names only `run_herdr_conformance`, so AC 4 is its check. | MET |
| Issue 2 | The test kit depends on `holler-pane` only | This branch does not change the test kit's manifest. T and F ran the `cargo tree` direction check, and it printed nothing. | MET |
| Issue 3 | Workspace gates; a CHANGELOG link; no unwrap, expect or panic in library code | See AC 6 and AC 7. | MET |

## Spec compliance

Everything is compliant, and I found no silent deviation.

**Public API.** It matches the brief's "Public API (exact)" item by item:
- the four constants and their values;
- `HerdrOp`, with its derives and its `herdr.<method>` names;
- the defaults of `Placement` and `HerdrVersion`, and `Sent`;
- `FakeHerdr`'s eight methods, with their signatures, and `ProbeCall`;
- `FakeProber`'s `new`, `script`, `calls`, `Default` and `Prober` impl;
- `HerdrFixture`, `herdr_cases`, and `run_herdr_conformance` with its bounds.

There are no flat re-exports, and `lib.rs` and `conformance/mod.rs` are unchanged.

**Fake behaviour:**
- **Faults first.** Every port method enters the `FaultSwitch` first (`herdr.rs:247, 260, 271, 289, 294, 303, 314`), and
  the state lock is taken only after that. So a fault changes nothing.
- **`ensure_pane` order.** The steps run in the brief's order 1 to 5: the session and workspace, the range, the occupant,
  the split rule (split-only placement only), then the mint.
- **The mint.** `mint` uses `checked_add` and stores the counter only after it succeeds. An overflow is `unavailable` and
  creates nothing.
- **Close and vanish.** `close` and `vanish` share `remove` and never touch the counter, so an id is never reused.
- **Sends.** `send_text` and `send_keys` look up the pane before they push to `sent`, so a failed send is in
  `faults().calls()` but not in `sent()`.
- **`read`.** It returns `last_lines` over `str::lines`.
- **`snapshot` order.** Workspaces come in declaration order (a `Vec`), then by row and column (a `BTreeMap` keyed on
  `(row, col)`).
- **`version`.** The selected `HerdrVersion` affects only `version()`.
- **`vanish` and `print`.** They take only the state lock, so they bypass the faults and the call log.
- **Messages.** Each message is one line, because the long ones use `\` string continuations. Each names what the brief
  asks for: the cell and the workspace size, the split rule, both versions, or the session and workspace.

**`FakeProber`:**
- It has no fault switch (Decision 9).
- It records each run before it answers.
- An unscripted argv answers the brief's exact message, `format!("no probe scripted for {:?}", argv.as_slice())`.

**The suite:**
- The 11 cases sit in one `CASES` table, over the crate-private `Scratch`.
- Every case that places panes ensures `r1c1` first, case 2 included. This is F's documented choice, and it follows the
  brief's rule for every case.
- No case closes a workspace's last pane.
- Keys are pressed only as `enter`.
- The checks go through `run_cases`, `succeeds`, `expect_code` and `expect_eq`. The helpers take `(what, got, want)`, and
  every case passes them in that order.
- Case 3 reads "exactly one pane" over the whole workspace. Under the fixture contract this is equivalent to the brief's
  wording, and it is stricter against a broken port. F documented it.

**ASSUMPTION comments.** All nine are present as `// ASSUMPTION (#640):` at the code they concern, in 13 blocks:

| # | Subject | Where |
|---|---|---|
| 1 | A workspace's size | `herdr.rs:187`; `conformance/herdr.rs:49` |
| 2 | Versions beyond protocol 22 | `herdr.rs:26` |
| 3 | The root pane | `herdr.rs:190`; `conformance/herdr.rs:52` |
| 4 | After a close | `herdr.rs:295` |
| 5 | Key names | `herdr.rs:272`; case 10 at `conformance/herdr.rs:337` |
| 6 | An unserved session or workspace | `herdr.rs:358` |
| 7 | Provisional vocabulary | `herdr.rs:42` and `:32` |
| 8 | Only `version()` refuses | `herdr.rs:318` |
| 9 | Out of the workspace is `grid-out-of-range` | case 2, at `conformance/herdr.rs:189`; also names section 9's `profile apply` row, per A's note 2 |

The spike sections these comments cite (2, 4, 5, 7 and 13) match the spike's headings and the line numbers the brief
cites.

**Decisions already made:**
- `GridPos` cells only.
- A workspace declared with a size; outside it is `grid-out-of-range`; `r2c1` accepted and `r1c2` refused.
- Ids `w<N>:p<M>`, upper-case base 36, never reused, and stable when a sibling closes.
- An occupied cell returns its pane.
- A split-only mode.
- Two versions: the protocol 22 string, and the "Herdr protocol 22 (0.9.1)" message.
- Wedged, vanished and slow faults.
- `send_text` and `send_keys` recorded for I4.
- `FakeProber` scripted per argv.
- The test kit depends on `holler-pane` only.

Each is implemented as stated.

**F's three declared deviations** are all within the brief:
- the files are larger than estimated, all under 600 lines;
- one extra CHANGELOG sentence;
- A's optional ADR sentence.

## Quality audit

**Correctness and failure handling:**
- Errors are the closed `PaneError` variants, plus one open code made with `RefusalCode::from_static`, the mechanism
  ADR-0021 section 9 prescribes.
- A poisoned lock is taken over with `PoisonError::into_inner` (`herdr.rs:240`, `prober.rs:70`).
- Each fake has a single state mutex, so a send's screen append and its `sent` push are atomic, and no write is lost under
  concurrency.
- No lock is nested: the switch's lock is released before the state lock is taken. There is no ordering hazard.
- `base36` cannot drop a digit, because `rest % 36 < 36` always gives a valid digit.
- One unspecified edge, a zero-size workspace, fails closed (Advisory 6).

**Build guards:** see AC 7. There is no `unwrap`, `expect` or `panic` in `src/`, every allow carries its `// #683` link,
no file reaches 600 lines, and there is no dead code (`dead_code = "deny"`, and clippy is clean).

**Protocol:**
- There is no wire or protocol change, no golden file and no `v2.md` edit, and none is needed.
- The error-code table (ADR-0021 section 9) and section 10 are updated consistently.
- `grid-unreachable` follows section 9's rule for open codes (a `from_static` constant, raised as `Refused`). It needs no
  verb row until a verb raises it (ASSUMPTION 7).

**Tests:**
- They are in-process fakes with no cross-process behaviour, so no hub or body harness applies.
- No fixed sleep is used for synchronization. The 80 ms delay is the behaviour under test, and it is asserted as a lower
  bound only.
- The RED-first evidence is in T-red's handoff, and the commit history confirms role separation.
- Every test asserts behaviour, not implementation. None would pass with the change removed.

**Documentation:**
- The CHANGELOG entry is present (AC 6).
- There is no new log event, CLI surface or protocol field, so no README or `docs/` change is due.
- Each filled stub's `//!` doc now describes its module.
- The module lists in `lib.rs` and `conformance/mod.rs` (slice a's) are still accurate.

**Public-repository privacy:** I grepped every added line, the handoffs included, for:
- host and tailnet names;
- personal names and e-mail addresses;
- RFC 1918 and CGNAT ranges;
- home-directory paths;
- key and secret patterns.

The only hits are the words "token store" and "Secrets" in handoff prose. The fixtures are neutral: `scratch`,
`elsewhere`, `live`, `127.0.0.1`, `ollama`, `qwen38`.

**Commit and PR hygiene:**
- All 8 commits have Conventional Commit subjects that pass `.githooks/commit-msg`'s regex.
- All are authored with the GitHub no-reply address.
- All carry a `Co-Authored-By: Claude …` trailer.
- None has a session link. Neither do the last 40 commits on `main`, so this matches the repository's practice.
- No PR exists yet, so its AI disclosure is still due (Advisory 4).

## Scope check

**In scope:** the change is exactly the brief's blast radius. That is the issue's list, widened by Decision 10 (the
suite's stub and the three tests) and by AC 8 (ADR-0021).

**Over-delivery, all justified and small:**
- T's four extra tests, each pinning a rule the brief states.
- One extra CHANGELOG sentence.
- A's optional ADR sentence about the extent.
- ASSUMPTION 9's mention of section 9's `profile apply` row (A's note 2).

**Under-delivery:** none.

**Decomposition:** none was needed. The largest test file is 494 lines, so the brief's fallback
(`tests/herdr_mutants/mod.rs`) did not apply.

## Verdict

**PASS**

- **Acceptance:** all 8 of the brief's acceptance criteria are met, and so are the issue's three acceptance items. Each
  has a proving test that asserts the behaviour.
- **Spec:** the public API, the fake's behaviour, the 11 cases, the nine ASSUMPTION comments and the two ADR-0021 items
  match the brief, with no silent deviation.
- **Quality:** the build guards, privacy and documentation are clean.
- **Scope:** it matches the brief.
- **Remaining work:** the items below belong to the PR step, not to F.

## Advisory notes

None of these blocks the merge. Items 1 to 4 are PR-time work for the run's own agent.

1. **Rebase before opening the PR: CHANGELOG conflict with #689.**
   - After F finished, `origin/main` moved from `e410e9d` to `90997a2` (#689, slice b, the JSON-envelope checker; merged
     2026-10-09 13:36 MDT).
   - `git merge-tree --write-tree HEAD origin/main` reports one conflict, in `CHANGELOG.md`: both entries were inserted
     after the #676 entry. Keep both: #681's entry, then #683's.
   - Nothing else overlaps. #689 changed only `lib.rs`'s doc comment, the test kit's `Cargo.toml` and `Cargo.lock` (adding
     `serde_json` for `envelope.rs`), `envelope.rs` and its test.
   - After the rebase, this branch's own diff still touches no manifest (AC 5), and CI re-runs the guards.
2. **The hand-off to #640 is still only in code comments** (A-dup warn 3). #640 had no comments when I checked on
   2026-10-09; it was last updated 2026-10-08 18:56 MDT. Put this list in the PR body, and post it once as a comment on
   #640:
   - the suite to pass: `run_herdr_conformance`, 11 cases, in an opt-in scratch session;
   - the fixture contract: a workspace of 2 rows by 1 column whose extent the adapter knows, empty or holding only its
     root pane at `r1c1`;
   - the 13 `ASSUMPTION (#640)` blocks, by file and line (the table under Spec compliance);
   - the two `holler-pane` doc lines: `error.rs:414-416` and `ports.rs:127-128`;
   - ADR-0021 section 9's `profile apply` row;
   - ASSUMPTION 7's provisional `grid-unreachable` and `SUPPORTED_VERSIONS`.
3. **No test-kit cleanup issue has been filed yet** (A-dup warns 1 and 2; none existed on 2026-10-09).
   - Add this slice's items to the one cleanup issue that #684's gate scoped:
     - `CaseGuard`;
     - an `assert_fails_on` that can name several cases;
     - `timeout(op)`;
     - the per-fake `lock()` methods, against `feed::lock`;
     - `NOT_FOUND`;
     - the `HerdrFixture` and `HarnessRig` shapes.
   - Settle it before #640 or #642 writes a suite runner.
   - Slice b has now merged, so the trigger "after slices b to e merge" is partly met.
4. **PR hygiene.**
   - Add the `CONTRIBUTING.md` AI disclosure to the PR body with `gh pr edit`.
   - The PR closes #683 and references #638 and #633.
   - The squash commit should keep a `Co-Authored-By` trailer.
5. **T-green's question about the ADR sentence: it is wanted.** The sentence is "How the adapter learns a workspace's
   extent is #640's."
   - A offered it in round 2 (warn 1).
   - It records a deferral, not a rule, and it mirrors ASSUMPTION 1.
   - It sits inside item (a), so AC 8(c) still holds.
6. **A zero-size workspace.** `with_workspace(name, 0, n)` is accepted, and every cell of it is then `grid-out-of-range`.
   That fails closed, and the brief does not specify it. A later slice may prefer `usage` when such a workspace is
   declared.
7. **Sizing, for O.**
   - This slice is 2,215 Rust lines, against an estimate of about 1,650. That is 34% over, the same overrun as slice a.
   - It is also about 10% over the guideline of about 2,000 lines for one run.
   - The excess is docs, the 13 ASSUMPTION blocks and rustfmt's layout.
   - Later slice estimates can carry that margin.
8. **Carried from T-green.** On this host, `holler-cli`'s `body_run_test::fresh_hello_and_presence_on_every_reconnect`
   also fails on `main`. T-green suspects a fixed-port collision. It is unrelated to #683, and worth an issue if it
   recurs.
