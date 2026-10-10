# Handoff-A: Phase 3 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-640-implementation (head `e39458d`)
**Brief reviewed:** `docs/handoffs/640-brief.md`   **Reuse map:** `docs/handoffs/640-brief.md`, section "Reuse map (extend, do not duplicate)" (there is no separate survey)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with no block findings and seven warns. The plan builds on the right objects: part 1's `protocol` (requests,
`decode_reply`, the parsers, `check_supported`), `layout` (`grid_of`, `GridMap`) and `plan` (`plan_splits`, `Extent`,
`Step`), the frozen `HerdrPort` and its types, and #638's conformance suite. It adds two modules where ADR-0021 §5 puts
them (an adapter crate that depends on `holler-pane` alone), with no new normal dependency, no second encoder, decoder
or tree walk, and no parallel error type. The two things that are new to the codebase, a worker thread per exchange to
bound `connect`, and a public `Transport` seam so that a JSON-level Herdr can stand in for the socket, are each
justified in writing (Decisions 3, 14 and 18). The warns are about consistency with neighbouring code (`Timeout.op`
naming, `excerpt`, the deadline arithmetic), forward compatibility for #647, #649 and #642, and carrying this part's
decisions into part 3's ADR rows.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Decision 3: "Expiry is `Timeout { op: format!("herdr.{}", request.method()) }`", which propagates up through every port method unchanged | pattern consistency (the error vocabulary at the port boundary) | The test kit names a timeout by the **port** method: `PortOp::as_str` is "`<port>.<method>` ... It is also the `op` of the `timeout` a wedged call answers" (`holler-pane-testkit/src/fault.rs:20-21`). `fake_herdr_test.rs:288-301` pins `FakeHerdr` to `herdr.ensure_pane`, `herdr.read`, `herdr.close`, `herdr.snapshot` and `herdr.version`. Under this plan the real adapter names the **wire** method instead: `ensure_pane` times out as `herdr.session.snapshot`, `herdr.layout.export`, `herdr.pane.split` or `herdr.workspace.create`, `read` as `herdr.pane.read`, and `version()` as `herdr.ping`. A verb test (#644 and later) that checks `timed out: herdr.ensure_pane` against `FakeHerdr` passes there and fails in production. That is a fake-versus-real divergence of the same kind as Decision 20, and it is not listed. No adapter-level AC pins `op` yet, so it is cheap to settle now. | Pick one rule in the brief. (a), recommended: the transport keeps `herdr.<wire method>` (AC 5, 6, 11 and 12a are unchanged), each port method rewrites a `Timeout` to `herdr.<port method>` exactly as `HerdrOp::as_str` spells it, and one adapter-level AC checks this (e.g. a test transport answers `Timeout { op: "herdr.layout.export" }`, and `ensure_pane` returns `op == "herdr.ensure_pane"`). (b): keep the wire names, and record the divergence next to Decision 20 and in "For the operator". |
| 2 | warn | "the part-1 sources are **not** changed" (brief :1469, :1713), together with the new `unavailable` messages of Decision 8 (version string), Decision 10 (a workspace with no tab) and Decision 11.6 (the new pane id and where it landed; AC 16) | reuse / anti-duplication | These messages quote text that Herdr sent. The crate's rule for that (messages "quote what Herdr sent (cut to 64 characters)" on one line, `protocol.rs:20-22`; ADR-0021 §9, "Every message is one line") is carried out by the private `protocol::excerpt` (`protocol.rs:580`). That function is already part 1's own copy of `holler_pane::error::excerpt` (`error.rs:688`, `pub(crate)` in a frozen crate). The Reuse map does not name it, and the freeze leaves F two options: a third copy in `adapter.rs`, which is a Phase 7 near-copy, or quoting that breaks the crate's message rule. | Add a Reuse-map row for `protocol::excerpt`, and allow one visibility-only edit to `protocol.rs`: `fn excerpt` becomes `pub(crate) fn excerpt`. State it as the single exception next to "part-1 sources are not changed". Every Herdr-sent string that `adapter.rs` or `transport.rs` quotes goes through it. |
| 3 | warn | Decision 3, "`deadline = Instant::now() + config.timeout`", and Decision 6, which refuses only "a zero `timeout`" | pattern consistency (cross-cutting: library code never panics) | `Instant + Duration` panics on overflow, so `HerdrConfig { timeout: Duration::MAX, .. }` passes validation and then panics at `connect`'s ping. The workspace's clippy denies (`panic`, `unwrap_used`, `expect_used`) cannot see an arithmetic panic. The three neighbouring deadline sites avoid it with `Instant::now().checked_add(window)`: `holler-hub/src/panes/store.rs:268`, `holler-hub/src/profile/store.rs:319` and `holler-pane-testkit/src/feed.rs:201`. | In Decision 6, also refuse with `usage` (naming the field `timeout`) a timeout for which `Instant::now().checked_add(timeout)` is `None`, or one above a stated cap. Add that case to AC 27 (e.g. `Duration::MAX`). Decision 3's deadline then cannot overflow, and the pinned `exchange(.., deadline: Instant)` stays as it is. |
| 4 | warn | Decisions 7 and 8 (`connect` refuses an unsupported protocol, so no `HerdrAdapter` exists for one), the forward-compat table, operator item 3, and the public `Transport` / `connect_with` | forward compatibility / seam consistency | (a) ADR-0021 §9 makes `herdr-version-unsupported` a **finding** of `pane doctor` (#647), and `FakeHerdr::set_version(Unsupported)` models it as a live port whose `version()` refuses while its other calls work. In production no port object exists, and `Wiring::connect` ("Fails when something they need cannot be reached", `holler-cli/src/pane/wiring.rs:30`) would fail every verb, doctor included. The fake's path is then reachable only if #649 keeps the connect error and answers it through `version()`. Neither the forward-compat table nor item 3 says so. (b) `connect_with` accepts any `Transport`, but the issue's scope is "the adapter uses the local socket and has no remote path". | (a) Add the point to the #647 and #649 forward-compat rows and to operator item 3: wiring must not fail `pane doctor` on `herdr-version-unsupported`. (b) Add one doc line on `Transport` and `connect_with` calling them a test seam, and saying that production wiring builds the adapter only through `connect`. No code change in this part. |
| 5 | warn | AC 16, 17, 28 and 29 each call for "a test transport ..." (rewrite `down` to `right`, close the split target first, record deadlines, garble replies) | test-helper duplication | The pinned `wire_herdr` API gives these four no shared seam, so four near-identical structs wrapping `Arc<WireHerdr>` that implement `Transport` are likely in `adapter_test.rs`. The overlay lists test-harness helpers as Phase 7 candidates. | Pin one interceptor in `tests/wire_herdr/`, e.g. `Tap::new(Arc<WireHerdr>, hook)`. Its hook sees each request line and its deadline, and may rewrite the line or replace the reply. The four ACs each configure that one wrapper. |
| 6 | warn | Decisions 3 and 18 and `adapter::DEFAULT_TIMEOUT`: this is the first adapter crate with I/O (`holler-adapter-host` and `-opencode` are 5-line skeletons) | abstraction level / future duplication | This part sets the pattern for the other two adapters. The OpenCode spike says "Every request to OpenCode needs a client timeout": a frozen server accepts the connection and never answers, which is AC 5's silent server (`docs/research/opencode-pane-spike.md:189-200`). So #642 needs the same one-deadline-per-call exchange and the same I5 default of 10 s. Today that default is prose in seven `holler-pane` doc comments and nowhere in code. ADR-0021 §5 bars an adapter from depending on another adapter, so #642 cannot import this one. With one user today, keeping both in this crate is right. | No change in this part. Record it for #642's brief, which should look here first. If a second copy is needed, raise the shared home (an amend-first `holler-pane` constant for I5's default, and a shared bounded-exchange helper if both shapes match) rather than adding a second `DEFAULT_TIMEOUT` and worker. |
| 7 | warn | ADR-0021 §10 ("How the adapter learns a workspace's extent is #640's") and Scope (the ADR rows are part 3's) | ADR currency | This part makes decisions that the ADR delegated to #640 or does not yet state: (1) a workspace's extent is configuration, per label (`HerdrConfig.workspaces`); (2) the version gate runs at `connect` and in `version()`, and the other methods trust it (Decision 8); (3) the `op` rule, if finding 1 changes it. The overlay wants the ADR updated in the same change. Deferring to part 3 is acceptable: #640 is one issue, its blast radius (`crates/holler-adapter-herdr/**`) does not cover `docs/adr/` in this PR, and the deferral is written down rather than silent. | Part 3's brief names these three by item when it writes the §9 and §10 rows. The conformance suite's own ASSUMPTION (`holler-pane-testkit/src/conformance/herdr.rs:189-196`) expects (1). |

No finding blocks: the plan extends what the map names, and its one deliberate test-code duplication (the wire fake
re-implements `base36`, `last_lines` and Herdr's tree rules instead of using `src/`) is justified in the brief as the
oracle's independence, and is enforced by AC 35.

Checked and consistent (no finding):

- **Layering and dependency direction.** Inside the crate, `adapter` uses `transport`, `protocol`, `layout` and
  `plan`, and `transport` uses `protocol::Request`. Taking `&Request` instead of a raw line makes the part-1 method
  allow-list (`ALLOWED_METHODS`) hold at the type level, the same way the hub's `send_over` builds its own envelope.
  No cycle. No dependency on `holler-hub`, `holler-cli` or `holler-body` (AC 41).
- **The port contract.** `HerdrPort` is implemented as frozen. `PaneError` is the only error type, as `protocol.rs`
  already does (Reuse map: "No parallel error type"). `Usage` for a bad config follows `FakeHerdr::with_workspace`'s
  `usage` for a bad declaration.
- **Plan, act, observe** (Decision 11) follows I3's wording. "Never relocates a healthy pane" (ADR §10) holds: a
  misplaced pane is left in place, and healing it is #647's.
- **Recording `host.herdr_api_version`.** The epic credits #640 with it, but the adapter cannot write a `Pane` record
  without breaking ADR §5 and I1. Deferring it to #644 through `version()` is the right reading.
- **State and persistence.** The adapter persists and caches nothing (AC 37). There is no secret in any message: typed
  text never reaches an error (AC 12; `Request`'s `Debug` already hides it).
- **Size.** The largest planned file is `adapter_test.rs` at about 650 lines (lint warns at 600 and fails at 900). The
  brief names where every new item goes and when the test file is split.
- **Test-module pattern.** `tests/wire_herdr/mod.rs` follows part 1's `tests/common/mod.rs`, a shared module with
  `#![allow(dead_code)] // #640`.

## Notes for O

PASS, so nothing is required. If the brief is reopened before T writes RED, findings 1, 2 and 3 each cost one decision
line and at most one AC, and are cheaper now than at Phase 7. If it is not reopened: under finding 2, F quotes
Herdr-sent text with `{:?}` and writes no third `excerpt`, and the anti-duplication gate will check that; under finding
3, F should at least keep `connect` from panicking on an overflowing timeout. Findings 4, 6 and 7 are for the #647,
#649 and #642 briefs and for part 3; finding 5 is T's to apply while writing the wire fake.

## Patterns referenced

1. `crates/holler-adapter-herdr/src/{protocol,layout,plan}.rs`: the part-1 objects this plan extends (all pub items
   the Reuse map names exist as described).
2. `crates/holler-pane-testkit/src/{herdr.rs,fault.rs,conformance/herdr.rs,conformance/mod.rs}`: `FakeHerdr`, the
   `PortOp` op vocabulary, the 11 cases and the fixture/guard runner.
3. `crates/holler-hub/src/control.rs:485-518` (the JSON-line Unix-socket client pattern) and
   `crates/holler-hub/src/panes/store.rs:268` (the `checked_add` deadline idiom).
4. `crates/holler-cli/src/pane/wiring.rs`: the seam #649 fills, where `HerdrAdapter::connect` will be called.
5. `docs/adr/ADR-0021.md` §2, §5, §9, §10, §12; `docs/research/opencode-pane-spike.md:189-200`.
