# Handoff-A: Phase 3 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (up-front plan review, fourth pass)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `9762a97`; merged with `origin/main` at `0ad2d8a` in `1d6a5ab`; `origin/main` has since moved to `dc300ab`)
**Brief reviewed:** `docs/handoffs/663-brief.md` at `9762a97` (sha256 `304b936c...`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" table (this run has no survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with no block and three warns (W-16 to W-18). This pass replaces the third pass's BLOCK (`71f9ae2`).

- **B-2 is resolved.** The brief now builds both forms of the reconcile step on #701's `findings::doctor_command(None, false)`,
  the doctor command line's one builder. The profile form composes on top of it in `profile_scope.rs`, and
  `RECONCILE_STEP_UNSCOPED` is gone. AC 5 greps for any second spelling, and AC 14e and F5 say the same.
- **W-13 to W-15 are resolved.** The brief now states the `try_wait` rule with the std source, the `Instant` fact and
  lint.sh's real size gate.
- **Main moved under the brief once more.** #705 (`dc300ab`, 19:43 MDT) merged a second private group-kill runner, in the
  OpenCode adapter. `holler-pane` cannot use it (dependency direction), so the plan's private runner still stands, and #696
  already plans the fold. That merge also produced the first macOS run of the `kill -s KILL -- -<pgid>` form, and it passed
  (W-16).
- **Two smaller warns.** A profile name that starts with `-` makes the printed step fail to parse (W-17). #662b plans a
  byte-identical copy of `single_quoted` in the same crate, which its brief already accepts as a follow-up (W-18).

Neither warn changes the plan, and F needs no amendment to start.

## Findings

Numbering continues from the earlier passes (B-1, B-2, and W-1 to W-15).

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-16 | warn | The Reuse map row "a bounded subprocess runner: none exists on `main`"; Decision 20's list for #696; Evidence H's macOS paragraph | duplication (cross-crate); dependency direction; forward-compat | **Main now holds a second private group-kill runner, and the row's premise is false.**<br>**What merged:** #705 (#642 part 1) merged at 19:43 MDT as `dc300ab`, after the brief's base `0ad2d8a` (19:32 MDT). It added `crates/holler-adapter-opencode/src/exec.rs`:<br>- `pub(crate) fn kill_group(pgid, op, bound)` (line 26) runs `kill -s KILL -- -<pgid>` through the `kill` program, the same form as `probe.rs`'s;<br>- a private `run(command, op, bound)` (line 48) polls `try_wait` to a deadline;<br>- `server.rs:106` puts the server in its own group with `process_group(0)`, and `server.rs:170` calls `kill_group(child.id(), ..)`.<br>**Why the plan still stands:** `holler-pane` cannot use it. The adapter depends on `holler-pane`, not the reverse; ADR-0021 section 5 fixes `holler-pane`'s dependencies; and the function is `pub(crate)`. #696's scope already counts it ("switch the host and OpenCode adapters to it, removing the private copies"). So this is not a parallel path. The row's justification still holds, only for a stronger reason than the one it gives.<br>**Where the two runners differ** (for #696, beyond Decision 20's three points):<br>- (a) `exec.rs:27` refuses a pgid of 0 or 1, since `kill` reads `-0` as its caller's own group and `-1` as every process it may signal. `probe.rs`'s `kill_group` has no guard. It needs none today, because it only ever receives a live child's pid, which is never 0 or 1. An exposed runner that takes any pgid does.<br>- (b) On a `try_wait` error, `exec.rs:68` still calls `child.kill()` and then `child.wait()`. Decision 15 forbids that signal for the probe, because the pid may have been reused.<br>- (c) `exec.rs:56` names the program and carries `io::Error`'s text (`cannot run {program}: {error}`). Decision 19 forbids both for a probe.<br>**The macOS evidence:** on #705's `test (macos-latest)` leg, `serve_kills_its_process_group_when_the_deadline_passes ... ok` (log line at 19:27:00 MDT; Actions run `38013074383`, job `114099775442`). The test is not opt-in, uses the same `kill` form, and asserts that a background `sleep 30` in the group is gone within 2 s. So Evidence H's "AC 8f and 8g ... are the first macOS evidence" is stale. That strengthens the plan, and it answers the outside brief gate's NV-7 (round 2). | **No plan change.**<br>**Ruling for F:** keep the runner private and std-only, as briefed. Do not import or copy `exec.rs`. Do not add (a)'s guard: it is unreachable in `probe.rs` and so untestable, and it belongs to #696.<br>**F or T-green:** record #705's macOS run (the run and job ids and the log line) in `evidence.md`, so the diff gate sees it.<br>**O (Phase 11):** add (a) to (c) to #696 as a comment. Name `holler-adapter-opencode/src/exec.rs` (merged) and `holler-adapter-host/src/exec.rs` (#641, in flight) as the private copies it folds, and Decision 15's no-signal rule as the one to keep.<br>**Optional for O:** the Reuse map row names `exec.rs` and the dependency-direction reason. |
| W-17 | warn | Decision 8's "Quoting" paragraph; AC 5; AC 14e (ADR step 6's "exactly") | pattern consistency; cross-cutting (printed command lines) | **The printed step does not parse for a profile name that starts with `-`.**<br>**The neighbour's rule:** `findings.rs:12-18` (Evidence C2) makes a remedy safe to print because a `PaneName` has "no space, shell metacharacter or leading `-`". Decision 8's single quotes cover the first two for a `ProfileName`. Its grammar allows the third: `ProfileName::parse` (`profile.rs:42`) refuses only an empty name, over 64 characters, a control character or an empty slug. So `-Demo` is valid (slug `demo`, `profile.rs:78`).<br>**The parser:** `--profile` is `#[arg(long, value_name = "NAME")] pub profile: Option<String>` (`holler-cli/src/pane/args.rs:21-25`), with no `allow_hyphen_values`. `profile show`'s `NAME` is a plain positional.<br>**Observed:** with clap 4.6.6 (the workspace's `Cargo.lock`), on a scratch copy of these argument shapes:<br>- `doctor --profile -Demo` is `UnknownArgument`.<br>- `doctor --profile --fix` is `InvalidValue` ("a value is required for '--profile <NAME>'"), so `--fix` never runs.<br>- `show -Demo` is `UnknownArgument`.<br>- `doctor --profile=-Demo` and `show -- -Demo` parse.<br>**Effect:** for such a name the step exits 2 and runs nothing. That is harmless, but it is not the "paste into a shell" promise. The same limit hits every `--profile` verb and #662b's planned `holler profile delete '<q>' --keep-panes` suggestion, so the root is the `ProfileName` grammar (#637's), not this story. | **No plan change.** The step's text stays.<br>**Optional for F:** one sentence in `reconcile_step`'s rustdoc: a name with a leading `-` is quoted, but clap still reads it as an option, so for that name the step does not parse.<br>**O, a follow-up (amend-first, `holler-pane/src/profile.rs`):** refuse a leading `-` in `ProfileName::parse`, the property `findings.rs:14` relies on for pane names. That fixes every `--profile` verb and keeps ADR step 6's "exactly" text true.<br>**Timing:** file it before #662b's `create` merges. No verb creates a profile on main yet. Serde reads a stored name back through `parse`, so once the rule lands, a stored name with a leading `-` would fail closed (`store-corrupt`).<br>**Not recommended:** printing `--profile=<P>` and `holler profile show -- <P>` instead, which changes AC 2-5's text, ADR step 6 and what #644 pins. |
| W-18 | warn | The Reuse map row for `findings::quoted` ("`single_quoted` stays, with one caller; no crate's `src` on main has a POSIX shell-quoting helper"); F5 | duplication (in flight, same crate); cross-story | **A second copy of `single_quoted` is planned in the same crate, and F5's #646 citation has moved.**<br>**The copy:** #662b (worktree `0662-profile-verbs`, branch at `76c774f`, T-red at 19:57 MDT) plans, in its Decision B1, `pub(crate) fn shell_word(text: &str) -> String` in `holler-cli/src/profile/delete.rs`. Its body is `format!("'{}'", text.replace('\'', r"'\''"))`, byte-identical to `single_quoted` (`profile_scope.rs:261`). Its brief records the overlap (lines 28-29): "#663 has a private `single_quoted` (E15); 662b adds its own `pub(crate)` copy. Folding them is a Follow-up, not a blocker." The Reuse map's statement is true of main today. Whichever story merges second creates the duplicate.<br>**F5's #646 reference:** `issue-646-implementation` is now at `ec2b6b3` (its own Phase 3 PASS, 19:58 MDT). The quote F5 names is at lines 521-526 there, not `d06e6ff:517-527`. It still quotes the deleted const, but harmlessly: 646a does not use the step, and 646b's brief is written only after #663 merges (`646-brief.md:31`). No branch has code calling the old API (#644 is brief-only at `7195993`, and #662b does not use the step). | **Pre-ruling for Phase 7:** #663's private `single_quoted` is not a block, whichever story merges first. Folding it into `profile/delete.rs` would make the shared pane helper depend on another story's verb file (ruling 2: one verb, one file, one owning story).<br>**O:** add a follow-up F6 that folds the two shell-word helpers into one, in a shared non-verb module of `holler-cli` (for example `output.rs`, beside `emit_error`). Name F6 in both stories' follow-ups.<br>**O, when it applies F5:** cite #646 at `ec2b6b3:521-526`. |

### The earlier passes' findings

- **B-2: resolved.** Each item of the third pass's "Notes for O" is in the brief:
  1. **The re-base.** The branch is merged with `0ad2d8a`, and the Evidence is re-based on it. C2 adds `findings.rs:12-18,
     36, 303-316`, ADR 0003 lines 61-68 and `reconcile.rs:219-243`. Each quote matches the branch.
  2. **The Reuse map** has a `doctor_command` row, marked **reuse**.
  3. **The profile form** composes on top, in `profile_scope.rs` (Decision 8; D-3's second choice).
  4. **The unscoped form** is option (i): `reconcile_step(profile: Option<&ProfileName>)`, with no const. AC 5 makes it a
     call, adds a regression-guard equality with `doctor_command(None, false)`, and greps for any second spelling. F5 has
     #644 and #646 call it.
  5. **AC 14e** names `doctor_command` as the builder, and gives C9's reason that the step names no pane on purpose. It
     drops "until #647" and F's follow-up sentence, which AC 14's grep checks.
  6. **AC 10** uses lint.sh's 900-line gate, and accepts the 600-line warning with a journal note.
  7. **W-13 and W-14** are covered below.
  8. **The optional fold** of W-7 to W-9's wording into Decisions 5 to 7 is done.
- **W-13: resolved.** The `try_wait` rule is now in:
  - Decision 15, as "A `try_wait` error sends no signal", with the diff gate's round-1 B-1 rejected and the reason given;
  - Decision 19, which lists both new reasons;
  - the Risks;
  - AC 14a, and edit f in section 12;
  - H2, which quotes std 1.98.1's `try_wait`, `Child::kill`/`send_signal` and `ExitStatus::code`.

  The rule still holds at `dc300ab`: no crate's `src` installs a `SIGCHLD` handler or calls `waitpid`. #705's
  `reap_when_it_exits` (`server.rs:177`) waits on its own `Child` only, so it cannot reap the probe's leader.
- **W-14: resolved.** Decision 15 rejects `saturating_add`, which std does not have (H2 lists `Instant`'s methods), and allows
  F to measure the budget with `elapsed()` instead.
- **W-15: resolved** (item 6 above).
- **W-5 stands:** it is accepted for Phase 7. The three private copies of the membership rule are hoisted by F2.
- **W-11 is still pre-ruled for Phase 7.** The probe tests' scratch directory and their bounded poll stay private and
  minimal.
- **W-12 (1) and (3) are O's, at Phase 11.**

### Also checked (no finding)

- **The composition adds no copy.**
  - No crate's `src` builds a `holler profile show` line; it appears only in doc comments.
  - `to reconcile` is spelled only in #663's own file.
  - `findings` remedies return bare command lines.
  - `holler-body`'s `quote_id` (`acp_driver/auth.rs:129`) puts a value in double quotes for display; it is not a shell
    word.
  - #662b's messages spell only its own verbs' lines (`holler profile delete ...`).
- **The re-entry RED premises hold at `9762a97`:**
  - AC 14's grep prints 3, and all three lines are in #663's own hunks (main's ADR-0021 prints 0, so the AC can be met);
  - AC 5's three greps print 1, 0 and 4;
  - AC 11 holds: no manifest change and no new `unsafe`;
  - AC 12 lists only the allowed files;
  - the file sizes are 597 and 573 lines.
- **The merge.** `git merge-tree` of the branch with `dc300ab` conflicts in `CHANGELOG.md` only.
- **Edit f** (section 12's last sentence, extended in place) does not overlap #644's planned section 12 edit. #644 adds a
  new paragraph after the operator's paragraph (`644-brief.md:2057` at `7195993`), and no in-flight story rewrites that
  sentence.
- **Decision 12's reason is imprecise; the decision stands.** `AssertUnwindSafe` would remove the need for an `UnwindSafe`
  bound. But the decision matches the codebase:
  - no crate's `src` calls `catch_unwind`;
  - the workspace's clippy lints deny panic paths;
  - ADR-0021 section 12 leaves a crash to the next doctor run.

  Only the wording is off. If the diff gate raises it, this is the answer.
- **No wire change.** `ProbeResult`'s shape is unchanged, so no golden file or protocol doc moves.

## Notes for O and F (PASS: nothing is required before T)

- **F:** build Decision 8 as briefed, and keep the runner as it is (W-16's ruling). You may take W-17's rustdoc sentence.
  Record #705's macOS run in `evidence.md` (W-16), or leave that to T-green.
- **O, at Phase 11:** file the W-17 grammar follow-up (before #662b's `create` merges) and F6 (W-18). Add W-16 (a) to (c) to
  #696. Cite #646 at `ec2b6b3:521-526` when applying F5.

## Patterns referenced

- `crates/holler-pane/src/findings.rs:12-18, 36, 303-316`: `doctor_command` and the rule that keeps a printed remedy safe.
- `crates/holler-adapter-opencode/src/exec.rs:26-76` and `server.rs:106, 170, 177-183`, at `dc300ab` (#705).
- `crates/holler-cli/src/pane/args.rs:21-25` (`ProfileOpt`) and `crates/holler-pane/src/profile.rs:42, 78`
  (`ProfileName::parse`, `slugify`).
- ADR-0021 sections 1, 2, 5, 8 and 12, on the branch and at `dc300ab`.
- The in-flight briefs: #662b at `76c774f` (lines 28-29, Decision B1); #646 at `ec2b6b3` (lines 31, 521-526); #644 at
  `7195993` (line 2057).
