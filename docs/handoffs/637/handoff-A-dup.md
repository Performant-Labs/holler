# Handoff-A-dup: Phase 7 - #637 slice a, the `holler-pane` crate  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-637-implementation
**Diff base:** 27da2da (merge base with origin/main)   **Diff head:** 40b722b
**Reuse map:** docs/handoffs/637-brief.md §Files "Reuse map" (no separate survey.md exists)
**Verdict:** PASS

## Summary

PASS, with 0 blocks and 6 warns.

F extended every object the Reuse map named and built no parallel path:
- `PaneName` wraps `holler_proto::vocab::SessionName` and calls `SessionName::parse`. The grammar is not copied, and `vocab.rs` is untouched.
- No second clock exists. The slice only holds `i64` timestamp fields, so it needs no `now_millis` call.
- `error.rs` follows `holler_proto::Code`: one `PaneCode` enum, one `ALL`, and one `match` per mapping, with `ALL_CODES` derived at compile time.
- No new dev-dependency was added.

None of the stack's Phase 7 candidates is copied: the token store, `Lockout`, `Roster`, `log::emit` and the test harness.

The warns are contract details that freeze when #637 merges. Rows 1 and 5 need an operator decision. Rows 2, 3 and 4 are doc or test changes for F and T with no decision needed. Row 6 is optional.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/ports.rs:36-46` (`Watch<T>`) | **The idle signal reuses `timeout`.** When nothing changes within I5's bound, `next()` yields `Err(PaneError::Timeout)` and the stream stays usable.<br>**The collision:** `timeout` is also what a wedged or slow store returns under I5, and #638's fakes inject slow and wedged calls. A consumer cannot tell "idle" from "hung", and must special-case one error code. The consumers are #643's `pane watch`, #647 and #649's client.<br>**The precedent says the opposite:** the hub's own long-poll, `control/wait`, treats "nothing happened in this window" as an ordinary success, not an error (`crates/holler-hub/src/control_server.rs:477-481`).<br>**Source:** this was my own Phase 3 suggestion (row 4); I missed that precedent. No test pins it yet. | **Operator decision before merge.**<br>**(a) Recommended:** make the item `Result<Option<T>, PaneError>`, with `Ok(None)` meaning "nothing within the bound". The change is the alias and its doc in `ports.rs`, plus two test lines (`tests/common/mod.rs:124`, `tests/ports_test.rs:83`).<br>**(b)** Keep `Err(Timeout)` and document that a consumer treats it as "keep polling" and detects a hung store another way.<br>**Either way** an idle long-poll stays `{events: [], cursor}` on the wire, as `control/wait` does. |
| 2 | warn | `crates/holler-pane/src/ports.rs:63-66` (`PaneStore::delete`), `crates/holler-pane/src/profile.rs:344-350` (`ProfileStore::delete`), `crates/holler-pane/tests/common/mod.rs:148-156` | **What deleting a missing record returns is unstated.**<br>- #639 (amended 2026-10-09) says a missing record is `pane-not-found`.<br>- The trait doc says only that a stale generation is `generation-conflict`.<br>- The in-test `MemPaneStore::delete` returns `Ok(())` for a missing record when `expected_generation` is 0.<br>**Risk:** #638's conformance suite and #639's store are written in parallel from the trait doc, so they can disagree.<br>**Why now:** `delete` arrived in revision 6, after the Phase 3 review. | **F:** add one doc line to each `delete`: a missing record is `pane-not-found` (`profile-not-found` for a profile).<br>**T:** make `MemPaneStore::delete` follow that rule.<br>**No operator decision:** #639's text already decides it. |
| 3 | warn | `crates/holler-pane/src/reply.rs:105-112` (`decode_params`) | **A second params decoder: justified, but missing the first one's rule for absent `params`.**<br>**The existing decoder:** `holler_proto::typed_params` (`crates/holler-proto/src/envelope/dispatch.rs:13-31`) is the codebase's one typed params entry point.<br>**Why a second is right:**<br>- Control frames are not `Envelope`s, because `decode` refuses methods outside `CATALOG`.<br>- `typed_params` drops the serde error, so a guard's code would be lost.<br>- Phase 3 row 6 asked for this helper.<br>**What is missing:** `typed_params` decodes a request with no `params` from `{}`. `decode_params(Value)` leaves that to each caller, so #639, #661 and #649 each decide it for `pane/list`, `profile/list` and the `*/watch` methods (whose `since` defaults). | **F:** state in the doc that a request without `params` decodes from `{}`, as `typed_params` does, or take `Option<Value>` and apply the rule inside.<br>**T:** pin it with one test: absent params decoded as `WatchParams` give `Cursor(0)`. |
| 4 | warn | `crates/holler-pane/tests/common/mod.rs:136-156`, `crates/holler-pane/tests/ports_test.rs:57-80` | **The crate's own test doubles re-implement the CAS rule.**<br>- `generation.rs:13-14` says the fakes and stores call `next_generation` "instead of writing it again".<br>- The in-test `MemPaneStore` and `MemProfileStore` write the rule again four times, two of them with an unchecked `+ 1`.<br>- The RED tests predate the helper, which explains it.<br>**Risk:** these are the only `PaneStore` and `ProfileStore` implementations in the tree, so #638 is the likely next copy. | **T:** route both doubles' `cas_put` and `delete` through `holler_pane::next_generation`, so the crate's own examples follow its rule. |
| 5 | warn | `crates/holler-pane/src/pane.rs:117-123` (`HarnessKind`) | **A second harness-id vocabulary.**<br>**The existing one:** `holler_proto::vocab::HARNESS_IDS` (`vocab.rs:269-275`: "a new harness is a new config row + a new id, not a protocol bump"). holler-body keeps a harness as a `String` from it (`crates/holler-body/src/config.rs:100-102`).<br>**The new one:** `HarnessKind { Opencode }`, a closed enum.<br>**It is defensible:**<br>- The epic's non-goals exclude a new harness.<br>- A closed enum fails closed on a persisted record.<br>**The gaps:** nothing links the two lists, and decisions.md does not record the choice. After merge, adding a harness to panes changes a frozen type. | **Operator decision. Recommended default: keep the enum.**<br>**(a)** Keep it, record the choice in decisions.md, and add one test that every `HarnessKind` serde name is in `HARNESS_IDS`.<br>**(b)** Use a `String` checked against `HARNESS_IDS`, as holler-body does. |
| 6 | warn | `crates/holler-pane/src/ports.rs:72-79` (`HerdrSpec`), `crates/holler-pane/src/profile.rs:148-171` (`SpecHerdr`, `SpecHost`, `SpecHarness`) | **Two frozen names differ only in word order.**<br>- `HerdrSpec { session, workspace, grid }` is `ensure_pane`'s input and a root re-export.<br>- `SpecHerdr { workspace, grid }` is a profile spec's placement, and #664's apply builds the first from the second.<br>**Cost:** a mix-up is a compile error, so the cost is readability for 13 parallel stories. Renaming after merge is a breaking change. | **F, optional, before merge:** rename the profile sub-objects (for example `ProfileSpecHerdr`, `ProfileSpecHost`, `ProfileSpecHarness`), or add a "see also" line to both types. |

### Checked, no duplication

**Helpers with no shared object to extend:**
- **`is_valid_code`:** the hub's `token.rs::valid_label` is the ADR 0005 label grammar, a different rule.
- **`excerpt`:** each crate keeps a private sanitizer, and none is reachable from holler-pane. They are `holds.rs::sanitize_reason`, body `acp_driver/auth.rs::sanitize` and `log.rs::escape_field_value`.
- **`deserialize_parsed`:** one helper serves four types. `Profile` uses `try_from`, as `a2a/part.rs` does.
- **The slug:** no slug helper exists anywhere in `crates/`.

**New objects the brief justifies in writing:**
- `PaneReply` beside the CLI envelope (decision 8: it is not the CLI's envelope, and has no `schema_version`).
- The pane `Hold` beside `SessionHold` (decision 9; the type's doc says so).
- `PANE_METHODS` and `PROFILE_METHODS` in holler-proto beside the hub's `CONTROL_METHODS` (ruling 6; the module doc says so).
- `Cursor` and `Watch` (decision 7).

**Structure and gates:**
- The tests' `common/mod.rs` holds JSON fixtures and one in-memory store. Nothing there copies the CLI's integration-test harness (`support/mod.rs`).
- The five manifests follow the existing skeleton template (holler-body's first manifest).
- `scripts/golden-diff-summary.sh` shows no drift.
- `git diff --name-only origin/main...HEAD` stays inside the brief's blast radius.

## Notes for O

PASS: the run can continue. The two decisions below cost least before merge, because each changes a type that freezes:
1. **Row 1 (operator):** how the watch stream signals "idle". I recommend (a), `Ok(None)`.
2. **Row 5 (operator):** keep the closed `HarnessKind` and link it to `HARNESS_IDS` with a test (recommended), or switch to a string.

Rows 2, 3 and 4 need no decision:
- Rows 2 and 3 are one doc line each for F.
- Row 4 is a test-double change for T, and row 2 also has one for T.

Row 6 is F's option.

I also agree with T-green's advisory 1. A `compile_fail,E0423` doctest on `RefusalCode`'s private field is the one frozen invariant no gate checks, and it costs one doc block now.

## Out of scope but noticed

- **#642 (OpenCode adapter) has an object to extend.**
  - holler-body's `http_attach_driver.rs` already speaks OpenCode's HTTP API. It checks that a session exists, interrupts through `/api/session/{id}/interrupt` plus the classic `/abort`, and lists sessions with `GET /session`.
  - `query.rs` (`probe_attach_session_live` and `probe_one`, from line 229) probes an endpoint.
  - #642's reuse map should name both, so the adapter does not become a second OpenCode client.
- **#639 has a long-poll pattern to follow.** The hub already long-polls with the roster's change channel (`roster.rs:206`, `subscribe` at `:559`) and `control/wait`'s subscribe-before-check loop (`control_server.rs:486-529`). That is the pattern for `pane/watch`.

## Patterns referenced

- `crates/holler-proto/src/`:
  - `error.rs`: the single-source `Code::ALL`.
  - `vocab.rs`: `SessionName`, `HARNESS_IDS`.
  - `envelope/dispatch.rs`: `typed_params`.
  - `methods.rs`, `hold.rs`, and `log.rs:465-495`.
- `crates/holler-hub/src/`:
  - `control_server.rs:1-140` and `:466-540`: the control dispatch and `control/wait`.
  - `serve.rs:61-65`: `CONTROL_METHODS`.
  - `roster.rs:195-250`.
  - `holds.rs:210-223`.
  - `token.rs:413-427`.
- `crates/holler-body/src/`: `config.rs` (the `harness` and `command` fields) and `acp_driver/auth.rs:140-160`.
- Issues #633, #637, #638, #639, #642, #643, #649 and #661, fetched with `gh issue view` on 2026-10-09 at about 04:30 MDT.
