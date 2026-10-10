# Handoff-A: Phase 3 - #644 `pane launch` and `relaunch`  (up-front plan review, re-review)

**Date:** 2026-10-09
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `40f486e`, which contains `origin/main` `d9eabbb`)
**Brief reviewed:** `docs/handoffs/644-brief.md` (as amended through `40f486e`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 2403-2421 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This replaces the first review's BLOCK (`d2636ba`, still in git history).

## Summary

PASS. The amended brief fixes all four blocks from the first review, and each fix holds against the code now on `main`:

- relaunch's base is #662a's merged `spec_from_pane`;
- the relaunch rules are the engine's own first step (E0);
- launch writes `driven: None`, which is what #647's merged reconcile expects;
- the reconcile step is #663's merged `reconcile_step`, appended only when the message lacks it. That is the rule #663 wrote into ADR-0021 section 8 step 6.

T's dependency greps from the Test plan all pass on this branch, so RED will not stop on a signature. There is no new block.

Nine stories merged after the brief's evidence baseline (`3bdd129`), and they set patterns the brief does not name yet. That gives eight warns. Two of them change code:

- route untrusted text through the pane domain's sanitizers (warn 1);
- make O1 call `reconcile::shown_differs` (warn 2).

## Resolution of the first review's findings

| # | First review | Status | Checked against |
|---|---|---|---|
| 1 | block: `spec_of_pane` duplicates the Pane-to-spec mapping | Resolved. Removed; relaunch's base is `profile_snapshot::spec_from_pane`; no `"fixed:"` literal (brief lines 1813-1842, 2020-2021, 2416-2417). | `crates/holler-pane/src/profile_snapshot.rs:18-68` (merged #662a; signatures as pasted in I-1) |
| 2 | block: relaunch rules held only in the CLI; `move_grid` | Resolved. E0 in `tx_launch::relaunch`, `grid_given`, `spec.pane == name` in both engines, and AC 19b on the engine (lines 2028-2044, 2303-2311). | the scope runs its guards before any write (`crates/holler-cli/src/pane/profile_scope.rs:104-118`) |
| 3 | block: `last_observed.driven` inferred | Resolved. Launch writes `None` and relaunch keeps the stored value (C-14, decision 22, AC 1, AC 20). | `crates/holler-pane/src/reconcile.rs:33-35` ("left as stored"); ADR-0021 lines 492-493 |
| 4 | block: a second reconcile-step function | Resolved. #663's `reconcile_step(Option<&ProfileName>)` with the exact-substring append rule, pinned by AC 16h, 16j and 16k over the real `StoreScope`. | `profile_scope.rs:49-56, 122-156, 205-216`; ADR-0021 lines 331-339 |
| 5-10 | warns | Resolved in the brief: `now_millis` named (H-3); the rig rule (decision 25); B10 after R (decision 23); the cell follow-up (C-10); the lost-update risk (decision 24); the ADR edit rules (decision 20). Warns 4 and 7 below update two of these for code that has merged since. | |

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Messages the engine and verbs compose (step 5, E0, O1, O2, B2, B10, rollback) and the success line (`launched ... session <sid> ...`) | cross-cutting: untrusted text | Every merged pane verb that prints an untrusted value sanitizes it: reconcile and doctor use `findings::quoted` / `embedded`, park uses `quoted`, and the read verbs use `list::text_value`. The brief's templates insert these raw: the harness's session ids, the TUI's shown session, Herdr pane ids, workspace labels, and B10's adapter message. The brief's security section covers secrets only. | Route each untrusted value in a composed message through `findings::quoted`, and an adapter's message through `findings::embedded` (it is `pub(crate)` in `holler-pane`, so `tx_launch.rs` can call it). Route the success line's sid and workspace through `quoted` (as park does) or `list::text_value`. An error passed through unchanged stays as it is (`output.rs`). No AC changes: a quoted plain id still contains the id. |
| 2 | warn | O1 (`not Some(sid)` -> roll back) | pattern consistency: reuse | `reconcile::shown_differs` is documented as "the one form of this comparison", and both reconcile and #643's `SessionSync::of` call it. O1 would add a third, inline form. | O1 calls `holler_pane::reconcile::shown_differs(Some(&sid), shown.as_deref())`. Add it to the Reuse map. The semantics are the same and no AC changes. |
| 3 | warn | C-10; "What a rollback leaves ... reported by doctor"; AC 7's parenthetical; the #647 row of Forward-compat | cross-story contract | The merged doctor sees little of what a failed or crashed launch leaves. `unregistered-herdr-pane` appears only in a whole-fleet pass, and servers and tmux sessions are observed only through in-scope records. With `--profile P`, the printed step (`pane doctor --profile 'P'`, then `profile show 'P'`) reports nothing: P's spec was restored, so `show` has nothing to show either. A whole-fleet doctor reports only the Herdr pane. After a crash, the next launch is refused with `grid-occupied` and then `port-in-use`, because nothing stops a server that has no record. | Correct those four claims to match what the merged doctor reports. Widen the cell follow-up to cover the port and the tmux session. Add a follow-up for #663/#647: after a failed launch, the reconcile step or `pane doctor --profile` should reach leftovers that have no record. The step's text does not change in this story (decision 15 stands). |
| 4 | warn | Decision 25; the AC rig description (lines 2120-2136); the size check's `launch_rig.rs` fallback | test-helper duplication; file structure | Decision 25's condition has fired. `crate::list::Rig` (#643) is on `main`, and so are `crate::park::rig::Rig` (#646a) and doctor's private rig (#647). The brief's "no shared rig exists on `main` today" is stale, and a standalone launch rig would be the fourth copy. The two-verb precedent is `park.rs` declaring `pub(crate) mod rig;` in `park/rig.rs` (and `doctor/rig.rs` the same way), with no `#[path]`. | T takes decision 25's first branch and builds on `crate::list::Rig`. Its fields are `pub`, so `Rig { herdr: FakeHerdr::new("scratch").with_workspace("main", 3, 3)?, ..Rig::new(..)? }` needs no edit to `list.rs`. T adds only the linked host, the hook wrappers, and a call-log span named `mark`/`calls_since` as in doctor's rig. Anything T adds goes in `tests/pane_verbs/launch/rig.rs`, declared in `launch.rs` and reached as `crate::launch::rig`, not in `launch_rig.rs`. If T judges list's rig unfit, T follows park's precedent (#662's names) and says why in `handoff-T-red.md`. |
| 5 | warn | Decision 4 (`--herdr-session`, required); the #649 and #664 rows of Forward-compat | forward-compat: wiring | The merged `HerdrAdapter` serves exactly one session, `HerdrConfig.session`. It refuses any other session before it sends a request, and it stamps every snapshot pane with that session. In production the flag therefore has only one valid value. A wrong value fails at A1 inside the act, after P's first write (P's generation moves by 2), rather than in the plan. Keeping the flag is right for this run: the frozen `HerdrPort` cannot tell the engine its session, and the wiring is `Unwired`. | Add to the #649 Forward-compat row: default `--herdr-session` from the wiring's `HerdrConfig.session`, so the flag becomes optional (relaxing a required flag breaks no caller). #664's apply takes the session from the same place. Say this in the section 8 "as built" note. |
| 6 | warn | `TxOptions.now_ms: fn() -> i64` | pattern consistency: clock | The sibling engine takes the clock as a value: doctor fills `ReconcileRequest.now_ms: i64` once with `now_millis()`, and park takes `since: now_millis()` when the request is built. The brief's engine calls a function pointer at R instead. Both shapes work, and tests can pass a constant to either. | Keep the binding shape for this run, and have F's `TxOptions` doc say why: `last_observed.at` is stamped at R, after O1, which can be up to the budget after the CLI started. Aligning the engines' clock shapes joins the rig and engine follow-ups. |
| 7 | warn | Decision 20 (the ADR-0021 edits); decision 15 and C-16; the follow-up "the reconcile step names the pane" | ADR consistency | (a) `pane doctor [PANE]` has merged (ADR-0003 line 61), and section 8 step 6 (lines 331-339) says the step "names no pane on purpose". The follow-up contradicts that recorded decision, and C-16 and decision 15 are stale. (b) Step 6 already holds decision 15's append rule. (c) Decision 20's line numbers are about 40 lines out: the launch row is now line 376, the operator's paragraph ends at 510 and is followed by #647's "no hub timer", and "Deferred" starts at 567. Its content anchors still hold. (d) Section 9's reason for `unavailable` (line 434, "cannot be reached (also a garbled reply)") does not cover the way decision 14 uses it. | (a) Drop that follow-up, and cite step 6's reason in decision 15. (b) The "as built" note cites step 6 for decision 15 rather than restating it. (c) F anchors every ADR edit by content, not line number. (d) Amend the reason cell at line 434 in the same change, e.g. "..., or the live state a verb needs is not there after its act (#644)". Optionally, name `grid-unreachable` in the launch row as the Herdr adapter's code: it is #640's open code, and A1 and B3 can return it. |
| 8 | warn | `PaneLaunch.name`, `PaneRelaunch.name` | naming | The sibling `Args` structs call their positional field `pane` (`PaneGet.pane`, `PanePark.pane`, `PaneDoctor.pane`). | Name the field `pane`. Tests go through argv and never see the field, so F can rename it freely. |

### Finding detail (the evidence behind each row)

**1. Untrusted text.**
- The merged rule: `crates/holler-pane/src/findings.rs:19-21` says "Untrusted text (session and Herdr ids, the record's host names, adapter messages) reaches a message only through `quoted` (one value) or `embedded` (an adapter's message), so no control sequence reaches a terminal". `quoted` is at line 332 (`pub`) and `embedded` at line 338 (`pub(crate)`).
- Other places that follow it:
  - `crates/holler-cli/src/pane/park.rs:31-34`, park's text output;
  - `crates/holler-cli/src/pane/list.rs:270-291`, `text_value` ("Every stored string the read verbs print in text mode goes through here"), shared by `get.rs` and `watch.rs`.
- Why it matters here:
  - `shown_session` will read the TUI's title, which #642 maps back to a session id (`crates/holler-adapter-opencode/src/lib.rs:40-42`). An agent inside OpenCode can rename its own session, so O1's message can carry text the agent chose to the operator's terminal.
  - `create_session`'s ids come from the server's JSON.
- The brief's line 2641 ("may quote session ids, pane ids, ports") is about secrets, not about control sequences.

**2.** `crates/holler-pane/src/reconcile.rs:171-181`; `crates/holler-cli/src/pane/list.rs:226-239`.

**3.**
- The whole-fleet gate: `reconcile.rs:192` (`whole_fleet`) and `:311-317` (`unregistered_herdr_panes` runs only then).
- Strays come only from in-scope servers (`reconcile.rs:371-402`), and every chain starts from a record (`reconcile/observe.rs:96-109`).
- `crates/holler-adapter-host/src/lib.rs:31-39`: `stop_owned` stops only what `run` started. A crashed launch's server therefore keeps its port until #695, which stops a server by its recorded pid.
- ADR-0021 line 499 ("A crash between steps leaves state that the next pane doctor run finds and reports (#644's acceptance)") overstates the merged doctor as well. That sentence is #644's issue acceptance, so S decides whether it is met; this row flags only the contract gap.

**4.**
- `crates/holler-cli/tests/pane_verbs/list.rs:35-95`.
- `park.rs:8` and `park/rig.rs:1-12`: it uses #662's names "so a later consolidation of the rigs is mechanical".
- `doctor.rs:10` (`mod rig;`, private) and `doctor/rig.rs:110-272` (`Calls`, `mark`, `calls_since`, `ports_with`).

**5.** `crates/holler-adapter-herdr/src/adapter.rs:4-6, 44-54` (one session), `133-155` (`extent_of` refuses another session before any request), `326-344` (`snapshot`).

**6.** `reconcile.rs:62-73`; `crates/holler-cli/src/pane/doctor.rs:55-60`; `park.rs:88-99`.

**7.** ADR-0021 lines 331-339, 376, 434, 499, 509-517, 567-575; `profile_scope.rs:37-48`; ADR-0003 line 61.

**Checked against the merged code, no finding:**
- `HOST_NAME = "localhost"` matches the test kit's `sample_pane` (`crates/holler-pane-testkit/src/fixture.rs:52`).
- `OP_LAUNCH` and `OP_RELAUNCH` follow the `<group>.<name>` form of the other `op` strings (`"harness.serve"`, `"herdr.snapshot"`).
- The three open codes are new: nothing else declares them.
- The `data` shape: the profile verbs mix name-and-slug rows (`profile list`, `profile delete`) with the full `Profile` (`profile create`, `profile show`), so there is no dominant shape to drift from.
- `effective_spec` has no analogue on `main`: `profile create` takes no spec flags.
- `pub(crate)` for the shared verb helpers matches `profile/delete.rs` and `profile/list.rs`.
- The `PartialEq, Eq` and `Debug` the API derives are implemented by `PaneError`, `Profile`, `ProfileName` and `PaneName`.
- `holler_cli::pane::profile_scope::StoreScope` is reachable from the `pane_verbs` target (AC 16k).
- B2 and AC 22 match the merged `TmuxHost`, which does not stop a server that `serve` started.
- C-9 matches the OpenCode adapter's resolver precondition (`lib.rs:100-104`), and decision 7 matches its "no per-pane environment" (`lib.rs:56-58`).
- The surface excerpts E-10 to E-12 are as quoted.

## Notes for O

A PASS needs no amendment. There is no O agent in the automated path, so these warns go straight to the phases that act on them:

- **T:** warn 4 (build on `crate::list::Rig`; put any rig file in `launch/rig.rs`).
- **F:**
  - in code: warns 1 and 2 (the sanitizers and `shown_differs`), and warn 8 (the rename);
  - in the `TxOptions` doc: warn 6;
  - in the ADR edits: warn 7;
  - in the follow-ups and the "as built" note: warns 3 and 5.
- **Phase 7 (anti-duplication):** re-checks warns 1, 2 and 4 against the diff.

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs` and `reconcile/observe.rs` (#647): the sibling engine over `Ports`; `shown_differs`; the clock taken as a request value; the whole-fleet rule.
- `crates/holler-pane/src/findings.rs:19-21, 303-345`: `doctor_command`, `quoted`, `embedded`.
- `crates/holler-cli/src/pane/profile_scope.rs:37-56, 122-216` (#663), and ADR-0021 section 8 step 6 (lines 328-339).
- `crates/holler-cli/src/pane/{park,doctor,list,get}.rs` and `crates/holler-cli/src/profile/create.rs`: how the CLI verbs take the clock, sanitize text, share helpers and shape their `data`.
- `crates/holler-adapter-herdr/src/adapter.rs` (#640), `crates/holler-adapter-opencode/src/lib.rs` (#642), `crates/holler-adapter-host/src/lib.rs` (#641); `crates/holler-cli/tests/pane_verbs/{list.rs, park/rig.rs, doctor/rig.rs}`.
