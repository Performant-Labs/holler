# Handoff-A: Phase 3 - `holler profile apply` (#664)  (up-front plan review, round 2)

**Date:** 2026-10-11 (08:14 MDT)
**Branch:** `issue-664-implementation` (head `fc11ee9`; its merge base is `origin/main` `263a2d7`, which includes #808)
**Brief reviewed:** `docs/handoffs/664-brief.md`, unchanged since round 1's PASS (`7bd8242`)   **Reuse map:** none (no `survey.md`); "Reuse expectations for Phase 7" below stands in for one   **Wireframe:** N/A (`uiSurface: false`)
**Why a round 2:** the run took the rework route. After T-green (`fc11ee9`), the outside diff gate (`deepseek-v4-pro`, `docs/handoffs/664-diff-result-r1.md`, gitignored) returned BLOCK. F had reported `archChanged`, so the script sent the run back to A before T and F run again. The task's wording is "before any code exists", but F's code exists (`7f23e0f`). This pass reviews the plan and uses that code as evidence. Round 1's full table is in `7bd8242`.
**Verdict:** BLOCK

## Summary

BLOCK. The brief gives the up-front observation two shapes that cannot both hold:

- **§9 (the operator's text)** bounds the observation by one I5 per-call bound: "the up-front health observation runs under one I5 bound (default 10 s) ... a run of N panes is bounded by 10 s plus N step bounds".
- **C1 step 3 and C4 (the brief gate's clarifications)** put each recorded pane's `health` *and* shown session on "one thread per pane", and count the observation as "the slowest pane's chain". Two blocking calls in a row take two per-call bounds.

F kept §9's arithmetic by running one thread per call. The outside diff gate blocked that on "one thread per pane", which was its only blocking finding. Neither shape is drift from the codebase, and the code is sound. The problem is the brief: no implementation meets every sentence of it. A third pass of T and F on this brief would end in the same block from the diff gate, or, after a switch to a per-pane chain, in an objection to §9's "10 s plus N step bounds" from the diff gate or S. So the brief must say one thing before the run goes on. Notes for O give the amendment. The recommended option needs no operator decision and keeps F's code. My round-1 PASS should have caught this contradiction and did not.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | The observation's threads and bound. §9 (brief:73) on one side. On the other: C1 step 3 (brief:106-107), C4 with R6 NV-2 (brief:181, 192-200), and C7's note (brief:308). | Concurrency, and the I5 bound the module doc records | **The contradiction.** §9 says health is observed "one thread per pane", the observation runs under "one I5 bound (default 10 s)", and the run is bounded by "10 s plus N step bounds". §2 also needs each pane's shown session. C1 step 3 puts it on the same per-pane thread as `health`, and C4 counts "the slowest pane's chain". A chain of two blocking port calls takes up to two per-call bounds (20 s by default), so it breaks §9's arithmetic.<br><br>**What F built.** F ran one thread per call (`tx_apply/observe.rs:48-77`) and recorded it as a deviation (`handoff-F.md:46-51`, `:145-146`). The module doc (`tx_apply.rs:38-49`) and ADR-0021:469 ("one thread per call") state it.<br><br>**What the diff gate did.** It blocked "2N+1 threads" against "one thread per pane" (`664-diff-result-r1.md:25-33`, `:59`, `:67`). It also noted that the bound was kept.<br><br>**Why the code is not the problem.** Both shapes are sound. The per-call fan-out is `reconcile::observe_all`'s mechanism (scoped `thread::Builder` threads, each joined, a lost thread answered) at a finer grain. That fits apply, whose two reads of a pane are independent; reconcile's chain has dependent calls. No ADR or doc mandates per-pane threads. Only `reconcile.rs:7-8` describes its own pass that way.<br><br>**Why another cycle cannot converge.** Under the brief's precedence rule, §9 wins where a clarification differs (brief:89-93). So F's shape is arguably compliant, but the gate read C1 step 3 literally. | Amend the brief so the observation has one shape and one bound. **Option 1 (recommended, O-level):** a clarification that keeps §9's arithmetic, and so F's code. Under it, C1 step 3's "one thread per pane" and C4's "chain" read "one thread per call" and "the slowest call". This applies the brief's own precedence rule, so it needs no operator decision. **Option 2 (needs the operator):** a per-pane chain, reconcile's shape. This changes §9's arithmetic to two per-call bounds plus N step bounds. F would restructure `observe.rs` and rewrite `tx_apply.rs:38-49` and ADR-0021:469. Ready-to-paste text is in Notes for O. |
| 2 | warn | `crates/holler-pane/src/tx_apply/observe.rs:4-7` (F's code; the plan's reuse expectation 2) | Pattern consistency, doc accuracy | The doc says the observation has "the shape of `reconcile`'s `observe_all`". It does not. `observe_all` runs one thread for Herdr and one per pane, each pane's thread running a chain (`reconcile.rs:251-278`; `reconcile/observe.rs:3`, `:96-109`). `observe.rs` runs one thread per call. The mechanism is shared and the shape is not. | Under option 1, F restates it in one sentence: `observe_all`'s mechanism, at one thread per call rather than per pane, because a pane's two reads are independent and the observation then takes one per-call bound. Under option 2, the restructure makes the sentence true. |
| 3 | warn | §5 ("exactly one of" six outcomes, brief:62), §7 (`{pane, action, outcome, code?, diff?}`, brief:69), §4 ("a named code") | Contract shape (#666 consumes it) | Round 1's warn 9 is now realized in code, but not in the brief. F's vocabulary:<ul><li>actions `create`, `adopt`, `report`, `relaunch`, `refuse`, `observe`;</li><li>outcome `planned` for a dry-run step that would act;</li><li>code `spec-not-launchable`;</li><li>a `detail` key (grounded in O2's "the step's detail");</li><li>absent `code`, `diff` and `detail` print `null`.</li></ul>The code and ADR-0021 name all of these (`tx_apply.rs:125-201`). §5 calls its outcome set exhaustive without `planned`, and §7 writes `code?`/`diff?`. A gate that reads them literally can flag them. | Name them in the same amendment: they fill what §4, §6, §7 and O2 leave open, so that is O-level. Then S and the outside gate judge #666's contract against the brief. |
| 4 | warn | §9's twins follow-up and §13's follow-up | Deliberate duplication | The twins `leave`/`join` (`tx_apply/run.rs:216-240`) are faithful, private copies of `detach` and `join`'s write for one pane, which is acceptable as a reviewed duplication. But both follow-ups are only drafted (`handoff-F.md:220-252`), not filed. Reuse expectation 4 counts the twins issue as part of what makes the duplication acceptable. | The run's agent files both before the PR, and F cites the twins issue in `run.rs`'s module doc. |
| 5 | warn | C5 and C1 step 6 (round 1, warn 5) | Transactions | Unchanged. A P edit after a step's re-read can still be overwritten inside `edit_spec`. A row is chosen once from the observation and is not re-decided at its step. F records both under Known issues (`handoff-F.md:253-265`). | No change for this story. The two remedies stay the operator's, as in round 1. |

### Round 1's warns, now

| # | Status |
|---|---|
| 1 | **Done.** T merged #808 before writing tests. The check is the `(session, pane_id)` pair only (`observe.rs:161-166`), and a labelled pane is adopted (`apply/observe.rs:72-77`). |
| 2 | **Partly done.** The mechanism is followed, and a lost thread is `unavailable` naming the call. The grain is finding 1, and the doc claim is finding 2. |
| 3 | **Done.** `plan` zips P's specs with `diff_profile`'s rows (`tx_apply.rs:328-341`). `spec_records` keeps only the records P's specs name, so there are no `Extra` rows and the zip pairs one to one (`profile_diff.rs:287-308`). |
| 4 | **Done.** `reconcile_step` is appended when a step's launch or relaunch had acted (`profile/apply.rs:97-101`). Stored strings go through `quoted`, and `show`'s `text` and `difference_line` are reused. |
| 5 | **Open for the operator.** See finding 5. |
| 6 | **Done.** `apply/rig.rs` is additive and copies `launch/rig.rs` by name. `profile_verbs/rig.rs` gains only `serve_server`. |
| 7 | **Done.** The engine is split into `tx_apply/{observe,run}.rs` (546, 184 and 240 lines). F made two edits outside the Blast radius and justified both: `show.rs` visibility, and one word in `docs/research/herdr-api-spike.md`. |
| 8 | **Done.** `ORCHESTRATOR_NOT_RELAUNCHED` and `SPEC_NOT_LAUNCHABLE` are `RefusalCode::from_static` constants. ADR-0021 section 9's row lists them, and "Apply as built" defines "adopt" against the never-adopt sentences. |
| 9 | **Named in code.** See finding 3. |

### Drift re-check of F's architecture (verified, no action)

- **Layering.** The engine is in `holler-pane`, works only through `Ports` and holds no I/O. The verb in `holler-cli` types the name, calls the engine and prints through `emit` and `ErrorBody`.
- **Untouched shared code.** `tx_launch.rs`, `lib.rs` and every `Cargo.toml` and `Cargo.lock` are unchanged. There is no new dependency and no wire or protocol change.
- **Dependency direction.** `holler-pane` does not reach `holler-cli`. The verb reuses `reconcile_step`, `list::count` and `show`'s two helpers, made `pub(crate)`, which follows the profile verbs' existing sharing pattern (`super::delete::detach`).
- **Error handling.** Every step answer goes through `class_of` (`tx_apply.rs:509-518`, `observe.rs:172-184`). There is no local code list.
- **The diff gate's other notes need no change:**
  - **Its quoting warn** (`other_profile` prints `record.name` unquoted): a `PaneName` follows `SessionName`'s grammar (`pane.rs:35-41`) and is printed unquoted throughout `holler-pane` (for example `reconcile/observe.rs:137`). Profile names refuse control characters and print as they are in `list` and `create`.
  - **NV-1:** `Ports` is `Copy`, since the code compiles.
  - **NIT-1:** `PaneError` implements `Display`.

## Notes for O

1. **Amend the brief so the observation has one shape and one bound** (finding 1). Recommended: option 1. It applies the brief's own precedence rule to §9, so it needs no operator decision, and it keeps F's code. Add, for example:

   > **C21. The observation's threads and bound (outside diff gate r1; from §2, §9 and the precedence rule above).** §9 observes each recorded pane's `health` on a thread of its own and bounds the up-front observation by one I5 per-call bound, so a run is bounded by 10 s plus N step bounds. The shown session that §2 also needs (C1 step 3, C12) is read on a thread of its own beside that pane's `health`, and Herdr's snapshot on one more: every call of the observation runs at once, one thread per call, and the observation takes one per-call bound, as §9's arithmetic says. A pane's two reads are independent, so neither waits on the other. Where C1 step 3 says "its `health` and its shown session, one thread per pane" and C4 counts "the slowest pane's chain", they read "one thread per call" and "the slowest call", since §9 wins where they differ. The fan-out is `reconcile`'s mechanism (scoped `thread::Builder` threads, each joined, a thread that cannot start or panics answered `unavailable` naming its call) at one thread per call rather than per pane.

   Option 2 is the per-pane chain, `reconcile`'s own shape. It needs the operator, because §9's sentence would become "two per-call bounds (20 s by default) plus N step bounds". F would then restructure `observe.rs` and rewrite `tx_apply.rs:38-49` and ADR-0021:469.
2. **In the same amendment (recommended, not blocking): name the step vocabulary as built** (finding 3):
   - actions: `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name), and `observe` (a failed observation);
   - a dry-run step that would act is `planned`, and row d's is `reported`;
   - a `usage` answer for a step's spec is refused `spec-not-launchable`;
   - the step object adds `detail`, and absent `code`, `diff` and `detail` print `null`.
3. **Then start a fresh run on this worktree.** Never use `resumeFromRunId`, which replays this verdict. `resumeAtPhase` needs `designApproved: true` and a `resumeReason`.
   - `resumeAtPhase: 4` lets A re-review the amendment first.
   - Under option 1, the only code change left is finding 2's one doc sentence (F).
4. **Before the PR,** file §9's twins follow-up and §13's follow-up (both drafted in `handoff-F.md`, "Known issues"), and have F cite the twins issue in `run.rs`'s module doc (finding 4).
5. **Confirm the rigor.** The brief's Pipeline section says `rigor: in-session`, but the outside model ran both the brief gate (`664-brief-result-r1..r3.md`) and this diff gate. So the run is at `second-opinion` or above. Confirm the intended rigor when you relaunch.

## Reuse expectations for Phase 7 (A-dup)

A-dup checks these against the diff after the amendment:

1. **Met.** The branch contains #808. "Listed" compares `(session, pane_id)` only, and never `HerdrPane` `==`, `contains` or `same_place`.
2. **Mechanism met; grain per the amendment.** The observation uses `observe_all`'s mechanism: scoped `Builder` threads, each joined, and a lost thread answered. The grain is per call under option 1 or per pane under option 2. `observe.rs`'s doc must describe the grain it has (finding 2). There are no channels, no `thread::spawn` and no new dependency.
3. **Met.** Each row's `PaneDiff` is `diff_profile`'s row. None is built by hand.
4. **Met, issue pending.** The twins are exactly `detach` and a one-pane `join`: private, with no `undo` and no profile delete. The follow-up issue must be filed (finding 4).
5. **Met.** Steps are classified through `class_of`.
6. **Met.** An acted failure's message carries `reconcile_step(Some(P))`, and stored strings are quoted or escaped.
7. **Met.** Apply's live rig is additive, and `profile_verbs/rig.rs`'s existing helpers behave as before.
8. **Met.** `orchestrator-not-relaunched` and `spec-not-launchable` are constants in `tx_apply.rs`, listed in ADR-0021 section 9's apply row.

## Patterns referenced

- **The house fan-out:** `crates/holler-pane/src/reconcile.rs:7-8`, `:251-283` (`observe_all`), and `reconcile/observe.rs:1-9`, `:96-120` (the per-pane chain and the lost chain).
- **I5:** `docs/adr/ADR-0021.md` section 2 (`:94-99`, every port call bounded) and section 12 (`:680-704`, "I5 is satisfied by the per-call bound").
- **`tx_launch.rs`'s "Bound (I5)" form**, which the brief's C4 follows.
- **The diff under review:** `crates/holler-pane/src/tx_apply.rs`, `tx_apply/observe.rs` and `tx_apply/run.rs`, and `crates/holler-cli/src/profile/apply.rs`.
- **The pairing `plan` relies on:** `crates/holler-pane/src/profile_diff.rs:287-308` (`diff_profile`).
