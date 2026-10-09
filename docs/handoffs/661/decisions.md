# Decisions — #661 the hub profile registry

## A (Phase 3, up-front plan review) — 2026-10-09T14:41:53-06:00
- **Decided:** PASS on docs/handoffs/661-brief.md at 075a83a, with 8 warns and no blocks (see handoff-A.md). The plan extends #639's pane registry through the seam #639 documented for #661 (`persist`, `feed`, `handlers::run` via `RegistryEntry`, plus `PaneStoreOptions`, `WATCH_WAIT`, `log_fault` and `next_generation`). The one new object, `profile::store::Store`, is justified in writing by the issue's "one comparison" limit on `panes/**`. D7 sits under the pane lock after `next_generation`, as ADR "Decisions taken" 2 and the fake require. D8 reads the profile registry before the pane lock, so no lock nests. No wire, golden, error-code or ADR change. No size block: #639 landed the same shape in one run (12 files, +2,342 lines), and the 661a/661b seam is clean if F invokes the scope cap.
- **Assumed:** T and F read handoff-A.md (the run context points them at it; T's role doc does not say so explicitly), so findings 3-6 are addressed to them directly. #639's registry passes the 18 non-membership pane conformance cases; it has never run the suite. If it does not, the brief's escalate rule applies; I did not run it.
- **Hedged:**
  - Finding 1: the mirrored store is about 120 lines across ten items, not "about 60". It is a warn, not a block, because the brief justifies the new object in writing and generalising #639's store would exceed the issue's blast radius. It needs a tracked follow-up, not "a follow-up may".
  - Finding 3: the brief lists `drain_profiles`, a near-copy of `pane_support::drain`, while its own Reuse map calls such a copy an A-dup block. I made it a warn for T to fix rather than stop the run for a one-line brief edit.
  - Finding 8: D9 makes "reconcile is the net" depend on #647 handling `profile-not-found`. That is a product consequence for the operator, not drift.
- **Evidence:**
  - Read in full: panes/{mod,store,persist,feed,handlers}.rs, profile/{mod,rename}.rs, pane_dispatch.rs, holler-pane/src/{profile,reply,generation,pane}.rs and error.rs:380-710.
  - Test kit: profile_store.rs, pane_store.rs, conformance/{mod,profile_store,profile_store/log,profile_store/watch}.rs, conformance/pane_store.rs:1-120 and :370-534, and fixture.rs.
  - Tests: tests/pane_support/mod.rs, pane_dispatch_test.rs, pane_handlers_test.rs:1-200 and pane_registry_test.rs:60-194.
  - Docs and issues: ADR-0021 §1-2 and §6-8 plus "Decisions taken"; issue #661; epic #633 "Skeleton split" and "Decisions 2026-10-09".
  - Commands: `git show --stat 2a6f349` (the #639 size), `wc -l` on every touched file, `rustfmt --check` on panes/, profile/ and tests/pane_* (clean), and clippy.toml (complexity 15, too_many_lines 100). Grepped tests/pane_* for an existing pane moved between profiles; none exists.
