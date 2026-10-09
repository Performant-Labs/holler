# Decisions — #639 the hub pane registry

## A (Phase 3, up-front plan review) — 2026-10-09T08:35:52-06:00
- **Decided:** PASS on docs/handoffs/639-brief.md at c957bbd, with 10 warns and no blocks (see handoff-A.md). The registry extends the objects that already exist: `write_atomic`, `next_generation`, `decode_params` and the params structs, `PaneReply` via `reply_line`, and a synchronous store behind `spawn_blocking` whose `JoinError` fails closed, as `token.rs` does. The brief justifies each of its three departures from `holds.rs` in writing (D1, D3, D4), and the draft ADR-0021 §7 ratifies them, so they are decisions, not drift. I did not block on size: the work is 5 production files in one module.
- **Assumed:** ADR-0021 is a draft on branch `issue-634-implementation` (9b1ecfc, no PR yet). I treated it as the intended standing spec, and its §6-§8 match the brief's D1-D7 and D9. If the ADR changes before it merges, this review's "no ADR conflict" conclusion has to be checked again.
- **Hedged:** W-1 is a warn and not a block, but it is the one I would most want F to act on. `check_membership` runs on the async thread today. It is harmless while it is a no-op, but #661's lookup through the blocking `ProfileStore` port would turn it into blocking work on an executor thread. The fix is to widen `panes::dispatch` to take `&Arc<ProfileState>`; `pane_dispatch.rs:71-74` sanctions that, it changes no AC and no file outside the radius, and it contradicts only the brief's "signature unchanged" sentence. W-9's test-file split needs O to approve the extra file in the radius, or AC 30 fails. W-7(b), #665's multi-record rename, is a cross-story gap I recorded for the operator; it is not for #639 to build.
- **Evidence:** I read the brief, the outside review (`639-brief-result-r1.md`; all 12 claims it said needed verification are confirmed), issue #639 and epic #633 (contract and rulings 1-9), and ADR-0021 on its branch. Code read: `panes/mod.rs`, `pane_dispatch.rs`, `profile/{mod,rename}.rs`, `holds.rs`, `holds/tests.rs:1-45`, `token.rs:259-285,721-799`, `control_server.rs:1-200`, `control.rs:15-130`, `state.rs`, `serve.rs:75-135,340-380`, `tests/pane_dispatch_test.rs`, `tests/logging_guard_test.rs`, `holler-proto/src/{atomic_file.rs,log.rs:105-131,methods.rs:108-135,envelope/dispatch.rs:20-34}`, `holler-pane/src/{ports.rs,reply.rs,generation.rs,lib.rs,pane.rs,error.rs:300-380,520-610,profile.rs:300-332}`, `scripts/lint.sh`, and the `holler-hub` and workspace `Cargo.toml`. I also ran `wc -l` on the hub sources and tests, grepped the logging helpers (`fn emit`/`log`/`warn`, `Severity::Error`), and checked `gh pr list --head issue-634-implementation` (it returned none).

## T-red (Phase 4, author the tests) — 2026-10-09
- **Decided:** Valid RED (PASS). 30 new or amended tests (27 new, 3 amended) across `pane_registry_test.rs` (14), `pane_feed_test.rs` (7, a new file), `pane_handlers_test.rs` (6), the three amended assertions in `pane_dispatch_test.rs`, and the shared `pane_support/mod.rs`. I moved `sample_pane` and `pane_outcome` into `pane_support`, so there is one copy of each (A's W-2). I renamed two #669 tests whose names said "stub" and "not_implemented". AC 27 gets no behavioural test, because `check_membership` is `Ok(())` until #661; S greps for it as the brief says.
- **Assumed:** `PaneState::load_with` is infallible and returns `PaneState`; a corrupt file is reported by every method rather than by `load_with`. That is what the brief and #669's infallible `load` imply. The tests pass `&Arc<PaneState>` and `&Arc<ProfileState>` to `dispatch`, so they compile with either `&ProfileState` or A's W-1 `&Arc<ProfileState>`.
- **Hedged:** (1) I split the feed tests into `pane_feed_test.rs`: the single file the brief names was 726 lines after rustfmt, and AC 29 caps new files below 600. That is one path beyond the brief's list (A's W-9), so O has to approve it for AC 30. (2) I added a sixth case to AC 9's table, "two entries share a cursor", from A's W-5. The brief's D2 validation does not list it. O either tells F to validate it or has the case deleted from `corrupt_cases()`. (3) A compile failure is the RED of the store tests; the role doc prefers an assertion failure, but the brief's test plan accepts this because the API under test does not exist. A scratch stub with only the API surface turns every one of them into an assertion failure on `NotImplemented`.
- **Evidence:** `cargo test -p holler-hub --no-fail-fast --test pane_registry_test --test pane_feed_test --test pane_handlers_test --test pane_dispatch_test` against the tree: 85 errors, all E0432/E0422/E0599 on `PaneStoreOptions`, `load_with`, and the five `PaneStore` methods. The same command against the scratch stub: all new tests FAILED on `NotImplemented`, with 7 of the 10 `pane_dispatch_test` tests unchanged and passing. A throwaway reference implementation in `~/.cache/t639-ref` (not delivered) turned all 30 green, with 25 clean repeat runs. `rustfmt --check` and `scripts/lint.sh` are clean.

## F (Phase 5, implement) — 2026-10-09T09:09:17-06:00
- **Decided:**
  - I implemented the registry as the brief's five files in `crates/holler-hub/src/panes/` (`mod`, `store`, `persist`, `feed`, `handlers`; 994 lines, the largest 340), plus the CHANGELOG entry.
  - All 30 new and amended tests passed on the first run, and I edited no test.
  - I acted on every warn A routed to F:
    - W-1: `dispatch` takes `profiles: &Arc<ProfileState>`, and `check_membership` runs inside `pane/cas_put`'s blocking closure, before the CAS and outside the pane lock.
    - W-3: one fixed-severity helper, `log_fault`, for exactly two `error` events. D4's failed save is logged as `pane_registry_write_failed`.
    - W-4: `feed.rs` is pure. `Store::poll` and the `Watch` iterator are in `store.rs`.
    - W-5: `PaneEvent` is the file entry, with a generic `Doc<E>`, `load_doc` and `save_doc`. They return a `Problem`; the caller builds the `what` with `Problem::what(label, path)`. The checks read an entry through the crate-private `RegistryEntry` trait, and distinct entry cursors are now validated.
    - W-6: `handlers::run` is the one shared pipeline.
    - W-8: `feed.rs` states D6 rule 4 and cites the draft ADR-0021 §7.
    - W-10: the `WATCH_WAIT` doc, and the operating limits in `mod.rs`.
  - D1 is kept in its literal order: the next document is a borrowed view of the entries, built and saved before memory changes.
- **Assumed:**
  - #661 will implement `RegistryEntry` for `ProfileEvent` from `profile/`. That is a crate-local trait on a foreign type, which is allowed. #661 then calls `persist`, `feed` and `handlers::run` without editing `panes/`.
  - The draft ADR-0021 §6 sentence "An idle window answers `{"events": [], "cursor": since}`" and the brief's D6 "the reply's cursor is always head" differ only for `since = 0` against a registry whose records are all deleted. I followed the brief, and both resume correctly.
  - O has not yet recorded the approval of `pane_feed_test.rs` in the radius (T's W-9 request). AC 30 depends on it.
- **Hedged:**
  - Widening `dispatch` contradicts the brief's sentence "The `dispatch` signature is unchanged". I did it because A ranked W-1 as the warn it most wanted acted on, and `pane_dispatch.rs:71-74` sanctions it. It changes no AC and no file outside the radius.
  - A tombstone is written as `"pane": null`, because the frozen `PaneEvent` has no `skip_serializing_if`. D2 wrote it as an optional member (`"pane"?`). Both forms read back.
  - Reading the version before the full parse is my addition, for operator clarity: a file from a newer build reports its version rather than a shape error at a line and column.
  - #665's multi-record `profile/rename` (A's W-7 b) is still a gap for the operator. This store offers single-record writes only.
- **Evidence:**
  - The final tree passes every gate: `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings), `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (97 test binaries, 964 passed, 0 failed, 5 ignored), `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`, and `rustfmt --check --edition 2021` on `panes/*.rs` and the five pane test files. All exited 0.
  - 25 repeat runs of each of the four pane test binaries: 0 failures in 100 runs.
  - The fail-closed tests run with `--nocapture` show each AC 9 case failing on its own check (listed in handoff-F's Tier 1).
  - `evidence.md` has 12 entries, each checked by a script against its cited line range, with 0 mismatches.
  - `cargo doc -p holler-hub --no-deps` gives no warning in `panes/`.

## T (Phase 7, GREEN)
- **Decided:** suite is GREEN; no test edited; PASS.
- **Assumed:** CI's `--skip roster_stays_accurate_under_concurrent_body_load` still applies (ci.yml:128, per F).
- **Hedged:** two mutations (no notify; ignored save error) both killed by the intended tests.
- **Evidence:** docs/handoffs/639/handoff-T-green.md; workspace 964 passed, 0 failed.
