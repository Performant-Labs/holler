# Handoff-A-dup: Phase 7 - #638 slice a: the fault switch, the fake `PaneStore` and its conformance suite  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-638-implementation
**Diff base:** 939d79c   **Diff head:** 8e38d8f
**Reuse map:** docs/handoffs/638-brief.md, "Extend vs new" (this run has no survey.md), with handoff-A.md W-3 and W-4
**Verdict:** PASS

## Summary

PASS. F extended every object the reuse map named and built no parallel path:

- **Extended objects.** The diff fills the empty `holler-pane-testkit` crate and implements the frozen `PaneStore` trait.
- **One generation rule.** Every generation goes through `holler_pane::next_generation`, at two call sites (`pane_store.rs:154`, `:181`). There is no other generation arithmetic.
- **One cursor allocator.** Every cursor comes from the one allocator, `Log::append` (`feed.rs:85`).
- **Errors and profiles.** It returns only the closed `PaneError` variants and compares profiles with `ProfileName::slug`.
- **The two new objects are single and justified in writing.** There is exactly one `FaultSwitch` and one feed in the crate.
- **No hub code copied.** Nothing was copied from the hub, apart from the 10-line `usage` rule that finding 4 discusses.
- **`MemPaneStore` untouched.** The accepted near-duplicate in `holler-pane/tests/common` is unchanged, and `holler-pane` has no diff at all.

**Phase 3 checks: W-3 and W-4 both hold.**

- **W-3 (one write path).** `seeded`, `cas_put` and `concurrent_put` share one `put`. `delete` and `concurrent_delete` share one `remove`. `concurrent_*` calls `next_generation(current, current)`. `seeded` and `concurrent_*` bypass the faults and the call log.
- **W-4 (one feed).** Everything generic about the watch lives in `feed.rs`, over the crate-private `Change` trait. That covers the log and its head, cursor allocation, the `Cursor(0)` snapshot, "after `since`", `usage`, the idle wait on the `Condvar` and the iterator with its `WatchNext` fault hook. The `Change` trait has the shape of the hub's `RegistryEntry` (`panes/mod.rs:152` on main) and is not a copy of it.

**The brief's named check holds.** No second CAS rule or feed exists. All 19 case functions go through the shared helpers: `expect_code` 11 times, `drain` 4 times, `next_item` 4 times. No case has its own watch loop or code match.

**Four warns.** All are about future duplication or naming, and none blocks:

- Finding 1 is the one to act on before #682 starts.
- Findings 2 to 4 are follow-up decisions for O.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane-testkit/src/conformance/pane_store.rs:508-518` | **A generic suite helper sits in the port's file.** `increasing` (cursors strictly increase) needs nothing but `Cursor`. Every other record-independent helper is in `conformance/mod.rs` (`run_cases`, `succeeds`, `expect_code`, `expect_eq`, `next_item`, `drain`). **Slice c needs it.** #682 runs "the watch cases 14 to 18 of the pane suite, for profiles". Brief decision 1 and #682 say it never edits `conformance/mod.rs`, and #682's radius does not include this file. So #682 can only copy `increasing` verbatim. **The cases have the same problem, at a larger scale.** The five watch cases and their helpers (`:290-379`, `:465-506`, about 130 lines) are written against `&dyn PaneStore` and `PaneEvent`. | **Move `increasing` now.** Move it to `conformance/mod.rs` as `pub(crate)`, beside `drain`. It keeps its caller, so `dead_code` is satisfied. Either the run's agent does this before merge (a two-line move, no behaviour change), or #682's brief names it as a planned extension of `conformance/mod.rs`. Slices d and e have no watch stream, so they cannot conflict with it. **Decide in #682's issue, before it runs,** how the profile watch cases relate to cases 14 to 18. Either they are an accepted near-copy, stated in its brief so its A-dup can pass them. Or they are shared, which puts this file and `conformance/mod.rs` in #682's radius. |
| 2 | warn | `crates/holler-pane-testkit/tests/fake_pane_store_test.rs:52-63` | **A third drain-to-idle helper.** This is a test-local copy of `conformance::drain` (`conformance/mod.rs:112-137`). The copy exists because `conformance::drain` is `pub(crate)`, which `tests/` cannot reach. Its signature is the same as the hub's `drain` (`crates/holler-hub/tests/pane_support/mod.rs:78-88` on main, from #639 after this branch was cut). That makes three copies, two of them in this diff. **The testkit is the place to share it.** The hub and the CLI both dev-depend on the testkit. The next users are the `pane watch` tests (#643 onward) and the hub's suite run (#649 or #661). **Why it passes.** The brief made `drain` crate-private, F kept the brief's exact API (handoff-F decision 7), and A's Phase 3 note made a public `drain` optional. | **O decides when.** Make `conformance::drain` `pub`. This is additive: it is already generic over `T` and returns `Result`. Then replace the test's copy with `drain(&mut watch).unwrap()`. Do it in this PR, or in the issue of the next user, which then also removes the hub's copy. |
| 3 | warn | `crates/holler-pane-testkit/src/pane_store.rs:241-259` (and its doc at `:66-67`) | **One name for two different checks.** **The fake.** Its private `check_membership` makes the `pane-in-other-profile` check. **The hub.** On main, its public `holler_hub::profile::check_membership` (`profile/mod.rs:67`) is the other half. ADR-0021 "Decisions taken" item 2 (lines 538-541) says it keeps the check "that the named profile exists", while `pane-in-other-profile` runs in the registry's CAS. **The fake's own doc** (`:66-67`) cites the hub's `check_membership` for the existence check. So one doc paragraph uses one name for both checks. **The risk.** #661 adds the CAS-side check to the hub. If it follows the fake's naming, a second `check_membership` lands in the hub crate. | Rename the fake's private function after what it refuses, e.g. `refuse_profile_move`. This is XS: the function is private, so no API changes. |
| 4 | warn | `crates/holler-pane-testkit/src/feed.rs:97-109` | **The `usage` rule is written twice.** `Log::check_since` (a cursor ahead of the head is `usage`) restates the hub's `check_since` (`crates/holler-hub/src/panes/feed.rs:88`, rule 1). Only one word of the message differs. **Why it is justified.** The testkit may not depend on the hub. Neither ADR-0021 nor `ports.rs` states the rule (brief decision 4: "not a suite case"), so it has no shared home. **The cost.** Nothing keeps the two copies in step. If the hub's rule changes, verb tests on the fake keep passing against the old behaviour. **Precedent.** `generation.rs:13-14` is this epic's pattern for a store rule that the fake and the hub must share. | **No change in this slice.** **O decides whether the rule belongs to the port.** If it does, do three things in a follow-up. Amend ADR-0021 §7 and the `Watch` doc. Add one `check_since` to `holler-pane` that both the hub and `feed.rs` call, as both call `next_generation`. Add a suite case for it. |

**Drift during rework.** There was no rework cycle: one F pass, then T-green PASS. No drift was introduced.

### Checked and consistent

- **Layout (AC 2).**
  - `lib.rs` declares exactly the 12 modules, in AC 2's order. `feed` is private, and there is no `pub use`.
  - `conformance/mod.rs` declares exactly the 6 suite modules.
  - Each of the 12 stubs is `//!` lines only, naming what it will hold, its slice and its issue (Phase 3 W-1's wording).
- **Public surface.** It is exactly the brief's, with W-12's rename (`PortOp::as_str`). Every shared helper is `pub(crate)`.
- **Layering inside the crate.**
  - `fault` depends on nothing in the crate.
  - `feed` depends on `fault`.
  - `pane_store` depends on `feed` and `fault`.
  - `conformance` depends on `fixture` and `holler_pane`, and names no fake. A fake names a suite only in a doc link.
- **Existing seams checked first.**
  - `holler-pane` has no `Pane` constructor or `Default`, so `sample_pane` duplicates nothing.
  - `PaneCode` is `pub(crate)` in `holler-pane`. The suite's code literals (`generation-conflict` and the others) follow the workspace's tests: 27 literal uses, none through `PaneCode::`.
  - No fault-injection mechanism existed in the workspace before this one.
  - The per-type `lock()` helper with `unwrap_or_else(PoisonError::into_inner)` is the workspace pattern (`holler-body`, the hub's `store.rs:134`).
  - `Timeout { op }` had no naming convention before `"<port>.<method>"`.
- **This overlay's Phase 7 candidates.** The diff adds nothing like the token store, `Lockout`, `Roster`, the `log(Severity, ...)` helper, or the harness helpers (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`). It adds no logging.
- **Dependencies.** The one new edge is `holler-pane-testkit -> holler-pane` (one `Cargo.lock` line). No other manifest changes.
- **Size.** The largest file is `conformance/pane_store.rs` at 518 lines, under the 600-line warning.
- **Merge.** `git merge-tree` of HEAD with today's `origin/main` (`3f9fbf2`, which includes #639 and #676) completes with no conflicts.

## Notes for F

None required (PASS).

## Notes for O (carry-over for slices b to e)

- **Phase 3 W-1 is still open.**
  - **What the issues list now.** #681 to #684 list only their `src/` stubs and `CHANGELOG.md` (#681 also its `Cargo.toml`). None lists its `src/conformance/<port>.rs` stub or its `tests/*.rs` files. Each later run's "diff lists only blast-radius paths" check would therefore fail by design.
  - **#682 also needs finding 1's decision.**
- **#682's audit log is not a second feed.**
  - **What it is.** `ProfileStore::log` returns per-profile audit entries (`at`, `actor`, `change`), which `ProfileEvent` does not carry, and the log must stay readable after a delete. A per-profile map in `profile_store.rs` is therefore new data, not a parallel feed. #682's A-dup should not reject it as one.
  - **The atomicity choice.** `Feed::write` gives its closure only `Log<E>` under the lock. To keep the audit log atomic with the CAS, #682 must either take a second lock inside the `write` closure (no edit to `feed.rs`), or plan an edit to `feed.rs` and list it in its radius.
- **Out of scope but noticed (main, not this diff).**
  - **A stale hub doc.** The hub's `profile/mod.rs:61-63` doc still says #661 adds both `pane-in-other-profile` and the existence rule to `check_membership`. That is stale against ADR-0021 item 2 (and the hub's own `panes/mod.rs:20-22`). #661 should fix it with the change itself.
  - **A second `sample_pane` with live names.** The hub's `pane_support::sample_pane` (fully populated, generation 7, profile optional) uses live names (`hj`, `kiwi`). It is a second `sample_pane` with a different shape from the testkit's. Any later fold onto `holler_pane_testkit::fixture` belongs to the hub's tests.
