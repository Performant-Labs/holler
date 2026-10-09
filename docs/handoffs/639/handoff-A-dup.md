# Handoff-A-dup: Phase 7 - #639 the hub pane registry  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Diff base:** af3d8df (merge base with `origin/main`)   **Diff head:** 2188fe7
**Reuse map:** docs/handoffs/639-brief.md §Files "Reuse map"
**Verdict:** PASS

## Summary

PASS. F extended every object the Reuse map named and built no parallel path:

- `write_atomic` saves the file (`persist.rs:154`).
- `next_generation` is the only CAS rule, used by both `cas_put` and `delete` (`store.rs:171`, `store.rs:193`).
- `decode_params` and the `reply.rs` params structs decode requests.
- `PaneReply` reaches the wire through `reply_line`.
- The poison-tolerant lock is the `holds.rs:293` idiom.
- The log event has the `holds.rs` shape.

The three things the brief rejects in advance are absent: a second CAS helper, a second code validator, and a per-file copy of `sample_pane`. None of the overlay's Phase-7 candidates was copied: the token store operations, `Lockout`, `Roster`, the `log(Severity, ..)` helpers, and the test harness helpers. The new objects (`Store`, `Doc<E>`, `Ring<E>`, `handlers::run`, `RegistryEntry`) are the brief's own file split, or what A asked for at Phase 3 (W-4, W-5, W-6). ADR-0021 §7 also requires the two registries to share one generic load and save.

There are three warns and no blocks. The one to settle before merge is W-1. ADR-0021 was merged to `main` during this run (094ebfa) and is not yet on this branch. Its §6 says an idle long-poll answers `cursor: since`. The code answers the head, which differs in one edge case.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/src/panes/store.rs:260-264`; `crates/holler-hub/src/panes/feed.rs:35-38` | **Drift from the merged ADR-0021.** §6 (`origin/main`, line 219) says: "An idle window answers `{"events": [], "cursor": since}`". `Store::poll` always answers `cursor: table.head`. The two differ only when `since` is 0 and every record has been deleted (head > 0, no live record). Both resume correctly, F and T documented the divergence (it follows the brief's D6, written before the ADR merged), and no test covers that case. The overlay still requires the code and the ADR to agree in the same change. Separately, `feed.rs:37` cites "the draft ADR-0021 §7". §7 is merged and does ratify rule 4 (ADR lines 255-257), so only the word "draft" is stale. | S/O decide before merge. Either (a) answer `since` when the window ends with no events (one line in `Store::poll`; every idle case the tests pin has `since == head`, so they still hold), or (b) amend the §6 sentence in this PR, which adds `docs/adr/ADR-0021.md` to the radius. Drop "draft" in `feed.rs:37`. |
| 2 | warn | `crates/holler-hub/src/panes/mod.rs:19-20`; `persist.rs:17-19`, `69-75`; `store.rs:249-307`; `handlers.rs:37-40` | **The claim that #661 "edits nothing here" no longer holds.** (a) ADR §7 (lines 241-243) persists the profile change log in `profiles.json` with the records, through "the same generic load and save helpers". `Doc<E>` is `{version, cursor, entries}` with `deny_unknown_fields`, so `load_doc::<ProfileEvent>` would refuse a file that carries the log. (b) ADR "Decisions taken" item 2 (lines 498-501) puts `pane-in-other-profile` inside the pane CAS, so #661 edits `Store::cas_put`, with its radius widened for that. (c) The `Condvar` long-poll (`Store::poll`) and the `Feed` iterator are typed on `PaneEvent` in `store.rs`, and `profile/watch` and `ProfileStore::watch` need both. #661 must lift them to a generic form or copy about 60 lines, and a copy is a Phase-7 rejection for #661. (d) `NoParams` is private, and `profile/list` needs the same type. This is not a #639 defect: no parallel path exists today. | F (optional, cheap): reword `mod.rs:19-20` and `persist.rs:17-19` to say what #661 extends, and make `NoParams` `pub(crate)`. O: put `panes/{persist,store,handlers}.rs` in #661's radius. Its brief says to extend `Doc` for (a) and to lift, not copy, for (c) and (d). Since #661 edits `panes/` anyway, that story can also consider moving the shared parts (`persist`, `feed`, `handlers::run`, `RegistryEntry`) to a neutral module, so `profile/` stops importing registry machinery from `panes::`. |
| 3 | warn | `crates/holler-hub/tests/pane_handlers_test.rs:27-45`; `pane_dispatch_test.rs:104-111`, `327-328`; `pane_support/mod.rs:6-7` | **Small test-helper near-copies.** `Rig::on` and `fresh_deps` both build the same `(Arc<PaneState>, Arc<ProfileState>)` pair with `short_opts()`. `Rig` repeats `PaneDeps`'s two public fields, and neither builder uses `pane_support::load`, which is exactly `PaneState::load_with(state, short_opts())`. `check_membership_accepts_any_pane` (which T edited) still builds its own temp dir and `HubState`, although its file imports `temp_state`. The `pane_support` doc names three includers, but `pane_feed_test.rs` is a fourth. This is not a block. The copies are a few lines each, test-only, and built on the shared `temp_state` and `short_opts`. Nothing on the brief's rejection list or the overlay's harness list was copied. | Add `pane_support::deps(&HubState) -> PaneDeps` (short window) and use it in `fresh_deps` and in `Rig`, which can hold a `PaneDeps`. Use `temp_state()` in `check_membership_accepts_any_pane`. Fix the doc line. T can fold this into any later edit. |

No other duplication; apart from these, the extension is clean. Rework introduced no drift: F's departures from the brief (the `&Arc<ProfileState>` widening, `poll` and `Feed` in `store.rs`, `PaneEvent` as the file entry, `Problem`, distinct cursors, logging the failed save) are all A's Phase-3 warns, acted on as routed.

### Checked and clean

- **Reuse map, row by row:**
  - `write_atomic`: `persist.rs:154`.
  - `next_generation`: `store.rs:171`, `store.rs:193`. The only other `checked_add` calls are the cursor (`store.rs:88`) and the deadline (`store.rs:251`).
  - `decode_params` and the params structs: `handlers.rs:25`, `handlers.rs:50`.
  - `PaneReply` and `reply_line`: `handlers.rs:57`, `handlers.rs:77-85`, and `mod.rs:200`.
  - The lock idiom: `store.rs:134`, `store.rs:269`.
  - The `Component::Control`/`Direction::Local` event shape: `store.rs:325-340`.
  - Timestamps: the store adds none.
- **`log_fault`** is not a copy of the generic `emit`/`log(Severity, ..)` wrappers (`holds.rs:249`, `circuit.rs:120`, `serve.rs:48`). Its severity and fields are fixed, it serves exactly two events, and it follows the hub's per-module helper pattern (`live.rs:58`, `control_server.rs:61`, `circuit/auth.rs:171`). This is the case W-3 allowed at Phase 3.
- **`handlers::run`** is the first generic blocking runner in the hub. `token.rs:739-799` has per-function `*_async` wrappers and no shared helper, so there was nothing to extend.
- **`params_of`** is not a copy of `holler_proto::typed_params`, which takes an `Envelope` and returns `WireError`. The control socket hands over a raw `&Value`, and `decode_params`'s doc gives the "absent means `{}`" step to the caller (`reply.rs:111-114`).
- **`Problem::parse`** is not a copy of `PaneError::from_decode` (`holler-pane/src/error.rs:534`). `from_decode` is crate-private and keeps serde's message on purpose, while `Problem::parse` drops it on purpose (D3).
- **`save_doc`'s `create_dir_all(parent)`** matches the hub's `ensure_dirs` (no mode is set) and the body-side precedent (`holler-body/src/identity.rs:157-160`, `connection_state.rs:62-64`).
- **No feed, cursor or watch** existed in the hub before this diff, so `feed.rs` and `Store::poll` duplicate nothing.
- **`RegistryEntry` fits `ProfileEvent`.** `ProfileEvent { cursor, name, profile: Option<Box<Profile>> }` has a `Profile { name, generation, .. }`, so the abstraction has its second user and is not speculative.
- **The port is not bypassed.** `PaneState` implements `PaneStore` by delegating to `Store`, and the handlers call the same `Store` methods through the inner `Arc`, plus `poll`, which the port does not expose. That is one implementation with two thin entry points.
- **Tests.** There is one `sample_pane` (`pane_support/mod.rs:18`). `sample_pane_at` wraps it. There is one reply parse-back (`pane_outcome`, with `outcome` over it). `temp_state` is not a copy of `StateDir`, which lives in holler-cli's test support, out of the hub tests' reach. #669's test already used `tempfile`.
- **Radius.** The diff touches only `panes/**`, the five pane test files, `CHANGELOG.md` and `docs/handoffs/639*`. `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `profile/**`, `holler-pane`, `holler-proto`, every `Cargo.toml` and `Cargo.lock` are unchanged (`git diff --quiet`). The largest file is 523 lines, and no file is near the 800-line mark.

## Notes for O

- **The merge with `main` conflicts in `CHANGELOG.md`.** `main` added the ADR-0021 entry at the same place (`git merge-tree` reports one conflict). Keep both entries. Merging `main` also brings ADR-0021 onto the branch, which W-1 needs for S's audit.
- **`pane_feed_test.rs`** still needs O's radius approval in `decisions.md` for AC 30. T and F both raised it, and I see no entry yet. On the architecture side there is no objection: it is the split A asked for at Phase 3 (W-9). `pane_support/mod.rs` is already in the brief's list.
- **Out of scope, but noticed:** `pane_support::sample_pane` and holler-pane's `tests/common/mod.rs` `pane_json()` hold the same fixture. Both are older than #639. Once #638 fills `holler-pane-testkit`, both should move to one test-kit fixture.

## Patterns referenced

- `crates/holler-hub/src/holds.rs` (persistence, lock idiom, `emit`); `crates/holler-hub/src/token.rs:255-300`, `655-670`, `715-799` (store save, `JoinError`, `spawn_blocking` wrappers)
- `crates/holler-proto/src/envelope/dispatch.rs` (`typed_params`); `crates/holler-pane/src/{reply.rs, error.rs:532-540, profile.rs:304-317, pane.rs:256-268}`
- `crates/holler-hub/src/{pane_dispatch.rs, profile/mod.rs, state.rs}`; `crates/holler-cli/tests/support/mod.rs` (`StateDir`); `crates/holler-pane/tests/common/mod.rs`
- `docs/adr/ADR-0021.md` as merged on `origin/main` (094ebfa): §6, §7, "Decisions taken" item 2
