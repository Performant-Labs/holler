# Handoff-F: Phase 5 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `8fe683b`, plus this phase's uncommitted changes; F does not commit)
**Issue:** #663

## What was done

- `crates/holler-cli/src/pane/profile_scope.rs`: T's RED stub is replaced by the real scope. The file now has:
  - module docs (the I8 order, the decisions and the bound of Decision 11);
  - `RECONCILE_STEP_UNSCOPED` and `reconcile_step`;
  - `StoreScope` and `StoreScope::new`, and `impl ProfileScope for StoreScope`;
  - private helpers: `stored`, `members`, `member`, `plan`, `restore`, `may_have_landed`, `belongs`,
    `check_filed_under`, `check_joins`, `with_edit`, `single_quoted` and `with_context`.

  T's `mod tests` is byte-identical: the sha256 from the blank line before `#[cfg(test)]` to the end of the file is
  `d2645ea0...` before and after.
- `crates/holler-pane/src/probe.rs`: the stub body and its module docs are replaced by the runner:
  - `run_probe`, which checks its arguments;
  - the private mechanics: `run_bounded`, `spawn`, `start_reader`, `read_capped`, `wait_for_exit`, `kill_and_reap`,
    `kill_group` and `wait_until`;
  - the private verdict: `verdict`, `error` and `contains`;
  - three consts: `MAX_OUTPUT`, `CLEANUP` and `POLL`.

  `ProbeResult` is unchanged, and T's `mod tests` is byte-identical (sha256 `3512b7ca...` before and after).
- `CHANGELOG.md`: one `### Enhancements` entry under `## [Unreleased]`, linking #663 (AC 13).
- `docs/adr/ADR-0021.md`: the five in-place amendments of AC 14 (a-e), each cited `(#663)`.
- `docs/handoffs/663/evidence.md` (new): the evidence appendix.

## Design decisions

1. **The scope's messages, as built.** For the fixture of AC 2-4, `Demo Alpha` and `demo-c1r1`, they read:
   - First-write `timeout` (Decision 6): `timed out: profile_store.cas_put; the write may have landed, so profile
     "Demo Alpha" may hold the edit of demo-c1r1, and nothing live was changed; to reconcile, run holler pane doctor
     --profile 'Demo Alpha' and then holler profile show 'Demo Alpha'`
   - A restore that fails other than by a conflict (Decision 5): `<store's own text>; profile "Demo Alpha" still holds
     the edit of demo-c1r1, but the live change or its record failed (unavailable: act); <step>`. After a `timeout` it
     says `may still hold` instead (W-7 (a)).
   - The restore conflict (Decision 7): `profile conflict: "Demo Alpha" was changed by another writer during the live
     change to demo-c1r1, so its specs were not restored after that change or its record failed (unavailable: act); the
     other writer's version stays; <step>`.

   The name in prose is `{:?}`, as the fake writes it, and only the step's command lines are shell-quoted. Every name
   printed is the stored record's spelling: the store finds a profile by slug (evidence.md).
2. **"the live change or its record failed"** (W-9, which the review offered as optional; taken). With `--profile`, the
   record step runs inside the act (#644's row R), so "the live change failed" would be false after a pane-record
   conflict. The new wording is true in both cases. The conflict text changes "after that change failed" to "after that
   change or its record failed", the smallest edit of the fake's text, so F1 stays a small alignment.
3. **`with_context` is one `match &mut error`.** It has two arms:
   - the 18 single-string variants bind one name, `text`, through an or-pattern (`message: text`, `what: text`,
     `op: text`, and `Refused { message: text, .. }`), and the context is appended in place;
   - the 5 payload-less variants come back unchanged.

   There is no `_` arm, so a new variant fails to compile there. The grouping mirrors `error.rs`'s `detail()`.
   Rejected alternative: rebuilding each variant in 23 arms, which is the same rule in about 25 more lines and would have
   pushed the file past 600.
4. **`reconcile_step` is `RECONCILE_STEP_UNSCOPED` plus `--profile '<P>' and then holler profile show '<P>'`.** The
   shared prefix is written once. `single_quoted` is `format!("'{}'", text.replace('\'', r"'\''"))`, Decision 8's
   character-for-character rule.
5. **Step 2's error is a free function, `may_have_landed`, mapped on the first `cas_put`.** That keeps `edit_spec` a
   readable list of the steps. Only `PaneError::Timeout` is extended (Decision 6); every other error passes through
   exactly (AC 4's `unavailable` case and case 10's `generation-conflict`).
6. **Probe: the mechanics answer a private `Outcome`, and `verdict` alone maps it to `ProbeResult`** (Decision 20). The
   variants are `Exited { status, stdout }`, `TimedOut`, `Overflow`, `SpawnFailed(io::ErrorKind)`, `ReaderFailed`,
   `ReadFailed` and `WaitFailed`. The brief's "read error" is split three ways so that each failure gets its own fixed
   reason. The module docs name the three points #696 parameterizes: stderr, the signal and its grace, and the kill
   program.
7. **The pid-reuse rule is coded as one invariant.** The runner signals only while `try_wait` has not yet returned the
   leader's status:
   - `run_bounded` kills from every path before stdout has ended, because nothing has reaped the leader yet;
   - `wait_for_exit` kills only when `wait_until` answers `Ok(None)` (still running at the deadline);
   - a `try_wait` error sends **no** signal and answers `WaitFailed`, because the child may have been reaped elsewhere
     and its pid reused;
   - `kill_group` kills the `kill` process itself only on `Ok(None)` too.

   The module docs state the rule at the code (the brief's pid-reuse risk).
8. **The reader** is `ChildStdout::take(MAX_OUTPUT + 1).read_to_end`. One byte past 1 MiB is enough to know the cap was
   passed, memory stays bounded, and EINTR is retried by std. The thread is named `holler-probe-stdout` and is
   detached: its `JoinHandle` is dropped in `reader.ok().map(|_detached| receiver)`, so no kill path ever joins it
   (Decision 15).
9. **Platform guards (W-10, taken).** `process_group(0)` and the `kill` spawn sit in `#[cfg(unix)]` blocks, as
   `make_own_process_group` does. On other platforms only `Child::kill` runs, the path Decision 16 already has for a
   missing `kill`. AC 9's count of two `Command::new` is unchanged.
10. **Timing.** The deadline is taken in `run_probe` before the spawn. `recv_timeout` gets the time left to the
    deadline, the polls sleep `min(left, 10 ms)` so they never overshoot it, and the cleanup budget is
    `Instant::now() + 1 s`, checked (`unwrap_or_else(Instant::now)`). Measured here: `probe::tests` takes 0.53 s in
    parallel and 1.38 s serially.

## Reuse / extend-vs-new

Per the brief's Reuse map, with no new object beyond what it names:
- **Extended (implemented):** `holler_pane::ProfileScope`, the frozen trait, by `StoreScope` in the file ADR-0021
  section 5 names.
- **Reused:**
  - `ProfileStore` and `PaneStore`, the scope's only I/O;
  - `Argv::as_slice`;
  - `Prober` and `SystemProber`, unchanged (they call the new free `run_probe`);
  - the test kit's suite, fakes and fixture, from T's tests only.
- **Re-implemented on purpose:** `FakeProfileScope`. Production code cannot depend on the test kit (ADR-0021 section 5).
- **The third private copy of the membership rule:** `check_joins`, accepted as W-5. It uses the hub's exact text shape,
  `"{} is in profile {:?}, not {:?}"`, and compares slugs. F2 hoists one copy.
- **New and private to `probe.rs`:** the bounded runner. No public item is added to `holler-pane`; #696 exposes the
  runner later.

Phase 7 pre-checks, all true:
- `PaneInOtherProfile` is produced in `crates/holler-cli/src` only by `check_joins` (`profile_scope.rs:233`). Line 284 is
  a pattern inside `with_context`.
- `profile_scope.rs` has one formatter (`reconcile_step`), one const, one quoting function (`single_quoted`) and one
  append helper (`with_context`, with no `_` arm).
- `probe.rs`'s only `pub` items are the existing `ProbeResult` and `run_probe`.
- `probe.rs` spawns only `argv[0]` and `kill`, with no `libc` and no `unsafe`.
- The ADR diff touches only places a-e, with no table row and no heading changed.

## Architecture notes for A

- **Layers.** `StoreScope` is CLI-side (`holler-cli/src/pane`) and works only through the `holler-pane` ports.
  `run_probe` is `holler-pane`'s one direct side effect, as ADR-0021 section 5 already said; it uses std only.
- **Public interface.** Against `origin/main`, `holler-cli` gains `StoreScope`, `StoreScope::new`, `reconcile_step` and
  `RECONCILE_STEP_UNSCOPED`, exactly the signatures of Decisions 1 and 8 that T's stub already declared. `holler-pane`'s
  public surface is unchanged.
- **Dependencies and wire.** No new dependency and no manifest change. No wire or schema change: `ProbeResult` keeps its
  shape, and only what `Pane.probe.last` will hold changes.
- **New run-time dependency.** On Unix the runner spawns the `kill` program from `PATH`. Both Linux and macOS ship it.
- **ADR changes.** Sections 1, 2 and 8 are amended in place. Section 2 now narrows the I5 bound for `edit_spec` and
  `run_probe`, and qualifies section 12's per-call bound by cross-reference; section 12 itself is not edited (W-8).
- **Self-report: `archChanged: true`.** The cycle adds public API to `holler-cli` and amends ADR-level rules. All of it
  was planned and reviewed at Phase 3; this follows the role's "when in doubt" rule.

## Deviations from spec / wireframe

No wireframe (no UI). The deviations from the brief are all small and all recorded here:

1. **Decision 5's text.** `may still hold` after a timed-out restore (W-7 (a)), and "the live change or its record
   failed" in place of "the live change failed" (W-9). AC 2 asserts neither phrase.
2. **Decision 7's text.** "after that change or its record failed" in place of "after that change failed" (W-9). AC 3
   and case 11 assert neither phrase.
3. **Decision 19 gains two fixed reasons**, both echoing nothing:
   - `the probe timeout is too large`: a timeout that `Instant::now().checked_add` cannot represent is refused before
     any process starts, like a zero timeout. The alternatives were a panic on `Instant + Duration` overflow, or an
     unbounded wait.
   - `the probe's exit status could not be read`: a `try_wait` error after stdout ended (design decision 7).

   The rustdoc's verdict list names the first.
4. **Decision 16, "killed and reaped itself if that budget runs out".** A `kill` process still running past the budget
   is killed, then reaped by one non-blocking `try_wait`. If it has not ended by then it is left, a zombie, for the
   caller's exit, the same fate Decision 15 gives the leader. Blocking to reap it would break the 1 s bound. It happens
   only if `kill` itself hangs for a second.
5. **AC 14's ADR text, with the review's warns taken:**
   - (b) cross-references section 12 (W-8 (1)) and says "a follow-up amends the frozen trait docs" with no number
     (W-12 (2));
   - (c) states the rule by outcome and adds the requirement on #649's store client (W-7 (b));
   - (d) uses W-7 (c)'s timeout wording;
   - (e) names the three errors that carry the step (W-8 (2)), and the fence bullet gains W-9's clause.

   In (a), the existing sentence "until #663 fills it, the stub answers `Error`" is left as it is (merge hygiene: add a
   sentence, never rewrite one); the new sentence after it states the runner's rule.
6. **The "names no pane" sentences in the ADR and the rustdoc read correctly whatever the merge order** (Known issue 2).
   Step 6 says the step "names no pane, so the doctor run covers every pane of P, and naming the pane once `pane doctor`
   takes one (#647) is a follow-up". The fence bullet says "for now the profile-scoped or the bare form of step 6".

## Tier 1 self-check (incl. tests now GREEN)

```
$ rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs crates/holler-cli/src/pane/profile_scope.rs
exit 0
$ wc -l
  573 crates/holler-pane/src/probe.rs
  597 crates/holler-cli/src/pane/profile_scope.rs
$ bash scripts/lint.sh        -> exit 0; neither file in its warnings (both < 600)
$ bash scripts/changelog-check.sh
changelog-check: ok
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.20s     (exit 0)
$ cargo test -p holler-cli --lib pane::profile_scope
test pane::profile_scope::tests::reconcile_step_single_quotes_the_profile_name ... ok
test pane::profile_scope::tests::set_of_a_spec_for_another_pane_is_usage_before_any_write ... ok
test pane::profile_scope::tests::pane_store_fault_fails_a_remove_before_the_profile_write ... ok
test pane::profile_scope::tests::restore_conflict_names_the_act_error_and_the_reconcile_step ... ok
test pane::profile_scope::tests::first_write_timeout_says_the_edit_may_have_landed ... ok
test pane::profile_scope::tests::restore_failure_keeps_its_code_and_names_the_unrestored_edit ... ok
test pane::profile_scope::tests::store_scope_passes_the_profile_scope_conformance_suite ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
$ cargo test -p holler-pane --lib probe::tests
test probe::tests::empty_argv_and_zero_timeout_are_errors_without_a_process ... ok
test probe::tests::no_expect_and_exit_zero_is_ok ... ok
test probe::tests::one_missing_string_is_failed_naming_it ... ok
test probe::tests::non_utf8_output_still_matches ... ok
test probe::tests::argv_is_never_given_to_a_shell ... ok
test probe::tests::reasons_never_echo_argv_or_output ... ok
test probe::tests::all_expected_strings_present_is_ok ... ok
test probe::tests::non_zero_exit_is_error_even_with_every_string ... ok
test probe::tests::output_over_the_cap_is_error ... ok
test probe::tests::hung_command_is_error_at_the_timeout ... ok
test probe::tests::background_child_holding_stdout_is_a_timeout ... ok
test probe::tests::timeout_kills_the_whole_process_group ... ok
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
$ cargo test -p holler-pane --lib probe::tests -- --test-threads=1      -> 12 passed, finished in 1.38s
$ (10 consecutive runs of cargo test -p holler-pane --lib probe::tests)  -> 10 passed runs, 0 failed
$ cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success
test result: ok. 1 passed; 0 failed; ...                                 (8m, the regression guard)
$ cargo test -p holler-pane / -p holler-cli --lib / -p holler-pane-testkit / -p holler-cli --test docs_cli_test
all ok (holler-cli --lib: 16 passed)
$ HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
125 suites: 1402 passed, 0 failed, 5 ignored                             (exit 0)
$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!
AC 9  (probe.rs above its #[cfg(test)], comments excluded):
      forbidden-token grep -> nothing; Command::new count -> 2; shell-shape grep -> nothing; #[cfg(test)] count -> 1,
      and the test module is the file's last item
AC 11 git diff origin/main -- crates | added non-comment lines with \bunsafe\b -> nothing;
      git diff --name-only origin/main -- '*Cargo.toml' Cargo.lock -> nothing
AC 12 git diff --name-only <merge-base 3bdd129> -> CHANGELOG.md, crates/holler-cli/src/pane/profile_scope.rs,
      crates/holler-pane/src/probe.rs, docs/adr/ADR-0021.md, docs/handoffs/663* only
AC 14 hunks at ADR lines 80, 88, 275, 293, 298-301 only; no ^[-+]| line, no ^[-+]# line; (#663) in every edit; the
      added text holds the exact step text, RECONCILE_STEP_UNSCOPED and 1 MiB; added lines <= 124 columns
leftovers: no hlr-probe-663-* directory under the temp dir; no `sleep 30` or `yes` started by the tests
```

**Why the workspace run sets `HOLLER_STATE_DIR`.** Without it, four tests of `crates/holler-cli/tests/logging_test.rs`
fail on this host:
- `banner_names_resolved_level_and_format`
- `debug_flag_beats_env`
- `env_none_loses_to_flag_noisy`
- `log_output_stays_off_stdout`

Each runs `holler roster` and expects it to fail for lack of a hub, but this host runs a hub on the default control
socket, so `roster` succeeds. With `HOLLER_STATE_DIR` pointed at a scratch directory all 11 pass. These tests and that
behaviour predate #663; CI has no hub.

## Evidence appendix

`docs/handoffs/663/evidence.md`. It holds 10 facts from unchanged code:
- `PaneError`'s variants and payloads, `Refused.message`, and the `Display` prefixes;
- the failure class of the codes kept;
- `cas_put` returning the bumped record;
- `ProfileName` refusing control characters;
- the fake store's lookup by slug, and its call log counting failed calls;
- `SystemProber` calling the free `run_probe`.

## Tests that look wrong (for T)

None. All 19 authored tests pass as written, and none was edited.

One note for T-green: run the workspace gate with `HOLLER_STATE_DIR` set to a scratch directory. Otherwise
`logging_test.rs` fails on any host with a live hub (see above), and its `roster` calls also reach that hub, read-only.

## Known issues

1. **`origin/main` moved during this run**, to `e612878` (#701, #647 part 1). The branch is still based on `3bdd129`,
   so it needs a rebase before the PR merges. An in-memory `git merge-tree` of this phase's working tree against
   `origin/main` shows:
   - `docs/adr/ADR-0021.md` auto-merges (#701's hunks are in sections 11 and 12 and "Deferred");
   - **`CHANGELOG.md` conflicts.** #701 also appended its entry right after #688's. The fix is to keep both entries.
2. **#701 gave `pane doctor` a `[PANE]` positional** (ADR 0003 now reads `holler pane doctor [PANE] [--fix] [--profile
   NAME]`). The step stays `holler pane doctor --profile '<P>'`, which on that main still runs and checks every pane of
   P. AC 5 pins its exact text, and Decision 8 chose it while `pane doctor` had no positional. Naming the pane would mean
   a `reconcile_step` that takes the pane, a new AC 5 text and a change to #644's planned call. **Follow-up for O**,
   beside F5.
3. **`profile_scope.rs` is at 597 lines**, three under the 600-line warning. T's test module is 296 of them. A later
   addition to this file should come with a trim.
4. **`FakeProfileScope` and `StoreScope` differ in the three messages until F1**, as the brief's Risks say. The codes
   are equal on every path.
5. **The first macOS evidence is CI's macOS job.** That covers the `kill -s KILL -- -<pgid>` form (8f, 8g), `printf`'s
   octal escapes (8l), and the timing bounds (8e-8g).

## Files changed

- `crates/holler-cli/src/pane/profile_scope.rs`
- `crates/holler-pane/src/probe.rs`
- `CHANGELOG.md`
- `docs/adr/ADR-0021.md`

Pipeline files, not production: `docs/handoffs/663/handoff-F.md`, `docs/handoffs/663/evidence.md` and the
`docs/handoffs/663/decisions.md` entry.
