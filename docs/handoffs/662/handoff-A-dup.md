# Handoff-A-dup: Phase 7 - #662a profile verbs: the pure core and the read verbs  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`; this run is 662a only)
**Diff base:** `3bdd129` (`origin/main`, unchanged at review time)   **Diff head:** `80b59c3`
**Reuse map:** `docs/handoffs/662-brief.md`, "Reuse map (extend, do not duplicate)" (lines 1937-1952)
**Verdict:** PASS

## Summary

PASS. F extended every object the Reuse map named and built no parallel path:

- every result goes through `emit`, and every code comes from the closed `PaneError` set;
- membership is one `is_member`, called by both verbs, with no inline filter;
- the comparison is one `diff_spec`/`diff_profile`, built on the one `spec_from_pane`;
- grids, argv, env names and probe results print through their records' own `Display`/`Serialize`;
- the test rig composes `verb_harness::run_verb_with` and the test kit's fakes.

No frozen file, manifest, test-kit file or `tests/common` file changed, and the public API is exactly the brief's.

Five warns, none a block:

1. `count` is the workspace's third private pluralizer.
2. `SpecField::ALL` has no completeness guard against `ProfileSpec`.
3. #647 (PR #701, open) adds a near-copy of `write_escaped` to `findings.rs`.
4. Merging with #647 conflicts in two doc files, both mechanical.
5. Three cross-story `is_member` and #644 items from round 2 are still open.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/src/profile/list.rs:93` | `count(n, noun)` is the third copy of one 7-line pluralizer. The other two are `holler-adapter-herdr/src/plan.rs:262` (`usize`) and `holler-pane-testkit/src/herdr.rs:498` (`u16`): same body, both private. `hub_cmd.rs:110` pluralizes inline. Neither copy is reachable from holler-cli: the CLI does not depend on the adapter, and the kit's copy is private (and the kit is a dev-dependency). There is no shared seam and no dominant pattern, so this is not a parallel path. Placement: `pub(crate)` in `list.rs`, so `show.rs` (and 662b's `create.rs`) import from a sibling verb file. The frozen `profile/mod.rs` leaves no other home, and both verbs are #662's. | No change in this run. Later profile verbs (#664, #665) should use `super::list::count`, not write a fourth copy. Move it to a shared `profile` module when `mod.rs` next opens. |
| 2 | warn | `crates/holler-pane/src/profile_diff.rs:56` | `SpecField::ALL` is a second list of `ProfileSpec`'s fields, and nothing ties it back to the record. AC 2e's test checks each `ALL` entry against the spec's JSON, but not the converse. `spec_from_pane` copies `model` and `context` whole, so a field added to `ModelSpec` or `ContextCeilings` (both shared with `Pane`) compiles everywhere. `diff_spec` would never compare it, and `diff_spec(&spec_from_pane(p), p)` would stay empty. A field added to `ProfileSpec`, `SpecHerdr`, `SpecHost` or `SpecHarness` breaks only `profile_snapshot.rs`, never `profile_diff.rs`. #664 (apply), #665 (drift) and #650 would all inherit the gap. The crate's other closed table has this guard (`error_class_test.rs:37`: "A code added to `ALL_CODES` fails here until this test names its class"). | T adds one test to `profile_diff_test.rs`: for a fully populated spec, the leaf paths of `serde_json::to_value(&spec)`, minus `pane` and with `herdr.grid` as one leaf, equal the paths of `SpecField::ALL`. No production change. |
| 3 | warn | `crates/holler-pane/src/profile_diff.rs:196` | A near-copy, in flight. #647 (PR #701, open; branch `00f7ff2`, its A-dup PASS) adds `holler_pane::findings::embedded` (`pub(crate)`: each `is_control()` character through `escape_debug`, cut at 200 characters), beside `findings::quoted` (`{:?}` plus `error::excerpt`). 662a's private `write_escaped` is the same loop with `escape_default`. A script over all 65 control characters found one difference: NUL (`\u{0}` against `\0`). Neither is on `main` yet, so against the codebase 662a builds no parallel path. The brief's Follow-up "Shared read-verb text forms" already calls for this to be settled "before the second of #643, #647, #662 merges". With #701 open, 662a is likely the second. | The MO settles it under that Follow-up, after both merge: one escape in holler-pane that both call, for example `embedded` as the cut plus `FieldValue::Text`'s `Display`, or one shared `pub(crate)` helper. Do not fold at 662a's rebase: either fold edits the other story's file. |
| 4 | warn | `docs/adr/ADR-0021.md:525`, `CHANGELOG.md:167` | Merging with #647. `git merge-tree --write-tree HEAD issue-647-implementation` (`80b59c3` with `00f7ff2`) conflicts in exactly two files. **ADR-0021, "Deferred to named stories":** #647 deletes the reconcile bullet and 662a deletes the port-policy bullet next to it. **CHANGELOG.md, `[Unreleased]` / `### Enhancements`:** both branches append an entry at the same place. The code, `stub.rs`, `cli-surface.txt` and ADR-0003 merge cleanly. Round 2's warn 4 predicted the ADR conflict (F's Known issues). The CHANGELOG conflict is new. | The run's own agent resolves both at rebase, and neither is a stop. ADR-0021: delete both bullets. CHANGELOG: keep both entries. Then re-run `bash scripts/changelog-check.sh` and AC 10's greps. |
| 5 | warn | `crates/holler-pane/src/profile_diff.rs:261` | `is_member` is the one production copy, as the map asks, and both verbs call it. Three items from round 2 are still open outside this diff. **(a)** #643's `pane/watch.rs::names_profile` (branch `3140f35`, T-red PASS) compares `profile.slug() == self.slug` inline, which is `is_member`. Round 2's warn 2(a) asked for a Follow-up covering the other merge order, and the brief does not have one. **(b)** The kit's private `belongs` (`holler-pane-testkit/src/profile_scope.rs:235-239`) is now identical to `is_member`. **(c)** #644's brief (`7195993`, lines 1414-1415) still quotes the superseded "`port_policy` is **not compared** (`SpecField::COMPARED` omits it)". No `COMPARED` exists anywhere in the tree. | The MO relays (a) to #643's F: switch `names_profile` to `profile_diff::is_member` once 662a is on `main`; if #643 merges first, a follow-up makes the switch. The MO relays (c) to #644's O, to fix at its rebase. (b) is an optional kit follow-up: the fake may keep its own copy as an independent oracle. |

### Checked and consistent (the evidence behind the PASS)

- **Every row of the Reuse map is reused as named.**
  - `emit` and `ErrorBody::from` carry every result and error; `class_of` gives every exit code, inside `emit`.
  - The codes: `ProfileNotFound`, and `Usage` through `ProfileName::parse`. No `Refused`, no new code, no edit to `error.rs`.
  - `slug()` is the only slug rule.
  - `FieldValue::Grid` uses `GridPos`'s own `Display` and `Serialize`.
  - `Argv::as_slice` and `EnvVarName::as_str` give argv and env values; argv prints as a JSON array.
  - `ProbeResult` is printed in its own serde form.
  - `spec_from_pane`, `is_member` and `diff_profile`/`diff_spec` are one copy each. `diff_spec` goes through `spec_from_pane`, and `show` flattens `PaneDiff` rather than copying its fields.
  - `tests/common` and the kit's fixtures are reused, not edited.
- **The public API is exactly the brief's.** The added `pub` lines in holler-pane are the brief's 15 items, and nothing else. `SpecField` serializes through `as_str` (round 2's warn 1), and a spec's pane prints through `FieldValue`'s `Display` (warn 5). #644's pre-flight grep matches its 3 lines.
- **The analogues checked are not duplicated:**
  - `holler_proto::log::escape_field_value` is private to the wire crate, and the brief records it.
  - `holler-cli/src/hub_cmd.rs:132` `printable` is another rule: it replaces with `?`, for text from unauthenticated peers.
  - `holler_pane::error::excerpt` quotes and truncates.
  - `holler-hub/src/profile/entry.rs:187` `summary` counts spec-to-spec changes for the log, not spec against live.
  - The adapter's `plan_splits` and `grid_of` plan a Herdr layout; they do not detect drift.
  - `reconcile`, `findings`, `tx_*` and `import` are empty stubs on `main`, as is `pane/profile_scope.rs` (5 lines).
  - `HarnessKind` and `PaneRole` have no `as_str`, so `serde_text` is the single-source path, the inverse of `parse_role`.
  - Neither `Argv` nor `ProbeResult` has a `Display`, so `write_json_strings` and `probe_text` are the first copies.
- **The overlay's Phase 7 candidates are not copied.** The token store, `Lockout`, `Roster`, `log(Severity, ...)` and the hub's test helpers are untouched. The rig composes `run_verb_with`, `Unwired`, `check_envelope`, the seven fakes and `sample_*`.
  - One small item, not a parallel path: `rig.rs:27` declares its own `static UNWIRED`. The harness's copy is private (`verb_harness/mod.rs:23`); `unwired_ports().scope` would reuse it.
  - The pure tests' per-file `argv`/`env` builders follow `ports_test.rs:32`.
- **Layering and dependencies.**
  - The two holler-pane modules call no port and do no I/O.
  - The only new edges are `profile_diff` → `profile_snapshot` (one crate) and holler-cli → `holler_pane::profile_diff`.
  - No new crate dependency, and no change to a manifest or `Cargo.lock`.
- **Size.** The largest touched file is 336 lines, and the largest production file 333.
- **Frozen files are untouched:** `profile/mod.rs`, `output.rs`, `wiring.rs`, `cli.rs`, holler-cli's `lib.rs`, and `holler-pane/src/{lib,profile,pane,error,ports}.rs`. So are the test kit, `tests/common`, `verb_harness` and `profile_verbs/main.rs`.
- **My re-runs:**
  - `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` is clean, and `bash scripts/lint.sh` exits 0.
  - `origin/main` is still `3bdd129`.

## Notes for F

None required (PASS). Findings 3 to 5 are for the MO to relay. Finding 4 is resolved by the run's own agent at rebase,
and finding 2 is one optional test for T.
