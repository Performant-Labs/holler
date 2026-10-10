# Decisions — #647 the reconcile engine and `holler pane doctor`

## A (Phase 3, up-front plan review) — 2026-10-09T17:23:37-06:00
- **Decided:** PASS on docs/handoffs/647-brief.md at a98258a (sha256 0f98bbc7b52e5c78...), with 0 blocks and 7 warns (see
  handoff-A.md).
  - The objects and layers are right:
    - a pure engine and finding types in `holler-pane` (ADR-0021 §5), working only through `Ports`;
    - a thin verb in its own file;
    - closed codes only, with exit codes from `class_of`;
    - scoping through `ProfileScope::resolve`;
    - record writes only on change (`panes/mod.rs:27-28`).
    No frozen file is edited, and the only copy of something already on `main` is the clock read (W-3).
  - W-1 (the most consequential): reconcile becomes the writer of `harness.health` and defines what `last_observed.at` and
    `shown: None` mean. #643, #645, #646 and #648 read those fields, and #643 reads them differently. The meanings would
    live only in this brief, because AC 28 forbids any other ADR-0021 edit.
  - W-2: the SHOWN-vs-record comparison should be one `pub` pure function in `reconcile.rs`, the only unfrozen place the CLI,
    the hub and the engine can all import.
  - W-3: use `holler_proto::clock::now_millis()`, not a hand-written `SystemTime::now()` read.
  - W-4: make the doctor-command builder `pub`, since #644 and #663 build the same command line in flight. On C-8, never
    print raw OpenCode, Herdr or tmux commands.
  - W-5: three vocabulary problems:
    - `FindingKind` keeps two sources (serde derive and `code()`);
    - `ObservedHealth` spells an observation differently from the finding codes and the record;
    - `observe-failed` has two meanings.
    Also, #650's findings have no Forward-compat line.
  - W-6: test placement and shared test helpers:
    - AC 25 belongs in `holler-pane/tests/`;
    - reuse the rig that merges first;
    - AC 26 should call `try_parse`;
    - one text-escaping helper.
  - W-7: two blind spots of the frozen ports to record as risks: a foreign TUI on a shared data directory, and strays that
    accumulate.
- **Assumed:**
  - The briefs and A handoffs in `.claude/worktrees/{0643-read-verbs,0644-launch-relaunch,0663-profile-scope-probe}` are the
    current plans of those stories. #644's brief was being amended in its working tree while I read it, so I cite its
    content rather than its line numbers.
  - None of these plans is merged. I used them as evidence of what is being planned, not as patterns.
  - The issue (#647), the epic (#633), ADR-0021, ADR 0003 and the merged code are the authority.
- **Hedged:**
  - W-1 is a warn, not a block. The ADR is silent on these meanings rather than contradicting the plan, and the frozen docs
    are themselves ambiguous: `pane.rs:188` says "When it was observed", `ports.rs:198` says "if it can tell", and the fake
    says the home screen (`harness.rs:135-136`).
  - This also differs from #642's B-1. That block was an item the ADR deferred to the story by name, while the brief
    forbade the edit. Here the only item deferred by name (the hub timer) is handled.
  - #643's A rated the same `shown: None` question a warn, so the rating is consistent.
  - W-2, W-4 and W-6(b, d) are forward duplication between unmerged plans. There is no dominant pattern on `main`, so per
    the role rule they are warns. At Phase 7 I will reject a second rig or text helper if one is on `main` by then.
  - W-3 is the only finding about an object already on `main`. It stays a warn because the fix is one call and F reads this
    handoff. #644's A rated the same point a warn.
  - W-7(a) rests on the documented fake (one shared data directory) and on `shown_session`'s signature, not on a run of the
    real adapter.
- **Evidence:**
  - Read in full:
    - the brief and its two outside-review results (r1, r2);
    - `holler-pane/src/{lib,ports,pane,error,profile,grid,reply,probe,generation}.rs` and the stubs;
    - `holler-cli/src/output.rs`, `pane/{mod,args,wiring,doctor,relaunch,reset,switch,list,get,launch,close}.rs`;
    - `tests/verb_harness/{mod,parse}.rs` and `tests/pane_verbs/main.rs`;
    - `holler-pane-testkit/src/{lib,fixture}.rs`;
    - `docs/adr/ADR-0021.md` and `docs/adr/ADR-0003.md`.
  - Also read:
    - `holler-hub/src/panes/mod.rs:1-60` and `holler-proto/src/{clock,error}.rs` (excerpts);
    - the sanitizers in `hub_cmd.rs`, `acp_driver/auth.rs`, `log.rs`, `holds.rs` and `adapter-herdr/src/protocol.rs`;
    - `holler-pane/tests/{ports_test,error_test}.rs` (headers and the code-table test);
    - `tests/pane_verbs/process/{docs_rows,stub}.rs` (excerpts);
    - the test kit's public API and op names;
    - `scripts/lint.sh` and `clippy.toml`;
    - `crates/holler-cli/Cargo.toml` and `crates/holler-pane/Cargo.toml`.
  - Sibling plans: `643-brief.md` (Decisions 10-12) and `643/handoff-A.md`; `644-brief.md` (excerpts) and
    `644/handoff-A.md`; `663-brief.md` (excerpts); `642/decisions.md` (for the journal format).
  - Issues read with `gh issue view`: 647, 633 (with comments), 644, 645, 646, 648, 649, 650, 665, 694.

## T (Phase 4, author tests / RED) — 2026-10-09T17:34:29-06:00
- **Decided:**
  - **The scaffold.** Following the brief's test plan, T landed only the Decision 10 types and the Decision 11
    args, so that RED fails on an assertion and not on a compile error: `findings.rs`, `reconcile.rs` (`reconcile()`
    returns `NotImplemented`) and `doctor.rs` (still the stub `run`).
  - **The suite.** It is 31 tests in `pane_verbs` (`doctor.rs`, `doctor/read_only.rs`, `doctor/surface.rs`, built on
    `doctor/rig.rs`) plus one unit test.
  - **AC 25's placement.** It lives in `crates/holler-pane/tests/findings_test.rs`, per A's W-6(a): it needs no fake,
    and that is the cheapest sufficient tier.
  - **AC 26's parser.** It parses through `verb_harness::parse::try_parse`, per A's W-6(c).
  - **AC 24's order.** It reads the in-flight maximum before it unwraps the result, so its RED shows the invariant
    (`left: 0`).
  - **One test outside the numbered ACs:** `record_write_conflict_is_reported_not_retried` (Decision 6, R-6).
  - **The test files are split** so that each stays under lint's 600-line warning.
- **Assumed:**
  - The fakes' documented semantics are the contract (C-2): `list_sessions` answers `timeout` on a frozen server and
    `unavailable` on a killed one, `shown_session` never reaches the server, and every port shares the `"default"`
    data directory until `set_data_dir`.
  - Remedy strings are exactly Decision 3's.
  - A finding's `session` is pinned only where an AC names it: the shown S2 in AC 1, and the foreign session in AC 3.
  - Exact-set assertions (for example, AC 2 is exactly `[server-wedged]`) follow from Decision 4: an unhealthy
    server's list is not trusted.
- **Hedged:**
  - The added `findings_test.rs` is not in the brief's Files list or AC 33. O should amend both, as A's W-6(a)
    suggests.
  - `ObservedHealth`'s value spellings and the text summary line are deliberately unpinned, so F can act on A's
    W-5(b).
  - The scaffold keeps the serde derive on `FindingKind`, as the brief specifies; A's W-5(a) is F's to take.
- **Evidence:**
  - RED run: `cargo test -p holler-cli --test pane_verbs doctor` gives 0 passed and 31 failed. The failures:
    - 21 engine tests: `reconcile must complete the pass and report: not implemented`;
    - AC 24: `left: 0 right: 4`;
    - the verb tests: exit `1` where `0` is expected, or `not-implemented` where `unavailable` is expected.
  - Passing at RED:
    - `findings_test`: 1 passed;
    - `cli_surface_test`, `docs_cli_test` and `pane_cli_process`: 3, 3 and 34 passed;
    - the other 63 `pane_verbs` tests.
  - Clean at RED: clippy (`-D warnings`), `lint.sh` (exit 0), `rustfmt --check` on the touched files, and gitleaks.

## F (Phase 6, implement) — 2026-10-09T17:58:06-06:00
- **Decided:**
  - **The engine.** `reconcile.rs` holds the API, the fan-out and the fleet-level rules; `reconcile/observe.rs` (the
    brief's named fallback) holds one pane's chain: observe, compare, repair, record. The split was made up front: the
    two together are about 830 lines.
  - **Threads.** `thread::Builder::spawn_scoped`: a thread that cannot start or panics is an `observe-failed`, never a
    panic of the pass (Decision 9). One thread runs the two Herdr calls, one runs each pane's chain.
  - **The fix.** It acts, then observes again even after a failed select; the re-observation is what is recorded.
    `fixed` needs a successful select and the session of record shown afterwards. A select that was acknowledged while
    the TUI still shows another session is `failed`, with `fix_error.code` `shown-driven-mismatch`, because ADR-0021
    leaves the closed code of a post-act mismatch to #644 and #645.
  - **Dedupe.** It is on the whole finding (`sort` + `dedup`), with the message as the last sort key. Decision 2's
    4-tuple would merge the two pane-less Herdr `observe-failed` findings that AC 21 requires.
  - **First observation.** A record at `last_observed.at == 0` gets its first observation written even when the values
    equal the defaults (A's W-1 edge case). After that, a record is written only on a change.
  - **A's warns, taken as written:**
    - W-2: a `pub` `shown_differs`;
    - W-3: `holler_proto::clock::now_millis()`;
    - W-4: `pub` `doctor_command` and `FindingKind::remedy` as the one remedy table;
    - W-5(a): `FindingKind` serializes from `code()`;
    - W-5(b): `ObservedHealth` is `healthy | server-wedged | server-down | unknown`;
    - W-5(c): `observe-failed`'s doc covers the record write;
    - W-6(d): `findings::quoted` is the one quoting helper, used by the CLI renderer too;
    - W-1: the meanings are in `reconcile.rs`'s module doc, and edit (c) uses A's wording.
- **Assumed:**
  - Decision 3's remedies and the fixture's documented semantics (C-2) are the contract, as T assumed.
  - `ports` and `herdr_pane` are filled only where the server or Herdr pane is the subject, since no AC pins them beyond
    the strays.
- **Hedged:**
  - ADR-0021 section 1's rows (W-1) are not edited, because AC 28 forbids it. O decides the amend channel.
  - AC 31's grep only works with paths made relative (the worktree path contains `reconcile`); recorded for T and S.
  - The 4 `logging_test` failures in the first `cargo test --workspace` are environmental: a live hub on this machine
    answers `holler roster`. They pass with an isolated `HOLLER_STATE_DIR`.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_verbs doctor`: 31 passed, 0 failed.
  - `pane_verbs` 94, `cli_surface_test` 3, `docs_cli_test` 3, `pane_cli_process` 34 and every `holler-pane` target
    passed.
  - `observation_runs_concurrently`: 20/20.
  - `HOLLER_STATE_DIR=<fresh dir> cargo test --workspace --no-fail-fast`: 126 targets, 1415 passed, 0 failed, 5
    ignored, exit 0.
  - Clean: clippy `--workspace --all-targets -D warnings`; `lint.sh` (exit 0); `changelog-check`; `rustfmt --check` on
    the touched files; `cargo machete`; no `unsafe`; no manifest diff; gitleaks.
  - The text output was checked by eye once, with a temporary print that has since been removed.

## T (Phase 7, verify / GREEN + Tier 2) — 2026-10-09T18:08:43-06:00
- **Decided:**
  - GREEN is valid: 31/31 RED tests pass on `9500955`, and F changed no test file.
  - I added one test, `acknowledged_select_that_switches_nothing_is_not_fixed`. A mutation spot-check showed that
    Decision 5's "equal to S is `fixed`, anything else `failed`" was unpinned: M6 survived all 31 tests. The new test uses
    the test kit's `Quirk::SelectAckedWithoutTui` and fails under M6. It is a suite gap, not an F defect.
  - I appended 3 test-kit facts to `evidence.md`: `select_session`, the fake Herdr's unsupported `version()`, and
    `FakeProfileScope::resolve`'s refusals.
- **Assumed:**
  - CI's `--skip roster_stays_accurate_under_concurrent_body_load` form of `cargo test --workspace` is the AC 30 gate.
  - An isolated `HOLLER_STATE_DIR` is a valid environment, as F found.
- **Hedged:**
  - `findings_test.rs` is still outside the brief's Files list (AC 33). It is left to O or S as bookkeeping, not
    routed to F.
- **Evidence:**
  - Tier 1 on `9500955`: lint 0, changelog 0, clippy 0, workspace 126 targets / 1414 passed / 0 failed, docs_cli 0,
    wire_selftest 0, machete 0, test-hooks 0.
  - After the test edit: `pane_verbs` 95/95, doctor 32/32, clippy clean, lint 0, rustfmt clean, gitleaks clean.
  - `observation_runs_concurrently`: 20/20.
  - Mutations M1-M5 and M7 are caught. M6 was not caught, and is caught after the new test.

## A (Phase 7, anti-duplication gate) — 2026-10-09T18:18:44-06:00
- **Decided:** PASS on `3bdd129..baf176f`, with 0 blocks and 5 warns (see handoff-A-dup.md).
  - F extended every object the brief's Reuse map names and copied none:
    - `Ports`, and `ProfileScope::resolve`;
    - `PaneStore` `list`, `get` and `cas_put`;
    - the closed codes, through `ErrorBody::from` and `emit`;
    - `GridPos`, `ProfileOpt`, and the name parsers;
    - `error::excerpt`, wrapped and not copied;
    - `now_millis` (W-3);
    - the test kit's fakes, `run_verb_with`, `check_envelope` and `try_parse` (W-6(c)).
  - Every new object is the brief's own (the engine, the findings, the named fallback submodules) or one Phase 3 asked
    for (`shown_differs`, `doctor_command`, the one remedy table, `quoted`, `findings_test.rs`).
  - The warns:
    - D-1: the doctor rig is private to `doctor`, and #643's branch has a second rig in the same `pane_verbs` binary;
    - D-2: three quoting rules (`quoted`; #643's `text_value`; the Herdr adapter's private `excerpt` copy on `main`);
    - D-3: #663 and #644 plan their own `holler pane doctor` spelling in `profile_scope.rs`;
    - D-4: `fix_error.code` holds a finding-kind code for a post-act mismatch, pending #644/#645's code (ADR-0021:536);
    - D-5: a fourth crate-local control-character sanitizer, with no shared one to extend.
- **Assumed:**
  - `origin/main` at `3bdd129` is current: `git ls-remote` agrees, so no sibling rig or helper has merged. I did not run
    `git fetch`, so that the remote-tracking refs other worktrees diff against stay unchanged.
  - The #643, #644, #662 and #663 branches and briefs are plans, not patterns. I used them only as evidence of forward
    duplication.
- **Hedged:**
  - D-1 to D-3 are warns, not blocks. No rig, quoting helper or reconcile-step builder is on `main`, so #647 cannot be
    folding onto one. The role rule is to block a parallel path to an existing object only. Whichever sibling merges
    second faces the fold at its own gate.
  - D-4 is a warn because ADR-0021 defers the post-act code by name, and the choice is documented and tested.
  - D-5 is a warn because the codebase has no dominant sanitizer pattern (role rule: cite the ambiguity, do not block).
  - Not re-rated, still with O: W-1's ADR-0021 §1 half, W-7, and AC 33's Files list.
- **Evidence:**
  - Read in full:
    - every changed code file: `findings.rs`, `reconcile.rs`, `reconcile/observe.rs`, `pane/doctor.rs`;
    - the tests: `pane_verbs/doctor.rs`, `doctor/{rig,read_only,surface}.rs`, `findings_test.rs`;
    - the diffs of `cli-surface.txt`, `process/stub.rs`, ADR 0003, ADR-0021 and the CHANGELOG;
    - the brief, and handoff-A, -T-red, -F and -T-green.
  - Neighbours read on `main`:
    - `holler-pane/src/{reply,lib,probe}.rs` and the stubs;
    - `output.rs` (lines 90-360), `pane/args.rs`, `pane/profile_scope.rs`;
    - `verb_harness/{mod,parse}.rs` and `pane_verbs/main.rs`;
    - the sanitizers in `log.rs`, `acp_driver/auth.rs`, `holds.rs` and `adapter-herdr/src/protocol.rs`.
  - Greps over `crates/`:
    - `Ports {` construction sites;
    - `impl *Port for`;
    - `check_envelope`;
    - call-log snapshot helpers;
    - `PaneNotFound`, `PaneRole::Orchestrator` and `pane doctor` literals;
    - control-character escaping;
    - personal infrastructure names in the diff.
  - Sibling branches inspected with `git show`:
    - `issue-643-implementation`: `pane_verbs/list.rs` and `src/pane/list.rs`;
    - `issue-662-implementation`: `profile_verbs/rig.rs`;
    - the #663 and #644 briefs (reconcile step).
