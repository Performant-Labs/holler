# Handoff-F: Phase 6 (F) - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (re-entry)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `f79cd05`, T's re-entry RED, plus this phase's uncommitted changes; F does not commit)
**Issue:** #663

This replaces the previous run's F handoff (at `2d9c7a0`, in git history). That run built the scope and the probe runner,
and T-green passed them at `93fb653`. This run implements only what the amended brief (`9762a97`) changed: Decision 8 on
`doctor_command`, and AC 14's ADR wording.

## What was done

- `crates/holler-cli/src/pane/profile_scope.rs`: `reconcile_step(profile: Option<&ProfileName>) -> String` gets its body
  and rustdoc (Decision 8). They replace T's stub, including its `let _` line.
  - Both forms start from `holler_pane::findings::doctor_command(None, false)`, imported with one new `use` line.
  - The rustdoc carries W-17's sentence about a name with a leading `-`.
  - Nothing else in the file changed: not the scope, not its helpers, not T's tests.
- `crates/holler-pane/src/probe.rs`: `run_probe`'s rustdoc gains one 4-line bullet stating the `try_wait`-error rule: no
  signal, `Error`, and a child still in the group is left. No code changed.
- `docs/adr/ADR-0021.md`, AC 14's six edits:
  - **a** (section 1): the exceptions sentence is added. The runner signals only while the leader is unreaped, so a
    `try_wait` error sends no signal. That path, an escaping child and a missing `kill` are section 12's exceptions.
  - **e** (section 8, step 6): the previous run's sentences are re-worded in place. Both forms come from `reconcile_step`
    on `doctor_command(None, false)` (#701). The step names no pane on purpose, for C9's reason. The unscoped form is
    `reconcile_step(None)`.
  - **e** (the record-fence bullet): the parenthesis now reads "(the pane doctor command line; the profile-scoped or the
    bare form of step 6, which names no pane, for the reason given there)".
  - **f** (section 12, its last sentence): extended in place with the probe runner's three cases, pointing at section 1.
  - **b, c, d**: unchanged from the previous run. They already read as AC 14 asks.
- `docs/handoffs/663/evidence.md`: two new sections:
  - the reconcile step's builder (3 facts, from the branch);
  - the group kill on macOS (4 facts, from `dc300ab`; W-16).
- `CHANGELOG.md`: unchanged. The previous run's entry quotes only the scoped step, which is byte-identical.

## Design decisions

1. **`reconcile_step` builds the doctor line once, then adds the profile form.**
   - The function binds `let doctor = doctor_command(None, false);`.
   - A `let ... else` returns `to reconcile, run {doctor}` for `None`.
   - Otherwise it returns `to reconcile, run {doctor} --profile {name} and then holler profile show {name}`, with `name =
     single_quoted(profile.as_str())`.

   The scoped output is byte-identical to the previous run's text, so AC 2-4's substrings do not move. A two-arm `match`
   was the alternative, but rustfmt splits its long arm over three lines, and the `let ... else` reads as "the bare
   line, or the profile form on top of it".
2. **`use holler_pane::findings::doctor_command;`**, the way `pane/doctor.rs` imports from `findings`. The rustdoc then
   links [`doctor_command`], and `cargo doc` prints no warning for either changed file.
3. **The rustdoc carries the reason the step names no pane** (C9). It also says that verbs call the function rather than
   spell a step of their own (F5), and states W-17's limit. A's "optional for F" was taken.
4. **The `probe.rs` rustdoc bullet.** AC 14a's ADR sentence now calls the three cases "documented exceptions". The public
   rustdoc of `run_probe` already documented two of them, the missing `kill` and the escaping child. The bullet documents
   the third. The other mention of the `try_wait`-error rule is `wait_for_exit`'s private doc.
5. **ADR wording.**
   - e cites `holler-pane/src/reconcile.rs` and the two codes, without line numbers, as the ADR cites code elsewhere.
     Line numbers would drift.
   - The fence bullet drops main's "for that pane", as AC 14e's new parenthesis requires.
   - f is one clause appended to the sentence, so the words "No verb leaves work running after it exits" are kept. a's
     sentence quotes the same words.

## Reuse / extend-vs-new

- **Reused:** `holler_pane::findings::doctor_command` (#701), per the Reuse map row marked **reuse**. The production code
  of `profile_scope.rs` spells `holler pane doctor` nowhere (AC 5's grep prints 0). The doctor line has one builder, and
  the profile form composes on top of it in `profile_scope.rs` (Decision 8; D-3's second choice). So `findings.rs` and
  its remedies stay profile-free.
- **Kept private:** `single_quoted`, whose one caller is now `reconcile_step`. Under W-18's pre-ruling it is not a block.
  F6 folds it with #662b's planned `shell_word` in a shared non-verb module.
- **No new object.** The probe runner is unchanged and stays private. Under W-16's ruling it neither imports nor copies
  the adapters' `exec.rs`.

## Architecture notes for A

- **Public interface.** In this cycle `reconcile_step`'s signature became `Option<&ProfileName>` (T's RED stub), and
  `pub const RECONCILE_STEP_UNSCOPED` is gone. F gave the function its body. Against `origin/main`, `holler-cli` gains
  `StoreScope`, `StoreScope::new` and `reconcile_step`. No caller outside the file exists yet; #644 and #646 call it (F5).
- **Dependencies.** `profile_scope.rs` now calls `holler_pane::findings`, a public module of a crate `holler-cli` already
  depends on. `pane/doctor.rs` already uses it. No manifest change, and the dependency direction is unchanged.
- **ADR.** Sections 1, 8 and 12 amended in place (a, e, f). b, c and d are unchanged since the previous run.
- **Self-report: `archChanged: true`.** The cycle changed a public interface (the signature and the removed const) and
  amends the standing spec in three sections. All of it was planned in the brief and reviewed at Phase 3 (fourth pass,
  PASS). This follows the role's "when in doubt" rule.

## Deviations from spec / wireframe

No wireframe (no UI). Deviations from the brief, all small:

1. **The `probe.rs` rustdoc bullet** (design decision 4). The brief expected `probe.rs` untouched in this run. The change
   is rustdoc only: 4 lines, so the file is at 577, under 600. T's test module is byte-identical.
2. **AC 14e's `reconcile.rs:222-243`** is cited as the file and the two codes, without the line numbers (design decision
   5).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --lib pane::profile_scope
test pane::profile_scope::tests::reconcile_step_single_quotes_the_profile_name ... ok
test pane::profile_scope::tests::pane_store_fault_fails_a_remove_before_the_profile_write ... ok
test pane::profile_scope::tests::set_of_a_spec_for_another_pane_is_usage_before_any_write ... ok
test pane::profile_scope::tests::restore_conflict_names_the_act_error_and_the_reconcile_step ... ok
test pane::profile_scope::tests::first_write_timeout_says_the_edit_may_have_landed ... ok
test pane::profile_scope::tests::restore_failure_keeps_its_code_and_names_the_unrestored_edit ... ok
test pane::profile_scope::tests::store_scope_passes_the_profile_scope_conformance_suite ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
   (RED was 3 passed; 4 failed. The four now pass: AC 5's equalities and AC 2-4's step substrings)
$ cargo test -p holler-pane                      -> every suite ok; lib (probe::tests) 12 passed in 0.53s; ports_test 6 passed
$ cargo test -p holler-pane --lib probe::tests -- --test-threads=1    -> 12 passed, finished in 1.39s
$ cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success    -> 1 passed (AC 8m)
$ cargo test -p holler-cli --lib                 -> 16 passed; 0 failed
$ cargo test -p holler-pane-testkit              -> 202 passed; 0 failed
$ cargo test -p holler-cli --test docs_cli_test  -> 3 passed; 0 failed
$ HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
exit 0; 131 suites: 1494 passed, 0 failed, 5 ignored
$ cargo clippy --workspace --all-targets -- -D warnings      -> Finished, exit 0
$ bash scripts/lint.sh                           -> exit 0; one warn for this story:
warn: crates/holler-cli/src/pane/profile_scope.rs is 605 lines        (accepted by AC 10; under the 900 gate)
$ bash scripts/changelog-check.sh                -> changelog-check: ok
$ cargo machete                                  -> cargo-machete didn't find any unused dependencies in this directory. Good job!
$ rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs crates/holler-cli/src/pane/profile_scope.rs   -> exit 0
$ cargo doc -p holler-cli -p holler-pane --no-deps   -> no warning in either changed file
$ wc -l   -> 605 crates/holler-cli/src/pane/profile_scope.rs; 577 crates/holler-pane/src/probe.rs
AC 5  (profile_scope.rs above #[cfg(test)], comments excluded): 'holler pane doctor' -> 0; 'doctor_command(None, false)'
      -> 1; RECONCILE_STEP_UNSCOPED in the file -> 0
AC 9  (probe.rs above #[cfg(test)], comments excluded): forbidden tokens -> nothing; Command::new -> 2; shell shapes ->
      nothing; #[cfg(test)] -> 1
AC 11 (working tree vs the merge base 0ad2d8a): added non-comment lines with \bunsafe\b -> nothing; Cargo.toml/Cargo.lock
      -> nothing
AC 12 (working tree vs 0ad2d8a): CHANGELOG.md, the two .rs files, docs/adr/ADR-0021.md and docs/handoffs/663* only
AC 14 (working tree vs 0ad2d8a): hunks at a (section 1), b (section 2), the fence bullet, steps 2, 5 and 6, and f (section
      12) only; no ^[-+]| and no ^[-+]# line; (#663) in every hunk; the added text holds the exact step text,
      doctor_command, try_wait and 1 MiB; grep -cE 'RECONCILE_STEP_UNSCOPED|takes one \(#647\)|for now the' -> 0; the awk
      of section 12's first paragraph holds "No verb leaves work running after it exits", "section 1" and "(#663)";
      no added line over 124 columns
Tests untouched: sha256 from #[cfg(test)] to the end of each file equals HEAD's (d21d0961... and 3512b7ca...)
```

The workspace run points `HOLLER_STATE_DIR` at a scratch directory so that no test reaches a hub this host may run on
the default control socket. That is the same reason the previous run gave. CI runs no hub.

## Evidence appendix

`docs/handoffs/663/evidence.md`. This run adds two sections:

- **The reconcile step's builder** (copied from the branch):
  - `doctor_command(None, false)` is exactly `holler pane doctor` (`findings.rs:36, 306-316`);
  - `findings` is public (`lib.rs:42`);
  - a pane-scoped doctor refuses a pane with no record (`reconcile.rs:226-235`).
- **The group kill on macOS** (W-16), copied from `origin/main` at `dc300ab` (#705):
  - `exec.rs:32-34` runs the same `kill -s KILL -- -<pgid>` form;
  - `server.rs:106, 169-172` puts the server in its own group, then kills the group, then the child;
  - the test `hermetic_test.rs:638-671` is not opt-in;
  - the macOS job's log line shows it passed (run `38013074383`, job `114099775442`, 19:27:00 MDT).

These files are not on the branch, so the gate may not attach them. Their blobs are identical at #705's CI head `4155e06`
and at `dc300ab`.

## Tests that look wrong (for T)

None. All 7 scope tests and 12 probe tests pass as written, and none was edited.

## Known issues

1. **`origin/main` moved twice since the branch's merge base `0ad2d8a`:**
   - `dc300ab` (#705, 19:43 MDT);
   - `e327569` (#706, #641's host adapter, 20:06 MDT).

   `git merge-tree --write-tree HEAD origin/main` conflicts in `CHANGELOG.md` only (keep both entries). Neither commit
   touches `ADR-0021.md`, `profile_scope.rs` or `probe.rs`. The merge step resolves this; F does not merge.
2. **#706 merged a third private group-kill runner,** `crates/holler-adapter-host/src/exec.rs:202` (`pub(crate) fn
   kill_group`, `kill -s <SIG> -- -<pgid>` with `LC_ALL=C`). It is the copy A's W-16 named as "in flight".
   - `holler-pane` cannot use it, for the same dependency-direction reason as #705's, so W-16's ruling stands.
   - **For O (Phase 11):** the #696 comment should name it as merged.
3. **`profile_scope.rs` is 605 lines,** so `lint.sh` prints its 600-line `warn:`. AC 10 accepts this and T-green journals
   it. The file has 295 lines of room under the 900-line gate.
4. **W-17 is documented, not fixed.** For a profile name with a leading `-`, the printed step does not parse (clap reads
   the name as an option). The rustdoc says so. The fix is O's follow-up in `ProfileName::parse`.
5. **`FakeProfileScope` and `StoreScope` still differ in three messages** until F1, as the brief's Risks say. The codes
   are equal on every path.

## Files changed

- `crates/holler-cli/src/pane/profile_scope.rs`
- `crates/holler-pane/src/probe.rs` (rustdoc only)
- `docs/adr/ADR-0021.md`

On the branch from the previous run, unchanged in this one: `CHANGELOG.md`.

Pipeline files, not production: `docs/handoffs/663/handoff-F.md`, `docs/handoffs/663/evidence.md` and the
`docs/handoffs/663/decisions.md` entry.
