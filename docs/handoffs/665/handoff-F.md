# Handoff-F (rework 1): after S REWORK - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (22:45 MDT)
**Branch:** `issue-665-implementation`, on `bde41cf` (S's REWORK)
**Issue:** #665
**Reworks:** `handoff-S.md` items 1 and 2, which are production.
- Items 3 and 4 are test-only, so they are T's. They are listed under "Tests that look wrong (for T)", with the three test strings that item 1 changes.
- The first round's handoff follows this section, unchanged.

## What was done

**Item 1: the spec count is pluralized.**
- `crates/holler-cli/src/profile/import.rs` and `export.rs`: `render` prints the count through `super::list::count`, as `create`, `show`, `list`, `delete` and `apply` do.
- A one-spec profile now prints `imported profile Fleet (fleet): 1 spec, generation 1` and `exported profile Fleet (fleet) to FILE: 1 spec`.

**Item 2: the no-echo claim is made true, and stated for what it covers.**
- `crates/holler-cli/src/profile/export.rs`:
  - `spec_refused` keeps both grid codes but replaces their messages. The new messages name the member and the spec, never the text:
    - `grid-ambiguous`: `ambiguous grid position: herdr.grid of spec N (write r<row>c<col>, c<col>r<row> or <row>,<col>)`;
    - `grid-out-of-range`: `grid position out of range: herdr.grid of spec N (rows and columns run from 1 to 65535)`.
  - `Document::specs` now routes a text position's refusal (`read_position`, D8 step 4) through `spec_refused` too. The object form's `pos` label and zero cell already reach it through the decode (step 5).
  - New constants `GRID_FORMS` and `GRID_BOUNDS`.
  - The module docs restate the rule.
- `crates/holler-cli/src/profile/import.rs`:
  - the `--help` text now says "no refusal quotes what a spec holds: a bad position or member is named by the spec's number";
  - the module docs say the same, and say that the document's name is printed as a name.
- `docs/pane-control.md`, the import subsection, and `CHANGELOG.md` carry the same wording.

**S's advisory 8 (smaller items):**
- `crates/holler-hub/src/profile/store.rs`: the 132-character module-doc line is rewrapped (doc only).
- `docs/pane-control.md`, the rename subsection: the new name's log gets one entry "after the log of a profile deleted earlier under the new name, if there was one". It used to say the log "begins with" the rename (D4).
- `CHANGELOG.md`: the bullet names the `error` event `profile_rename_restore_failed`, as the registries' earlier entries name theirs.

**Handoffs:** eight `evidence.md` entries under "F rework 1", and one `decisions.md` entry.

## Design decisions

- **Option (b) of S's item 2 (make the claim true), not (a) (narrow it), for three reasons:**
  - The brief's import acceptance lists `grid-ambiguous` (`hj-c1r2`) and `grid-out-of-range` (`r0c1`) among the refusals, then says "**the refused value is never printed**". Option (a) would have documented an exception to that.
  - Not echoing the file is the rule the repository's file importers already keep. `pane import`'s launcher table names "the line and column of the error's span, and never the error's text", and `spec_refused` already redacted `usage`.
  - The spec's number is better than the excerpt for an operator: it says *which* spec, in a file with several.
- **The claim is scoped to what a spec holds.** Round 1's "no refusal prints what the file holds" cannot be made true:
  - the document's name is printed as a name, the way `--name` is: on success, by `profile-exists` (create's `taken`, which A asked import to reuse), and quoted by `ProfileName::parse` when it is not a valid name;
  - the `version` refusal prints the number.

  Hiding the name would make `profile-exists` say nothing useful. So the `--help`, both module docs, the guide and the CHANGELOG now say "no refusal quotes what a spec holds", and the import module docs name the exception.
- **Where the arms go.** They go in `spec_refused`, the one place that already maps a spec's refusal. Both steps route through it, so there is one redaction path, not two. Positions are still read for every spec before any spec decodes, so the order of D8's steps 4 and 5 is unchanged.
- **The hint text is repeated in two constants.**
  - holler-pane's `FORMS` and `BOUNDS` are private. Making them public would edit `grid.rs`, which is outside the Blast radius.
  - Cutting the hint out of the original message is not safe. The `pos` message has no hint, and the excerpt it carries could hold `" ("`.
  - The member's path comes from `SpecField::Grid.as_str()`, the path `profile show` and the drift finding use, not from a literal.
- **One message for both sources of a grid code.** The `pos`-label case keeps the generic hint "write r<row>c<col>, ...", which fixes it too: export writes the text form, and writing the position as text removes the label. A label-specific message would rest on what `GridPos`'s deserializer happens to refuse today.

## Reuse / extend-vs-new

- **`super::list::count`** is the shared pluralization helper. Nothing new was written for it (A-dup warn 2's first option).
- **`spec_refused` is extended** with two arms. It is not a parallel redaction path, and `Document::specs` routes both steps through it.
- **`SpecField::Grid.as_str()`** supplies the member's path.
- **New:** only `GRID_FORMS` and `GRID_BOUNDS`, which are message text. The reason is above.

## Architecture notes for A

None. No layer, public interface, dependency, protocol field or error code changes. The change is two private constants and two match arms in one CLI module, plus doc and message text. The grid refusals keep their codes and exit classes (3).

## Deviations from spec / wireframe

None new. The scoped wording ("what a spec holds") is narrower than round 1's prose, which claimed more than the code did and more than the brief asked.

## Tier 1 self-check (incl. tests now GREEN)

- **Gates, on the final sources:**
  - `cargo clippy --workspace --all-targets -- -D warnings` exits 0;
  - `bash scripts/lint.sh` exits 0, and its warnings are only pre-existing files of other crates;
  - `bash scripts/changelog-check.sh` prints `ok`;
  - `cargo machete` finds no unused dependency;
  - `rustfmt --edition 2021 --check` passes on the three touched `.rs` files.
- **Sizes:** `export.rs` is 306 lines, `import.rs` 129 and `profile/store.rs` 562.
- **`cargo test -p holler-cli --test profile_verbs`: 109 passed, 3 failed.** The three failures are exactly the pins of `1 specs` that item 1 changes, each now `1 spec`:
  - `export::export_out_writes_the_file_at_0600_and_replaces_one_that_exists`;
  - `import::import_creates_a_detached_profile_at_generation_one_in_both_formats`;
  - `import::import_name_overrides_the_files_name_and_is_parsed`.

  Every refusal test passes, including the grid codes, the canary tests and `a_document_of_another_shape_is_usage_and_never_quoted`.
- **The workspace run** was `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the pipeline's verify command with `--no-fail-fast`), finished about 22:50 MDT. Across 154 targets: **2,431 passed, 3 failed, 32 ignored.**
  - The total, 2,434, is T-green's last count.
  - The 3 failures are the three `profile_verbs` pins above. No other target failed, and #693 did not recur.
  - The run exits 101 until T updates those three strings.
- **Item 2, checked by hand** (F wrote no test). A scratch hub ran on its own temporary state dir, as the process tests do, and was stopped and removed afterwards. Four documents had a made-up canary in spec 2's position: a text position, `r0c1`, an object-form `pos` label and an object zero cell. Each was refused in text and in JSON with its code and exit 3, as `herdr.grid of spec 2` with the hint or the bounds. The canary appeared 0 times, and `holler profile list` printed `no profiles`.
- **The control.** With the two new arms removed, the text position printed `ambiguous grid position: "r1<canary>" (write ...)` and the label printed `pos "<canary>" disagrees with row 2 and column 1`. The arms were restored, and `git diff` shows no trace of the control.

## Evidence appendix

`docs/handoffs/665/evidence.md`, section "F rework 1 (after S REWORK, items 1 and 2)", holds eight entries:
- `count`;
- `GridPos::parse`'s excerpt;
- the object form's `pos` and zero-cell refusals;
- `from_closed` keeping the grid text, while the other guards are unit variants;
- the private `FORMS` and `BOUNDS`;
- `ErrorBody`'s message as `Display`;
- `SpecField::Grid`'s path;
- `ProfileName::parse` quoting an invalid name, which is why the claim is scoped to specs.

A script checked every quoted line against its cited lines: 0 mismatches.

## Tests that look wrong (for T)

F edits no test. Items 1 to 4 below are needed for GREEN or are S's REWORK. Item 5 is optional.

1. **Item 1 changes three pinned strings.** Change `1 specs` to `1 spec` in:
   - `crates/holler-cli/tests/profile_verbs/import.rs:80`: `"imported profile Fleet (fleet): 1 spec, generation 1\n"`;
   - `crates/holler-cli/tests/profile_verbs/import.rs:159`: `"imported profile Fleet Copy (fleet-copy): 1 spec, generation 1\n"`;
   - `crates/holler-cli/tests/profile_verbs/export.rs:318`: `format!("exported profile Fleet (fleet) to {out_text}: 1 spec\n")`.
2. **Item 2(b) is unpinned.** S asks T to pin it with `CANARY`, for example in `import/refusals.rs`'s `a_bad_position_is_its_grid_code`, using the file's own `refused` helper (code, exit 3, never printed, nothing written):
   - `spec["herdr"]["grid"] = json!(format!("r1{CANARY}"))` is `grid-ambiguous`;
   - `spec["herdr"]["grid"] = json!({"row": 2, "col": 1, "pos": CANARY})` is `grid-ambiguous`;
   - optionally, `json!({"row": 0, "col": 1})` is `grid-out-of-range`, and the text message names `herdr.grid of spec 1`.

   The control is the one above: without the two arms, the first two print the canary.
3. **S item 3 (test-only).** The spec-level `usage` redaction is unpinned. Add members of the wrong type that hold `CANARY`, each refused as `usage` (exit 2) and never printed:
   - `spec["context"]["soft"] = json!(CANARY)`;
   - `spec["role"] = json!(CANARY)`.
4. **S item 4 (test-only).** "The stamp as `updated`" is checked only as `>=`. Assert equality:
   - in `crates/holler-hub/tests/profile_rename_test.rs:148`, `stored.updated == rig.log("Day Shift")[0].at`;
   - in conformance case 23 (`crates/holler-pane-testkit/src/conformance/profile_store/rename.rs:43-61`), `renamed.updated` equals the `at` of the new name's one log entry.
5. **Optional.**
   - `import/refusals.rs:1-3` says each refusal "never prints what the file holds". The rule now stated is "no refusal quotes what a spec holds", because the document's name is printed as a name. A test that put `CANARY` in the file's `name` would fail by design.
   - S advisory 3: the name and doc of `process/profile_io.rs::rename_on_an_empty_hub_reaches_the_real_handler` overstate it, since the CLI's own `get` answers first.

## Known issues

- **S advisory 1, for the rebase before merge (not F's):** the branch conflicts with `origin/main` in `CHANGELOG.md`, `docs_pane_control_test.rs`, ADR-0003, ADR-0021 and `docs/pane-control.md`.
- **S advisory 2:** `docs/protocol/v2.md:817` still says "the proposed `profile/rename`". The brief's "Not touched" list puts v2.md out of reach, so fix it at the rebase or as a follow-up.
- **A-dup warn 3** (an opaque `Moves` type for `panes::Locked`) is optional and left as a follow-up.
- **`spec_refused`'s `other => other` arm** relies on holler-pane's guards keeping their no-echo rule. Today only the two grid codes carry text from a spec's decode (`evidence.md`).
- **A correction to round 1** (S advisory 4): `agent-key-invalid` does not pass through import. As an open code it folds into `usage` in `from_decode`, and the spec's generic message replaces it. The behaviour matches the docs, and only round 1's prose below was wrong.

## Files changed (this round)

Production:
- `crates/holler-cli/src/profile/export.rs`
- `crates/holler-cli/src/profile/import.rs`
- `crates/holler-hub/src/profile/store.rs` (module doc only)

Docs:
- `CHANGELOG.md`
- `docs/pane-control.md`

Handoffs:
- `docs/handoffs/665/decisions.md`
- `docs/handoffs/665/evidence.md`
- `docs/handoffs/665/handoff-F.md`

---

# Handoff-F: Phase 5 (implement; the Workflow script's phase 6) - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (21:20 MDT)
**Branch:** `issue-665-implementation`, on `d79ce40` (T's RED)
**Issue:** #665

## Confirmation

The automated run has no operator to confirm with, so the table is recorded here.

| Field | Value |
|---|---|
| GitHub issue | #665 (confirmed by the operator 2026-10-11, 19:40 MDT) |
| Working branch | `issue-665-implementation` |
| Build plan phase | epic #633; one story, three parts (the brief's question 2, decided at launch) |
| Input documents read | `docs/handoffs/665-brief.md`, `665/handoff-A.md`, `665/handoff-T-red.md`, `665/decisions.md`, ADR-0021 and ADR-0003 as T amended them, `docs/pane-control.md` |
| Acceptance criteria count | 21 numbered, plus the conformance, verb, process and docs blocks |
| Handoff document path | `docs/handoffs/665/handoff-F.md` |

There is no scope split. The brief and the launch decided on one story, and each part is small and follows a pattern already in the code.

## What was done

**A. The rename transaction (hub)**
- `holler-hub/src/profile/store.rs`: `Store::rename(from, to, expected, actor, panes: Option<&PaneState>)` and the pure step `renamed`.
  - The step checks, then builds the record and two entries: OLD's tombstone with `deleted`, then NEW's entry with `renamed {from}`. Both are at the new generation and share one stamp. A rename that keeps the slug gets one entry.
  - The transaction: the profile lock, then the pane lock. The member moves; the pane save, then the profile save, then the restore if the profile save failed; and only then the commit to both memories.
  - The twin helpers are generalized over a slice of changes (`entries_with`, `save`, `commit`), with `apply` split out and `after(cursor)` added. `check_name` and the rename share `taken`.
- `holler-hub/src/panes/store.rs`: the same generalization, plus `Store::locked` and the guard-holding `Locked`. `Locked` has `moves`, `save`, `restore` and `apply`. A failed restore is logged as `profile_rename_restore_failed`.
- `holler-hub/src/panes/mod.rs`:
  - `PaneState::locked`;
  - the crate-private re-export of `Locked`;
  - the operating limit: the pane lock is held across two saves during a rename.
- `holler-hub/src/profile/rename.rs`: `dispatch` is `panes::handlers::run` over `ProfileRenameParams`, calling `Store::rename` with the pane registry.
- `holler-hub/src/profile/mod.rs`:
  - `ProfileState::rename`, the port, is the record half;
  - `dispatch` takes `panes: &Arc<PaneState>`;
  - the module docs and operating limits no longer say "never reads the pane registry".
- `holler-pane-testkit/src/profile_store.rs`: the fake's `rename`. It runs in one `feed.write` with the fake's own `entry`, `record` and `next_generation`, and its own `taken`, which `check_name` now shares.
- Doc comments only:
  - `holler-pane/src/profile.rs`: `ProfileStore::rename`, worded per A's warn 3;
  - `holler-pane/src/reply.rs` and `holler-proto/src/methods.rs`: "PROPOSED" dropped;
  - `holler-cli/src/pane/wiring/store.rs`: the `profile/rename` line.

**B. The verbs (CLI)**
- `holler-cli/src/profile/rename.rs`: `holler profile rename OLD NEW`. It reads OLD, refuses NEW equal to OLD's stored name (`usage`), then makes one `ProfileStore::rename` call with the generation it read and the actor `holler profile rename`.
- `holler-cli/src/profile/export.rs`: `holler profile export NAME [--out FILE]`.
  - The shared `Document`, with `of` and `text` for writing, and `read` and `specs` for reading.
  - The shared `file_error`.
  - `--out` is written by `write_atomic` at 0600.
- `holler-cli/src/profile/import.rs`: `holler profile import FILE [--name NAME]`. It reads at most 1 MiB, takes D8's steps in order, refuses a taken name with create's `taken`, and stores through `insert_profile`. The profile is detached, at generation 1, with the actor `holler profile import`.
- `holler-cli/src/profile/create.rs`: `taken` is now `pub(crate)`.
- `holler-cli/src/profile/mod.rs`: its doc line.
- `holler-cli/Cargo.toml`: `serde_json` declares `preserve_order` (A warn 4a), with the `# for holler-cli: ...` marker. The workspace `Cargo.toml` comment now says holler-cli relies on it.

**C. The drift finding**
- `holler-pane/src/findings.rs`: `FindingKind::ProfileDrift`, appended last to the enum and to `ALL`, with code `profile-drift` and no remedy.
- `holler-pane/src/reconcile/drift.rs` (new): `findings(profiles, scope)`.
  - It reads the profiles with one `list`, and only when a pane in scope names a profile.
  - It runs `diff_profile` over each profile and its members in scope.
  - It makes one `Finding::about` per pane with a differing row. The message is `differs from profile <quoted> on <fields> (see holler profile show)`.
  - A failed `list` is `observe-failed`, built with the parent's `failed_message`.
- `holler-pane/src/reconcile.rs`: one `mod drift;`, one call after the stray step, and one sentence in the module docs.

**Docs**
- `docs/pane-control.md`: every edit of the brief's "The guide and its test: the exact edits":
  - section 3: the opening paragraph, and three subsections in place of the PROPOSED one;
  - section 5: the table row, "five name none", and the drift paragraph;
  - section 6: the bullets;
  - section 7: the sentence;
  - section 9: the row;
  - section 12: the bullet.
- `docs/adr/ADR-0021.md`, beyond T's amendment:
  - section 9's `export` and `import` rows state the file-error split (deviation 1);
  - "Rename as built" names `profile_rename_restore_failed`.
- `CHANGELOG.md`: one Enhancements bullet linking #665.
- `docs/handoffs/665/evidence.md` (new) and an F entry in `decisions.md`.

## Design decisions

- **The transaction shape is a guard-holding handle (A warn 2b, the preferred option), not a closure.**
  - The profile store takes its own lock, then `PaneState::locked()`, so the pane registry never calls into the profile registry. ADR-0021 section 7's lock-order sentence stays true as written, with no amendment.
  - The pane registry's load check comes first, so a corrupt `panes.json` with a missing OLD is `store-corrupt`, as H9 pins.
  - The closure alternative was rejected: it needs the section 7 amendment and a pre-check.
- **One `Store::rename` for both halves (A warn 3).** The port passes `None` (the record half), and `profile/rename` passes the pane registry. The checks, the log entries and the events are written once.
- **The twins are extended in place (A warn 2c).**
  - Both stores' `entries_with`, `save` and `commit` take a slice, and the single writes pass one change.
  - `apply` is split out of `commit`, so the rename commits memory only after both saves.
  - The restore is the pane store's unlogged `write` of the committed table, a real save. It is not a byte copy, and it is not a `commit_rename` or `save_two`.
- **A rename with no member touches no pane file and wakes no pane watcher.** The handle's methods do nothing for an empty list of moves. This includes the conformance cases run over the wire.
- **The member moves are computed under the pane lock.** A `pane/cas_put` that lands before the rename takes the pane lock is moved too. One that comes after is refused, either by the generation or by the membership hook. H14 pins this over 20 rounds.
- **Key order (A warn 4a).** Each pane of the document is `to_value(spec)` with `herdr.grid` replaced in place. `preserve_order`, now declared by holler-cli, keeps the spec's order. `ProfileSpec`'s fields are not mirrored, so a field added later is exported without an edit here.
- **Reading a document never echoes the file.**
  - The top level decodes as `Document`, with `deny_unknown_fields`, and maps any failure to `usage` from serde's category and position alone.
  - A spec's `usage` replaces serde's message with the spec's position, because serde's text can quote a key or a value.
  - A typed guard's own code passes through: `profile-secret-refused`, `command-not-argv`, `env-name-invalid`, the grid codes and `agent-key-invalid`.
- **Text forms.** They print `N specs` literally, as the tests pin `1 specs`, not through `count()`. The export path goes through `show::text`. In JSON, `out` is the path as given.
- **Drift runs over the in-scope members of each stored profile.** For a pane in scope this gives the same row `profile show` computes over all members. One `list` is made per pass, and only when a pane in scope names a profile. A pane naming a profile that does not exist is not reported (the brief puts that out of scope).

## Reuse / extend-vs-new

Against A's "Reuse expectations for Phase 7":

1. **Membership** is `profile_diff::is_member`: in the member moves (`Locked::moves`) and in `drift.rs`. The doctor rig's `profiles_of` is T's.
2. **The hub's write rules are reused.**
   - Generations come from `next_generation`, and the log entries from `log_entry`.
   - Both entries come from `ProfileEntry::next`, and the one stamp is `entry::stamp`, folded over OLD's entry and NEW's previous entry.
   - `entry.rs` is unchanged.
3. **The twins are extended in place**, under the same names, and the faults are logged through `log_fault`.
4. **`profile/rename.rs` follows the handler pattern:** `panes::handlers::run` with a closure.
5. **One `Store::rename`** serves the port and the transaction.
6. **The fake's rename** runs in one `feed.write` and uses the fake's own `entry`, `record` and `next_generation`. It is not a copy of `put` or `remove`.
7. The conformance cases are T's.
8. **The CLI verbs reuse what exists.**
   - Import stores through `insert_profile`, and builds the record as `Profile { panes, ..profile_from_panes(&name, &[]) }`.
   - `profile-exists` comes from create's `taken`, now `pub(crate)`.
   - The specs decode through `decode_params`, and the grids through `GridPos::parse`.
   - `--out` is written by `write_atomic` at 0600.
   - Paths print through `show::text`, the actors are `const ACTOR`, and the result goes through `output::emit`.
   - The shared document code is in `export.rs`, reached as `super::export::...`. The frozen `profile/mod.rs` gets only its doc line.
9. **Drift reuses the existing comparison and finding helpers:** `diff_profile`, `Finding::about`, `findings::quoted`, `SpecField::as_str` and `super::failed_message`. `reconcile.rs` gets one `mod` line, one call and one doc sentence.
10. The doctor rig is T's.
11. **D6 holds.** No port method, no `Ports` field or literal, and no `wiring/bind.rs` edit.

Nothing new was created where an extension was called for. Two things are new, each where the brief or A asked for it:
- `drift.rs`, which the brief names;
- the `Locked` handle, A's warn 2b option.

## Architecture notes for A

- **Layers.** Rename is hub-side: a store method and a handler. The verbs are CLI-side over the existing ports. The file format is CLI-side (ADR-0021 section 5). Drift is in `holler-pane`'s reconcile.
- **Changed public signatures in holler-hub:**
  - `profile::dispatch(.., panes: &Arc<PaneState>)`;
  - `profile::rename::dispatch(.., profiles: &ProfileState, panes: &Arc<PaneState>)`.
  - Every caller already passed an `&Arc` (`pane_dispatch::forward` and the tests).
- **New crate-private surface:**
  - `panes::Locked` and `PaneState::locked`: the only way the profile registry reaches the pane registry;
  - `profile::store::Store::rename`.
- **Lock order.** Profile, then pane, only in `Store::rename`. `check_membership` still takes and releases the profile lock before any pane lock (evidence).
- **New log event:** `profile_rename_restore_failed` (`error`).
- **Dependency feature:** `serde_json/preserve_order` is declared by holler-cli. It was already on through unification, so `Cargo.lock` is unchanged.
- **New finding kind** `profile-drift`, appended last.

## Deviations from spec / wireframe

1. **File errors (D8, D9) follow A's warn 5 split, not "every file error is `usage`".**
   - `usage`: a path that names nothing usable (`NotFound`, `NotADirectory`, `IsADirectory`, `InvalidInput`), not a regular file, over 1 MiB, or not a document.
   - `unavailable`: any other I/O failure, such as permission denied.
   - ADR-0021 section 9's `export` and `import` rows and the guide's "Ends with" lines say so.
   - T pinned only the `usage` cases, and they pass.
2. **The issue's "edit none of #661's files" does not hold (A warn 2d).** The rename has to edit `profile/store.rs` and `profile/mod.rs`, whose lock, save and commit are private. The brief's Blast radius already lists them. Since the brief is not edited after the plan check, this override is recorded here and in `decisions.md`.
3. **The guide uses placeholders the brief's prose leaves out.** It writes `holler profile apply NAME`, `holler pane relaunch PANE`, `holler profile show NAME`, `holler profile import FILE` and `holler profile export NAME` where the brief's text has the bare verb. `docs_cli_test` parses every inline `holler ...` span, so a bare verb with a required positional would fail it.
4. **The rename subsection does not say "the profile keeps its log".** The log is filed by slug (D4), so it says the old name's log ends with the rename and the new name's log begins with it.

## Tier 1 self-check (incl. tests now GREEN)

- **Build and lint:**
  - `cargo clippy --workspace --all-targets -- -D warnings` is clean.
  - `bash scripts/lint.sh` exits 0. No `.rs` file is near 900 lines; the largest touched are `profile/store.rs` (560) and `panes/store.rs` (513).
- **Format:** each touched module tree passes `rustfmt --edition 2021 --check`. rustfmt was run only on the leaf files F changed, and no other file moved (T's note on `cargo fmt -p`).
- **The workspace run.** The command is `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, finished about 21:12 MDT. Across 154 targets: **2,432 passed, 2 failed, 32 ignored.**
  - `pane_integration` :: `clients::the_profile_client_passes_the_profile_store_conformance_suite` fails cases `rename-of-one-slug-is-one-put` and `rename-feed-is-deletion-then-put`. This is a test sequencing defect; see "Tests that look wrong".
  - `remote_admin_test` :: `remote_admin_traffic_never_moves_the_bodys_last_seen`: `last_seen` moved by one second under the load of the full run. The roster and presence code it tests is outside this diff, it passed in T's RED run, and it passes 3 of 3 in isolation (`cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never_moves_the_bodys_last_seen`).
- **Re-run on the final sources**, after the formatting and doc edits:
  - `cargo test -p holler-hub -p holler-pane -p holler-pane-testkit -p holler-proto`: 729 passed, 0 failed. This includes:
    - `profile_rename_test` (10) and `profile_rename_faults_test` (8);
    - `pane_dispatch_test`, `profile_handlers_test` and `profile_registry_test`, whose 28-case suite runs on `ProfileState`;
    - `profile_store_conformance_test` (12: the fake passes, every mutant fails on its case);
    - `findings_test` (2).
  - `cargo test -p holler-cli`:
    - `--test profile_verbs`: 112 passed;
    - `--test pane_verbs`: 340 passed, the 7 drift tests and every existing doctor test included;
    - `--test pane_cli_process`: 38 passed, `profile_io.rs` included;
    - `--test docs_pane_control_test`: 18 passed;
    - `--test docs_cli_test`: 3 passed;
    - `--test cli_surface_test`: 3 passed;
    - `--test pane_integration`: 65 passed, 1 failed (the same wire case as above). The `no_hub` rows pass.

## Evidence appendix

`docs/handoffs/665/evidence.md`: 19 entries, each checked by a script against its cited lines. They cover:
- the lock order with `check_membership`;
- the stamp, `ProfileEntry::next` and the load check;
- `save_doc`'s atomic write (the inode H11 sees);
- `is_member` and `next_generation`;
- `write_atomic` creating no directory;
- `decode_params` and `from_decode`;
- the env and argv guards;
- `insert_profile`;
- `write_line`'s newline and `show::text`;
- `Finding::about` and `failed_message`;
- the wire client's eager first watch request, with the suite's case 19, behind the test defect below.

## Tests that look wrong (for T)

- **The cases.** `holler-pane-testkit/src/conformance/profile_store/rename.rs`, case 26 `rename_of_one_slug_is_one_put` and case 27 `rename_feed_is_deletion_then_put`.
- **What fails.** Both fail over the wire only, in `pane_integration/clients.rs` through `HubProfileStore`, with "next() after ...: expected the change of "demo-alpha" ..., got None". They pass on the fake and on `ProfileState`.
- **Why.** `HubProfileStore::watch` makes its first long-poll inside `watch()` and keeps that reply for the first `next()` (#649, D4; evidence). A watch opened at the head therefore yields the idle `Ok(None)` of a window that ended before the rename. The suite's own cases 19 and 21 read that first idle item before they write; cases 26 and 27 rename right after `open(store, last_cursor(store)?)`.
- **The test-only fix.** Either works on all three runners:
  - **(a)** read the first `next_item` (expect `None`) after `open` and before the rename, as case 19 does;
  - **(b)** take `last_cursor` before the rename, and open the watch from it after the rename, as case 20 does: `drain(&mut open(store, before)?)`, expecting exactly the deletion then the put.
- **Production code is unchanged.** The client's eager first request is #649's documented design, and the suite already accommodates it.

## Known issues

- **The fake's two appends are not atomic.** Its rename appends two events in one `feed.write`, so a `u64` cursor overflow on the second would leave the first in place. This cannot happen in a test, and the feed's head is private to `feed.rs`. It is recorded, not fixed.
- **The registry's fault events are not in the README.** The README "Debug output" table lists none of them (`*_registry_corrupt`, `*_registry_write_failed`), so `profile_rename_restore_failed` is documented in ADR-0021 "Rename as built" and in code only.
- **Inherited residuals, documented in the ADR and not fixed (brief D2):**
  - a pane join naming OLD, checked before a rename and written after it;
  - no fsync.
- **Not reported:** a pane that names a missing profile (the brief's follow-up, `pane-profile-missing`).

## Files changed

Production:
- `Cargo.toml` (comment)
- `crates/holler-cli/Cargo.toml`
- `crates/holler-cli/src/pane/wiring/store.rs` (doc)
- `crates/holler-cli/src/profile/create.rs`
- `crates/holler-cli/src/profile/export.rs`
- `crates/holler-cli/src/profile/import.rs`
- `crates/holler-cli/src/profile/mod.rs` (doc)
- `crates/holler-cli/src/profile/rename.rs`
- `crates/holler-hub/src/panes/mod.rs`
- `crates/holler-hub/src/panes/store.rs`
- `crates/holler-hub/src/profile/mod.rs`
- `crates/holler-hub/src/profile/rename.rs`
- `crates/holler-hub/src/profile/store.rs`
- `crates/holler-pane-testkit/src/profile_store.rs` (the fake)
- `crates/holler-pane/src/findings.rs`
- `crates/holler-pane/src/profile.rs` (doc)
- `crates/holler-pane/src/reconcile.rs`
- `crates/holler-pane/src/reconcile/drift.rs` (new)
- `crates/holler-pane/src/reply.rs` (doc)
- `crates/holler-proto/src/methods.rs` (doc)

Docs:
- `CHANGELOG.md`
- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`

Handoffs:
- `docs/handoffs/665/decisions.md`
- `docs/handoffs/665/evidence.md` (new)
- `docs/handoffs/665/handoff-F.md` (new)
