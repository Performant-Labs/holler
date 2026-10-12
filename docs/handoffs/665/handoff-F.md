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
