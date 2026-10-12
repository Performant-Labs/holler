# Handoff-S: #648 - the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ  (spec audit)

**Date:** 2026-10-11 (21:06 MDT)
**Branch:** issue-648-implementation (HEAD `9216165`; merge base with `origin/main` is `bda1069`; `origin/main` is at
`d6ae539`)
**Issue:** #648. The brief (`docs/handoffs/648-brief.md`) holds the issue's text: scope items 1 to 7, acceptance 1 to 7
and the "Decisions" block, checked against `gh issue view 648`. On top of that it adds acceptance 8 to 14, the refresh
decisions R1 to R10 and the evidence E1 to E20.
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md` and `evidence.md` in `docs/handoffs/648/`, plus the diff gate's `docs/handoffs/648-diff-result-r1.md`
**Verdict:** PASS

## A precondition

Met.
- **`handoff-A.md` (the plan review): PASS.** It has no blocks and eight binding warns (W-1 to W-8), each followed by T
  or F as written (see Spec compliance). W-9 to W-12 are for O, T and S.
- **`handoff-A-dup.md` (the anti-duplication gate): PASS.** It has five warns and one info, none blocking. Its two
  warns for O are carried in the advisory notes below.

## T precondition

Met. There are no blocking issues, and RED and then GREEN are both confirmed.

- **RED, at `4bc2088`:**
  - **The process tests.** Three of the four new ones failed on their assertions (`(1, "")` against `(2, "")`, and the
    header without the six columns). The fourth, `roster_usage_errors_are_clap_text_in_every_format`, is a declared
    pin.
  - **The in-process target** failed to build on exactly the four API items F adds.
  - **A scratch stub** of today's behaviour behind the new API gave 0 of 25 passing, and every failure was an
    assertion. A scratch trial gave 25 of 25. Both were reverted.
- **The tests came before the code.** The tests and goldens were committed at `4bc2088`. F's commit `f7f2bcf` touches no
  test file.
- **GREEN, at `f7f2bcf`:**
  - The targets passed 25 (`roster_panes_test`), 9 (`roster_cli_test`), 16 (`docs_pane_control_test`), 3, 3, 35
    and 3.
  - The workspace suite, the pipeline's `test.unit.command`, exited 0: 2402 passed, 0 failed and 32 ignored.
  - Clippy is clean with `-D warnings`. `lint.sh` exits 0, `changelog-check` is ok and `cargo machete` is clean.
  - Seven mutations of `roster_cmd.rs` each failed the matching tests: the pair rule, the join, the `NotRead` note, the
    profile scope, the profile filter, the policy code and the de-duplication.
  - `Blocking issues: None.`
- **GREEN still holds at HEAD.** `git diff --stat f7f2bcf HEAD` lists only four handoff files.

## Acceptance criteria

Test files are under `crates/holler-cli/tests/`. `rpt` is `roster_panes_test.rs`, `rp/` is `roster_panes/`, and `rct`
is `roster_cli_test.rs`.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Old columns byte-identical, the new trailing columns `-` for an agent with no pane | `rpt:394` `old_columns_are_unchanged_and_the_six_new_ones_follow_hold`: golden `fixtures/roster_panes/base.txt`, and each line checked against a frozen copy of today's format through HOLD (`rpt:367-386`), with the next byte a space. The fixture has the no-pane rows `alpha` and `io/alpha`; a pane per HEALTH value, with and without a profile; and `hj-c1r2` with SHOWN set and DRIVEN `None`, which prints `-`. There is no summary line. `io/alpha`'s CONN overflow is kept as today (HOLD is padded as a cell). | Met |
| 2 | Summary line first, names in roster order (golden); none, no line | `rpt:439` (`two_mismatches.txt`): the reply's order, `hj-c1r2, demo-c1r1`, not the store's name order. `zz-c2r2` (SHOWN `-`, DRIVEN known) is not named. The no-line case is `base.txt`, which holds the same kind of pane. The third golden, `one_mismatch.txt`, is `rpt:449`. | Met |
| 3 | `--json` parses, every key kept with its value, new keys present | `rpt:462`: the top level is exactly `rows`, and every key of the hub's own serialized `Row` is equal. The six keys are always present: `null`s for the no-pane rows, `pos` in `GridPos` form, `health` `{"unhealthy": "port closed"}` and DRIVEN `null`, never the session of record. W-10's key-collision guard is there too. The text word `unhealthy` is in `base.txt` (R5). | Met |
| 4 | Envelope: `check_envelope`, `ok`, v1, `data` = the payload; the same exit codes across formats | `rpt:539`: `data` has exactly the two keys, `data.rows` equals `--json`'s rows, and `mismatches` is `[]`, then the two names, then the one name. For the exit codes, 0 holds in every success case, and 3 is `rp/profile.rs:96` (the `profile-not-found` envelope). 2 is `rp/profile.rs:109` (no envelope) and `rct:401` (clap's text, empty stdout). `check_envelope` checks `schema_version` 1 and that the class matches the exit code (`holler-pane-testkit/src/envelope.rs`). | Met |
| 5 | POS never `c<col>r<row>`; golden `hj-c1r2` at `r2c1` | `base.txt` row `hj-c1r2/build ... hj-c1r2  r2c1`. `rpt:429-432` finds no `c1r2` outside the name. The JSON `pos` is `{"row":2,"col":1,"pos":"r2c1"}` (`rpt:511-522`). `--format=json`'s rows equal `--json`'s. | Met |
| 6 | PROFILE or `-`; `--profile P` lists P's panes; an unknown P is refused, exit 3 | PROFILE is `fleet` or `-` in `base.txt`. `rp/profile.rs:36` lists `demo-c1r1/main` and `zz-c2r2/work` in all three modes, and drops the no-pane rows and the pane with no profile. `rp/profile.rs:96` (3 in every mode, with an empty reply too) and `rct:308` (the binary, both formats). The old refusal test and its constant are deleted (`pane_verbs/process/legacy_verbs.rs`, `stub.rs`). | Met |
| 7 | The same values as `pane list`; read fresh; no adapter call | `rp/registry.rs:35`: the cells equal `PaneRow::from(&pane).cells()`, taken by column name. `rp/registry.rs:54`: `concurrent_put` between calls, and the line appears, then goes. `rp/registry.rs:89`: the Herdr, host, harness and prober call logs stay empty in every mode, with and without `--profile`, and the store's record fills its row. DRIVEN is `last_observed.driven`, never `session_of_record` (`demo-c1r1` shows `ses-a`, not `ses-record`). | Met |
| 8 | Registry unavailable: `-` columns, the note by code, exit 0 | `rp/registry.rs:160`: `not-implemented` gives golden `no_registry.txt`, an empty stderr, `null`s, `mismatches` `[]` and exit 0, in all modes. `rp/registry.rs:168`: `unavailable`, `timeout` and `store-corrupt` each print exactly one `note: pane registry unavailable (<code>)` line, exit 0, in all modes. `rp/registry.rs:190`: a failed `connect` (R2). | Met |
| 9 | One mismatch, `pane(s)` for N = 1 | `one_mismatch.txt` (`rpt:449`). | Met |
| 10 | Join by label; `-` and `null` unjoined; `--profile` drops unjoined rows | `rp/join.rs:34`: the unlabelled `hj-c3r1` joins, in text and JSON. `rp/join.rs:58`: `other/demo-c1r1` is never joined by its session. `one_mismatch.txt`: two labelled rows carry one pane, which is named once, and `alpha` shows `-`. `["hj-c1r2"]` comes from `rpt:539`, and the drops from `rp/profile.rs:36`. | Met |
| 11 | Exit codes per class and format | `rp/hubless.rs:52`: with no live hub, a seeded store and a failing one give the same bytes: exit 1, today's line in text and `--json`, and an `unavailable` envelope with today's message. The source is never called, under the state-dir lock (W-6). `rp/hubless.rs:103`: the policy refusal is 3 in every mode. Also `rct:401` (2), `rp/profile.rs:96` (3), `rp/hubless.rs:130` (R3's 2) and `rp/profile.rs:124` (R4's 1). | Met |
| 12 | The real wiring, end to end | `rct:308`, over a live hub and body with an empty registry: today's header followed by the six columns, `-` cells, no summary and no `note:`. `--json` gives six `null`s, and `--format=json` passes `check_envelope` with `mismatches` `[]`. `--profile nosuch` gives exit 3 and one `error:` line, or the `profile-not-found` envelope. | Met (advisory 7) |
| 13 | `--server` | (a) `rct:358`: `ws://127.0.0.1:1` with no hub gives exit 2, the line and an empty stdout, in both formats, so the check comes before any dial. `rp/hubless.rs:130` also puts it before the policy refusal. (b) `rct:377` over the remote-admin rig: exit 0, `-` cells, stderr exactly the note, and `--json` `null`s. The envelope's `mismatches` `[]` is checked in `rp/registry.rs:202`. | Met |
| 14 | `--profile` over a registry that cannot answer | `rp/profile.rs:124`: `FakeProfileScope` over a store whose standing fault is `unavailable` gives exit 1 with only the error (no note) in text and `--json`, and the `unavailable` envelope under `--format=json`. Without `--profile` it is AC 8's note and exit 0. `rp/profile.rs:147`: a failed `connect` under `--profile` (W-1). | Met |
| Scope 2 | The help and legend say a pane's name is not its position | `rpt:573` renders `roster`'s long help: the six columns, `hj-c1r2` and `r2c1`. That is `HELP_LEGEND` (`roster_cmd.rs:45-66`), attached as `after_help` (`cli.rs:110`), so `-h` shows it too. It was RED before the change. | Met |

Every test above but the declared pin failed before the change (T-red) and fails under at least one of T-green's
mutations or golden comparisons, so none passes with the change removed.

## Spec compliance

**Refresh decisions R1 to R10:**
- **R1** (no unmanaged panes): the roster makes no Herdr call, so neither `snapshot` nor `observe_pane` runs (the empty
  log at `rp/registry.rs:113`).
- **R2** (the registry over the real wiring, after the hub): implemented as stated.
  - `run` makes today's hub call first (`roster_cmd.rs:130-134`).
  - Only after a reply, and never with `--server`, does it call the deferred source (`:135-139`).
  - `main.rs`'s `run_roster` (`main.rs:278-287`) is the roster's one caller of `Wiring::connect`. It holds the
    `Wiring` in a `OnceCell` for the lent `Ports`.
  - A failed `connect` without `--profile` is the note and exit 0 (`roster_cmd.rs:298`).
- **R3** (`--server`): `Registry::NotRead`, the `-` columns, `null` keys, no summary, `mismatches` `[]`, the one note
  and exit 0, as stated. `--profile` with `--server` is refused first in `run` (`:124-126`), before any dial. That is
  the observable behaviour R3 asks for. A's binding W-4 placed the check in `run` rather than in `main.rs`'s format
  check, so this is recorded, not a silent deviation.
- **R4** (`--profile` with a registry that cannot answer): `scope.resolve`'s error goes through `emit_error` with its
  class's exit code (`:290-292`, `:165-168`). A failed `connect` under `--profile` is the same error (`:297`). Under
  `--profile`, `scope.resolve` is the one read, with no `pane_store.list()` beside it (W-1). That is what keeps AC 14's
  stderr to the one `error:` line.
- **R5** (`pane list`'s cells and JSON forms): the text cells are taken from `PaneRow::cells()` by name in
  `pane::list::COLUMNS` (`:463-473`). The JSON values come from `PaneRow`'s fields (`:488-509`).
- **R6** (DRIVEN is `last_observed.driven`): implemented as stated, and pinned by `base.txt`.
- **R7** (out of scope; a follow-up named): nothing in the diff touches it. Its follow-up is not filed yet, and is not
  due until #839 decides the join.
- **R8** (the split into the hub call and a pure function): `render` (`:153-184`) is the pure function, and the in-process
  tests call it with a literal reply.
  - `run`'s signature is W-1's deferred source, not the older "`run(roster, choice, ports)`". R2 supersedes that
    sentence, and the brief says R1 to R9 override older passages.
  - AC 11 (round 6) and AC 14 are in-process, as their own text requires (AC 11's injected stores cannot reach a
    binary).
- **R9** (docs, ADR-0021 and CHANGELOG):
  - The guide paragraph matches R9's text word for word; I compared it mechanically, ignoring line wrapping.
  - Section 12's bullet is gone.
  - Both `docs_pane_control_test.rs` edits are as listed.
  - ADR-0021's "Deferred to #648" is now R9's "Decided (#648)" sentence, and the line in "Deferred to named stories"
    is deleted.
  - The CHANGELOG entry is under `## [Unreleased]` / `### Enhancements` and links #633 and #648.
- **R10** (ADR 0003 line 94): the sentence is deleted.

**The issue's "Decisions" block:**
- **Columns from the registry.** The columns come from the registry, through the pane store or, under `--profile`, the
  scope, which reads the same store. No adapter is called, and `holler-hub` is untouched.
- **The pair rule.** It is `shown_driven_mismatch` (`:341-343`), and `reconcile::shown_differs` is not used.
- **The summary line.** Its byte format is pinned by three goldens.
- **`data`.** It is `{rows, mismatches}` and nothing else (`RosterData`, `:478-482`).
- **Exit codes.** They are 0, 2 and 3, plus R4's named exit 1 and today's hub-failure exit 1.
- **The `--profile` refusal.** It goes through `output::emit_error`.

**Recorded deviations (none silent; each is in handoff-F's Deviations and the decisions journal):**
- **ADR 0003 lines 91 and 99 gain roster carve-outs**, and the help of the global `--json` and `--format` changes to
  match (`cli.rs:63-64`, `:76-78`).
  - This is A's binding W-3, an approved exception.
  - Without it, ADR 0003 would state "`--format=json` is `--json` on every verb", which the new behaviour makes untrue.
  - These are factual corrections of R10's kind. They do not amend the `--json` shape, which is what the operator's
    "no ADR 0003 amendment" concerns (R10 itself reads it that way).
- **The CHANGELOG** says "(additive, so MINOR under ADR 0003)" where the brief's draft said "(additive, no ADR 0003
  change)". The draft's words would be untrue once W-3 edits ADR 0003. Two accurate sentences are added: `--profile`
  with `--server`, and the `note:` line.
- **A malformed `--profile` name is refused before the hub call** (`roster_cmd.rs:127-129`). This adds to W-1's order
  and does not change it.
- **The empty `--profile` result** prints `(no sessions of profile P on the hub)` (`:397-405`). The brief leaves this
  case open (A's W-12), and it is unpinned (advisory 5).
- **W-8's three ADR-0021 sentences**, an approved exception: the parked pane (`ADR-0021.md:270-272`), the roster among
  the wiring's users (`:323-326`), and `remote-policy-refused` among the open codes (`:818`).

## Quality audit

**Correctness and failure handling**
- **The hub call is unchanged.** It keeps today's messages. `NoLiveHub` still names the state dir, and
  `ControlError::RemotePolicyRefused`'s `Display` is its message, so today's line is kept byte for byte.
- **The codes matter only to the envelope.** A hub failure is `unavailable` (exit 1), or the roster-owned open code
  `remote-policy-refused` (exit 3, declared with `RefusalCode::from_static`, so it is checked at compile time).
- **One error path.** Every error goes through `fail` (`:211-219`). A `usage`-class error is one `error:` line on
  stderr in every mode. Any other error goes through `emit_error`: an envelope only under `--format=json`, and today's
  line under a bare `--json`.
- **Registry failures fail as the brief decided.** Without `--profile`, a store or `connect` error only empties the
  columns, with a note naming its code (`not-implemented` is silent). Under `--profile`, the error ends the run, so no
  empty list reads as "P has no pane".
- **A write that fails is reported.** `--json`'s bytes keep today's missing trailing newline and are flushed, and a
  failed write exits 1 (`:384-390`). Text and the envelope go through `emit`, which flushes.
- **Writes and concurrency.** The roster writes nothing, so no write can be lost.
- **Robustness.** A malformed reply (no `rows`) prints as an empty roster, as before. A non-object row gets no keys
  added.

**Build guards**
- **No panics.** There is no `unwrap`, `expect` or `panic` in `roster_cmd.rs`, `main.rs` or `cli.rs`.
- **Allows.** The only new `#[allow]`s are the four crate-level allows of the test target, each with `// #648`.
- **File sizes** (`wc -l` on every touched `.rs` file): `cli.rs` 784, `roster_panes_test.rs` 586, `roster_cmd.rs`
  578, `docs_pane_control_test.rs` 542, `roster_cli_test.rs` 408 and `main.rs` 348. The rest are under 240. All are
  under 900, and `lint.sh` exits 0.
- **Dead code.** There is none: `ROSTER_STORY`, `RosterResult` and `ROSTER_PROFILE_REFUSAL` are gone with no leftover
  reference, and `not_implemented_message` is still used by `prompt_target.rs`.

**Protocol**
- There is no wire change: `holler-hub`, `holler-pane`, `holler-proto` and the test kit are untouched.
- The six keys are CLI output, so `docs/protocol/v2.md` correctly stays unchanged.
- No existing golden or fixture changed. The four new goldens are under `tests/fixtures/roster_panes/`.
- ADR-0021 section 9's open codes now list the new code. The guide's section 9 lists open codes as "among them" and
  points to ADR-0021, so it needs no edit.

**Tests**
- **The tier is the cheapest that sees the bytes and the port calls.** The in-process tests use the test kit's fakes
  and `check_envelope`, and the fixture rows are the hub's own `Row`, serialized. The process tests, over a real hub and
  body, cover only what needs the binary: `main.rs`, `Wiring::connect`, clap and the remote-admin rig.
- **No fixed sleeps.** `wait_for` polls, and `Duration::from_secs(10)` is a stop bound, not a sleep.
- **No new dependency.** `tempfile` was already a dev-dependency, and `Cargo.toml` gains only the `[[test]]` entry.
- **The live hub is out of reach.** The in-process hub calls run under one lock, with `HOLLER_STATE_DIR` on an empty
  scratch dir (W-6).
- **RED-first evidence** is in handoff-T-red.

**Documentation**
- **CHANGELOG:** an `[Unreleased]` entry that links the issue.
- **The guide:** section 4 has R9's paragraph, and section 12 lost its bullet. ADR 0003 and ADR-0021 are updated in
  the same change.
- **Help:** `holler roster --help` carries the legend.
- **README:** unchanged, correctly. It names the command and shows no roster output, and no other doc shows the
  table's header. No script parses the roster's columns (`scripts/`, `agent-skills/`, `plugins/`).

**Public-repository privacy**
- **The grep.** I grepped every added line of `git diff origin/main...HEAD` for hostnames, tailnet and MagicDNS names,
  private IP ranges, local paths, personal and account names, mail domains and secret patterns.
- **Code, tests and docs:** no hit. The fixtures use `host-a`, `host-b`, `example.com` and `127.0.0.1`.
- **The one hit is not a leak.** It is in the brief's "Comments from the issue thread" heading: the issue commenter's
  public GitHub handle, copied from the public issue, as main's `docs/handoffs/642-brief.md` already does. It is not
  private infrastructure.

**Commit and PR hygiene**
- **Commits.** Every commit subject is a Conventional Commit (`chore(#648): ...`), and every commit carries
  `Co-Authored-By:`. That is the pipeline's convention, as on main's squash of #851.
- **The PR.** It is not open yet, so its AI disclosure and squash subject are owed then (advisory 3).

## Scope check

- **Delivered as briefed:**
  - `roster_cmd.rs`, the roster helper in `main.rs` and `Command::Roster`, and the `--profile` comment in `cli.rs`;
  - the tests and goldens, the guide and its test, ADR 0003, ADR-0021 and CHANGELOG.
- **No `roster_panes.rs`.** The merge fits `roster_cmd.rs` (578 lines).
- **Not touched:** `holler-hub` (as the brief requires), `README.md` and `docs/protocol/v2.md`, each correctly.
- **Over-delivery: none of substance.** Every extra item is a reviewed decision of A (W-3, W-4 and W-8), or fills a case
  the brief leaves open (the empty `--profile` line). `table_line` replaces the two fixed `format!` strings, which the
  six new columns and HOLD's new padding require. That is not an unrelated refactor.
- **Under-delivery: none.** Every acceptance criterion and scope item has a proving test.

## Verdict

**PASS.**
- All fourteen acceptance criteria and the seven scope items are met, each by a test that fails without the change.
- The refresh decisions and the issue's decisions are implemented as stated. The four deviations are recorded and
  approved, not silent.
- The build guards, the docs and the privacy check are clean.

## Advisory notes

None of these blocks the run.

1. **Whether the columns fill in production (A's W-12, asked of S).**
   - **This matches the issue's intent as decided.** The issue's own decision joins a row to a pane by "the agent's
     name equalling the pane's name". AC 10 (rounds 4 to 6, with E18's facts in view) made that precise as "the part
     of `Row.name` before the first `/`". F implemented it, in one function, `pane_label` (`roster_cmd.rs:331-333`),
     whose doc names #839.
   - **What it means in production.** The columns fill only for a row whose label is a pane's name. Nothing in the
     code mints a body's label from a pane's name. The setup wizard labels one body per remote host as
     `<hub>-<prefix>-<remote_host>` (`agent-skills/herdr-setup/SKILL.md:1341`), and `scripts/pane-restart:358`
     documents an orchestrator row whose label puts the host's name before the pane's.
     - Such rows show `-` in all six columns.
     - With DRIVEN `-` until #839 (R6), the summary line cannot fire yet in any case, which the guide says.
   - **Recommendation for O.** When R7's follow-up is filed, or in #839's brief, ask whether the row-to-pane relation
     should move to the pane's session of record once #839 decides the address. `pane_label` is the one place to
     change.
2. **ADR 0003's header line.**
   - **The convention.** Each earlier story that changed ADR 0003's text added a header line: `**Clarified by:**`
     for #454, #676 and #851, and `**Amended by:**` for #670.
   - **What #648 did.** It changes lines 91, 94 and 99 without one, following R10 ("nothing else in that file
     changes") and the operator's "no ADR 0003 amendment".
   - **The choice.** If the operator wants the header to list every story that changed ADR 0003, add
     `**Clarified by:** #648` with a one-line summary, at the merge or in a follow-up.
3. **Owed when the PR opens.**
   - **The disclosure.** Add the AI disclosure to the PR body per `CONTRIBUTING.md` (`gh pr edit`).
   - **The squash subject.** Use a Conventional Commit subject; the issue's title is one.
   - **The merge.** Merge `origin/main` first. `d6ae539` (#851) touches `cli.rs`, `main.rs`, ADR 0003, the guide,
     CHANGELOG and `stub.rs`. A trial merge (`git merge-tree --write-tree HEAD origin/main`) has no conflict, but re-run
     `docs_cli_test`, `docs_pane_control_test`, `pane_cli_process` and `changelog-check.sh` on the merged tree.
4. **The refreshed brief had no outside brief gate.**
   - **When the gates ran.** This run's outside brief-gate results (`648-brief-result-r1` to `r3`, deepseek-v4-pro)
     date from 05:48 to 06:12 MDT on 2026-10-11. That is before the refresh commit, `3e57ca0`, at 19:27 MDT. The diff
     gate ran on deepseek-v4-pro at 20:47 MDT and passed.
   - **What the brief says.** Its own Pipeline section says the refreshed brief needs a new brief gate and that none
     has passed.
   - **The record.** If the run is journalled as `second-opinion`, it should say that the refresh (R1 to R10, AC 12
     to 14) had A's plan review and the launch's confirmation of R3, R4, R8 and R10, but no outside brief gate.
5. **The empty `--profile` line is unpinned.**
   - **What it is.** `(no sessions of profile P on the hub)` (`roster_cmd.rs:397-405`) is F's answer to a case the
     brief leaves open. It is better than today's line, which would claim the hub has no session.
   - **Why it matters.** No test pins the line, and it is user-visible. A one-line test would pin it.
6. **`WIDTHS` is not tied to the column arrays.**
   - **What it is.** `WIDTHS` has 14 entries (`roster_cmd.rs:85`) for the 9 + 6 columns, and nothing at compile time
     ties it to `HUB_COLUMNS` and `PANE_COLUMNS`. `table_line` zips the cells with it (`:430`).
   - **What can go wrong.** If a column is added to either array without a width, its cell is silently dropped from
     every line. Only the goldens would catch that.
   - **The fix.** One `const` assertion is a cheap guard. The diff gate raised the same point (its W-1 and W-2).
7. **AC 12's process test checks named keys, not every key.**
   - **What it checks.** For today's keys, `rct:308` checks eight of them by name.
   - **Why it is covered.** "Every existing key with its value" is proven in-process by AC 3's test, over the hub's
     own `Row` and through the same `render` path. AC 12's own purpose, the binary going through `Wiring::connect`, is
     fully pinned.
8. **Follow-ups for O to file** (none is filed yet; `gh issue list` finds no roster follow-up):
   - (a) W-8's doc drift. `pane/list.rs:239-240` still says `PaneRow.driven` is "printed, never compared", and
     `holler-pane/src/reconcile.rs:181-183` names the roster as a reader of `shown_differs`. Add the product question
     of whether the roster should flag a parked pane.
   - (b) A-dup's finding 1. The pair rule's twins say "keep the two equal" (`say_cmd.rs:300`, `dispatch.rs:86`) and do
     not name `roster_cmd::shown_driven_mismatch`. This belongs in #839's brief.
   - (c) R7's follow-up, once #839 decides the join.
   - (d) W-9. Decide between `note:` and `warning:`, and whether `data` should carry a signal for a partial read.
9. **Cosmetic, no rework.**
   - `usage()` copies `output::usage_body` (A-dup 3).
   - The state-dir lock and the whole-word matcher are second copies on the test side (A-dup 4 and 5).
   - `roster_cmd.rs` has two rustfmt diffs. rustfmt is run by neither CI nor `lint.sh`.
