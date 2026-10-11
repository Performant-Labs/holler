# Handoff-A: #650 - Import today's panes from the launcher's files  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-650-implementation` at `017cc6c` (base `origin/main` at `7284001`, with #802 merged in; the branch's own commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/650-brief.md` (as amended at `ac17120`)   **Reuse map:** the brief's own evidence, E1 to E46 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This review replaces the first run's (BLOCK, with 2 `block` and 6 `warn` findings; in git at `a204891`).

## Summary

PASS: 0 `block`, 6 `warn`. The brief resolves all eight findings of the first review, and I checked each fix against the code on the merged tree:

- **Secrets.** A `source-unreadable` message names the file and a position only. Acceptance 7 first proves that the secret is in the parser's own text, then asserts that it appears in no output.
- **Unhealthy server.** It is classified as reconcile classifies it (`server-wedged` or `server-down`, recorded as `Unhealthy(<code>)`). The hub's prompt gate then refuses it (`dispatch.rs:92`), as it refuses a reconciled pane.
- **`--herdr-session`.** It filters the snapshot by session, as launch's occupancy check does (`tx_launch.rs:506`).
- **The rest.** The undo's remedy steps are named. The finding key is `kind`. The ADR rows are listed. `push_spec` and `text` are widened, and they are reachable because `profile/mod.rs` declares `pub mod show`. Each pane is observed on its own thread.

**#802.** It changes nothing the import calls. Launch now records `pid: None`, as the import's record already did. The rest of the plan holds:

- the crate split;
- the reuse of `insert_profile` and `profile_from_panes`;
- the dependency rules: no manifest change, and `holler-pane` stays on `serde`, `serde_json` and `holler-proto`.

**The warns.** All six are corrections to the brief:

- one finding code duplicates reconcile's spelling;
- the record's health line is stale;
- the test kit cannot build one acceptance case as written;
- some references are stale after #802;
- the remedy's profile name is not quoted;
- the text form's finding lines name no pane.

Finding 1 should be settled before T writes the code table. A finding code is stable once merged (ADR-0021 `:599-600`), so it cannot be renamed after merge.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | "The finding vocabulary" (`:69`): `no-tmux-session` and `session-not-found` among "the new codes"; the problems list (`:57`) | pattern consistency: finding vocabulary | `no-tmux-session` is a second name for reconcile's `tmux-session-missing`: the same call (`host.ps`), the same error (`pane-not-found`), the same condition ("the pane's tmux session does not exist", `findings.rs:55-56`, `reconcile/observe.rs:133-146`). Join item 3 (`:29`) cites reconcile's code in the same sentence, and the brief's own rule is "a condition that reconcile already names keeps reconcile's spelling". `session-not-found` is not new: it is the closed `PaneError` code, with the same meaning (`error.rs:118`). | Report `FindingKind::TmuxSessionMissing.code()` and drop `no-tmux-session`, in item 3, the problems list and the vocabulary. List `session-not-found` with the reused codes, taken from `PaneCode::SessionNotFound.as_str()` (a `pub(crate) const fn`, `error.rs:45`, `:98`), as `command-not-argv` is. That leaves ten new codes, not twelve. |
| 2 | warn | "The record the import builds" (`:45`): `health: healthy only when this run's health answered healthy, otherwise unknown`; acceptance 6 | consistency with reconcile (the mapping the brief copies) | The record line predates the amendment: item 4 and acceptance 6 now record `Unhealthy(<code>)` for `Ok(false)`. There is a second gap. For `Ok(true)` followed by a failed `list_sessions`, acceptance 6 records `unknown`, but reconcile records `healthy`: its `Server::Healthy { sessions: None }` (`reconcile/observe.rs:158-166`) maps to `Health::Healthy` (`ObservedHealth::recorded`, `reconcile.rs:152-161`). The brief says it copies that mapping. | State the mapping once, as reconcile's: `Ok(true)` is `healthy` whether or not the list was read, `Ok(false)` is `Unhealthy(<code>)`, and a `health` `Err` is `unknown`. Amend the record line and acceptance 6's list case to match, or say why the import differs. Only display depends on the difference: the hub's gate refuses `Unhealthy(_)` alone (`dispatch.rs:92`). |
| 3 | warn | Acceptance 13 (`:101`): "a fake Herdr with the label `hj-c1r2` in two sessions" | test kit ownership | `FakeHerdr` serves exactly one Herdr session. `snapshot` stamps every entry with that session (`herdr.rs:362-366`), and `ensure_pane` in any other session is `unavailable` (`:370-386`). The test kit's `src/` is frozen for this story. The first review said the fake can hold several sessions; that was wrong. | The second half needs no wrapper: pass a `--herdr-session` the fake does not serve. The first half needs two panes labelled `hj-c1r2`, one of them reported in another session by a `HerdrPort` wrapper. `DisplacingHerdr` in `tests/pane_verbs/launch/displaced.rs` does exactly that with `Displace::Session`, but it is private (`mod displaced`, `launch.rs:14`). Either widen that module to `pub(crate)` (a visibility-only edit; add it to the blast radius), or state that the import's rig holds its own wrapper. |
| 4 | warn | "What the engine observes" (`:23`): "Nothing else is called: not ..., `run`, ..., `serve`, ..."; acceptance 2 (`:90`); line references | stale after #802 | #802, now merged into this branch, replaced `HarnessPort::serve` with `serve_command` and `verify_serving` (`HarnessOp::ServeCommand`, `VerifyServing`). Several line references have moved:<br>- ADR-0021 `:298-300` is now `:374-376`<br>- ADR-0021 `:512-513` is now `:599-600`<br>- the open-codes row `:584` is now `:671`<br>- `tx_launch.rs:456` is now `:506`<br>E12, E19, E23 and E35 were cut before #802: doctor's rig now starts servers through `start_server`, and launch records no `pid`. | Replace `serve` in the list with `serve_command` and `verify_serving`. Write acceptance 2 as an allow-list over each fake's `faults().calls()`: `HerdrOp::{Snapshot, Version}`, `HostOp::Ps`, `HarnessOp::{Health, ListSessions}`, reads of the two stores, and no prober call. Update the line references. |
| 5 | warn | Acceptance 10 (`:98`): the remedy commands `holler profile delete fleet`, `holler profile delete fleet --keep-panes`, `holler profile create fleet --from-current` | pattern consistency: commands a message suggests | #662 B1 puts every profile name in a suggested command through `single_quoted` (`profile/delete.rs:202-208`). `create`'s own remedy does so (`profile/create.rs:249-260`: `holler profile delete 'NAME' --keep-panes`). The unquoted spelling would pin a second form in a test. | Build the commands with `single_quoted`, and have acceptance 10 match `holler profile delete 'fleet'` and the rest in the same form. |
| 6 | warn | "The output" (`:65`), text form: the pane lines, then each finding as `  <level> <code>: <message>` | pattern consistency: report text | A finding line names no pane. Doctor's text form names a subject on every finding line (`<pane> <pos>` or `-`, `pane/doctor.rs:96-113`), and several messages (`observe-failed`, `session-not-found`) do not name their pane. The brief also does not say:<br>- where a finding with `pane: null` prints: `ignored-entry`, a directory whose name does not parse, an unreadable launcher table;<br>- how such a problem counts in `import-blocked`'s "N pane(s)". | Choose one form and state it:<br>- each pane's findings follow its line, and pane-less findings print first under `-`; or<br>- the subject is on each finding line, as in doctor.<br>Also say how a pane-less problem counts. For example: an unreadable launcher table blocks every pane, each with its own `source-unreadable`. |

## Deliberate copies (for the Phase 7 gate)

The brief or a fixed boundary justifies each of these copies, so the anti-duplication gate should not flag them:

- **From reconcile.** `reconcile.rs` and `reconcile/observe.rs` are frozen (#647), and these items are private there:
  - the unhealthy-server classification (`observe_server`'s `Ok(false)` arm);
  - the mapping of `ObservedHealth::recorded`;
  - the format of `failed_message` (`<port>.<method> failed (<code>): <message>`);
  - the thread-per-pane scaffold of `observe_all` and `joined`;
  - the call-name constants.

  `observe_server` also needs a `Pane` record, which the import does not have yet. If the import uses the public `reconcile::ObservedHealth` as its classification type, the copy shrinks to the four-arm mapping.
- **The failing-N-th-write seam.** `NthCasPut` lives in `tests/profile_verbs/rig.rs`, which is a separate test target (`profile_verbs`, `Cargo.toml:484-486`), so `pane_verbs` cannot import it. The import's tests need their own copy, with `ApplyThenFail`. `park/failures.rs`'s `FailNthCasPut` is private and answers only `Conflict`.
- **A no-writes check over doctor's `Calls`.** The launch and close rigs each keep their own `Calls` type and their own check, so one more in the import's rig follows that pattern.
- **The `--herdr-session` field.** Launch declares it inline (`pane/launch.rs:61-63`), not in `args.rs`.
- **The import's undo is not a copy.** Decision 5 makes it new code: it deletes the records it wrote, where `create`'s undo detaches.

## Notes for T and F

- **Faults in a concurrent pass.** Each pane is observed on its own thread, and a fake's `fail_next` fails whichever call comes next.
  - A per-call fault is deterministic only in a one-pane fixture: acceptance 6's `health` or `list_sessions` error, and a `host.ps` error. Doctor's tests do it that way (`doctor/surface.rs:558-562`).
  - `freeze(port)` gives `server-wedged` and `kill(port)` gives `server-down`, each on a single pane's port.
  - The call logs interleave across panes, so assert on counts and sets, not on order.
- **`import_test.rs` holds pure cases only.** `holler-pane` has no dev-dependency on the test kit. The file can hold the code table and any join or plan function that takes plain values.

## Notes for O

- **What PASS means here.** The run goes on to T. Findings 1 and 2 cost nothing to settle before T writes the code table and acceptance 6, and finding 1 cannot be changed after merge. If the run stops for any other reason, fold all six into the brief.
- **For the cutover (#654), not this story.** Since #802, the OpenCode adapter refuses every agent with `agent-unsupported` until #642 part 3 merges (ADR-0021 `:609`). The import still records a pane's `opencode_agent` (from `OC_DEFAULT_AGENT`) correctly. But relaunching such a pane is refused until then, because launch and relaunch pass that agent to `serve_command` in their plan (`tx_launch.rs`, `ServeSpec { agent: spec.opencode_agent }`).

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs` and `reconcile/observe.rs`: the vocabulary (`tmux-session-missing`), the server classification and the recorded health, the thread per pane, and `observe-failed`'s message.
- `crates/holler-pane/src/findings.rs` and `error.rs`: `FindingKind::code` and `PaneCode::as_str`, the single sources of the codes.
- `crates/holler-cli/src/profile/create.rs`, `delete.rs` and `show.rs`: `insert_profile`, the undo and its remedy, `single_quoted`, `push_spec` and `text`.
- `crates/holler-pane-testkit/src/herdr.rs`, `crates/holler-cli/tests/pane_verbs/launch/displaced.rs` and `doctor/rig.rs`: what the fakes can model, and the wrappers that compose them.
- `docs/adr/ADR-0021.md` sections 5, 9, 10 and 13 on the merged tree, and the #802 diff (`263a2d7..7284001`).
