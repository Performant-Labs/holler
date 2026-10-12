# Handoff-A-dup: Phase 7 - #647 the reconcile engine and `holler pane doctor`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`)
**Diff base:** `3bdd129` (`origin/main`; `git ls-remote` shows `main` has not moved)   **Diff head:** `baf176f`
**Reuse map:** `docs/handoffs/647-brief.md`, "Reuse map (extend, do not duplicate)" (lines 924-939), plus the Phase 3
warns W-2, W-3, W-4 and W-6 (`docs/handoffs/647/handoff-A.md`)
**Verdict:** PASS

## Summary

PASS: no blocks, five warns. F extended every object the Reuse map named and copied none of them:

- the engine reaches the world only through `Ports`;
- scope comes from `ProfileScope::resolve`, and output from `output::emit` and `ErrorBody::from`;
- only closed codes are used;
- `error::excerpt` is wrapped, not copied;
- the clock is `holler_proto::clock::now_millis()`;
- the tests compose the test kit's fakes, `run_verb_with`, `check_envelope` and `try_parse`.

Every new object is one the brief names (the engine, the findings, the two fallback submodules) or one Phase 3 asked for:
`shown_differs`, `doctor_command`, the single remedy table, `quoted`, and `findings_test.rs`.

Nothing on `main` is duplicated. The warns are about seams with unmerged sibling work:

- #643's branch has its own rig in the same test binary, and its own terminal-quoting helper;
- #663 and #644 plan their own spelling of the reconcile step.

Also in the warns: one interim error-code choice that ADR-0021 leaves to #644/#645, and a fourth crate-local text
sanitizer in a workspace that has no shared one.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| D-1 | warn | `crates/holler-cli/tests/pane_verbs/doctor.rs:10` (`mod rig;`) | The doctor rig is the first fake-world rig for `pane_verbs`, so it duplicates nothing on `main`. But it is private to `doctor`, so #644, #645 and #646 cannot reach it without editing `doctor.rs`, which `pane_verbs/main.rs:8-9` reserves to its own story. #643's branch (`50bcc83`, at T-green) has its own `pub(crate) struct Rig` (`pane_verbs/list.rs:39`) in the same binary, overlapping `ports()`, `json_data` (its `ok_envelope`) and `both_formats`/`assert_refused` (its `assert_fails`). The brief's Forward-compat never named a shared rig (Phase 3 W-6(b); T-red's notes). | Declare it `pub(crate) mod rig;`, the convention of #662's `profile_verbs/rig.rs` ("reached by the other verb files as `crate::list::rig`"). O names one `pane_verbs` rig: whichever of #643 and #647 merges second folds its general helpers onto the first one's, and that story's A-dup gate checks it. |
| D-2 | warn | `crates/holler-pane/src/findings.rs:332` (`quoted`) | `quoted` is now the one `pub` quoting rule in `holler-pane`: `{:?}`, cut at 64, by delegation to `error::excerpt`. The engine and the CLI renderer both use it. Two other rules sit beside it. #643's unmerged `pub fn text_value` (`holler-cli/src/pane/list.rs:261`) quotes only when needed and does not cut. `holler-adapter-herdr/src/protocol.rs:580` (on `main`, #699) is a private exact copy of `excerpt` (`EXCERPT_LIMIT = 64`), written because `excerpt` is `pub(crate)` in the frozen `error.rs`. That is also why the public face of a general helper now sits in a domain module. | O picks one terminal-text rule for the pane verbs before the second of #643 and #647 merges (Phase 3 W-6(d), still open). File a follow-up for #640 to call `holler_pane::findings::quoted` instead of its copy. At the next amend-first change to `error.rs`, make `excerpt` `pub` and keep `quoted` as its alias. |
| D-3 | warn | `crates/holler-pane/src/findings.rs:306` (`doctor_command`) | The pane form of the reconcile step is `pub` here (Phase 3 W-4, done). The amended #663 and #644 briefs (unmerged) still spell the same command themselves, in `holler-cli/src/pane/profile_scope.rs`: `RECONCILE_STEP_UNSCOPED = "to reconcile, run holler pane doctor"`, and `reconcile_step(&ProfileName)` giving `... holler pane doctor --profile '<P>' and then holler profile show '<P>'` (663-brief.md:1804-1811; 644-brief.md:1437-1438, 1558). After both merge, `holler pane doctor` is spelled in two crates. `doctor_command` has no `--profile` form for them to call, because no doctor remedy carries one (Decision 8(a)). | O tells #663 and #644 to build the unscoped step from `findings::doctor_command(None, false)`. O also decides who owns the profile form: a `profile` parameter on `doctor_command` (#647 owns the verb's grammar), or #663 composing on top of it. The later story's A-dup gate checks this. |
| D-4 | warn | `crates/holler-pane/src/reconcile/observe.rs:331` | When the select is acknowledged and the TUI still shows another session, `fix_error.code` carries a finding-kind code (`shown-driven-mismatch`). Every other value of that field is a `PaneError` code. ADR-0021:536 leaves "which closed failure code a mismatch observed after `act` carries" to #644 and #645, so F invented no error code. F documented the choice on `FixError::code` (`findings.rs:194-196`), and T pinned it (`acknowledged_select_that_switches_nothing_is_not_fixed`). It is still part of doctor's versioned `--json` surface, and it mixes two vocabularies in one field. | Add one line to the #644/#645 hand-off: when they fix the post-act mismatch code, doctor's `fix_error.code` for this case follows it, or they adopt `shown-driven-mismatch`, so there is one vocabulary. |
| D-5 | warn | `crates/holler-pane/src/findings.rs:338` (`embedded`) | This is the fourth crate-local control-character sanitizer, each with its own policy: `holler-proto/src/log.rs:483` `escape_field_value` escapes and is private; `holler-body/src/acp_driver/auth.rs:152` `sanitize` replaces with a space and cuts; `holler-hub/src/holds.rs:216` `sanitize_reason` drops, trims and cuts. None is reachable from `holler-pane` except log.rs's private one. `embedded` is that rule plus the brief's 200-character cut (Decision 8), and F's handoff records the check. So this is no parallel path: there is no dominant shared pattern to extend. | No change in this story. O may file an audit item: one `holler-proto` helper with an explicit policy, which the four call sites fold onto. |

No duplication; the extension is clean. No rework cycle has run, so there is no rework drift to re-check.

### Checked and consistent (no finding)

- **The Reuse map, row by row.** Each named object is reused and none is copied:
  - `Ports` only. No adapter, no I/O, no new dependency (`git diff 3bdd129 -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'`
    is empty).
  - Scope: `ProfileScope::resolve` for `--profile`; `pane_store.get` plus `pane-not-found` for a named pane; `list` for
    the all-records set (`reconcile.rs:222-243`). There is no membership check of doctor's own.
  - `cas_put` only on a change or on a first observation (`observe.rs:354-386`), never retried.
  - Closed codes only, through `ErrorBody::from` and `emit` (`doctor.rs:41-44`). There is no `RefusalCode` and no new
    code.
  - `GridPos` `Display` and `Serialize` (`doctor.rs:98`; `Finding.grid`). No row or column is formatted by hand.
  - `ProfileOpt` is flattened, and `PaneName::parse` and `ProfileName::parse` type the arguments in `run`
    (`doctor.rs:47-62`). `args.rs` has no typed accessor that this would copy.
  - Tests: `run_verb_with`, `check_envelope`, `sample_pane`, `sample_profile`, `FakeProfileScope::new`, the seven fakes,
    and `verb_harness::parse::try_parse`. The rig composes them and copies none.
- **The new objects are the brief's, or ones Phase 3 asked for.**
  - `reconcile/observe.rs` and `doctor/{rig,read_only,surface}.rs` are the named fallbacks.
  - `holler-pane/tests/findings_test.rs` is W-6(a).
  - `reconcile::shown_differs` is W-2. The hub depends on `holler-pane`, so the `send_prompt` gate (#646) can import it.
  - `doctor_command` and `FindingKind::remedy` are W-4 (one remedy table, `findings.rs:121-143`).
  - `FindingKind`, `FixState` and `ObservedHealth` each serialize from one string (W-5(a, b)). `ObservedHealth`'s
    unhealthy spellings are taken from `FindingKind::code()`.
- **Near neighbours on `main`, checked and not duplicated.**
  - `FixError {code, message}` sits beside two similar types:
    - `ReplyError {code, message, detail?}` (`reply.rs:37-43`, the wire reply's error half);
    - `ErrorBody` (`output.rs:126-141`, the CLI envelope, in a crate `holler-pane` cannot import).
    The three are different contracts, and the brief declares `FixError` (Decision 10). Reusing `ReplyError` would put
    the wire's `detail` and its `skip_serializing_if` into doctor's `--json` shape, against AC 19.
  - `ObservedHealth` beside `Health`: declared in the brief (Decision 10), mapped into `Health` by one function
    (`reconcile.rs:153-161`), with the same strings.
  - `HealthGate` (`surface.rs:458-514`) is the test-local wrapper AC 24 asks for. The kit's `set_delay` would make the
    test depend on a duration, and the kit's `Mutant` wrappers live in its own test targets and are not exported.
  - `Rig::mark` and `calls_since`: the kit offers only `calls()`, and nothing on `main` snapshots call logs.
  - Nothing on `main` builds a `holler pane doctor` command line, constructs `pane-not-found` outside `error.rs`, or
    checks the orchestrator role, so the engine's one-line versions copy nothing.
- **The overlay's stack checks.**
  - No prompt path: `send_prompt` is untouched.
  - No wire, golden-file or protocol change.
  - ADR-0021 and ADR 0003 are edited in the same change.
  - No new persisted state: writes go through `PaneStore::cas_put`, only on a change.
  - The threads run in the CLI process, not the hub (`std::thread::scope`, no runtime).
  - No secret reaches the output (AC 22).
  - The largest touched file is 586 lines.
  - A grep of the diff finds no personal infrastructure name.
  - The diff touches none of the hub-side candidates (`token.rs`, `Lockout`, `Roster`, `log(Severity, ...)`, the hub
    test harness), and adds no near-copy of them.

### Carried from Phase 3 (unchanged, not re-rated here)

- **W-1, standing-spec half.** ADR-0021 §1's `harness` and `last_observed` rows still name no meaning for what reconcile
  writes. AC 28 forbids the edit, so F stated the meanings in `reconcile.rs`'s module doc (lines 26-40). O decides the
  amend channel.
- **W-7.** The frozen ports' two blind spots remain, as the brief says: a foreign TUI on a shared data directory, and
  strays that accumulate.
- **AC 33 bookkeeping.** `crates/holler-pane/tests/findings_test.rs` is still not in the brief's Files list (T's
  advisory).

## Notes for F

None: PASS, nothing to fold.

## Notes for O

These are not required for a PASS. They are the decisions only O or the MO can make:

1. **D-1.** Decide whether to route the one-word `pub(crate) mod rig;` to T before merge. It is cheapest now; after merge
   it becomes an edit to another story's file. Then name the `pane_verbs` rig with #643, whose branch is at T-green.
2. **D-2 and D-3.** Tell #643, #644 and #663, and #640 for the adapter's `excerpt` copy, which helpers this branch adds:
   - `holler_pane::findings::quoted`;
   - `holler_pane::findings::doctor_command`;
   - `holler_pane::reconcile::shown_differs`.
   Each later story's A-dup gate should then reject its own copy.
3. **D-4.** Put the post-act mismatch code in the #644/#645 hand-off.
4. **D-5.** Optionally file an audit item for the four sanitizers.

## Patterns referenced

- `docs/handoffs/647-brief.md` (the Reuse map and Decisions 8, 10 and 12) and `docs/handoffs/647/handoff-A.md`
- `crates/holler-pane/src/{reply,error,lib}.rs` and `crates/holler-cli/src/output.rs` (the existing error bodies,
  `excerpt`)
- `crates/holler-cli/tests/verb_harness/{mod,parse}.rs` and `crates/holler-cli/tests/pane_verbs/main.rs` (the shared
  test pieces and the one-file-per-verb rule)
- `crates/holler-proto/src/log.rs`, `crates/holler-body/src/acp_driver/auth.rs`, `crates/holler-hub/src/holds.rs` and
  `crates/holler-adapter-herdr/src/protocol.rs` (the sanitizers and quoting helpers)
- The unmerged branches, used as evidence of plans and not as patterns: `issue-643-implementation`
  (`pane_verbs/list.rs`, `src/pane/list.rs`), `issue-662-implementation` (`profile_verbs/rig.rs`), and the #644 and #663
  briefs
