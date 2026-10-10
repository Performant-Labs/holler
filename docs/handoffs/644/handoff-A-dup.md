# Handoff-A-dup: Phase 7 - #644 `pane launch` and `relaunch` (anti-duplication gate)

**Date:** 2026-10-10
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `c844f06`, which merges `origin/main` `cec1f82`)
**Diff reviewed:** `git diff origin/main...HEAD` (merge base `cec1f82`):
- production: `crates/holler-pane/src/tx_launch.rs`, `crates/holler-cli/src/pane/launch.rs`, `crates/holler-cli/src/pane/relaunch.rs`;
- tests: `tests/pane_verbs/{launch.rs, launch/rig.rs, launch/guards.rs, launch/profiles.rs, relaunch.rs, process/stub.rs}`;
- the surface fixture, ADR-0003, ADR-0021 and `CHANGELOG.md`.

**Reuse map:** the brief's "Reuse map (extend, do not duplicate)", `docs/handoffs/644-brief.md:2403-2421`
**F's handoff:** `docs/handoffs/644/handoff-F.md`
**Verdict:** PASS

## Summary

PASS. F extended every object the Reuse map names, and built no parallel path to any of them:
- the engine works only through `Ports` and `ProfileScope::edit_spec`;
- relaunch's base is #662a's `spec_from_pane`, the port policy goes through `FIXED_PORT_POLICY_PREFIX`, and the reconcile step is #663's `reconcile_step`;
- O1 calls `reconcile::shown_differs`;
- every untrusted value goes through `findings::{quoted, embedded}` or `list::{text_value, optional_text}`;
- relaunch reuses launch's helpers with no copy, in the same way `unpark.rs` reuses `super::park`;
- the test rig builds on `crate::list::Rig`, in `launch/rig.rs`, as the plan review asked.

The diff touches no frozen file and no manifest.

There are six warns, none of them a block:
- **Fix before merge (warn 1).** The merge of #640 into this branch brought in an ADR-0021 contradiction about a `timeout`'s `op`. Fixing it is one cell, plus one PROPOSED mark that is now settled.
- **Consolidation candidates (warns 2-5).** Each is a small near-copy that the crate layering or the blast radius put out of reach. None copies an object the map named, and F or T explained each choice in writing.
- **Size headroom (warn 6)** in `tx_launch.rs`.

## The plan review's warns, re-checked against the diff

Phase 3's "Notes for O" handed warns 1, 2 and 4 to this gate.

| Phase 3 warn | Status | Where |
|---|---|---|
| 1. Untrusted text through the domain's sanitizers | Resolved | Every untrusted value goes through `findings::quoted` or `embedded`: ids, workspaces, directories, a spec's pane, the policy, the prober's reason and a close's error. These are at `tx_launch.rs:82, 367, 381-382, 432, 435, 477, 678-680, 687, 735-737, 756`. The success line uses `list::text_value` and `optional_text` (`launch.rs:386-387`). The three files define no quoting helper of their own: no `{:?}`, no `escape_debug`, no `single_quoted`. |
| 2. O1 calls `reconcile::shown_differs` | Resolved | `tx_launch.rs:674` |
| 4. Build on `crate::list::Rig`, in `launch/rig.rs` | Resolved in shape | `launch/rig.rs:44` (`use crate::list::Rig as Fakes`). It is declared at `launch.rs:10` and reached from `relaunch.rs:15` as `crate::launch::rig`. `tests/pane_verbs/main.rs` is not edited. Some near-copies remain: see warn 4 below. |
| 6, 7, 8 (F's) | Resolved | 6: the `TxOptions` doc, `tx_launch.rs:87-92`. 7: the ADR edits are anchored by content, and the `unavailable` reason is amended (ADR-0021 line 463). 8: the positional field is `pane` (`launch.rs:60`, `relaunch.rs:40`). |
| 3, 5 (F's) | Resolved | Both are recorded in the section 8 "as built" note (ADR-0021 lines 345-372). |

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn (fix before merge) | `docs/adr/ADR-0021.md:462` and `:551`; `crates/holler-pane/src/error.rs:457`; `tx_launch.rs:63-66` | **ADR-0021 now contradicts itself on a `timeout`'s `op`.** The section 9 row came with #640 (`abdcbb6`, merged here in `c844f06`) and says `op` "names the port method that ran out, as `<port>.<method>` ... in every implementation and fake"; `PaneError::Timeout`'s doc (#640) says the same. This change's section 12 paragraph, and the binding `OP_LAUNCH`/`OP_RELAUNCH` that AC 24 pins, use `pane.launch` and `pane.relaunch`. That op names the verb's own run budget, not a port method. F flagged it (Known issue 2), but the ADR text is still unamended. Separately, ADR-0021 line 505 still marks "#644's launch and relaunch record it" as **PROPOSED**, and this change does record `host.herdr_api_version` (`tx_launch.rs:332`, AC 1; F's Known issue 3). | Extend the `timeout` reason cell in this change, for example "...; a verb's own run budget names the verb instead: `pane.launch`, `pane.relaunch` (#644)". Mark line 505's recording decided ("#644's launch and relaunch record it"). Both are one line, in section 9's table (which this change already edits one cell of) and in section 10. The `error.rs:457` doc is frozen for this story, so it is a one-line follow-up for its owner. |
| 2 | warn | `tx_launch.rs:775` (`with_note`) vs `crates/holler-cli/src/pane/profile_scope.rs:279` (`with_context`) | **Two implementations of one error operation, in two crates:** "append `; <text>` to a `PaneError`'s one payload and keep its code". F could not call `with_context`: `holler-pane` cannot depend on `holler-cli`, and F's design decision 6 says so in writing. The two behave identically. `with_note` goes through the crate's own `detail()`/`from_wire()` pair, which is lossless for every closed code: `detail` and `from_closed` cover the same variants, `Timeout.op` included, and `Refused` is handled explicitly. Their mechanisms differ, though. `with_context` names every variant, while `with_note` uses `from_wire`, the decoder for wire replies, for an in-process rebuild. | No change in this story. Follow-up: put one copy in `holler-pane`, for example `PaneError::with_context` in `error.rs` once its owner can edit it, and have `profile_scope::with_context` delegate to it. Building it on a `pub(crate)` `from_closed` would state the intent better than `from_wire` does. |
| 3 | warn | `launch.rs:126` (`spec_of`) vs `crates/holler-cli/src/pane/get.rs:133` (`spec_for`); `launch.rs:110` (`stored_profile`) vs `profile_scope.rs:77`, `profile/show.rs:79-84`, `profile/delete.rs:83-88`, `profile/create.rs:137-139` | **Two CLI lookups now exist twice.** (a) `spec_of` is the same one-line lookup as `get::spec_for`, `profile.panes.iter().find(\|spec\| spec.pane == name.as_str())`, borrowed instead of cloned. `spec_for` is private to #643's file, so reusing it would have meant editing a file outside the blast radius, and the map does not name it. (b) `stored_profile`, which reads P or answers `profile-not-found`, is spelled inline in the three profile verbs and held privately by `StoreScope::stored`. No shared helper exists, so the codebase has no dominant pattern to follow (Hard Boundaries: warn, not block). The `profile-not-found` text now has three spellings: `{:?}` (show, delete, create), `quoted` (launch, the same `{:?}` with a 64-character cut) and a plain `to_string()` (`StoreScope`). | No change in this story. Follow-up: one CLI home for both. `list.rs` already holds the shared `profile_name`, so `spec_of` and `stored_profile` could move there, with `get.rs` calling `spec_of(..).cloned()` and the profile verbs calling `stored_profile`. Alternatively, a `Profile::spec_for(&PaneName)` once `profile.rs` thaws. |
| 4 | warn | `tests/pane_verbs/launch/rig.rs:146, 178, 317, 419, 432, 545` vs `tests/pane_verbs/doctor/rig.rs:51, 112, 250, 262, 333, 354, 386` and `tests/pane_verbs/list.rs:151` | **Test-rig near-copies.** The rig wraps `crate::list::Rig` and does not copy its fakes, as asked. It does hold near-copies of doctor's private rig: (a) the call-log span `Calls`/`mark`/`calls_since` (it keeps probes as `Vec<ProbeCall>` where doctor keeps a count); (b) the live seeder `Seed`/`seed_live`, which runs the same chain as doctor's `make_live` plus `record_of` (`ensure_pane`, `ensure_session("/srv/demo")`, `serve`, `create_session`, `attach_tui`, then a `sample_pane` record). Three "data of a successful JSON run" helpers now exist: `data_of`, `doctor::rig::json_data`, and `crate::list::ok_envelope(run).data`, which the rig could already reach. Not a block: Phase 3 warn 4 asked for the span "as in doctor's rig", doctor's rig is private (`doctor.rs:10`, `mod rig;`), T-red explained `seed_live` in writing, and brief decision 25 defers the rig consolidation. | No change in this story. The brief's follow-up "One `pane_verbs` rig" should name these pieces to fold into one module: the call-log span, the live seeder and the JSON-data helper. `data_of` could call `crate::list::ok_envelope` today, which also asserts that stderr is empty. |
| 5 | warn | `tx_launch.rs:750` (`same_pane`) vs `crates/holler-pane/src/reconcile.rs:329-336, 351-358` | **The Herdr join key is spelled three times in `holler-pane`.** The key is (Herdr session, pane id). `tx_launch` names it in a private `same_pane`, and reconcile builds the same tuple inline in `missing_herdr_panes` and `unregistered_herdr_panes`. No named object existed to extend, and the brief defines the key explicitly. If the key changes (for example, #640's adapter serves one session, so the id alone would do), three places have to move together. | No change in this story. Follow-up: one predicate beside `HerdrPane` that both engines call (`pane.rs` is frozen now), or a `pub(crate)` helper reconcile can reach. |
| 6 | warn | `crates/holler-pane/src/tx_launch.rs` (786 lines) | **Size headroom.** The file is 786 lines: `lint.sh` warns at 600 and fails at 900, and this stack's flag is about 800. F followed the brief's fallback (one shared step runner, no new file), so this is within the plan. The next edit has about 110 lines of room, and #700's `--agent` key or #649's `--herdr-session` default may land in the engine. | No change in this story. The next story that grows the file should follow the sibling engine's precedent: a private child module declared from the engine file (`reconcile.rs:45`, `mod observe;` in `reconcile/observe.rs`), which needs no edit to the frozen `lib.rs`. Two self-contained seams are available: (a) the occupancy helpers (`refuse_occupied`, `occupant`, `same_pane`, `cell`); (b) the rollback helpers (`Made`, `roll_back`, `closed`, `with_note`). |

## Checked, no finding

- **`Ports` and `edit_spec`.** The engine's only effects are port calls: `tx_launch.rs` has no `Command`, no `std::process`, no `send_text` and no `send_keys`. The only call that is not a port call is `Instant::now()` for the budget, which the brief allows. P is written only through `edit_spec` (`tx_launch.rs:410-412, 519-521`); the verbs never write it.
- **`SpecFlags::validate` and `SpecValues`.** `effective_spec` takes the typed values and re-parses no guarded flag. It adds only the overlay, the required fields, the `--model` split, the ceilings' order, the port policy and `--expect` without a check, as the map says.
- **`output::{emit, emit_error, ErrorBody, VerbCtx}`.** Used at `launch.rs:87, 424, 431`, with no table of codes. Exit classes come from `class_of` through `emit`.
- **The three open codes.** `RefusalCode::from_static` (`tx_launch.rs:49-55`), declared nowhere else. The OpenCode adapter's own held-port check answers `unavailable` at `serve` (`holler-adapter-opencode/src/server.rs:67-85`). That is a later point than step 6, so it is not a second representation of `port-in-use`.
- **The probe.** Only `ports.prober.run_probe` is called (`tx_launch.rs:429`), never the free `run_probe`. The engine's step 3 is the only place that turns a `ProbeResult` into `ProbeFailed`. `profile/show.rs::probe_text` renders a *stored* result in the CLI: a different job, in a crate the engine cannot call.
- **No literals or helpers the map forbids.** No `next_generation`, no `"fixed:"` literal, no `"to reconcile"` or `holler pane doctor` literal, and no reconcile-step function in the three production files.
- **Relaunch reuses launch's helpers.** `launch::{effective_spec, emit_outcome, Verb, stored_profile, spec_of}` are shared with no copy (`relaunch.rs:19`). The first verb's file holds them, as `park.rs`/`unpark.rs` do, because the frozen `pane/mod.rs` admits no module.
- **One mapping in each direction.** `spec_from_pane` is the only Pane-to-spec mapping (`relaunch.rs:75`). `Plan::record` maps the other way (spec plus live to Pane), which has no analogue: `profile_snapshot.rs` holds only Pane to spec.
- **E0 against `profile_diff`.** E0 compares the cwd and cell fields directly, as the brief specifies. `profile_diff::diff_spec` is the all-field drift report, not the analogue.
- **`Budget`, `cell`, `closed`, `Problems`.** None has an analogue in `holler-pane` or the CLI. The adapters' `deadline_after` helpers are per-crate and outside this diff.
- **The `data` shape.** `ProfileRef {name, slug, generation}` is the brief's own shape, and the profile verbs share none (`ProfileRow`, `Deleted`, `Created`).
- **`PaneNotFound` for a missing record.** It is spelled inline in `relaunch.rs:68-73`, as in `get.rs`, `park.rs` and `reconcile.rs` (park's module doc says so on purpose).
- **Merged code.** Main's commits merged after F's base (#640, #713, #715) changed `holler-pane` only in doc text (13 lines), so they added nothing the diff should have reused.
- **Frozen files and manifests.** Over the brief's "Not touched" list, the rigs of #643, #646a and #647, and the manifests, `git diff --name-only origin/main HEAD` prints nothing.
- **Public repository.** A scan of the added lines for host, tailnet and account names found none; `p1/m1` is a test model id. S owns AC 31.

## Notes for F

None: the verdict is PASS.

**Routing:**
- **Warn 1:** fix before merge, as two one-line ADR-0021 edits, by F on a rework or by whoever merges. S judges whether it holds up acceptance.
- **Warns 2-5:** follow-ups for the orchestrator. Warn 4 extends the existing "One `pane_verbs` rig" follow-up. Warns 2, 3 and 5 are new consolidation follow-ups: the error-context helper, the CLI profile and spec lookups, and the Herdr join key.
- **Warn 6:** for the next story that touches `tx_launch.rs`.

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs` and `reconcile/observe.rs` (#647): the sibling engine. It has `shown_differs`, the inline (session, pane id) join, the `mod observe;` split, and `PaneNotFound` for a missing record.
- `crates/holler-pane/src/findings.rs:303-351`: `doctor_command`, `quoted`, `embedded`.
- `crates/holler-pane/src/error.rs:536-615`: `detail`, `from_closed`, `from_wire`.
- `crates/holler-pane/src/profile_snapshot.rs` and `profile_diff.rs` (#662a).
- `crates/holler-cli/src/pane/profile_scope.rs:37-56, 77-82, 274-309` (#663): `reconcile_step`, `StoreScope::stored`, `with_context`.
- `crates/holler-cli/src/pane/{list,get,park,unpark,doctor}.rs` and `crates/holler-cli/src/profile/{show,delete,create,list}.rs`.
- `crates/holler-cli/tests/pane_verbs/{list.rs, doctor/rig.rs, park/rig.rs}`.
