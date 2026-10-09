# Handoff-A: Phase 3 - #662a profile verbs: the pure core and the read verbs  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `5b47d82`; this run is 662a only)
**Brief reviewed:** `docs/handoffs/662-brief.md`   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 1912-1926 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on one finding. The production plan is sound:

- `profile_snapshot.rs` and `profile_diff.rs` sit where ADR-0021 section 5 puts them. They are pure functions over records
  of the same crate, and each is the one copy of its job: the snapshot, the comparison and the membership test.
- #644's brief, amended after its own A (`7195993`), now drops its `spec_of_pane`, depends on #662a merging first, and pins
  `spec_from_pane`, `FIXED_PORT_POLICY_PREFIX` and `fixed_port_policy` exactly as this brief states them.
- The verbs reuse `emit`/`emit_error`, `class_of`, the closed `PaneError` set, `ProfileName::parse` and `slug()`, and the
  serde forms of `GridPos` and `ProbeResult`. JSON comes from derived structs.
- No verb calls an adapter, the scope or the prober, and no frozen file or manifest is touched.
- `show` reads `probe.last` and never runs a probe.
- Both items ADR-0021 defers to #662 are decided and written into the ADR in the same change.
- The text-escaping rule matches the workspace's existing one (`holler_proto::log::escape_field_value`).

**The block is where the tests go.** Decision 11 puts the tests of holler-pane's two new pure modules in a holler-cli verb
test file, `tests/profile_verbs/show.rs`, because "holler-pane cannot take the test kit". That premise does not apply to
pure functions over records:

- `crates/holler-pane/tests/common/mod.rs` already provides a fully populated `Pane`, `ProfileSpec` and `Profile`.
- Every other holler-pane module is tested inside its crate (docs/testing.md:45).

Six warns follow. Most of them line this plan up with the in-flight #643, #644 and #663 plans. #643's own A (`fafd138`,
W-2 and W-3) raises the rig and the text-helper seams from the other side.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Decision 11 (lines 1753-1755); AC 1 "In `tests/profile_verbs/show.rs`" (line 1779) and AC 2; Files (T), lines 1900-1902 | file structure; pattern consistency | The tests of holler-pane's public pure API go into a holler-cli verb's test file. All nine holler-pane test files test that crate's API in `crates/holler-pane/tests/<topic>_test.rs` over `tests/common`, and docs/testing.md:45 documents the convention. The stated premise (no test kit in holler-pane) does not apply: these functions need no fake, and `tests/common/mod.rs` has the fully populated records AC 1a and 2c need. | Move AC 1 and AC 2 to `crates/holler-pane/tests/`: `profile_snapshot_test.rs` and `profile_diff_test.rs`, or one combined file as `argv_env_test.rs` does. Build over `common::{pane, spec, profile}` with the brief's neutral names. No manifest change is needed. `profile_verbs/show.rs` keeps AC 5. |
| 2 | warn | `is_member(pane: &Pane, profile_slug: &str)` (line 1556) | pattern consistency (typing) | The one definition of "live" takes a bare string, and nothing ties it to a slug. Every analogous check compares `ProfileName::slug()` on both sides: the test kit's `belongs(pane, &ProfileName)` (`holler-pane-testkit/src/profile_scope.rs:235`), its pane store (`pane_store.rs:228`) and the hub's (`panes/store.rs:347`). `is_member(p, profile.name.as_str())` compiles and matches nothing for a name with a space or a capital, which in 662b silently turns off `profile-has-live-panes`. No sibling plan pins this signature. | `pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool`, comparing slugs inside. Callers pass `&profile.name`. |
| 3 | warn | Forward-compat table (lines 1928-1936) | cross-story contract (parallel paths) | Three in-flight consumers are missing. (a) #644, amended in `7195993`, is now a hard consumer: relaunch's base is `spec_from_pane`; `port_of_policy` parses with `FIXED_PORT_POLICY_PREFIX` and requires `port_of_policy(&fixed_port_policy(p)) == Ok(p)`; after its rebase, T greps for the three signatures verbatim (644-brief lines 28, 1385-1406, 2146-2148). (b) #663's `StoreScope::resolve(P, None)` filters `PaneStore::list` by slug inline (663-brief Decision 2, line 1540), the same predicate as `is_member`. Its Reuse map (line 1525) already says shared membership logic belongs in holler-pane. (c) #643's `watch --profile` filter compares slugs inline (643-brief line 1069; #643's A, "Membership"). | Add the rows: `spec_from_pane`, `fixed_port_policy` and the prefix for #644, with signatures frozen; `is_member` for #663 and #643, to be reused when #662a merges first. In `spec_from_pane`'s doc, say that the port is copied as recorded: a record at port 0 gives `fixed:0`, which #644 treats as `usage` (644-brief line 1486). |
| 4 | warn | `SpecField::value` gives `FieldValue::Text` for `harness.kind` and `role` (lines 1503-1509) | pattern consistency | The plan does not say how `HarnessKind::Opencode` becomes `"opencode"`. Frozen `pane.rs` has no `as_str`, so the obvious body is one string literal per variant: a second copy of the serde names. The codebase's precedent is `pane/args.rs:142-144`, `parse_role`: "The accepted names are `PaneRole`'s serde names, so they cannot drift". | Take the text from serde (`serde_json::to_value(kind)` and its string, with a non-panicking fallback). Or add `Kind(HarnessKind)` and `Role(PaneRole)` variants that serialize themselves, as `Grid(GridPos)` does. If a literal is unavoidable, use an exhaustive `match` with no wildcard. AC 2e still pins the result. |
| 5 | warn | Decision 3: `SpecField::COMPARED` leaves out `PortPolicy` (lines 1724-1729, 1495) | forward compatibility | With `port_policy` not compared, a pane relaunched on another port shows as `matches` in `show`, `apply` (#664) and `profile-drift` (#665). Under #644's plan, "Only `FIXED_PORT_POLICY_PREFIX` followed by `<port>` exists" (644-brief lines 1483-1486). Comparing a spec's `fixed:<N>` with `fixed_port_policy(live.harness.port)` is then simply a port comparison. The rationale also cites an `auto` policy that exists nowhere in the code or ADRs (grep: no hits). | Keep the exclusion for 662a, because the kit's `sample_spec` still says `fixed`. Add a Follow-up: when the kit moves to `fixed:<port>` (644-brief line 2130), #664/#665 compare `port_policy` whenever the spec's value has the `fixed:` prefix. Drop `auto` from the rationale. |
| 6 | warn | Decision 12: `pub(crate) mod rig` inline in `tests/profile_verbs/list.rs` (lines 1756-1760) | duplication (test helpers); file structure | #643 plans a near-identical `Rig` over the same seven fakes in `tests/pane_verbs/list.rs` (643-brief Decision 10, lines 1081-1089). It is a different test target, so it can only be copied, not imported. #643's A (W-3) asks for one rig and a follow-up moving it into `tests/verb_harness/`. An inline rig also makes `list.rs`, #662's own verb file, the place #664 and #665 must edit to extend it. | Put the rig in its own file, `tests/profile_verbs/rig.rs`, declared from `list.rs` as `#[path = "rig.rs"] pub(crate) mod rig;`. This is how #644 handles `launch_rig.rs` (644-brief line 58), and it touches no frozen file. Another target can include it by `#[path]` instead of copying it. O names the base rig for both targets. |
| 7 | warn | "What each verb prints" (lines 1615-1640) vs #643's `pane get` text (643-brief Decision 5, lines 1035-1052; Decision 11, lines 1090-1097) | pattern consistency (cross-story) | The two read verbs, both in flight, print the same values two ways. Probe: `failed (missing "qwen38")` here, `failed missing=["qwen38"]` there. Absent: `none` here, `-` there. Keys: `host.cwd`/`herdr.grid` here, `project`/`pos` there. Escaping: control characters only here, `text_value`'s `{:?}` quoting there. JSON agrees, because both use the records' serde. #643's A (W-2) asks for one helper and one probe form, settled before the second of #643, #647 and #662 merges. | Keep this brief's probe form: it is the issue's acceptance text. O picks the shared forms and owner for the epic (#643's A suggests `output.rs`, owner #660), or records the divergence in Risks. `FieldValue`'s `Display` stays in holler-pane either way, because that crate cannot import a CLI helper. |

### Finding detail (the evidence behind each row)

**1. The pure modules' tests in a verb test file.**

- **The convention.**
  - `docs/testing.md:45`: "`crates/*/tests/` | per-crate integration tests ... | same convention, no cross-crate deps".
  - `crates/holler-pane/tests/` holds nine `<topic>_test.rs` files: `grid`, `argv_env`, `names`, `records`, `ports`,
    `error`, `error_class`, `adopted`, `rework`. Each tests holler-pane's own public API over `tests/common/mod.rs`.
- **The premise does not hold.** Decision 11 reasons: "holler-pane cannot take the test kit as a dev-dependency ... The
  tests of `profile_snapshot` and `profile_diff` go through their public API in `crates/holler-cli/tests/profile_verbs/show.rs`,
  with the kit's fixtures." But AC 1 and AC 2 call no port, so the kit's fakes are never needed. Its fixtures are not needed
  either:
  - `common::pane()` is a `Pane` with every field set: grid, cwd, workspace, model, role, env, context, command,
    `probe.check`, `probe.expect` and `probe.last`. AC 1a overrides only grid, role, the env names, the ceilings and the
    port.
  - `common::spec()` and `common::profile()` are fully populated too, which AC 2e needs.
- **No manifest change.**
  - `crates/holler-pane/Cargo.toml` has no `[[test]]` and no `autotests` key, so a new file in `tests/` is
    discovered automatically.
  - `serde_json`, which AC 2e uses, is a normal dependency; `records_test.rs` already uses it.
  - The modules are reachable as `holler_pane::profile_snapshot::...` and `holler_pane::profile_diff::...`
    (`lib.rs:50-51`).
- **Wave 3's other plans do not set a counter-pattern.** #647 and #644 test their holler-pane engines from holler-cli
  because those engines are **port-driven** and need the fakes (647-brief Decision 12: "All port-driven tests are in
  `tests/pane_verbs/doctor.rs`"). `profile_snapshot` and `profile_diff` are pure over records, the same kind of code as
  `grid.rs` and `argv.rs`.
- **What the plan as written causes:**
  - The spec of a shared pure API lives in #662's own verb file (epic ruling 2). #644, #650, #663, #664 and #665 consume
    `spec_from_pane`, `diff_spec`, `diff_profile` and `is_member`. A story that changes one of them would either edit
    #662's `profile_verbs/show.rs` or start a second home for the same tests.
  - `profile_verbs/show.rs` would test `profile_from_panes` (AC 1c), which `show` never calls.
  - `cargo test -p holler-pane` would not run its own modules' tests, so AC 11's `-p holler-pane` gate is empty for them.
  - About 16 cases (AC 1, 2 and 5) and a fully populated pane builder land in one file. That pushes it toward the lint's
    600-line warning, with no fallback named. `scripts/lint.sh` step 4 counts test files too.
- **The fix is cheapest now.** Moving it costs a few lines in the brief before T writes RED. After merge it means moving
  tests across crates and swapping their fixtures. The fix does not change the public API, so #644's pinned signatures are
  untouched.

**2.**

- The analogous helper: `holler-pane-testkit/src/profile_scope.rs:234-239`, `fn belongs(pane: &Pane, profile: &ProfileName)`,
  "compared by slug".
- `Profile.slug` is a plain `String` field, which makes the wrong call easy to write.
- The likely next callers both hold a `ProfileName`: #663's `resolve(profile: &ProfileName, ..)` and #643's
  `--profile P`.

**3.**

- 644-brief at `7195993`:
  - line 6, "Depends on #662a and #663, merged to `main` first";
  - line 28, the dependency row;
  - lines 1385-1406, evidence I-1, the three signatures pasted verbatim;
  - lines 1955-1956, the Reuse map;
  - lines 2146-2148, the post-rebase grep.
- 663-brief at `ec3a214`: line 1540 (Decision 2) and line 1525 (Reuse map row).
- 643-brief at `fafd138`: line 1069; `docs/handoffs/643/handoff-A.md`, "Checked and consistent", Membership.

**4.**

- `crates/holler-pane/src/pane.rs:117-130`: `HarnessKind` has no `as_str` or `Display`.
- `crates/holler-cli/src/pane/args.rs:142-150`: `parse_role` goes through serde.

**5.**

- 644-brief lines 1483-1486 and 1979-1982 (its decision 5) and line 2130 (the kit's `sample_spec` follow-up).
- This brief's Out of scope (line 1945) lists the comparison but no follow-up.

**6.**

- 643-brief lines 1081-1089.
- 644-brief line 58 (`launch_rig.rs` by `#[path]`).
- #643's A W-3 and #644's A warn 6 already count four planned rigs across the two test targets.
- A `#[path]` on a `mod` in the non-`mod.rs` file `list.rs` resolves against `list.rs`'s directory, so the rig lands at
  `tests/profile_verbs/rig.rs`. holler-cli's manifest lists its targets, and a file inside a target's directory is not a
  target.

**7.**

- 643-brief Decision 5 (the `get` text, `probe-last` and the `-` marker) and Decision 11 (`text_value`).
- #643's A W-2 lists the five existing escape helpers; none is reusable as is.
- `crates/holler-proto/src/log.rs:474-497`: `escape_field_value` escapes only `char::is_control` characters, with
  `escape_default`, and passes all other text through. That is exactly the rule this brief gives `FieldValue`'s
  `Display`, but the function is private.

## Notes for O

Amend the brief as follows, then start a **fresh** run (a `resumeFromRunId` would replay this verdict).

1. **Finding 1 (block): test placement.**
   - Rewrite Decision 11. Pure functions over records are tested in their own crate, in `crates/holler-pane/tests/`, over
     `tests/common` (docs/testing.md:45). Only port-driven code needs the test kit, and these two modules have none.
   - AC 1 and AC 2 move to `crates/holler-pane/tests/profile_snapshot_test.rs` and
     `crates/holler-pane/tests/profile_diff_test.rs`. Use one combined file if T's file budget binds; `argv_env_test.rs` is
     the precedent.
   - Each new file starts with `mod common;` and carries the neighbours' header
     `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #662`.
   - The fixtures are `common::{pane, spec, profile, pane_name, profile_name}`, overridden per case. Use the brief's
     neutral names wherever a test sets or asserts a name (`demo-c1r2`, `/srv/demo`, `demo-provider`).
   - AC 2c's round trip runs on `common::pane()` and on AC 1a's pane. The `sample_pane` half is already pinned at the verb
     level by AC 5b.
   - Amend these places to match:
     - AC 1's lead sentence and AC 2's location;
     - the Files (T) list: add the new file or files. `profile_verbs/show.rs` keeps AC 5, and `list.rs` keeps AC 4 and the
       rig;
     - AC 11, which now really runs AC 1-2 under `cargo test -p holler-pane`;
     - AC 12 and 14, which now cover the new files (`// #662` allows, `rustfmt --check`);
     - the Size check's T count;
     - the test plan's RED note. The signature stubs in `holler-pane/src` compile the new tests, and those fail on their
       assertions; 2c still passes vacuously, as noted.
   - Nothing else in the plan changes.
2. **Do not change** `spec_from_pane`, `FIXED_PORT_POLICY_PREFIX` or `fixed_port_policy`.
   - #644's amended brief (`7195993`) pastes them verbatim and greps for those exact lines after its rebase.
   - The other API items, `is_member` included, are not pinned by any sibling plan.
3. **Warns 2-4 (recommended, cheapest in the same amendment).**
   - `is_member(pane: &Pane, profile: &ProfileName)`, and update the callers in Behaviour.
   - Add the three Forward-compat rows, plus the port 0 sentence in `spec_from_pane`'s doc.
   - Say that `SpecField::value` takes `harness.kind` and `role` from serde, or switch to typed variants.
4. **Warns 5-7.**
   - Add the port-policy comparison to Follow-ups, and drop `auto` from Decision 3.
   - Move the rig to `tests/profile_verbs/rig.rs` through `#[path]`, and say which rig #643 builds on.
   - Settle the shared text forms (probe result, absent marker, escaping) with #643's W-2 before the second of #643, #647 and
     #662 merges, or record the divergence in Risks.

## Patterns referenced

- `docs/testing.md:45`, and `crates/holler-pane/tests/{common/mod.rs,records_test.rs,argv_env_test.rs,grid_test.rs}`.
- `docs/adr/ADR-0021.md`: section 3 (lines 112-154), section 5 (lines 169-193), section 8 (lines 264-306), section 9's
  failure-mode table (lines 331-351) and "Deferred to named stories" (lines 521-533).
- `crates/holler-pane-testkit/src/profile_scope.rs:234-239` (`belongs`), `crates/holler-cli/src/pane/args.rs:142-150`
  (`parse_role`), `crates/holler-proto/src/log.rs:474-497` (`escape_field_value`).
- `crates/holler-cli/tests/verb_harness/mod.rs`, `tests/profile_verbs/main.rs`, `tests/pane_verbs/main.rs`.
- The in-flight sibling plans, read from their worktrees at about 17:20-17:30 MDT. They are evidence of intent, not merged
  code:
  - `docs/handoffs/643-brief.md` and `643/handoff-A.md` (`fafd138`, A PASS);
  - `644-brief.md` (`7195993`, amended after A BLOCK);
  - `647-brief.md` (`a98258a`);
  - `663-brief.md` (`ec3a214`).
