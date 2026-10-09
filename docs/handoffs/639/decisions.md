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

## A (Phase 7, anti-duplication gate) — 2026-10-09T09:55:10-06:00
- **Decided:** PASS on `af3d8df...2188fe7`, with 3 warns and no blocks (see handoff-A-dup.md).
  - F extended every Reuse-map object: `write_atomic`, `next_generation` (the only CAS rule, used by `cas_put` and `delete`), `decode_params` and the params structs, `PaneReply` via `reply_line`, the lock idiom, and the `holds.rs` event shape.
  - The diff has no second CAS helper, no code validator and no `sample_pane` copy.
  - No overlay Phase-7 candidate was copied.
- **Assumed:** ADR-0021 was merged to `origin/main` during this run (094ebfa) and is not on the branch. I reviewed against the merged text, not the draft.
  - W-1: its §6 says an idle long-poll answers `cursor: since`, but `Store::poll` answers the head. The two differ only when `since` is 0 and every record has been deleted.
  - W-2: its §7 (the profile log lives in `profiles.json`) and "Decisions taken" item 2 (`pane-in-other-profile` inside the pane CAS) mean #661 will edit `panes/`, against the module docs' claim.
- **Hedged:** I made W-1 a warn, not a block. It is drift against an ADR, not a parallel path, which is what a Phase-7 block is for. F and T documented it, and S owns spec conformance. It still has to be settled before merge, by a one-line code change or a §6 amendment. W-2's poll/`Feed` and `NoParams` gaps are #661 reuse risks, partly the result of my own Phase-3 W-4, and are not #639 defects. W-3 is minor duplication of test builders.
- **Evidence:** I read every changed file in full, plus handoff-F, handoff-T-red, handoff-T-green, the brief, `holds.rs`, `token.rs:255-300,655-670,715-799`, `state.rs`, `pane_dispatch.rs`, `profile/mod.rs`, `holler-proto/src/envelope/dispatch.rs`, `holler-pane/src/{reply.rs,error.rs:380-540,profile.rs:300-332,pane.rs:251-268}`, `holler-pane/tests/common/mod.rs`, `holler-cli/tests/support/mod.rs` (`StateDir`), and ADR-0021 on `origin/main`. I grepped the hub for `spawn_blocking`, `JoinError`, serde `classify()`/`Category::`, `write_atomic`, `create_dir_all`/`ensure_dirs`, logging helpers, `Condvar`/`VecDeque`/cursor/watch, `params` extraction and `sample_pane` builders. `git diff --quiet` confirms the out-of-radius files are untouched, and `git merge-tree` shows a `CHANGELOG.md` conflict with `main`.

## S (Phase 10, spec audit) — 2026-10-09T10:09:26-06:00
- **Decided:** ADVISORY-HOLD (see handoff-S.md).
  - ACs 1-24 and 26-29 are met, each by a named test that asserts the behaviour. AC 27 is met by grep and reading, as the brief says.
  - AC 25 is met as tested. The brief contradicts itself on the idle long-poll cursor: D6 says "always head" and AC 25 says `cursor: since`. ADR-0021 §6, merged to `main` mid-run (094ebfa), decides the AC 25 form. The code follows D6.
  - The two differ when `since` is 0 and every record has been deleted, and no test pins either answer.
  - The brief's radius excludes ADR-0021, so the rule that code and ADR agree in the same change cannot be met inside this story.
  - Recommended fix (option A): keep the head, amend the §6 sentence in this PR (O widens the radius), have T pin the edge case with a handler test, and have F refresh three doc statements that went stale when the ADR merged (`mod.rs:19-20`, `persist.rs:17-19`, `feed.rs:37`).
  - AC 30 is met in substance. O still has to journal the approval of `pane_feed_test.rs` (A W-9).
- **Assumed:**
  - T's and F's recorded Tier 1 output reflects HEAD 31d2102; I did not re-run it.
  - The ADR-0021 text on `origin/main` is the standing spec.
- **Hedged:**
  - I chose ADVISORY-HOLD, not REWORK, because F faithfully implemented D6, and either way of reconciling it (amend the ADR or change the code) needs a decision outside F's radius.
  - The divergence is harmless for a client that resumes from the reply's cursor. I recommend amending the ADR rather than the code, because the head keeps the frozen `Watch` contract ("then every later change") for a from-zero watcher on an all-deleted registry.
  - Rule 2's re-evaluation after each wake can collapse a create-then-delete within one from-zero poll. That is D6 as written and practically unreachable through the verbs, so it is an advisory note only.
- **Evidence:**
  - Read: the brief, issue #639, every handoff in this directory, the outside diff review (`639-diff-result-r1.md`, PASS), the full `git diff origin/main...HEAD`, ADR-0021 on `origin/main` (lines 105-110, 216-222, 238-259 and 494-501), `holler-pane/src/ports.rs:28-52` and `holler-proto/src/atomic_file.rs`.
  - Grepped: `check_membership(` (it matches `handlers.rs:111`); `unwrap`, `expect`, `panic` and `unsafe` in `panes/` (none); `#[allow]` in the diff (all carry `// #639`); `sleep` in the pane tests (none); privacy patterns across the diff (only the existing `kiwi` fixture).
  - Also ran `wc -l` on the touched files (max 523) and `git merge-tree` against `origin/main` (only a `CHANGELOG.md` conflict).

## O (operator decision on S's ADVISORY-HOLD) — 2026-10-09
- **Decided:**
  - The idle `pane/watch` cursor is the head (S's option A, chosen by the operator). The code's D6 rule stands and ADR-0021 is amended to match: §6 now answers `{"events": [], "cursor": <head>}`, equal to `since` except for a watch from 0 over an all-deleted registry, and "Decisions taken" item 7 records it. The brief's AC 25 now matches D6, and its blast radius gains `docs/adr/ADR-0021.md` (§6 and item 7 only).
  - `crates/holler-hub/tests/pane_feed_test.rs` is approved in the radius (AC 30, A W-9): the split was forced by AC 29's 600-line cap (one file measured 726 lines).
  - `origin/main` is merged into the branch; the `CHANGELOG.md` conflict keeps every entry (#669, ADR-0021 #634, #670, #639).
- **Evidence:** merge 1d67ef3; ADR and brief 639f130; the edge-case handler test `an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head` 4a75b54 (passes on the code; a mutation answering `since` on an idle window fails it, `left: {"cursor": 0, ...}` against `right: {"cursor": 2, ...}`, and no other test, then reverted); the stale `mod.rs`, `persist.rs` and `feed.rs` notes refreshed in 3b5e7ee.

## T (Phase 7 re-verification at 95b7724) — 2026-10-09
- **Decided:** PASS. The suite is GREEN and no Tier 2 issue blocks. I changed no test.
- **Assumed:** The `roster_stays_accurate_under_concurrent_body_load` skip is the one CI applies.
- **Hedged:** The new edge-case test's non-vacuity rests on O's recorded mutation; I did not re-apply it.
- **Evidence:** Four pane binaries 10/7/7/14 passed, 15 repeat runs with 0 failures; workspace 1068 passed, 0 failed; clippy, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test` and `wire_selftest` all clean.
