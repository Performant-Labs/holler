# Handoff-S: #650 - Import today's panes from the launcher's files  (spec audit)

**Date:** 2026-10-11, 12:10 MDT
**Branch:** `issue-650-implementation` at `542b5dc`. The merge base is `7284001`. `origin/main` has since moved to `3286486`, with #644 and #642 part 3 merged.
**Issue:** [#650](https://github.com/Performant-Labs/holler/issues/650), read live with `gh issue view 650`.
- Its body is the text from before the brief gate's amendments.
- Its one comment, the operator's amendment of 2026-10-11 11:33 MDT, says the brief `docs/handoffs/650-brief.md` governs where the two differ, and that S audits against the brief.
- That comment settles three points: the health of a server that is not healthy, the JSON finding key `kind`, and the kept `// #650` stub line.
- I audited against the brief plus that comment.

**Handoffs reviewed:**
- `handoff-A.md` (the second plan review, PASS)
- `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`
- `handoff-A-dup.md` (PASS)
- `decisions.md`, `evidence.md`
- the outside model's diff gate, `docs/handoffs/650-diff-result-r1.md` (git-ignored; round 1 PASS, 0 block)

**Diff audited:** `git diff origin/main...HEAD` outside `docs/handoffs/`: 24 files and 4,234 added lines.
- 11 production files.
- 10 test files, the CLI surface fixture included.
- ADR-0003, ADR-0021 and `CHANGELOG.md`.

**Verdict:** PASS. Two actions remain for the run's agent before the merge, and neither is code:
- file acceptance 10's follow-up issue (Advisory 1);
- merge `main`, which touches docs only (Advisory 2).

## A precondition

Met.

- `handoff-A.md` is **PASS**, with 0 block and 6 warn. It is the second plan review, on the tree with #802 merged. T-red's decisions 1 to 6 settle every warn, and the operator's comment settles warn 2.
- `handoff-A-dup.md` is **PASS**, with 0 block and 2 warn. Both warns are test-only follow-ups.

## T precondition

Met.

- **RED** (`handoff-T-red.md`). The 39 new verb tests failed:
  - 38 failed where the argv is parsed, since the stub had no flags. With the flags declared on a scratch stub, each of the 38 failed on its own behaviour assertion against `not-implemented`.
  - The ADR test failed on its assertion.
  - `cli_surface_test` failed on the two new lines.
  - `import_test` failed to build on exactly the two items F adds.
  - No failure came from a rig or a fixture.
- **One commit for RED and F.** RED and F share `660efe8`, because a test sentinel tripped gitleaks. The RED evidence is in the handoff, not in a commit of its own.
- **GREEN** (`handoff-T-green.md`), with 0 blocking issues:
  - `pane_verbs` 306/0, `import_test` 5/0 and `cli_surface_test` 3/0.
  - The pipeline's `test.unit.command` over the workspace: 2160 passed, 0 failed and 25 ignored.
  - Workspace clippy with `-D warnings`, `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `wire_selftest`, `cargo machete` and `test-hooks.sh` all pass.
- **T's change at GREEN was test-only.** In two undo-order tests, the log is now read before `rig.records()` appends its own `List`. The expected orders did not change. The change is committed in `f5c75d3`.
- **Mutations.** T made two production mutations: the undo of a landed write, and `sources-disagree`. Each failed exactly the test that names the behaviour.

## Acceptance criteria

Test paths are under `crates/holler-cli/tests/pane_verbs/` unless they say otherwise.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | **Agreement.** Three agreeing panes import with their session of record. `herdr` is the snapshot entry with `label` `None`. Roles come from the flags, and model, effort, context, env names and agent from `pane.env`. `hj-c1r2` imports at r2c1. Port and cwd come from the launcher, never from the name. Name order, and the text line form. | `import/join.rs::agreeing_files_import_every_pane_with_its_session_of_record` (:60) is a real run:<br>- Each stored record equals `rig.expected_record` (`import/rig.rs:275`) field for field: label `None`, the launcher's port and cwd, the `pane.env` values, the env names in file order and de-duplicated, the role, `fleet`, `at` 0 and the Herdr version.<br>- It asserts that no port is `47000 + col * 10 + row`.<br>- `data.panes` is in name order, although the fixture made the panes C, B, A.<br>- B's `pos` is `{"row":2,"col":1,"pos":"r2c1"}`, and A's role is `orchestrator`.<br>`the_text_form_lists_each_pane_in_name_order` (:97) checks `[hj-c1r2, record, agent, r2c1, <session>]` and the order. `no_recorded_session_is_a_warning_and_prints_a_dash` (:365) checks the `-`. | Met |
| 2 | **A dry run writes nothing.** No store write. Every file has the same bytes after. The envelope is valid, `data.profile` is the profile it would create, `recorded` is 0, and the adapters make only the allowed reads. | `import/writes.rs::a_dry_run_reads_only_and_reports_the_profile_it_would_create` (:54):<br>- `assert_only_allowed_reads`, A's allow-list: Herdr `{Snapshot, Version}`, host `{Ps}`, harness `{Health, ListSessions}`, both stores `{Get, List}`, and no probe.<br>- `assert_no_store_write`.<br>- Every file under the state root keeps its bytes, and no file is added.<br>- `check_envelope`, the data keys, and the row and finding keys.<br>- `data.profile == profile_from_panes(fleet, expected)`, and `recorded` is 0.<br>- No record and no profile is stored. | Met |
| 3 | **A real run.** `fleet` has one spec per pane. `data.profile` is the stored profile, at generation 1. Each pane's `profile` is `fleet`, and `profile show` reports every row `matches`. The secret is nowhere, and no source file changed. | `import/writes.rs::a_real_run_creates_fleet_and_every_member_matches_it` (:126):<br>- `data.profile` equals the stored `fleet`: generation 1, and `created` set by the store's clock (1234).<br>- It equals the dry run's profile except for the three stamps.<br>- Every record's `profile` is `fleet`.<br>- `profile show fleet` returns two rows, both `matches`.<br>- The `pane.env` secret is in no output, no record and no profile.<br>- The files are unchanged. | Met |
| 4 | **Join failures.** `pane-unlabelled` and `pane-label-ambiguous`. A dry run exits 0, with the text finding line. A real run exits 3 with the pinned message and writes nothing. | `import/join.rs`:<br>- `a_pane_with_no_labelled_herdr_pane_is_unlabelled_and_blocks` (:120): the problem, `pos` null, the other pane still planned, and the line `  problem pane-unlabelled: `.<br>- `two_herdr_panes_with_the_label_are_ambiguous_and_block` (:145): both pane ids are named.<br>Both run `assert_real_run_blocked` (:39) in both formats: exit 3, the exact `import-blocked` message, one line on err, nothing on out, no record and no profile. | Met |
| 5 | **Disagreement.** The three cases of files that disagree. The stale id is in no record (I6). | `import/join.rs`:<br>- `files_that_name_different_sessions_disagree_and_block` (:221): each case is `sources-disagree`, `blocked`, with no session. The first case is refused in a real run.<br>- `agreeing_files_naming_a_session_the_server_lacks_record_no_session` (:273): no stored record holds the stale id. | Met |
| 6 | **The server.** It lacks the session. A call fails. It answers and is not healthy. | `import/join.rs`:<br>- `agreeing_files_naming_a_session_the_server_lacks_record_no_session` (:273): `session-not-found`, health `Healthy`.<br>- `a_failed_health_check_is_observe_failed_and_health_unknown` (:294).<br>- `a_failed_session_list_is_observe_failed` (:311): health `Unknown`, per point 1 of the operator's comment.<br>- `an_unhealthy_server_is_down_or_wedged_as_reconcile_says` (:335): `kill` gives `server-down` and `freeze` gives `server-wedged`. Each carries reconcile's sentence with the port and records `Unhealthy(code)`, and neither is `observe-failed`. | Met |
| 7 | **Hand-edited and missing files, and secrets.** `source-unreadable` names the file and a position only. The secret is first shown in the parser's text, then shown to be absent. `source-missing` covers the missing cases, and a missing state root exits 2. | `import/sources.rs`:<br>- `sessions.toml`: `Toml` (:59) and `Invalid` (:81).<br>- The launcher table (:115): line and column from `span()`.<br>- `pane.env` (:169): `export OC_MODEL = x`, a line with no `=`, a leading space and a non-numeric ceiling, each at `line 3`.<br>- A missing launcher entry, port or cwd (:144).<br>- A missing file, and each required variable named (:203).<br>- The state root (:240).<br>Each secret is asserted on neither out nor err, in both formats and in the real run. "No log line" holds by construction: the import modules and `holler_body::config::load` make no `tracing`, `log` or `eprintln` call, so err is the only channel. | Met (Advisory 4: one clause is pinned by reading only) |
| 8 | **Not OpenCode, and command shapes.** | `import/join.rs`:<br>- `a_pane_that_is_not_opencode_is_skipped_and_the_others_import` (:409).<br>- `a_command_array_is_the_argv_and_a_string_is_command_not_argv` (:423): the element `"x y"` stays one element.<br>- `a_directory_without_pane_files_is_ignored_and_blocks_nothing` (:442): a real run that exits 0. | Met |
| 9 | **Idempotence and `fleet`.** | `import/writes.rs`:<br>- `a_second_real_run_is_a_no_op` (:174).<br>- `an_existing_fleet_with_a_pane_to_record_is_profile_exists` (:194): exit 3, and no write call at all.<br>- `a_state_root_with_no_pane_writes_nothing` (:213): the "no candidate" exit. | Met |
| 10 | **A failure records nothing.** Faults (a) to (d), their codes and end states, the remedy text, never `pane close`, and "a follow-up issue is filed". | `import/writes.rs`:<br>- (a) :246: `generation-conflict`, exit 1, the store empty, `fleet` gone, and the log tail `[CasPut, Get, Delete]`.<br>- (b) :264: `timeout`, and the tail `[Get, Delete, Delete]`.<br>- (c) :285: `profile-conflict`, the store empty, `fleet` kept, and `holler profile delete 'fleet'`.<br>- (d) :307: `profile-conflict`, A still a member, `fleet` kept, and both commands.<br>`run_failing_write_2` (:226) asserts in every case that `pane close` is not named. The fault seam is `import/seams.rs::NthCasPut`, a declared copy.<br>**The follow-up issue is not filed.** `gh issue list --search` finds none. | The tests are met. The issue is still to file, by the run's agent (Advisory 1). |
| 11 | **Roles and flags.** | - `import.rs::an_import_needs_a_herdr_session` (:34): exit 2, `usage`, no adapter call and no store write.<br>- `an_unknown_orchestrator_is_a_usage_error` (:54).<br>- The existing `process/flags.rs::profile_parses_on_every_pane_verb_except_import` (`UnknownArgument`).<br>- `import/join.rs::a_name_that_spells_another_cell_is_information_and_the_snapshot_wins` (:201): both cells are named, and the record is at r1c1. | Met |
| 12 | **Vocabulary and docs.** | - `crates/holler-pane/tests/import_test.rs`, 5 tests: the pinned set; the codes are unique and kebab-case, and the JSON is the code; the reused codes come from their one source; the levels; `import-blocked` is a refusal, exit 3.<br>- `cli_surface_test` and `docs_cli_test` (T-green). `pane import` is not in `cli-surface.pending.txt`.<br>- `import.rs::adr_0021_lists_the_import_codes` (:81): section 9's row names all six codes, and the open-codes row names `import-blocked`.<br>- I read ADR-0021's section 10 (`:709-714`) and section 13 (`:793-800`, `:805-813`).<br>- `CHANGELOG.md:11` is under `[Unreleased]`, links #650 and #633, and passes `changelog-check.sh`. | Met |
| 13 | **The Herdr session.** | `import/join.rs::the_join_reads_only_the_named_herdr_session` (:178) covers both halves, through `import/seams.rs::ExtraHerdr` (A's warn 3). | Met |
| 14 | **Herdr's version.** | - `import/writes.rs::an_unsupported_herdr_ends_the_import_with_its_error` (:333): both runs, both formats, exit 3, `herdr-version-unsupported`, no report and no store write.<br>- `import_test.rs::herdr_version_unsupported_is_not_an_import_finding` (:124). | Met |

"Exit codes and what blocks" is covered too:

- Every `dry()` call over a pane with a problem exits 0.
- A usage error and an adapter error each keep their own code: acceptances 11 and 14.

## Spec compliance

**The operator decisions.** Each is implemented as stated. Unprefixed `import.rs` is the CLI's `pane/import.rs`.

1. **The Herdr label is the join key.** `holler-pane/src/import/assess.rs:164-211` filters the snapshot by `--herdr-session` first, then by label. The name's cell is information only (`:215-227`).
2. **A repeatable `--orchestrator`.** It is declared at `import.rs:90-92` and checked at `:198-216`. Every other pane is an agent (`assess.rs:134-138`). Both formats print the role.
3. **No label, or two or more, is a problem, and no record** (`assess.rs:177-209`). Any problem withholds the record (`:140-144`).
4. **Files that disagree** give a problem and no record (`assess.rs:263-324`), and a real run is refused. ADR-0021 section 13 step 2 is narrowed (`:808`).
5. **Exit codes, and all or nothing.**
   - A dry run exits 0.
   - `IMPORT_BLOCKED` is an open code through `RefusalCode::from_static` (`holler-pane/src/import.rs:45`), and the message matches the brief's text exactly (`:148-156`).
   - A real run checks first, then writes the profile through `insert_profile`, then each record at generation 0. The undo deletes (`pane/import/writes.rs:38-175`).
6. **`pane.env` values.** Only the five values are read; every other variable is kept by name (`pane/import/pane_env.rs:90-102`).
7. **A pane that is not OpenCode** is skipped and blocks nothing: `files.rs:101-112`, and the check comes before any observation (`holler-pane/src/import.rs:173-174`).
8. **A re-run** has no profile to write, so it makes no store write (`writes.rs:43-45`).
9. **`--herdr-session` is required.** A missing or blank value is `usage` before anything is read (`import.rs:152-158`).
10. **`sessions.toml`** is read through `holler_body::config::load` (`pane/import/sources.rs:219`). There is no second parser, and `ParsedConfig.warnings` is never printed.
11. **The record holds no label** (`assess.rs:188-191`), and `last_observed` stays empty (`:394-398`).

**The brief's sections.**

- **The input** (`sources.rs`, `pane_env.rs`, `launcher.rs`) is implemented as stated:
  - a pane directory, `ignored-entry`, and `source-unreadable` for a name that is not a pane name;
  - the `pane.env` grammar: a line number counts every line, and the missing variables are named;
  - `last-session`, where empty means absent;
  - an `owned-sessions` that holds no id is present and empty;
  - the launcher table, with its line and column from `span()`.
- **What the engine observes** (`holler-pane/src/import.rs:162-188`, `import/observe.rs`):
  - version and snapshot are called once each;
  - per pane: `ps`, `health`, and one `list_sessions`, also after `Ok(false)`, to tell wedged from down;
  - one scoped thread per pane;
  - the report is in name order, whatever order the threads finish in.
- **The record** (`assess.rs:366-411`) is the brief's field list, `HOST_NAME` included.
- **The writes and the undo** (`writes.rs`) implement steps (a) to (c):
  - each step uses the generation its own write returned;
  - (a) deletes only a member of `fleet`;
  - a completed undo answers the write's own code;
  - a failed step answers `profile-conflict`, with the remedies single-quoted (`'fleet'`, A's warn 5).
- **The output:**
  - The JSON key order follows `Imported`, `PaneRow` and `Finding`. The key is `kind`, and `pos` is `GridPos`'s object.
  - A `source-unreadable` message never carries text from the file or the parser:
    - `Toml` gives "not valid TOML";
    - `Invalid` gives the row and the field only;
    - the `toml::de::Error` is kept for `span()` only;
    - a string `command` is read into `()`.
  - Other untrusted text goes through `quoted` or `embedded`.
- **The finding vocabulary** (`import/findings.rs`) has 16 kinds. Each reused code comes from its one source (`FindingKind::code`, `PaneCode::as_str`).

**Where the code follows a recorded decision, not the brief's literal text.** None of these is silent. Each is in T-red's decisions, F's deviations, `decisions.md`, A's findings or the operator's comment.

- **`tmux-session-missing`, not `no-tmux-session`.** This leaves ten new codes, not twelve.
  - The brief lists `no-tmux-session` (`:29`, `:57`, `:69`).
  - The same brief says "a condition that reconcile already names keeps reconcile's spelling", and its join item 3 cites reconcile's `tmux-session-missing` for the same call and the same error. The brief contradicts itself.
  - The code follows the rule: A's warn 1 and T-red's decision 1.
  - The operator's comment gives the same reason for its own three points ("these reuse `pane doctor`'s vocabulary so the two reports agree").
  - This is a defect in the brief's text, not a deviation in the diff, so it is not an `ADVISORY-HOLD` (the precedent is #644's audit). A code is stable once merged, so record it on the issue (Advisory 3).
- **The health after `Ok(true)` and a failed list is `unknown`.**
  - The brief's record line (`:45`) says healthy. Its join item 4 and acceptance 6 say `unknown`.
  - Point 1 of the operator's comment settles it as `Unknown`, which is what the code records (`observe.rs:110-117`).
- **The `// #650` line in `stub.rs` is kept** (point 3 of the operator's comment). The forced-diagnostic test now takes the first stub of either namespace, since `pane import` was the last pane stub.
- **`text` in `show.rs` is not widened.** Only `push_spec` is called, so only `push_spec` widens. This is a trivial under-delivery, and nothing needs `text` widened.
- **The text form's layout.** The brief does not say where findings about no pane print (A's warn 6). F chose A's first option and documented it in `PaneImport`'s doc:
  - each pane's findings follow its line;
  - findings about no pane print first, under `-`;
  - a summary line ends the output.

  The tests pin the line forms. JSON is the versioned contract.
- **The launcher table is strict.**
  - No other key is accepted (`deny_unknown_fields`), as ADR-0021 section 13 now says.
  - Port 0 and a cwd that is not absolute are each `source-unreadable` for that pane, following the brief's shape: a port from 1 to 65535 and an absolute path.
- **`profile-exists` is checked with `profile_store.get` before any write**, so acceptance 9's case makes no write call. `insert_profile` still maps the race.

## Quality audit

**Correctness and failure handling**

- **It fails closed.**
  - A pane store `get` error ends the plan with its own error, before any report (`holler-pane/src/import.rs:171`).
  - An unreadable file is a problem.
  - An unreadable launcher table blocks every pane.
  - A chain that is lost (its thread failed to start, or panicked) is `observe-failed`, a warning. The brief classes every failed observation that way.
- **No write is lost.** Every write is a compare-and-swap: the profile and each record at 0, and each delete at the generation its write returned.
- **The undo never deletes another writer's record.** Step (a) deletes a record only when `get` shows it as a member of `fleet`.
- **The remedy names the records kept.** It uses `pane_list`, which prints `none` for an empty list (the outside model's W-2).
- **Threads.** The scoped threads are joined explicitly, so a panicked chain does not panic the scope.
- **Edge case (Advisory 6).** If the profile write itself lands and then times out, `fleet` is left with no members, the same contract as `profile create`.

**Build guards**

- No added `src` line holds `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or `unsafe`.
- There is one `#[allow]`, in `import_test.rs:1`, and it carries `// #650`.
- The largest touched file is `import/rig.rs` at 515 lines. The largest production file is `assess.rs` at 411. All are under `lint.sh`'s 600-line warning.
- Workspace clippy is clean (T-green).
- `rustfmt --edition 2021 --check` over every touched `.rs` file exits 0. It is my one read-only check.

**Contracts**

- No protocol, hub, golden or `docs/protocol/v2.md` change, and no manifest change.
- `holler-pane` takes no `toml` and no `holler-body` dependency (ADR-0021 section 5).
- The error tables are updated: section 9's row and the open-codes row. `class_of` is unchanged, since an open code is a refusal by rule.
- No frozen file is touched: `pane/mod.rs`, `ports.rs`, `pane.rs`, `profile_snapshot.rs`, `findings.rs`, `reconcile*`, `tx_launch.rs`, the Herdr adapter and the test kit's `src/`.

**Tests**

- They run in-process on the test kit's fakes. That is the right tier for a CLI verb over ports; the real adapters are #649's.
- No new test sleeps or measures time.
- The RED-first evidence is in T-red, and the mutations are in T-green.
- Each secret is proven present before its absence is asserted.
- The wedged and down classifications are genuine. In the fake, a frozen server's list times out and a killed server's list is unreachable (`holler-pane-testkit/src/harness.rs:358-362`, `:424-430`).

**Documentation**

- `CHANGELOG.md` has an `[Unreleased]` entry.
- ADR-0003's row is updated.
- ADR-0021's sections 9 (the row and the open codes), 10 and 13 are updated.
- `PaneImport`'s doc documents the JSON data.
- The operator guide is #652's. No README or `docs/` page lists `pane import`.

**Public-repository privacy.** I grepped every added line, the committed brief and handoffs included. The patterns: personal hostnames, tailnet names, IPs, machine and account names, home paths, e-mail addresses and keys. It is clean:

- some hits are made-up values: `/home/demo/...`, `127.0.0.1`, `/srv/demo`, and a fixture's `api_key = <made-up>`;
- the rest is the repository's own script: `$HOME/.holler-oc/`.

**Commit and PR hygiene**

- Every subject is a Conventional Commit (`chore(#650): ...`) with `Co-Authored-By`.
- No commit has a session link. The same is true of #644 and #802 on `main`.
- The PR is not open yet (Advisory 8).

## Scope check

- **Delivered.** Every file in the Blast radius, and nothing outside it except the pipeline's own `docs/handoffs/`.
  - `doctor/rig.rs` gains the unrecorded seed only: the `recorded` field, `unrecorded()` and one `if`.
  - In `stub.rs`, the `STUBS` entry is removed and the `// #650` line kept. The forced-diagnostic test's pick is widened, which is needed.
- **Over-delivery.** None of substance. The `-` group and the summary line are part of the text form, which F had to settle (A's warn 6).
- **Under-delivery.**
  - `text` is not widened, and nothing needs it.
  - Acceptance 10's follow-up issue is not filed. It is an outward act (Advisory 1).

## Verdict

**PASS.**

- Every acceptance criterion has a test that asserts the behaviour, and T-green's mutations fail the named tests.
- The eleven operator decisions and the brief's sections are implemented as stated. Every departure from the brief's literal text follows a recorded decision, and none is silent.
- The guards, the sizes, the contracts, the docs and the privacy check all hold.

Ready for O. Two actions come before the merge, and neither is F's (Advisories 1 and 2).

## Advisory notes (non-blocking for F)

1. **Before the merge: file acceptance 10's follow-up issue.** F and T take no outward action, and S reports only.
   - No such issue exists. I searched with `gh issue list --search` for "removes only a record", "pane forget", "delete a pane record", "record without closing" and similar phrases.
   - A suggested title: `feat(pane): remove a pane's record without acting on the live pane (follow-up of #650)`.
   - The body should say:
     - when an undo step of `pane import` fails, records of `fleet` can remain;
     - the only remedy today runs `profile delete --keep-panes`, then a re-run, then `profile create --from-current`, because no verb removes only a record, and `pane close` acts on the live pane.
   - Mark it part of epic #633, and add the AI disclosure line.
2. **Merge `main` before the PR can merge.** `origin/main` moved to `3286486` (#642 part 3) after A-dup's trial merge. `git merge-tree --write-tree HEAD origin/main` now conflicts in two text spots:
   - In `CHANGELOG.md`'s `[Unreleased]` Enhancements, keep both entries.
   - In ADR-0021's open-codes row (`:671`), keep both "#642's `agent-unusable`, the OpenCode adapter's" and "#650's `import-blocked`". `import.rs::adr_0021_lists_the_import_codes` asserts `import-blocked` in that row.

   No code file conflicts. #642's code reaches the import only through a doc comment in `ports.rs`, and the test kit is unchanged. Only CI verifies the merged build.
3. **Bring the issue's text up to date with the code.** A comment works, as the operator used for three points. Record:
   - `tmux-session-missing`, not `no-tmux-session`, and so ten new codes, not twelve;
   - `text` is not widened;
   - the text form's layout;
   - the strict launcher table: no other key, and port 0 or a cwd that is not absolute is `source-unreadable`;
   - how `import-blocked` counts N.

   The first matters most: a finding code is stable once merged (ADR-0021 section 9).
4. **Three message clauses are pinned by reading only** (test-only, size XS):
   - The row of an `Invalid` `sessions.toml` message (`pane/import/sources.rs:231-234`). `sources_toml_with_an_invalid_row_names_the_row_and_field_only` asserts the file and `mode`, but not the row.
   - "then run holler pane import again", in both remedies (`writes.rs:188-200`). Tests (c) and (d) assert the commands only.

   Fold these into the same pass as A-dup's warn 1, the three envelope helpers in `import/rig.rs` that copy doctor's and launch's.
5. **Consolidate the seams once the test kit unfreezes** (A-dup's warn 2). The test kit would gain a `PaneStore` that fails its N-th write and a pass-through `HerdrPort` with a snapshot hook.
6. **A profile write that lands and then times out** leaves `fleet` with no members, as `profile create` does.
   - A re-run answers `profile-exists`, whose message points at `holler profile show 'fleet'`.
   - The fix is `holler profile delete 'fleet'`, then a re-run.
   - It is worth one line in #652's runbook.
7. **The real fleet's files are untested here.** Only #654's dry run checks the launcher table's shape and the live `pane.env` and `sessions.toml`, as the brief says. Nothing a user runs changes until #649 wires the ports.
8. **PR hygiene, after the script opens the PR.**
   - Add the AI disclosure to the PR body with `gh pr edit`, per `CONTRIBUTING.md`, as `CLAUDE.md` directs.
   - Keep the `Co-Authored-By` trailer in the squash commit.
   - `CONTRIBUTING.md` also asks for a session link. None of the pipeline's commits carry one, here or on `main`.
