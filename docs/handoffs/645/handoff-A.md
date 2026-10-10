# Handoff-A: Phase 3 - #645a `pane switch` and `pane reset`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `47baab4`, based on `ce12cdb`)
**Brief reviewed:** `docs/handoffs/645-brief.md` (1,120 lines)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 962-974 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on one finding. The brief shares doctor's test rig by loading `doctor/rig.rs` a second time with `#[path]` from
`switch.rs`. Clippy rejects that: its `duplicate_mod` lint fires, and AC 25 and CI both run clippy with `-D warnings`. I
reproduced the error on this toolchain. Every way out is ruled out by the brief's own rules: no edit to #647's `doctor.rs`,
and no copied rig. So T would write RED tests that cannot pass GREEN. O has to choose a fix and write it in.

The rest of the plan matches the codebase. A pure engine in `holler-pane` over `Ports` (ADR-0021 section 5) serves both
verbs. The plan reads the record and makes every refusal check before any live change. Health is observed live (I6). The
record is cloned, four fields are set, and it is written in one compare-and-swap that is never retried, as reconcile's
`record` does. There is no Herdr or host call, so I4 holds by construction. The plan reuses `findings::doctor_command`,
`findings::quoted` and `reconcile::shown_differs`. The open codes are declared with `from_static` in the module that
raises them. `unavailable` for a mismatch agrees with #644. The CLI pairing (`pub(crate) enum Verb` and `emit_outcome` in
the first verb's file) and the JSON `data` shape match #644's. 645b's `--first` is routed through the hub's one prompt path
(`send_prompt`), as the choke-point rule requires. The six warns below are small edits to the brief.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Reuse map "Test world" row (line 972); Decision 16 (lines 1014-1017): `#[path = "doctor/rig.rs"] pub(crate) mod rig;` in `tests/pane_verbs/switch.rs` | pattern consistency (shared test helper); contradicts AC 25 | `tests/pane_verbs/doctor.rs:10` already loads `doctor/rig.rs` (`mod rig;`). Loading it again triggers clippy `duplicate_mod`, which `-D warnings` (AC 25; `ci.yml:280`) turns into an error. Reproduced on clippy 0.1.98. It also breaks the crate's pattern: a shared test module is declared once and reached by crate path (#662 on `main`: `profile_verbs/list.rs:4-5`, `show.rs:11`). | O picks one: **(a)** a named one-word edit in `doctor.rs:10` (`mod rig;` to `pub(crate) mod rig;`), with `switch.rs` and `reset.rs` using `crate::doctor::rig`; or **(b)** keep the include with `#[allow(clippy::duplicate_mod)] // #645` and justify it. (a) is preferred. See Notes for O. |
| 2 | warn | P5 (line 836); Decision 7 (lines 991-993); Risks | concurrency; choke point | "Not another pane's session of record" is enforced only as a read in the verb. Two concurrent switches of P and Q to the same session can both pass P5. Each `cas_put` fences only its own record, so both records then name one session: the wrong-session incident. The hub store does not enforce this rule, and doctor has no finding for it. | Add to Risks that P5 is a pre-check, not the authority. O files a follow-up: enforce the rule inside `pane/cas_put` under the pane lock (as `pane-in-other-profile` is), or add a doctor finding for a shared session of record. |
| 3 | warn | P3 message (line 834): `...; run holler pane relaunch <pane>` | duplication (remedy table) | `holler pane relaunch` is spelled outside "the one table of remedies" (`findings.rs:111-143`, whose `RELAUNCH` and `relaunch_command` are private). | Build it from the public `FindingKind::ServerDown.remedy(Some(&pane), FixState::NotFixable)`, or name doctor (`doctor_command(Some(&pane), false)`) instead. |
| 4 | warn | Decision 17(b); AC 24 (lines 944-946); AC 22 | ADR consistency | The switch/reset "as built" paragraph and the `unavailable` decision go into section 11. #644's amended brief (decision 20(c)) puts "Launch and relaunch as built", with the same `unavailable` decision, into section 8. After both merge, the ADR states one decision in two sections. | Put the paragraph in section 8 beside #644's. Keep in section 11 only the sentence on DRIVEN after a switch or reset. Whichever story lands second cites the other's `unavailable` sentence. Amend AC 24's list. Optional: ADR 0003 rows put a verb's own flag before `[--profile NAME]`, so use `[--as-operator] [--profile NAME]` (AC 22). |
| 5 | warn | `SwitchFailure::message` (API, lines 779-782) | duplication across stories | The engine writes "; to reconcile, run " and then `doctor_command(Some(&pane), true)`. #663's amended `profile_scope::reconcile_step(Option<&ProfileName>)` writes the same lead-in in `holler-cli`, in a form without the pane. Not a block: `holler-pane` cannot call `holler-cli`, and this form is section 8's "pane doctor command line for that pane". | O files a follow-up for one owner of the sentence: a `findings` function beside `doctor_command` that both call. F spells the lead-in exactly `to reconcile, run `. |
| 6 | warn | Forward-compat (lines 1063-1073) | naming (stable codes) | `SERVER_UNHEALTHY`, `ORCHESTRATOR_PANE` and `parse_session_id` are the workspace's first code and grammar for their conditions. ADR-0021 row 345 already has #646c declaring "open (#646) for an unhealthy pane". Codes are stable once merged, so two spellings of one condition would be permanent. | Add a Forward-compat row naming `holler_pane::tx_switch::{SERVER_UNHEALTHY, ORCHESTRATOR_PANE, parse_session_id}` as what later verbs reuse for the same condition. O passes it to 646c's brief. |
| 7 | warn | P1 (line 832): "record = `panes[0]`" | error boundaries | Indexing panics if a `ProfileScope` implementation returns no pane. The workspace denies `panic`, `unwrap` and `expect`, but no enabled lint catches indexing. | Take the first element with `into_iter().next()` and map `None` to `PaneNotFound { what: pane }`. |

### Finding detail (the evidence behind each row)

**1. The rig is loaded twice.**

- Where the file is loaded now: `crates/holler-cli/tests/pane_verbs/doctor.rs:9-11` declares `mod read_only; mod rig; mod
  surface;`. `mod rig;` in the non-`mod.rs` file `doctor.rs` resolves to `doctor/rig.rs`.
- What the brief adds: `#[path = "doctor/rig.rs"] pub(crate) mod rig;` in `switch.rs`, which resolves to the same file. The
  rig uses only crate-absolute paths (`crate::verb_harness`), so it compiles twice. The brief stops there ("two compilations
  of one file in one test crate are harmless (`tests/pane_verbs/main.rs` allows `dead_code`)").
- Clippy's `duplicate_mod` (suspicious group, warn by default) checks exactly this. `[workspace.lints]` in `Cargo.toml:19-30`
  does not configure it. AC 25 runs `cargo clippy --workspace --all-targets -- -D warnings`, and so does CI
  (`.github/workflows/ci.yml:280`).
- Reproduction: a scratch crate with the same layout (`tests/pane_verbs/main.rs` with `mod doctor; mod switch;`, `doctor.rs`
  with `mod rig;`, `switch.rs` with the brief's line), checked with clippy 0.1.98 (rustc 1.98.1, the workspace toolchain):

  ```
  error: file is loaded as a module multiple times: `tests/pane_verbs/doctor/rig.rs`
   --> tests/pane_verbs/doctor.rs:1:1   first loaded here
   ::: tests/pane_verbs/switch.rs:1:1   loaded again here
    = help: replace all but one `mod` item with `use` items
    = note: `-D clippy::duplicate-mod` implied by `-D warnings`
  ```
- The pattern elsewhere: a shared test module is declared once and reached by its crate path.
  - On `main` (#662): `tests/profile_verbs/list.rs:4-5` declares `#[path = "rig.rs"] pub(crate) mod rig;`, and `show.rs:11`
    uses `crate::list::rig::{...}`.
  - In flight: #643 plans `crate::list::Rig` (643-brief decision 10), and #646 plans `pub(crate) mod rig;` in `park.rs`
    reached as `crate::park::rig` (646-brief decision 11).
  - In this brief: `reset.rs` uses `crate::switch::rig`.
  - Every other `#[path]` include in `holler-cli/tests` loads a file once per test crate.
- Why the run cannot recover by itself: `cargo test` passes, so T's RED step does not see the problem. The first clippy run
  fails at GREEN. Every fix then breaks a rule the brief states: editing `doctor.rs` (Files, lines 958-960: "no #647
  file"), a copied rig (Reuse map, "Not" column), or a lint allow nobody approved. F does not own the test files. #647's
  part 2 (the `unattached` and `agent-cannot-dispatch` kinds) is still open and owns `tests/pane_verbs/doctor.rs`. That is
  why the choice belongs to O.

**2. One session, two panes.**

- The window: P5 reads `pane_store.list()`, and R writes P's record. In between, the real `select_session` waits up to
  about 2 s for the title to confirm (spike lines 237-239, brief R-2). A second switch of Q to the same session can pass its
  own P5 in that window. Both `cas_put` calls succeed, because each fences only its own record's generation.
- Nothing else catches it:
  - The hub store enforces only membership inside its compare-and-swap (`crates/holler-hub/src/panes/store.rs:339-348`).
  - Doctor's twelve kinds (`findings.rs:42-73`) have none for two records naming one session. Strays exclude every recorded
    session (`reconcile.rs:374-402`).
  - So the ADR's "the reconcile pass (#647) is the safety net" (section 8, lines 277-279) does not cover this race.
- The precedent for a fix: ADR-0021 "Decisions taken", item 2, moved `pane-in-other-profile` into the pane registry's
  compare-and-swap so that "no writer can slip in between the check and the write". #650's import also writes existing
  session ids into records (ADR-0021 section 13, step 2), so a store-level rule would serve both writers.

**3 to 7.** Evidence for the warns:

- **3:**
  - `findings.rs:35-38` (the private `DOCTOR`, `RELAUNCH` and `RESET` constants), `:111-143` ("the one table of remedies";
    `ServerDown` and `ServerWedged` map to `relaunch_command`), `:318-326`.
  - The same applies to O1's screen wording. "its home screen" and `session <quoted>` exist privately as `screen_text`
    (`reconcile/observe.rs:343-348`). A private copy in `tx_switch.rs` is acceptable under the no-#647-file rule, but its
    wording should match, so doctor and switch describe a screen the same way.
- **4:**
  - ADR-0021 section 8 (lines 265-306) holds plan, act, record, "fails loudly" and the reconcile step. Section 11 (lines
    438-456) is attach mode and the DRIVEN deferral.
  - #644's plan: 644-brief decision 20(c) and (d), on `issue-644-implementation` at `7195993`.
  - ADR 0003's merged row: `holler pane doctor [PANE] [--fix] [--profile NAME]` (`ADR-0003.md:61`).
- **5:**
  - 663-brief decision 8 (line 1992, on `issue-663-implementation` at `137c00f`): `pub fn reconcile_step(profile:
    Option<&ProfileName>) -> String`, built on `findings::doctor_command`, with `reconcile_step(None) == "to reconcile, run
    holler pane doctor"`.
  - #644 (644-brief decision 15) calls #663's function.
  - `findings::doctor_command`'s own doc (`findings.rs:303-306`) says the reconcile step is built there "rather than
    spelling it again".
- **6:**
  - ADR-0021 row 345 (`say`, `interrupt`, `answer` with `--pane`: "open (#646) for an unhealthy pane ...") and the
    stable-code rule (lines 329-330).
  - No `from_static` code, and no session-id grammar, exists anywhere in the workspace for these conditions. The only open
    code declared so far is `grid-unreachable`. The merged OpenCode adapter (#642 part 1) percent-encodes ids in URL paths
    and defines no grammar of its own.
- **7:** `ProfileScope::resolve`'s contract (`profile.rs:380-389`) says a named pane resolves to "just that one". Reconcile
  keeps the `Vec` and never indexes it (`reconcile.rs:226-227`).

## Notes for O

Amend the brief as follows, then start a **fresh** run. `resumeFromRunId` would replay this verdict.

1. **Finding 1, the rig.** Choose (a) or (b) and write it into Decision 16, the Reuse map "Test world" row (line 972), the
   Files list (lines 953-960), the Size-check row for `tests/pane_verbs/switch.rs` (line 61, "rig include") and the AC
   preamble (lines 865-866).
   - **(a) Preferred.** Name one cross-story edit: `crates/holler-cli/tests/pane_verbs/doctor.rs:10`, `mod rig;` to
     `pub(crate) mod rig;`, visibility only. `switch.rs` and `reset.rs` both `use crate::doctor::rig::{...}`, with no
     `#[path]` include and no `crate::switch::rig`.
     - Remove "editing `doctor.rs` to export it" from the "Not" column, and exempt this one line from "no #647 file".
     - Tell #647's part 2 brief about the change, since part 2 owns that file. It is a one-line merge at most.
     - This is the pattern `main` already uses (`crate::list::rig`).
   - **(b)** Keep the include and put `#[allow(clippy::duplicate_mod)] // #645` on the `mod` item (`scripts/lint.sh`
     check 1 accepts an allow that carries a `// #NNN` link).
     - Decision 16 then states why two compilations are accepted.
     - Follow-up F-2 grows to "fold the include into `crate::doctor::rig` when #647 part 2 lands".
   - Either way, AC 25 stays as written, and a copied rig remains a Phase 7 rejection.
2. **The warns**, each a small edit:
   - Finding 2: a Risks entry, plus a follow-up for the store-level rule or a doctor finding.
   - Finding 3: the P3 message built from `FindingKind::ServerDown.remedy(...)`, or from `doctor_command`.
   - Finding 4: the ADR paragraph in section 8 beside #644's, the `unavailable` sentence stated once, AC 24 updated, and
     optionally the AC 22 flag order.
   - Finding 5: a follow-up naming one owner of the reconcile-step sentence.
   - Finding 6: the Forward-compat row, and a note to 646c's brief.
   - Finding 7: P1 reworded to "the first resolved pane, or `pane-not-found`".

## Patterns referenced

- `crates/holler-cli/tests/profile_verbs/list.rs:4-5` and `show.rs:11`: a shared test module declared once, reached by
  crate path (#662, merged). Also `crates/holler-cli/tests/pane_verbs/doctor.rs:9-11` and `doctor/rig.rs`.
- `crates/holler-pane/src/reconcile.rs` and `reconcile/observe.rs:289-386`: the analogous select, observe and record steps
  (the brief's named extension point), including the clone-and-set write with one compare-and-swap.
- `crates/holler-pane/src/findings.rs:111-143, 303-334`: the one remedy table, `doctor_command` and `quoted`.
- `docs/adr/ADR-0021.md`: section 5 (lines 177-193), section 8 (265-306), section 9 (322-345), section 11 (438-456),
  "Deferred to named stories" (528-537), "Decisions taken" item 2 (547-550).
- The in-flight sibling briefs, read from their worktrees: `issue-644-implementation` (`7195993`; decisions 15 and 20, the
  Output section), `issue-646-implementation` (`e957a8d`; decisions 10 and 11), `issue-643-implementation` (decision 10) and
  `issue-663-implementation` (`137c00f`; decision 8), each at `docs/handoffs/<N>-brief.md`.
