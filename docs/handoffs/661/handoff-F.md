# Handoff-F: Phase 5 - #661 the hub profile registry (GREEN)

(The role doc's Phase 5, the implement phase; the Workflow driver numbers it phase 6.)

**Date:** 2026-10-09
**Branch:** issue-661-implementation
**Issue:** #661

## What was done
- `crates/holler-hub/src/profile/mod.rs` (edited, 69 -> 216 lines): module docs (layout, rules, operating limits, shape); `ProfileState { store: Arc<Store> }` (still no `Clone`); `load` = `load_with(state, PaneStoreOptions::default())`; new `load_with`; `impl ProfileStore for ProfileState` (each method delegates to the store, `rename` answers `not-implemented` and touches nothing); `dispatch` routes the six methods to `handlers` and `profile/rename` to `rename::dispatch` (signature unchanged); `check_membership` filled (D8).
- `crates/holler-hub/src/profile/entry.rs` (new, 207 lines): `ProfileEntry` (the D2 file entry) with its load checks (serde `try_from` over a `deny_unknown_fields` `RawEntry`), `RegistryEntry` for `ProfileEntry` and for `ProfileEvent`, `stamp` (D4) and `summary` (D5).
- `crates/holler-hub/src/profile/store.rs` (new, 415 lines): `Table` and `Store`, the twin of `panes::store`: `open`, `lock`, `read`, `get`, `list`, `log`, `cas_put`, `delete`, `commit`, `save`, `watch`, `poll`, the `Feed` iterator, `load_table`, plus the name rule (`check_name`) and `log_entry`.
- `crates/holler-hub/src/profile/handlers.rs` (new, 71 lines): one async fn per method, each `crate::panes::handlers::run` with a closure over the store's `Arc`.
- `crates/holler-hub/src/panes/store.rs` (edited, +40/-10): D7, `refuse_profile_move(stored, next)` called in `cas_put` right after `next_generation`; `log_fault` made `pub(crate)`; module and `log_fault` docs updated.
- `crates/holler-hub/src/panes/mod.rs` (edited, +16/-7): `pub(crate) use store::log_fault;` (`mod store` stays private, A finding 2); doc lines fixed per A finding 7 (`RegistryEntry`, `WATCH_WAIT`, `PaneStoreOptions`, the #661 reuse paragraph).
- `crates/holler-hub/src/panes/handlers.rs` (edited, +3/-2: one word plus a doc line): `pub(crate) struct NoParams`.
- `CHANGELOG.md` (+14): the `## [Unreleased]` / `### Enhancements` entry linking #661.

## Design decisions
- **One stamp per write (D4).** `stamp(entry, now) = max(now, last log at, live record's updated)`. It is the log entry's `at`, the record's `updated`, and a create's `created`. That gives D4's rules in one place: `at` never decreases along a log, `updated` never goes backwards, a create has `created == updated`, an update keeps `created`. A re-create after a delete starts a new `created` (at least the delete's `at`). Alternative: separate clamps for `updated` and `at`. Rejected because with two clamps, a record's `updated` and its last log `at` could disagree.
- **Summary pairing (D5).** Specs are grouped by `ProfileSpec.pane`. Within one pane name they are paired in order: unequal pairs count as changed, extra new specs as added, extra old ones as removed. With duplicate pane names (validating specs against each other is out of scope), `before + added - removed == after` still holds. The summary holds counts only.
- **Entry checks (D2 plus A finding 5).** A file entry is checked in this order:
  1. The event's name has the entry's slug.
  2. The log is not empty.
  3. A live event's name equals its record's name (A finding 5).
  4. A live record's log ends with a write at the record's generation. I also require that this last entry is not `Deleted`: a registry-written file never has that, and it mirrors the tombstone rule.
  5. A tombstone's log ends with `Deleted`.

  The error type is `&'static str` and the messages name no content. `persist`'s `Problem::parse` discards serde's message anyway (evidence.md).
- **Name rule against the live record only (D6).** `check_name` compares the submitted name with the live record's name. A tombstone holds no name, so a create may spell a deleted profile another way, and its log continues the old one. The `profile-exists` `what` uses the fake's wording.
- **D7 placement and name (A finding 6).** The comparison is a small helper, `refuse_profile_move`, not `check_membership`. It is called after `next_generation`, so a stale generation wins (AC 36), as in the fake. It compares by slug, and its `what` names the pane and both profiles in the fake's wording.
- **The hook (D8)** returns early for `profile: None` without touching the profile registry. Otherwise it does `profiles.store.get(name)?`, so a failed registry propagates `store-corrupt` (AC 39).
- **`dispatch`** clones the inner `Arc<Store>` (not the outer `Arc<ProfileState>`), the same way `panes::dispatch` does. `two_connections_share_the_one_pair_of_state_handles` still counts 3 owners.

## Reuse / extend-vs-new
Extended the #639 pane registry through the seam it documented for #661, per the brief's Reuse map. Reused and not copied:
- `panes::persist::{Doc, VERSION, load_doc, save_doc}` and `Problem::what`, through `RegistryEntry`
- `panes::feed::{Ring, select, check_since}`
- `panes::handlers::{run, NoParams}`
- `panes::{PaneStoreOptions, WATCH_WAIT}` (D3; no second options type)
- `panes::log_fault`
- `holler_pane::next_generation`
- the `holler_pane::reply` params types
- `EnvVarName`'s decode inside `decode_params` (no pre-scan)
- `holler_proto::clock::now_millis`
- `lock().unwrap_or_else(PoisonError::into_inner)`

The one new object, `profile::store::Store`, is the brief's written justification ("New object, justified"; D10). It mirrors `panes::store::Store`. Each mirrored item keeps its twin's name and order and carries a "Twin of `panes::store::...`" doc line (A finding 1): `Table::from_doc`, `Table::next_cursor`, `Table::entries_with`, `Store`, `open`, `lock`, `read`, `commit`, `save`, `watch`, `poll`, `Feed`, `load_table`.

A's finding 1 list did not name `Table::from_doc`. It is the 10-line half of `load_table` that builds the table from a loaded document. The pane `load_table` calls its own `from_doc` the same way. Here it is keyed by `entry.slug` instead of the pane name.

`Table::record`, `get`, `list`, `log`, `cas_put` and `delete` are not mirrors. They are the profile rules: slug lookups, the name rule, stamps and the log.

## Architecture notes for A
- **Layers:** as #639: `profile/handlers.rs` -> `panes::handlers::run` -> `profile::store::Store` -> `panes::persist` / `panes::feed`.
- **New modules:** `profile/{entry,store,handlers}.rs`, all private to `profile`.
- **Public API:** `ProfileState` was a unit struct and is now a struct with a private field. `ProfileState::load(&HubState)` is unchanged; `load_with(&HubState, PaneStoreOptions)` is new. `impl ProfileStore for ProfileState` is new. The signatures of `dispatch` and `check_membership` are unchanged.
- **#639 visibility:**
  - `panes::handlers::NoParams` is now `pub(crate)`.
  - `panes::store::log_fault` is now `pub(crate)`, re-exported as `crate::panes::log_fault`.
  - `mod store` stays private, so the pane `Store` and its writes stay behind `PaneState` (A finding 2).
- **Dependency direction:** `profile/` -> `panes/` for the shared parts. `panes/` -> `profile/` (the hook and `ProfileState`) already existed, from #669 and #639.
- **Lock order (ADR-0021 §7):** the profile store never takes the pane lock. The hook reads the profile store and releases its lock before the pane CAS takes the pane lock, so no lock is nested.
- **What did not change:** no new dependency, no `Cargo.toml` change, no wire, protocol, error-code or golden change. `CATALOG` is untouched.
- **New log events:** two `error` events through `log_fault`, `profile_registry_corrupt` and `profile_registry_write_failed`.

## Deviations from spec / wireframe
- **D10 literal wording, applied as A finding 2 asks.** `mod store;` in `panes/mod.rs` stays private. Only `log_fault` is opened, by `pub(crate) use store::log_fault;`. The intent of D10 (share it, copy nothing) is unchanged.
- **CHANGELOG text.** The entry carries the brief's content. Following #639's entry, it adds:
  - the two `error` log events, `profile_registry_corrupt` and `profile_registry_write_failed`;
  - the `unavailable` answer for a save that fails;
  - the `profile-exists` refusal;
  - the env-value refusal;
  - that `profile/rename` still answers `not-implemented`.

  This follows the stack rule that a new log event is documented.
- **README not updated.** Its "Debug output" table documents neither registry's events, which is #639's precedent. README is outside AC 45's blast radius, so I left it alone (see Known issues).
- No wireframe (no UI).

## Tier 1 self-check (incl. tests now GREEN)
```
$ cargo build -p holler-hub
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.55s
$ cargo test -p holler-hub --no-run
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.96s
$ cargo test -p holler-hub --test <each>        (T's authored and amended binaries, plus the untouched #639 ones)
profile_registry_test     test result: ok. 12 passed; 0 failed   (conformance suite: all 23 cases)
profile_persistence_test  test result: ok. 8 passed; 0 failed
profile_feed_test         test result: ok. 6 passed; 0 failed
profile_handlers_test     test result: ok. 11 passed; 0 failed
pane_membership_test      test result: ok. 6 passed; 0 failed    (pane conformance suite: all 19 cases)
pane_dispatch_test        test result: ok. 10 passed; 0 failed
pane_handlers_test        test result: ok. 8 passed; 0 failed
pane_registry_test        test result: ok. 14 passed; 0 failed
pane_feed_test            test result: ok. 7 passed; 0 failed
$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's own skip)
rc=0; 120 test binaries: 1297 passed, 0 failed, 5 ignored
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.22s   (no warning)
$ rustfmt --edition 2021 --check crates/holler-hub/src/{profile,panes}/*.rs crates/holler-hub/tests/{pane_,profile_}*.rs crates/holler-hub/tests/pane_support/mod.rs
rustfmt rc=0
$ bash scripts/lint.sh          -> rc=0 (only pre-existing `warn:` lines for files this change does not touch)
$ bash scripts/changelog-check.sh -> changelog-check: ok
$ cargo machete                 -> cargo-machete didn't find any unused dependencies in this directory. Good job!
$ grep -n "PaneInOtherProfile" crates/holler-hub/src/panes/store.rs   -> 348: (one hit, in refuse_profile_move, the CAS path)   [AC 41]
$ grep -rn "PaneInOtherProfile" crates/holler-hub/src/profile/        -> (no hit)                                           [AC 41]
$ grep -rn '"="' crates/holler-hub/src/profile/                       -> (no hit; no char '=' scan either)                  [AC 28]
$ git diff --name-only origin/main | grep -E "serve.rs|control_server.rs|pane_dispatch.rs|/lib.rs|rename.rs|holler-pane/|holler-proto/|holler-pane-testkit/|Cargo.toml|Cargo.lock|docs/adr|docs/protocol|persist.rs|feed.rs"
(none)                                                                                                                 [AC 45]
```
File sizes: the largest file this change touches is `profile/store.rs` at 415 lines. No file reaches 600. There is no `#[allow]` in `profile/` or `panes/`.

Extra check, not Tier 2. I disabled each of `entry.rs`'s five load checks in turn: the event's slug, the empty log, the live name, the live last generation, and the tombstone's last `Deleted`. Each time, `a_corrupt_file_fails_closed_and_is_never_rewritten` failed, so each check is the only guard for one row of T's corrupt table. Afterwards the file was restored byte-identical (`cmp` clean). The script was a scratchpad file outside the repo.

## Evidence appendix
`docs/handoffs/661/evidence.md` has 11 entries. They cover `persist`'s loader, parse report and checks, the feed's `select`, the params default and the hook's position in `pane/cas_put`, `ProfileCasPutParams`' `deny_unknown_fields`, `Profile`'s slug check, `EnvVarName`'s `=` refusal, `from_decode`, and `next_generation`.

## Tests that look wrong (for T)
None. Every authored and amended test passed against the implementation unchanged. One caveat, which T already recorded: AC 6's time windows use the real `now_millis()`, so they assume the wall clock does not step backwards during the test.

## Known issues
- **A finding 1, follow-up not opened (O or operator).** A should open an issue that makes the registry store generic. Its scope: lift `RegistryEntry`, `persist`, `feed`, `run`/`NoParams`, `log_fault` and the options into a neutral module, and make the store generic over the table. The "Twin of `panes::store::...`" doc lines in `profile/store.rs` carry no issue number until that issue exists. Add to its scope `crates/holler-hub/src/panes/persist.rs:17-21`. That doc line still says `profiles.json` uses `ProfileEvent` entries, but they are `ProfileEntry`. persist.rs is outside AC 45's paths, so A said to leave it.
- **A finding 8 (O or operator, before merge):** the cross-story notes on #647, #662, #663 and #664.
  - Under D9, a pane naming a deleted profile can only be rewritten with `profile: None`.
  - Membership compares slugs, so a profile's panes are selected by `ProfileName::slug()`.
- **Operator review:** D4, D9, D10 and D12 remain flagged, as the brief says.
- **README "Debug output" table.** It lists neither `pane_registry_*` (#639) nor `profile_registry_*` events. This gap predates #661, and README is outside this story's blast radius. Both are named in `CHANGELOG.md`.
- **Risks recorded in the brief, not fixed by design:**
  - the whole-file rewrite grows with the logs, and nothing prunes them;
  - the time-of-check gap across registries (the hook checks the profile exists, then the pane CAS writes);
  - the ABA gap after a delete and re-create (ADR-0021 §8, pinned by the suite).

## Files changed
- `crates/holler-hub/src/profile/mod.rs`
- `crates/holler-hub/src/profile/entry.rs` (new)
- `crates/holler-hub/src/profile/store.rs` (new)
- `crates/holler-hub/src/profile/handlers.rs` (new)
- `crates/holler-hub/src/panes/store.rs`
- `crates/holler-hub/src/panes/mod.rs`
- `crates/holler-hub/src/panes/handlers.rs`
- `CHANGELOG.md`
