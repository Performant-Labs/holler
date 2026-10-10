# Handoff-A-dup: Phase 7 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (worktree `.claude/worktrees/0663-profile-scope-probe`)
**Diff base:** `0ad2d8a` (the merge base; `origin/main` is now at `519947a`)   **Diff head:** `d9210bc`
**This cycle:** `e46b427..d9210bc` (T-red `f79cd05`, F `4bbe607`, T-green `d9210bc`). No earlier run reached this gate, so
the whole feature diff `0ad2d8a..d9210bc` was reviewed too, with every changed file read in full.
**Reuse map:** `docs/handoffs/663-brief.md`, "Reuse map (extend, do not duplicate)" (lines 2045-2061), plus the Phase 3
pre-rulings W-5, W-11, W-16 and W-18 (`docs/handoffs/663/handoff-A.md`)
**Verdict:** PASS

## Summary

PASS, with no block and two warns (D-1, D-2).

- **F extended every object the map named.** It implements the frozen trait, does its I/O through the two store ports only,
  builds the reconcile step on `doctor_command`, and reuses `Argv`, `Prober`/`SystemProber`, the suite and the fixtures.
  The workspace has one real `ProfileScope` (`StoreScope`) and one probe runner (`run_probe`, reached through
  `SystemProber`); the `Unwired` stand-ins are #649's to replace.
- **This cycle is the B-2 fold, and it is clean.** Both forms of the step come from `doctor_command(None, false)`, and the
  const is gone. In production code, `holler pane doctor` is now spelled only at `findings.rs:36`, on the branch and on
  main.
- **Every deliberate copy is one the brief justifies in writing:**
  - the fake's rules (Decisions 1-10);
  - `check_joins` (W-5, F2);
  - the private runner (Decision 20, W-16, #696);
  - `single_quoted` (W-18, F6);
  - the probe tests' scratch directory and counted poll (W-11).
- **D-1, the headline warn.** `StoreScope`'s private `belongs` is a line-for-line copy of the public
  `holler_pane::profile_diff::is_member` (#703). It is not a block:
  - the map does not name `is_member`, and F followed the map;
  - F wrote `belongs` 7 minutes before #703 merged;
  - I missed the copy in four Phase 3 passes.

  The fold is 5 lines and changes no test.
- **D-2.** #706 merged a second private bounded runner on main. W-16's ruling stands, and #696 gains six more
  differences to settle.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| D-1 | warn | `crates/holler-cli/src/pane/profile_scope.rs:218-222` (`belongs`), called at `:87` and `:95` | **A private copy of `holler_pane::profile_diff::is_member`.**<br>**The copy:** `belongs(pane: &Pane, profile: &ProfileName) -> bool` has the same signature and the same rule (`Pane.profile`'s slug equals the profile's slug) as `is_member` (`profile_diff.rs:254-265`; `pub mod profile_diff` at `lib.rs:50`).<br>**Why `is_member` is the one rule:** its rustdoc calls it what "a live pane of a profile" means (ADR-0021 section 3, line 165). Every other CLI caller of the rule uses it:<br>- `profile/list.rs:59`;<br>- `profile/show.rs:89`, which builds the same `pane_store.list()` filter as `StoreScope::members`;<br>- on main, `pane/watch.rs:215` (#709, `efd9a00`, 20:38 MDT).<br>**The concrete drift risk:** `watch.rs` takes its first member set from `ports.scope.resolve` and then updates it with `is_member`. Once #649 wires in `StoreScope`, one verb applies both copies to one set, so a later change to either copy makes `pane watch --profile` disagree with itself. The two are identical today, so nothing is wrong yet.<br>**Why a warn, not a block:**<br>- The map does not name `is_member`, and the brief gives a written reason to re-implement the fake's rules ("production code cannot depend on the test kit"). That reason is true, but it does not cover `holler-pane`.<br>- Main moved under the branch, as with W-16:<br>&nbsp;&nbsp;- F wrote `belongs` at `2d9c7a0` (19:06 MDT) on base `3bdd129`, where `profile_diff.rs` was an empty stub;<br>&nbsp;&nbsp;- #703 put `is_member` on main at 19:13 MDT (`ce12cdb`);<br>&nbsp;&nbsp;- the branch took it in at `1d6a5ab` (19:34 MDT);<br>&nbsp;&nbsp;- the re-based brief (`9762a97`) and A's fourth pass did not add it to the map. That miss is A's. | **The fold, 5 lines:**<br>- add `use holler_pane::profile_diff::is_member;`;<br>- write `members.retain(\|pane\| is_member(pane, profile));` at `:87`;<br>- write `Some(pane) if is_member(&pane, profile) => Ok(pane),` at `:95`;<br>- delete `belongs` and its doc line.<br>**What it changes:** no behaviour and no test (AC 1-7 cover both callers). It stays in the blast radius, and the file goes from 605 lines to about 600.<br>`is_member`'s module (`profile_diff`) is no reason to keep the copy: `watch.rs` already imports it for scoping.<br>**F:** take the fold in any rework this run (S, or the outside diff gate).<br>**O (Phase 11):** if no rework runs, add the fold to F2, which already edits this file's membership code. Add to F1 the fake's identical `belongs` (`holler-pane-testkit/src/profile_scope.rs:234-239`). |
| D-2 | warn | `crates/holler-pane/src/probe.rs:180-308` (the private runner), compared with `origin/main`'s `crates/holler-adapter-host/src/exec.rs` (#706, `e327569`, 20:06 MDT) | **A second private bounded runner on main.**<br>W-16 named one, #705's `holler-adapter-opencode/src/exec.rs` (`kill_group` at line 26, `run` at 48). #706 merged another: `run` (61), `wait` (118), `reap` (139), `collect` (145) and `kill_group` (202). Once #663 merges there are three.<br>**Not a parallel path:** `probe.rs` still cannot use either. The adapters depend on `holler-pane`, and both runners are `pub(crate)`. W-16's ruling stands.<br>**Where the host runner differs from `probe.rs`,** beyond W-16 (a)-(c):<br>- **(d) Order.** It polls for the exit first and reads the output after (lines 83-84), so the leader is reaped before stdout ends. That is safe there, because `run` never kills its own child's group. But it is the opposite of `probe.rs`'s pid-reuse order (Decision 15). A shared runner that offers a group kill must keep `probe.rs`'s order.<br>- **(e) A signal after a `try_wait` error.** `wait` calls `reap` there (line 124), that is, `child.kill()` and then a blocking `child.wait()`. Decision 15 forbids that signal (W-16 (b) again).<br>- **(f) The program in the error text** (`{program}: {e}`, lines 72 and 125). Decision 19 forbids it for a probe (W-16 (c) again).<br>- **(g) No output cap.** It drains both stdout and stderr. `probe.rs` caps stdout at 1 MiB and discards stderr.<br>- **(h) The `kill` program.** `kill_group` takes the `kill` path (the fake-`kill` seam #696 must keep), sets `LC_ALL=C` (line 212) and reads `No such process` (line 216). `probe.rs` runs `kill` from `PATH` and ignores the outcome.<br>- **(i) Polling.** It polls every 2 ms (line 21); `probe.rs` polls every 10 ms. | **No change here.**<br>**O (Phase 11):** the #696 comment that W-16 asks for should name both merged copies (#705's and #706's `exec.rs`) and list (a) to (i). The shared runner keeps Decision 15's order and its no-signal rule. |

No duplication of an object the map named, and the extension is clean. The rework added no architectural drift.

### Checked and consistent (no finding)

- **The Reuse map, row by row.**
  - `ProfileScope` is implemented as is. The other `impl ProfileScope`s are `Unwired` (#649's), the fake, and test
    mutants.
  - `ProfileStore` and `PaneStore` are the scope's only I/O.
  - The suite runs unchanged (AC 1). The fakes, `fail_next`, `concurrent_put` and the three fixture functions are reused as
    dev-dependencies.
  - `FakeProfileScope` is not imported, as the map rules.
  - `Prober`, `SystemProber` and `Argv` are unchanged and reused.
  - `wiring.rs` is untouched.
  - `findings::quoted` is not used: it is not a shell quote.
- **This cycle's diff (`e46b427..d9210bc`).**
  - AC 5's three greps print 0, 1 and 0.
  - Nothing on main builds a `holler profile show` line, and `to reconcile` appears in no crate on main.
  - The new `use holler_pane::findings::doctor_command;` follows `pane/doctor.rs:19`'s import of `findings`.
  - `reconcile_step(Option<&ProfileName>)` takes the same `profile` argument `edit_spec` does.
  - `findings.rs` is untouched, so its remedies stay profile-free (`findings.rs:12-18`).
  - The ADR hunks change sentences in place. `probe.rs` gained one rustdoc bullet and no code.
- **Error texts match their neighbours.**
  - `profile-not-found` carries `name.to_string()`, as the fake (`profile_scope.rs:100`) and the hub
    (`profile/store.rs:185`, `profile/mod.rs:212`) do. `profile/show.rs:82` quotes the name, but it is the only one, and
    not this story's.
  - `pane-not-in-profile` reads "`<pane> is not in profile "<P>"`", as in the fake and in #709's `get.rs:106`.
- **No other helper does what these do.**
  - `with_context`: no other `src` function appends to a `PaneError` payload. F2 hoists it.
  - `with_edit` and `check_filed_under`: no `Profile` or `ProfileSpec` method edits specs, and the only other copies are
    the fake's, which the map covers.
  - `contains`, `wait_until`, and the `Instant::now().checked_add(..)` deadline: neither `holler-proto` nor
    `holler-pane` has a shared helper, and `checked_add` is the workspace idiom (`holler-hub/src/panes/store.rs:268`,
    `holler-pane-testkit/src/feed.rs:201`).
- **Test-only copies.**
  - The probe tests' `Scratch` and `assert_gone_within_2s` are private and minimal. `holler-pane` has no dev-dependencies,
    so W-11's pre-ruling holds: pass.
  - The scope tests' `changed_spec`, `act_failed`, `Bench` and `edit_alpha` mirror the suite's private helpers
    (`conformance/profile_scope.rs:165, 403, 416`; `act.rs:202`), as AC 2 asks.
  - **Optional for O (F1):** export `changed_spec` from `holler_pane_testkit::fixture` before #644's and #646's verb
    tests make a fourth copy.
- **Forward-compat with #709, now on main.** `pane list`, `get` and `watch` call `resolve` and rely on what `StoreScope`
  answers:
  - every member, in name order;
  - `pane-not-in-profile` for a named pane outside P (`get.rs:96-110`, `list.rs` `rows_in_scope`, `watch.rs` `open`).
- **The overlay's stack checks.**
  - No prompt path; no wire, golden-file or protocol change.
  - ADR-0021 is amended in place, in the same change.
  - No persisted state, no new dependency or manifest line, and no added `unsafe`.
  - The files are 605 lines (accepted by AC 10) and 577 lines, both under the ~800-line flag.
  - The code, the ADR and the CHANGELOG name no personal infrastructure. The one personal name in the diff is the operator
    override line in `decisions.md`, which main's `640/decisions.md:1` also has.
  - Nothing copies `token.rs`, `Lockout`, `Roster`, `log(Severity, ...)` or the hub test harness.
- **The merge.** `git merge-tree --write-tree HEAD origin/main` (`519947a`) conflicts in `CHANGELOG.md` only. #707, #708
  and #709 touch none of the branch's files.

### Carried from Phase 3 (not re-rated)

- **W-5:** `check_joins` is the third private copy of the move rule. It is accepted and justified in the map; F2 hoists it.
- **W-11:** pass, as pre-ruled (above).
- **W-16:** stands. D-2 extends it.
- **W-17:** documented, not fixed. The rustdoc sentence is at `profile_scope.rs:47-48`, and the `ProfileName::parse`
  follow-up is O's.
- **W-18:** `single_quoted` is not a block, as pre-ruled. The fold has a third copy: the test-only `sh_quote`
  (`crates/holler-cli/tests/multiword_command_test.rs:63`, from #295, on main since 2026-09-10) has the same body. A test
  target cannot reach a private `src` helper, so the brief's "no crate's `src` has a POSIX shell-quoting helper" is still
  true. **O:** list all three copies in F6, so that F6's shared helper, if it is `pub`, absorbs the test's too.

## Notes for F

None required: PASS. If S or the outside diff gate sends F back this run, take D-1's 5-line fold in the same rework.

## Notes for O (Phase 11; none blocks)

1. **D-1:** add the `is_member` fold to F2 if no rework took it. Add the fake's copy to F1.
2. **D-2:** the #696 comment names both merged runners and the differences (a) to (i).
3. **W-18:** F6 lists three shell-word copies.
4. **Optional:** F1 exports `fixture::changed_spec`.

## Patterns referenced

- `crates/holler-pane/src/profile_diff.rs:254-265` (`is_member`), with `crates/holler-cli/src/profile/{list,show}.rs` and,
  on `origin/main`, `crates/holler-cli/src/pane/{watch,get,list}.rs` (#709).
- `crates/holler-pane/src/findings.rs:12-18, 36, 303-316` (`doctor_command`).
- `crates/holler-pane-testkit/src/profile_scope.rs` (the fake) and `src/conformance/profile_scope.rs`.
- On `origin/main`: `crates/holler-adapter-host/src/exec.rs` (#706) and `crates/holler-adapter-opencode/src/exec.rs` (#705).
- ADR-0021 sections 1, 3 (line 165), 8 and 12, and `docs/handoffs/647/handoff-A-dup.md` (D-3, the rule that a later story's
  gate rejects its own copy).
