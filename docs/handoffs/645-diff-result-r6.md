## Implementation Review (Round 1)

### BLOCK findings

**[B-1] crates/holler-pane/src/tx_switch.rs:24-33 — `SESSION_ID_MAX` is documented as 64 but the implementation uses byte length while `findings::quoted` counts characters; the doc comment asserts they "agree because an accepted id is ASCII" — this is wrong for the documented grammar.**

The `parse_session_id` doc says the accepted grammar is "ASCII letters, digits, `_` or `-`", and `SESSION_ID_MAX`'s doc says "It is the length `findings::quoted` cuts at. `quoted` counts characters and `parse_session_id` counts bytes, which agree because an accepted id is ASCII, so a typed id is never cut short in a message." However, the public API brief's grammar is `[A-Za-z0-9_-]{1,64}` (Decision 14), which admits a full 64 ASCII characters. `findings::quoted` → `error::excerpt` cuts at 64 **characters** (from Evidence Appendix, error.rs:691-698: `if text.chars().count() <= LIMIT`). A 64-character accepted id has 64 characters, so it is not cut. The doc's assertion is vacuous but the code is correct for the stated grammar. This is not a defect in behavior — but the acceptance criterion AC 12 requires the 65-`a` case to be `usage`, and the grammar is `{1,64}`, so no accepted id can be cut. The doc comment's correctness claim is a non-issue. However, the behavior is correct. No block.

**[B-2] crates/holler-pane/src/tx_switch.rs:26-44 — `parse_session_id` re-validates an `Existing` target at P0, but the CLI's `switch.rs` already calls `parse_session_id` before building the request, and the engine's P0 re-check can never fail for a request built by the CLI.**

The brief's Behaviour table says P0 is "Re-check an `Existing` id" with failure `usage (2)`. The CLI (`switch.rs:135-142`) calls `parse_session_id(&args.session).map(Target::Existing)` and then passes the result to `execute`, which calls `tx_switch::switch` only for `Ok(request)`. So by the time the engine runs, the target is already `Target::Existing(String)` with a validated id. The engine's P0 re-check (`plan`, `tx_switch.rs:169`) is dead code for the CLI path, but it is in the public API contract explicitly, and the engine's doc says "Re-check an existing target's id." This is not a contradiction — it is defensive for non-CLI callers. Not a block.

**[B-3] docs/adr/ADR-0021.md:625-628 — the ADR's "Deferred to named stories" bullet now says "#645 for switch and reset; #644 to follow for launch and relaunch", but the brief's Decision 17(c) says the bullet should name both stories in the one form, and the exact wording differs.**

The diff shows:
```
- Which closed failure code a mismatch observed after `act` carries: decided, `unavailable` (exit 1), section 8: #645 for
- switch and reset; #644 to follow for launch and relaunch.
```
The brief's Decision 17(c) says: "becomes \`Which closed failure code a mismatch observed after \`act\` carries: decided, \`unavailable\` (exit 1), section 8: #644 for launch and relaunch; #645 for switch and reset.\`" The implemented bullet inverts the order and changes "launch and relaunch; #645 for switch and reset" to "#645 for switch and reset; #644 to follow for launch and relaunch." The brief explicitly says "whichever of #644 and #645 lands second cites the first's (Decision 17(b))" and the reconciliation should be by anchor text. The implemented wording's order does not match the brief's prescribed one form. However, AC 24 requires: "the \`unavailable\` decision for a mismatch observed after the act is stated in section 8 once (if #644's paragraph is on \`main\`, the #645 paragraph cites it rather than restating it)". The diff's section-8 paragraph states the `unavailable` decision directly (bold text "A mismatch observed after the act is the closed failure `unavailable`, exit 1"). The diff's bullet says "#644 to follow" — which is consistent with #644 not being on `main` at this time. The brief's exact sentence may not be required since #644's paragraph is not on `main`. This is a possible deviation from the prescribed wording but not demonstrably wrong given the conditional. Not a block; may be a nit.

**[B-4] crates/holler-cli/src/pane/switch.rs:158-160 — `emit_outcome`'s `render` for `Verb::Reset` prints "reset demo-c1r1 to a new session..." but the brief's text output spec says `reset demo-c1r1 to a new session "ses_…" (was "ses_…")`, and the implementation matches. No defect.**

Continue examining for demonstrable blocks.

**[B-5] crates/holler-cli/tests/pane_verbs/reset.rs:73-76 — `assert_remedied` checks for `K::NoSessionOfRecord`, `K::SessionOfRecordMissing`, and `K::ShownDrivenMismatch`, but AC 15 requires "no `no-session-of-record`, `session-of-record-missing` or `shown-driven-mismatch` finding" — matches.**

No block.

**[B-6] crates/holler-pane/src/tx_switch.rs:333-334 — `recorded` sets `harness.health = Health::Healthy`, but the engine's P3 `check_health` passed `Ok(true)` and the record write uses `record.harness.health` from P1, so on a successful run the health is overwritten to `Healthy` regardless of what P3 observed. This matches the brief's `Health::Healthy` (P3 observed it). Not a defect.**

No block.

After careful review of the brief, diff, and source excerpts, I find no demonstrable BLOCK-level contradiction. The shortcomings are at most needs-verification for the parts of the evidence not visible, or nit-level for wording deviations.

### Needs-verification findings

**[NV-1] crates/holler-pane/src/tx_switch.rs:211-218 — the `read` function claims to find the pane "by the pane's name" rather than taking `into_iter().next()`, but the brief's P1 explicitly says "record = the first resolved pane, `resolved.panes.into_iter().next()` (never `panes[0]`)."** The evidence that would settle this: the rest of `read` beyond line 211 is not shown in the SOURCE EXCERPTS (the tx_switch.rs excerpt is truncated). The diff's diff header does not show the full `read` body either; the quoted module doc says the scope's answer is "searched by the pane's name". The brief at "The public API" and the Behaviour table both say the record is "the first resolved pane" via `into_iter().next()`. The implementation's doc comment claims a `.find()` by name. If the code uses `.find()` instead of `.next()`, it contradicts P1's "first resolved pane" but is arguably more robust. Given the excerpt does not show the `read` function body, settle by viewing tx_switch.rs:211-218.

**[NV-2] crates/holler-pane/src/tx_switch.rs:252-261 — `check_health`'s remedy for `ServerDown` is claimed to be `Some("holler pane relaunch <pane>")` and never `None` because `FindingKind::ServerDown` is matched in `findings.rs:127-131` with `pane.map(relaunch_command)`. The Evidence Appendix claims the fallback `doctor_command` is "never taken".** The excerpt `findings.rs:127-131` in the brief (D-9) shows `FindingKind::ServerDown => pane.map(relaunch_command)` in the match; the Appendix shows the same. This seems checkable and true, but the appendix's `findings.rs:319-320` excerpt is included and confirms. No unverifiable claim.

**[NV-3] crates/holler-cli/tests/pane_verbs/reset.rs:156-161 — AC 17's `reset_refusals_create_nothing` asserts `list_sessions(port)` unchanged by checking `sessions(rig, live(Q).port)` equals `case.setup`. The brief's AC 17 says "no `CreateSession` in calls and `list_sessions(port)` unchanged."** The test is visible and matches. No unverifiable claim; settle not needed.

**[NV-4] crates/holler-cli/src/pane/switch.rs:195-200 — the `emit_outcome` `ErrorBody` uses `ErrorCode::from(&failure.error)`.** The SOURCE EXCERPTS do not show the `ErrorCode::from(&PaneError)` implementation (output.rs:118-122 is cited in the brief but not excerpted). The claim "exit codes equal across formats" (AC-1 preamble, D-3) depends on `ErrorCode::from` and `emit` agreeing. The brief's D-3 excerpt shows `ErrorCode::from(&PaneError)` exists at output.rs:118-122 and `emit` uses the error's code's class for both modes. This is sufficiently evidenced by D-3. No finding.

**[NV-5] docs/adr/ADR-0021.md:362-391 — AC 24 requires the #645 paragraph to be "inside section 8 (between the `### 8.` and `### 9.` headings)"**. The diff shows the paragraph is inserted before `### 9. Failure taxonomy` (the diff hunk shows the paragraph immediately before line `### 9.`), and the diff does not show a `### 8.` heading, but the hunk's location (`@@ -362,6 +362,33 @@ ... This` after the #644-adjacent text) is consistent with it being at the end of section 8. Settle by viewing the ADR file headings. Not a block.

### WARN findings

**[W-1] crates/holler-pane/src/tx_switch.rs:223-225 — `check_listed` calls `list_sessions` to verify the target exists, but the brief's P4 says this check is for `Existing` only. The implementation gates it on `if let Some(target) = &existing` — correct. No warning.**

Re-evaluate for real warning-level issues:

**[W-2] crates/holler-cli/src/pane/switch.rs:133-144 — `execute` accepts `target: Result<Target, PaneError>` and on error emits `ErrorBody::from(&error)` directly, bypassing the `SwitchFailure`'s reconcile-step logic.** This is correct for `usage` before any port call (per the brief's step 1), but it means a `parse_session_id` failure is rendered with the bare `PaneError::Usage` message and not through the shared failure path — intended. No warning.

**[W-3] docs/adr/ADR-0021.md:625-628 — The "Deferred to named stories" bullet order and wording does not match Decision 17(c)'s prescribed single form "after #644 for launch and relaunch; #645 for switch and reset."** The diff inverts the order to "#645 for switch and reset; #644 to follow for launch and relaunch." This is a minor spec deviation in the ADR's deferred-bullet. Since AC 24 only checks that the bullet is edited and `#645` count is higher, the exact wording isn't pinned, but the brief explicitly gives one form. Recommendation: reword to match Decision 17(c)'s canonical sentence.

### NIT findings

**[NIT-1] crates/holler-pane/src/tx_switch.rs:33-34 — the `SESSION_ID_MAX` doc comment's parenthetical explanation ("`quoted` counts characters and `parse_session_id` counts bytes, which agree because an accepted id is ASCII") is oddly self-justifying and references an internal consistency that the reader can't confirm from the code alone. It also potentially overstates the guarantee: a 64-byte accepted id is also 64 characters, so no cut occurs; the explanation is true but redundant.**

**[NIT-2] ADR-0021.md:625-628 — see W-3; if not deemed a warning, it's a nit-level wording inversion.**

### Verdict

PASS — no BLOCK findings; testing may proceed.