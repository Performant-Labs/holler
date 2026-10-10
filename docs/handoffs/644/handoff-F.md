# Handoff-F: Phase 5 (Workflow phase 6) - #644 `holler pane launch` and `relaunch`

**Date:** 2026-10-10
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `e82aedb` plus this phase's working-tree changes, which the Workflow script commits)
**Issue:** #644

## What was done

- **`crates/holler-pane/src/tx_launch.rs`**: the engine, filling T's stubs behind the brief's binding API.
  - `port_of_policy`: `fixed:<port>` only, canonical decimal 1..=65535; the message names the form.
  - `launch` and `relaunch` share one path:
    - `Plan::new` derives the port and the target cell;
    - `Plan::observe` runs steps 3-6;
    - `run` calls `edit_spec` with the act;
    - `act_live` runs B1/B2, then A1-O2 (`bring_up`), R, then B10.
  - The rollback is `roll_back` (with `Made`), the budget is `Budget`, and the record is `Plan::record`.
  - The `TxOptions` doc says why `now_ms` is a function (A warn 6).
- **`crates/holler-cli/src/pane/launch.rs`**: the verb. Shared with relaunch:
  - `effective_spec` (with `Problems`);
  - `emit_outcome` (with `Report` and `failure_body`, which holds the reconcile-step rule);
  - `Verb`;
  - `stored_profile` and `spec_of`.

  Also the clap help text.
- **`crates/holler-cli/src/pane/relaunch.rs`**: the verb. It reads the record, then P, then picks the base: P's entry for the pane, else `spec_from_pane(&record)`. Every other helper comes from `launch.rs`.
- **`docs/adr/ADR-0021.md`**, decision 20 and A warn 7, each edit anchored by its content:
  - section 8: the "Launch and relaunch as built (#644)" paragraph;
  - section 9: "open (#644) ..." on the launch row, and the `unavailable` reason cell;
  - section 12: the "PROPOSED (#644, ...)" paragraph;
  - "Deferred": the mismatch-code bullet, marked decided.
- **`CHANGELOG.md`**: one `### Enhancements` entry linking #644.
- **`docs/handoffs/644/evidence.md`**: the facts in unchanged code that the diff relies on. Also this handoff and a `decisions.md` entry.

## Design decisions

1. **One plan and act path for both verbs.** `Plan.old: Option<&Pane>` is the only difference.
   - With it set, the run adds B1/B2 and B10, uses the record's generation at R, and keeps the record's hold, DRIVEN and profile.
   - Two functions with the same steps were the alternative. That would duplicate about 150 lines and break the brief's size fallback ("one step runner").
2. **The port policy is parsed before the probe.** For launch it is after step 2; for relaunch, after E0. It is a pure check, so a spec from P (or #664's) with a bad policy is `usage` before any port call. The brief's step tables do not place this check.
3. **Step 5 reads `pane_store.list()` only when the cell has an occupant.** The records only word the refusal ("the two predicates are two steps"). If that read fails, the run is still refused with `grid-occupied`, and the message names only the failed read's code.
4. **A pane counts as created when the plan's snapshot did not list it** (matched by session and pane id). The rule is the same for both verbs (decision 12). It is why a relaunch rollback never closes the record's own pane (AC 18b), while the new pane after a vanish is closed (AC 5b's case).
5. **The budget** is an `Instant`, checked before each of A1-A7 (B1-B9), O1, O2, and before R.
   - A budget that runs out before R rolls back, because R has not run.
   - B10 runs whatever the budget says.
   - No check calls a port, so the AC 24 call logs are exact.
6. **The rollback note** is `; rollback failed: host.stop_owned (<code>), herdr.close (<code>)`, naming each call that failed.
   - `with_note` appends it and keeps the error's code. It goes through the crate's own payload mapping (`PaneError::detail`, then `from_wire`) instead of listing every variant again.
   - `profile_scope::with_context` is the CLI-side copy, and `holler-pane` cannot call it.
7. **B8** calls `list_sessions` only when the record has a session of record; with none there is nothing to keep.
8. **O2** compares the whole `HerdrPane` (session, workspace, id and grid) with the snapshot, which is the brief's "listed at grid".
9. **Untrusted text** (A warn 1):
   - ids, workspaces, directories and a spec's pane go through `findings::quoted`;
   - the prober's reason and a close's error text go through `findings::embedded`;
   - the success line uses `list::text_value` and `optional_text`;
   - a profile name is quoted.
10. **E0's message says "the spec gives"**, not "the flags give", because #664 calls the engine without flags. It still names both whole positions (`r2c1 in workspace "main"`).
11. **`effective_spec`** gives one `usage` line: `missing --project, ... (a pane with no spec to start from needs each of them)`, then each refused value, joined by `; `.
12. **A scope that returns `Ok` without running the act** gives `unavailable`, never a false success. Neither scope does this. It is the `(Ok, None)` arm of `run`.
13. **Two extra `pub(crate)` helpers in `launch.rs`**, shared with `relaunch.rs`: `stored_profile` and `spec_of`.
    - The frozen `pane/mod.rs` admits no module, which is park's precedent.
    - `stored_profile` returns the stored `Profile`, so the run passes on the stored name, and the reconcile step compares exactly with the scope's.
14. **The outcome's text and `data`:**
    - `profile-not-found` quotes the name (`what: quoted(P)`), as the profile verbs do.
    - `data.spec_only` is `pane == null`.

## Reuse / extend-vs-new

- **Reused, per the brief's Reuse map:**
  - from `holler-pane`: `Ports`, `ProfileScope::edit_spec`, `GridPos`, `Argv`, `EnvVarName`, `PaneName`, `ProfileName`, `RefusalCode` and `Prober` (via `ports.prober`);
  - from #662a: `profile_snapshot::{spec_from_pane, FIXED_PORT_POLICY_PREFIX}`;
  - from #663: `profile_scope::reconcile_step`, and `StoreScope` in the tests;
  - from the CLI: `SpecFlags::validate`, `SpecValues`, `ProfileOpt`, `SpecOnly` and `output::{emit, emit_error, ErrorBody, VerbCtx}`;
  - `holler_proto::clock::now_millis`.
- **Reused, per A's warns:**
  - `findings::{quoted, embedded}` (warn 1);
  - `reconcile::shown_differs` for O1 (warn 2);
  - `list::{text_value, optional_text, profile_name}`, the read verbs' sanitizers and `--profile` parser.
- **New, which the brief's API names:**
  - the engine's private plan, act and rollback types;
  - the shared helpers in `launch.rs`.

  There is no second Pane-to-spec mapping, no `"fixed:"` literal, no quoting helper and no reconcile-step function. `next_generation` is not called.

## Architecture notes for A

- **Layers:** the engine is pure over `Ports`, in `holler-pane`, and the verbs are in `holler-cli`.
- **Unchanged:** no new dependency, module, file or manifest change, and no frozen file is edited.
- **Public API:**
  - `holler-pane`'s public API is exactly T's stubs (the brief's).
  - New in `holler-cli`, `pub(crate)` in `pane::launch`: `effective_spec`, `emit_outcome`, `Verb`, `stored_profile` and `spec_of`.
- **New intra-crate uses:**
  - `tx_launch` calls `crate::reconcile::shown_differs` and `crate::findings::{quoted, embedded}`;
  - `launch.rs` calls `super::list::*` and `super::profile_scope::reconcile_step`.
- **Pattern followed:** park's, where the shared verb helpers live in the first verb's file and the sibling uses them as `super::launch::...`.

## Deviations from spec / wireframe

1. The rollback note names the failed call before its code (decision 6 above). The brief's form was `; rollback failed: <code>`.
2. Step 5's `list()` runs only when the cell has an occupant (decision 3), and B8's `list_sessions` only when the record has a session of record (decision 7). The brief's tables list these calls unconditionally. No test pins either way.
3. The port policy is parsed right after step 2, or after E0 (decision 2).
4. E0 says "the spec gives" (decision 10).
5. ADR edits:
   - **Placement.** The section 8 note is at the end of section 8, after the I8 order, which is where the brief's "after line 305" pointed when it was written. Section 12's paragraph follows the operator's paragraph and comes before #647's "no hub timer"; there is no "Deferred to #647" paragraph any more.
   - **The `unavailable` reason cell** is also amended (A warn 7d), beyond decision 20's four edits.
   - **The section 8 note also records:**
     - what the merged doctor really reports about leftovers (A warn 3);
     - that #649 should default `--herdr-session` (A warn 5);
     - decision 15's rule, by citing step 6 rather than restating it (A warn 7b).
   - **`grid-unreachable` is not named on the row:** `main`'s #640 edit of the same row adds it (see Known issues).
6. The brief's follow-up "the reconcile step names the pane" is dropped (A warn 7a).

## Tier 1 self-check (incl. tests now GREEN)

| Check | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs -- launch:: relaunch::` | `test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 130 filtered out; finished in 0.40s` (RED was 3 passed, 54 failed) |
| the same, 10 runs in a row (`-q`) | `passed 10 of 10 runs` (T-green runs 20) |
| `cargo test -p holler-cli --test pane_verbs --test cli_surface_test --test docs_cli_test --test pane_cli_process` | `pane_verbs` 187 passed; `cli_surface_test` 3; `docs_cli_test` 3; `pane_cli_process` 34; 0 failed |
| `cargo test -p holler-pane` | every target `ok`, 0 failed |
| `cargo test --workspace` | exit 0: 135 `test result` lines, 1676 passed, 0 failed, 14 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `rustfmt --check --edition 2021` on the 3 production files and the 5 test files | exit 0 |
| `bash scripts/lint.sh` | exit 0; warns, among the existing ones, `crates/holler-pane/src/tx_launch.rs is 786 lines` (Known issues 7) |
| `bash scripts/changelog-check.sh` | `changelog-check: ok` |
| AC 25: both greps on the three production files | print nothing |
| AC 27: the four greps | print lines 541, 345, 405 and 608; the ADR diff's hunks are only at sections 8, 9 (two cells), 12 and the mismatch bullet |
| AC 29 against the merge base (`git diff d9eabbb -- crates`, and its `--stat` on the manifests) | no `+... unsafe` and no manifest change (Known issues 6 on the two-dot form) |

Text output, for example: `launched demo-c1r1 at r2c1 in main: session ses_00000000000000000000000001, harness port 48100 (pid 20000)`.

## Evidence appendix

`docs/handoffs/644/evidence.md`: nine entries, covering `from_wire` and `detail`, `shown_differs`, `quoted`, `edit_spec(None)`, the fake scope's restore message, the fake harness's `health` and `serve`, the pane store's generation rule, and the `list.rs` helpers.

## Tests that look wrong (for T)

None. Every authored test passes as written; none was edited.

## Known issues

1. **`origin/main` moved after this branch's merge base `d9eabbb`:** #713 (#660) and #640 merged since. A merge will conflict in:
   - `docs/adr/ADR-0021.md`: the section 9 launch/relaunch row (both sides append; resolve as main's text followed by this branch's `; open (#644) for ... (pane-exists, grid-occupied, port-in-use)`), and the `timeout` and `unavailable` reason cells (adjacent lines; keep both edits);
   - probably `CHANGELOG.md`: both add an entry at the end of `[Unreleased]` Enhancements; keep both.

   `crates/holler-cli/tests/pane_verbs/process/stub.rs` changed on both sides too, but the hunks do not touch, so git merges it. After that merge, main's new test `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` takes the first `pane` stub. That is now `pane switch`: still a stub, with no positional, so the test still passes. Merging is not F's work; this is for whoever opens or merges the PR.
2. **After the merge, `PaneError::Timeout`'s doc and the ADR's `timeout` reason (both from #640) say `op` names the port method that ran out, as `<port>.<method>`.** This engine's budget timeout carries `pane.launch` or `pane.relaunch`: the binding constants, which AC 24 pins. That op is the verb's own bound, not a port method. One of the two texts should state the exception (S or the MO).
3. **#640's ADR section 10 on main says the recording of `host.herdr_api_version` is "PROPOSED: #644's launch and relaunch record it".** This story does record it (AC 1), so that mark can become decided when merging.
4. **The issue's 2026-10-09 `--agent KEY` amendment (#700) is not implemented.**
   - It is not in the brief.
   - #700 is open, and `Pane.opencode_agent`, `AgentKey` and `--agent` do not exist: `grep -rn "opencode_agent\|AgentKey" crates` prints nothing.
   - The issue forbids this story to edit `holler-pane/**` or `cli.rs`.

   S should judge it against the issue. It sits in Scope, not in the Acceptance list.
5. **ADR-0021 section 12's "A crash between steps leaves state that the next pane doctor run finds and reports (#644's acceptance)" is not edited** (A warn 3 leaves the acceptance to S). The new section 8 note says what the merged doctor reports: only a whole-fleet pass reports an unrecorded Herdr pane, and no pass sees an unrecorded server or tmux session.
6. **AC 29's two-dot form (`git diff --stat origin/main -- ... 'crates/*/Cargo.toml'`) now lists `crates/holler-adapter-herdr/Cargo.toml`.** That is #640's change on main, not this branch's. The merge-base (three-dot) form prints nothing.
7. **`tx_launch.rs` is 786 lines.** `lint.sh` warns from 600 and fails at 900, and the stack's decompose line is about 850. This follows the brief's size fallback: one shared step runner, and no new file outside the blast radius. Most of the size is rustfmt's one-field-per-line struct literals and the docs; there are 595 code lines.
8. **The real-adapter gaps the fakes hide are unchanged** (brief Risks 1, 8 and 10; C-8, C-9; decision 7). They belong to #649, #640 and #642.

**Follow-ups for O/MO to file.** From A's warns:
- #649 defaults `--herdr-session` (warn 5);
- #647 and #663: a reconcile step or doctor pass that reaches leftovers with no record, and a remedy that frees a cell an unrecorded pane holds (warn 3);
- align the clock's shape (a function here, a value in reconcile and park) with the rig consolidation (warn 6).

From the brief, everything except "the step names the pane":
- C-8: the Herdr pane to tmux session link;
- decision 7: apply the model, effort and env;
- a session delete for rollback;
- `sample_spec`'s port policy;
- the operator's answer on the operation id;
- an expected generation on `edit_spec`;
- one `pane_verbs` rig.

## Files changed

- `crates/holler-pane/src/tx_launch.rs`
- `crates/holler-cli/src/pane/launch.rs`
- `crates/holler-cli/src/pane/relaunch.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`
- `docs/handoffs/644/evidence.md`, `docs/handoffs/644/handoff-F.md`, `docs/handoffs/644/decisions.md` (pipeline scratch)
