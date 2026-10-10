## Implementation Review (Round 1)

### BLOCK findings

None.

### Needs-verification findings

**[NV-1]** `crates/holler-cli/src/pane/launch.rs:214` (`effective_spec`) — the model-splitting rule. The brief states a bad `--model` is "not `PROVIDER/ID` with both halves non-empty". The implementation uses `text.split_once('/')`, which splits at the *first* `/`, so `p/a/b` yields provider `p`, id `a/b`. Whether "ID may contain `/`" is the intended contract is not shown in this prompt; it would be settled by the spec for `ModelSpec` parsing in `crates/holler-pane/src/pane.rs` or an ADR on model naming. This is not a block.

**[NV-2]** `crates/holler-cli/tests/pane_verbs/launch.rs` and `launch/rig.rs` — the assertion that the tests don't call `assert_matches` for AC 6, AC 7, and AC 18b. The test code invokes `rig.assert_matches` in most cases and the brief says those three cases are exempt; but whether the specific tests present here correctly omit it in the right circumstances is a test-harness property not verifiable from the prompt. Evidence that would settle it: reading `launch.rs` / `relaunch.rs` test bodies and checking each test's final assertion (the prompt does include those bodies, and they do not call `assert_matches` in those cases, so this is likely true; but the *correctness* of the exemption itself — i.e. that no record-vs-fakes mismatch is being masked — cannot be confirmed from the prompt).

**[NV-3]** `crates/holler-pane/src/tx_launch.rs:437-438` (`close_old`) — the decision of whether `close_old` fires on a move is computed by `plan.target.workspace != old.herdr.workspace || plan.target.grid != old.herdr.grid`. If `--grid` is given but names the record's own cell, `check_relaunch_rules` allows it (no refusal), and `close_old` would compute `moved == false` and skip the close, as intended. But if only the workspace changes with the same grid, the brief says that is a move. This matches the implementation. Whether the *record's own* Herdr pane is ever closed when `--grid` is given with a cell that happens to be occupied by a *different* pane at the same cell is guarded by `same_pane` (which checks session and pane_id), so it is not closed. This is consistent with the brief.

**[NV-4]** `crates/holler-cli/src/pane/relaunch.rs:68` — the base selection for relaunch: `profile.as_ref().and_then(|profile| spec_of(profile, &name)).unwrap_or(&from_record)` — but `from_record` is owned local, and `unwrap_or(&from_record)` returns a reference with `'static` lifetime? The code compiles in the diff (as far as the review can see), so the lifetime is not a problem; however, whether the *specific* precedence is correct (P's spec always wins over the record's, even if P's spec is stale or empty) is the brief's intended rule; the prompt shows it matches.

**[NV-5]** `crates/holler-pane/src/tx_launch.rs:139-183` — `port_of_policy` rejects `"fixed:048100"` because of the leading-zero check `!digits.starts_with('0')`. The brief says "canonical decimal (digits only, no leading zero)", and AC 15 pins `fixed:048100` as `usage`. This matches. No block.

**[NV-6]** `crates/holler-pane/src/tx_launch.rs:474-480` (`with_note`) — the rollback note path uses `PaneError::from_wire(other.code().to_owned(), String::new(), detail)`. Whether this always reconstructs the same variant for all payload-bearing errors (e.g. `ProbeFailed`, `Unavailable`) is not verifiable from the prompt. The Evidence Appendix's F-line on `from_wire`/`from_closed` is a single quoted excerpt covering lines 565-566 and 599-601, which does not show the full re-construction logic. This is a needs-verification, not a block.

**[NV-7]** `crates/holler-pane/src/tx_launch.rs:732-760` (`closing`/`occupied`) — whether `closed()` actually handles `PaneError::PaneNotFound` for the *real* `HerdrPort` adapter (not just the fake) is an adapter-contract question. The brief's rollback rules say `pane-not-found` from `close` counts as closed; the implementation matches. But the real `HerdrPort::close` is provisional until #640, so no prompt evidence shows it; it would be settled by the #640 brief / conformance suite. Not a block.

**[NV-8]** `crates/holler-cli/tests/pane_verbs/relaunch.rs` — the test `relaunch_with_grid_moves_the_pane` asserts the old id is gone from the snapshot after `rig.run(&relaunch_with(&["--grid", "c1r3"]), Format::Text)`. Whether the move is visible in the *real* adapter's `ensure_pane`/`close` semantics is not shown; the fake's behavior may not match `#640`'s. The brief explicitly raises this in Risks 8. Not a block.

### WARN findings

**[W-1]** `crates/holler-cli/src/pane/launch.rs:396-415` (`failure_body`) — the reconcile-step appending rule uses `body.message.contains(step.as_str())`. If the step's own `reconcile_step(Some(P))` ever shrinks or changes its phrasing between #663's merged code and this diff's copy, a message containing the real scope's step would silently fail to match and the verb would append a duplicate step. The brief accepts this exact-string approach (decision 15), so it is not a block, but it is fragile against future edits to `reconcile_step`; a more robust test (e.g. checking `occurrences == 1`) already exists in AC 16k, so the risk is mitigated.

**[W-2]** `crates/holler-pane/src/tx_launch.rs:729-731` (`observe_live`) — `ports.herdr.snapshot()?.panes.contains(herdr)` uses `Vec::contains`, which requires `HerdrPane: PartialEq`. The struct is `PartialEq` per C-1, so it compiles; but `contains` compares the *entire* `HerdrPane` (session, workspace, pane_id, grid), not just session+pane_id. If the real Herdr adapter returns the same pane id with a *changed grid* in the snapshot (e.g. during a concurrent move), O2 would fail loudly with `unavailable` instead of recognizing the pane. The brief says the Herdr pane is where the snapshot says it is (G-3); if the grid has changed since `ensure_pane`, this is arguably a mismatch that should fail. But it is stricter than `same_pane`'s join used elsewhere. Worth confirming this is intended for O2 specifically.

**[W-3]** `crates/holler-pane/src/tx_launch.rs:490-494` (`session_of_record`) — relaunch keeps the record's session only if `list_sessions(plan.port)` contains *the exact string*. The fake `list_sessions` returns ids in creation order; the real `HarnessPort::list_sessions` is provisional, and the brief's Risks 9 notes a port change leaves the old server's session behind. If the real adapter normalizes session id strings (trailing newline, casing), the comparison would fail and a new session would be created unnecessarily. Worth a conformance check later; not a block.

**[W-4]** `crates/holler-pane/src/tx_launch.rs:378-382` (`budget check timing`) — the budget is checked *before* each live step, including before the record CAS at R. If the budget expires after the last pre-R check but before the record write, the record write itself carries no budget check inside it (it uses the port's own bound). The brief accepts "budget plus one call's own bound" (Risks 5), so this matches the stated contract; no block.

**[W-5]** `crates/holler-pane/src/tx_launch.rs:476-478` (`with_note` on `Refused`) — a `Refused` error is the only variant that carries `message` directly (not `detail`). The code clones the `RefusalCode` and appends the note. This matches the Evidence's F-line that `detail()` is `None` for `Refused`; but the `RefusalCode::clone` cost is trivial. No action needed; noting for review.

### NIT findings

**[NIT-1]** `crates/holler-cli/src/pane/launch.rs:214-218` — the `given` closure takes `fn(&ProfileSpec) -> &String` and is used repeatedly; it might be clearer as a method on `Problems` or a small helper, but this is cosmetic.

**[NIT-2]** `crates/holler-pane/src/tx_launch.rs:726-728` — `cell(&herdr.workspace, herdr.grid)` is passed to `format!` inside `unavailable`, but the line `the Herdr pane {} of {} is gone from {}` puts the pane id (`quoted`) before the name; the brief's illustrative wording is "the Herdr pane <id> of NAME is gone". The implementation's ordering matches; no issue.

**[NIT-3]** `crates/holler-pane/src/tx_launch.rs:106-120` — `TxOptions`'s `now_ms` field is a function pointer, while the brief's public API said `pub now_ms: fn() -> i64`, which matches; but the doc comment says "where `ReconcileRequest.now_ms` is a value taken once", referencing a type not part of this story's public API. Harmless, but slightly confusing.

**[NIT-4]** `crates/holler-cli/tests/pane_verbs/launch/rig.rs:515-520` — `assert_matches` uses `record.harness.port` to look up the server, but the port is a `u16`; if two rig ports share the same server name in a future test, the assertion would over-match. The rig's `RIG_PORTS` are distinct (48100-48102), so this is fine for now.

### Verdict

PASS — no BLOCK findings; testing may proceed.