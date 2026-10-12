# Decision journal: #803, the setup wizard builds panes through `holler pane` and `holler profile apply`

Run handoff directory `docs/handoffs/803` · branch `issue-803-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its own entry.

## A, up-front plan review: 2026-10-12T08:41-06:00

- **Decided:** PASS, with 0 blocks and 12 warns (see handoff-A.md).
  - **What the plan gets right.** It extends `herdr.sh`, the one door with one scrub, and uses the CLI's existing verbs.
    The one new script, `lib/panes.sh`, is justified in writing and depends only on `instance.sh`'s printed contract.
  - **Every CLI contract the brief cites holds against the code:**
    - apply exits 0, 1 or 3, with `data.steps`;
    - launch requires `--herdr-session` unless `--spec-only` is given;
    - the JSON keys of `get`, `list` and `doctor` are as cited;
    - `profile delete` is refused while members remain;
    - ADR-0021 section 5 holds: verbs run CLI-side, configured from the CLI process's environment.
  - **What T and F must act on first:**
    - **W1.** The shared allow-list keeps `HERDR_*` for `holler` too, and there is no fail-closed guard for a lost
      `WIZARD_TMUX_SOCKET`.
    - **W2.** The profile name P is never defined.
    - **W3.** A rerun stops at `profile create`, which refuses `profile-exists`.
    - **W4.** Case 3's guard is attributed to a `pane-exists` refusal that apply never gives: apply adopts, reports or
      relaunches an unprofiled record whose name matches a wizard pane.
  - **Blast radius.** It misses two pinned tests that must change: `instance.rs:283-287` and `skilltext_838.rs:107`
    (W5).
  - **Accepted duplicate.** The port rule's copy in `panes.sh` is the one accepted duplicate, provided `[instance]` and
    `workspace` are read only from `instance.sh`'s output (W6).
- **Assumed:**
  - **Where the tmux server's environment comes from.** I assumed it is the environment of the process that starts it.
    That is D8's U2, and I did not run it. The host adapter passes on the CLI's environment minus `TMUX` and
    `TMUX_PANE` (`holler-adapter-host/src/lib.rs:236-245`), and OpenCode harnesses inherit the server's environment
    (`ProcessEnv::Inherit`).
  - **What an agent inside a Herdr pane carries.** I assumed it has `HERDR_SOCKET_PATH` and `HERDR_PANE_ID`
    (`docs/research/herdr-api-spike.md:108,149,400`).
- **Hedged:**
  - **W1's severity.** I kept W1 at warn, not block, by #831's precedent: a fix inside the plan's own intent, along an
    existing pattern (`tests/pane_integration/main.rs:104-115`, `SCRUBBED`), with no operator decision needed. If F
    cannot apply it within D8's "one list, shared", that is an escalation, not a workaround.
  - **The profile name.** Recommending the workspace label is a judgment call. `<prefix>` is also acceptable, if F
    records the choice.
- **Evidence:**
  - **The brief:** `docs/handoffs/803-brief.md` (sha256 `7ee5ef5d23f9f523...`). I read the decisions, Scope, Amendments,
    Acceptance, Blast radius, Depends on, Clashes and Rules sections, and listed the Evidence section.
  - **The wizard's code:** `lib/herdr.sh` and `lib/instance.sh` in full; the headers of `lib/ledger.sh` and
    `lib/stop-owned.sh`; `SKILL.md` lines 1-750 and its stage headings.
  - **The CLI and adapters:**
    - in `crates/holler-cli/src/pane/`: `wiring/env.rs`, `launch.rs`, `args.rs`, `close.rs`, `get.rs`, `list.rs` and
      `doctor.rs`;
    - in `crates/holler-cli/src/profile/`: `apply.rs`, `create.rs`, `delete.rs` and `import.rs`;
    - `crates/holler-pane/src/tx_apply.rs` lines 1-60, and `tx_launch.rs:94`;
    - `crates/holler-pane/src/pane.rs` (the `Pane` and `HarnessInfo` structs);
    - `holler-adapter-host` `lib.rs` and `tmux.rs`; `holler-adapter-opencode` `lib.rs`; `holler-adapter-herdr`
      `plan.rs` and `adapter.rs`.
  - **The tests:**
    - in `crates/holler-cli/tests/wizard/`: `main.rs`, `herdr.rs`, `proof.rs`, `skilltext.rs`, `instance.rs`,
      `skilltext_838.rs` and `skilltext_w5_rename.rs`, and the `proof_fakes/` scripts;
    - `tests/pane_integration/main.rs`;
    - the list of tests in `docs_pane_control_test.rs`.
  - **The docs and config:** ADR-0021 sections 5 and 11, and "Decisions taken"; `scripts/lint.sh`;
    `.aftersight/pipeline.config.json`.

## T, author / RED: 2026-10-12T09:10-06:00

- **Decided:**
  - **The RED is valid.** `wizard_scripts` has 72 failing tests and 231 passing. Every failure is an assertion about the
    missing `panes.sh`, the missing `herdr.sh holler` and `run` allow-list, or the missing skill and docs text. None is a
    compile, target or harness error (handoff-T-red.md).
  - **The printed contract of `panes.sh`:**
    - `grid` prints TSV lines;
    - `env` prints shell assignments that `eval`;
    - `plan` and `teardown` print `<tag> <shell-quoted holler args>`;
    - `plan --rerun` drops `profile-create` (W3);
    - the dry-run line carries `--format=json`.
  - **The profile name P is the workspace label**, `holler` by default (A's W2 recommendation).
  - **W1 is pinned as A proposed:**
    - `holler` runs with no `HERDR_*` and with none of the caller's own `HOLLER_*`;
    - exit 2 for an unset or empty `WIZARD_STATE_DIR`;
    - exit 2 for a non-default instance whose `WIZARD_TMUX_SOCKET` is not absolute;
    - exit 2 for the default instance with no `WIZARD_HERDR_SOCKET`.
  - **`run` refuses with exit 2 and names `holler pane`.** `label` is now the ordinary usage error.
  - **A fifth module, `panes_803_plan.rs`**, keeps `panes_803.rs` under the 600-line warning.
  - **The old pins.** I retired the ones whose subject is gone, including the whole of `skilltext_w4_stage2.rs`, and
    moved the rest to their new stage. I also moved four pins the brief's table missed: `herdr.sh run pane read` in
    `skilltext_w3_stage3`; "orchestrator `dir`/`cmd`" and "model endpoint" in `skilltext_w6_preflight`; and the
    `# Herdr: M local` H1 lookups in `w6_preflight` and `w5_stage0`.
- **Assumed:**
  - **Where the port index counts from.** A pane with its own `backend_port` still takes an index in the `base + i`
    count, as `instance.sh` counts sessions.
  - **What the refusal messages name.** The taller-column refusal names "column 2", and the no-orchestrator refusal
    names "orchestrator".
  - **How the skill types a Holler call.** Every `herdr.sh holler` line in a code block carries the `<herdr_env>`
    placeholder.
- **Hedged:**
  - **Proof that the RED can be satisfied.** A throwaway reference `panes.sh` and `holler` verb in T's scratch directory
    passed all 33 behavioural tests three times. They were then removed and `herdr.sh` restored with `git checkout`;
    nothing of them is in the tree or the handoff.
  - **What text alone can show.** The text pins check wording and order, not that an agent following the skill does
    the right thing. The operator's repeat of #778 remains the only end-to-end proof (brief U1 to U8).
  - **Stage 9's DRIVEN sentence** is pinned only as "DRIVEN" and "#839" (REFRESH AFTER MERGE of #839).
- **Evidence:** `cargo test -p holler-cli --test wizard_scripts` (231 passed, 72 failed); `cargo clippy -p holler-cli
  --test wizard_scripts -- -D warnings` (clean); `bash scripts/lint.sh` (exit 0, no new warning);
  `skilltext_w5_rename` (7 passed with the new files staged); the files are staged by explicit path.

## F, implement (Phase 6): 2026-10-12T09:55-06:00

- **Decided:**
  - **`lib/panes.sh` is new and is the only reader of `[[pane]]` and `layout`.** It reads `[instance]` and
    `workspace` only from the lines `instance.sh` prints (W6), and takes the first instance's port base from
    `instance.sh /dev/null` rather than a second `47001`. Its value parser (`trim`, `value_of`) is the copy
    A allowed; the port rule is `instance.sh`'s, applied to the OpenCode panes in layout order.
  - **The profile P is the workspace label (W2)**, and `plan` / `teardown` refuse any other P (exit 2), so P
    stays computed by `panes.sh` alone.
  - **W1, as A proposed, and a departure from D8's "that same allow-list".** There is still one list, not a
    copy: `base_env` in `herdr.sh`. `server-start` adds `HERDR_*` to it (unchanged behaviour); `holler` adds
    the instance's `HOLLER_*` literals and no `HERDR_*`. `holler` refuses (exit 2, nothing run) a
    `WIZARD_STATE_DIR` that is unset, empty or relative, a named instance whose `WIZARD_TMUX_SOCKET` is not
    absolute, and the default instance whose `WIZARD_HERDR_SOCKET` is not absolute, and it runs `check_pane`
    first (A's optional guard).
  - **One new variable, `WIZARD_HOLLER_BIN`** (the `holler` binary when it is off `PATH`), the twin of
    `HERDR_BIN`: the skill already handles a Homebrew `holler` off `PATH`, and the door could not run one
    without it.
  - **`run` is an allow-list** of `status`, `session list` and `workspace list` (words after them pass);
    `label` is gone.
  - **The skill's stages follow D2**, the briefing is written at the start of Stage 7 (D10), and every pane
    call in a code block is `<herdr_env> bash $WIZARD_LIB/herdr.sh holler ...`. Stage 0's
    `holler pane --help` is in prose, not a code block: a code line naming `holler pane` must go through the
    door with `<herdr_env>`, which does not exist before Stage 1, and `--help` touches no hub, Herdr or tmux.
  - **The README's scenario menu has no letters**: the test forbids `(B)` there, and a menu A, C, D reads as
    a mistake.
  - **Two pins that conflict with other tests are both met by a line break inside a code span** (ADR-0021
    decision 34's `holler profile apply`, against `docs_cli_test`; the skill's "never `herdr.sh run server
    stop`", against the 803 ban on `herdr.sh run `). Reported to T in handoff-F.md.
  - **Scope cap not applied.** The story touches more than six files, but O, A and T already settled its
    scope, and T's RED covers all of it; a split now would leave tests red.
- **Assumed:**
  - **The CLI's flag shapes and outputs as A checked them** (`--spec-only` upserts the spec, `pane get`'s
    record keys, `pane list`'s `unmanaged` keys, apply's text and exit codes); quoted in evidence.md.
  - **A body may share the hub's `HOLLER_STATE_DIR` on one machine**, as the README's Quick Start runs both
    with the default state directory.
- **Hedged:**
  - **Everything the brief lists as U1 to U8** stays UNVERIFIED in the text (the default socket path, tmux
    inheriting the door's environment, a real 4 x 4, a fresh server's first pane, Claude Code end to end, a
    credential after the scrub, the 10 s budget). Only the operator's repeat of #778 settles them.
  - **The apply timeout figure** (10 s plus about 15 s per create) is my reading of A's W10, not a measured
    number.
  - **#839 and #850 are not merged**, so Stage 9's DRIVEN sentence and D13's "no body row for a Claude Code
    pane" stand as the brief wrote them.
- **Evidence:** `cargo test -p holler-cli --test wizard_scripts` (303 passed, 0 failed; RED was 231 / 72);
  `docs_cli_test` and `docs_pane_control_test` green; the script modules also pass with mawk and busybox awk
  first on `PATH`, and `panes.sh` prints byte-identical output under gawk, mawk and busybox awk; the full
  Tier 1 results are in handoff-F.md; evidence.md quotes the unchanged code relied on.

## T, verify / GREEN (Phase 7): 2026-10-12T10:12-06:00

- **Decided:**
  - **GREEN, no blocking issue.** The wizard suite passes 303 of 303. The CI workspace command passes 2624 tests with 0 failures in 167 binaries, matching F. Clippy, lint, changelog-check, machete, `docs_cli_test` and `wire_selftest` are all clean.
  - **Repaired F's three "tests that look wrong", in `skilltext_803.rs` only.**
    - The 803 ban now scans flattened text with whole-word matching (`contains_phrase`). The `herdr.sh run ` allow-list exempts the one "never `" mention.
    - The ADR decision pin ends before the closing backtick, so `holler profile apply NAME` also satisfies it.
    - Neither pin depends on a line break any more.
  - **Appended 4 evidence entries** for what the fake `holler` models from unchanged code: apply's JSON steps, apply's exit 3 with no steps, `grid-unreachable` below a row of two, and session agreement.
- **Assumed:**
  - **CI does not run `cargo fmt --check`.** Its run list has no fmt step, and the repo already has existing fmt drift. I formatted only the file I changed.
- **Hedged:**
  - **The ADR's code span is still wrapped across two lines** (advisory in handoff-T-green.md). It passes, but a reflow would break `docs_cli_test` unless the text names `NAME`.
  - **U1 to U8 stay UNVERIFIED** until the operator's #778 repeat.
- **Evidence:**
  - **Mutation spot-checks:**
    - disabling the guard fails 3 tests;
    - leaking `ANTHROPIC_API_KEY` fails the scrub test;
    - admitting `pane split` fails the `run` test;
    - a forward teardown fails the teardown test;
    - a wrapped banned call fails the repaired ban;
    - "pane runs" passes it.
  - **Commands and outputs:** in handoff-T-green.md.

## A, anti-duplication gate (Phase 8): 2026-10-12T10:20-06:00

- **Decided:**
  - **PASS, with 0 blocks and 4 warns** (see handoff-A-dup.md).
  - **F extended every object the map named:**
    - `herdr.sh` keeps one allow-list in `base_env`: `server-start` adds `HERDR_*` to it, and `holler` adds
      the instance's `HOLLER_*` literals.
    - `panes.sh` reads `[instance]` and `workspace` only from `instance.sh`'s printed lines, and takes the
      first instance's port base from `instance.sh /dev/null`.
    - The test harness (`Env`, `Host`, `code_lines`, `section` and the rest) is reused through
      `pub(super)`.
  - **The fake `holler` matches the code:** its action and outcome words, `cell_key`, rows a to f, apply's
    text and exit 3, and the JSON of `get`, `list`, `profile list` and `doctor`.
  - **The warns:**
    - **(1)** Stage 11 and the docs tell the user to run `holler pane close` / `relaunch` in their own
      shell with only `HOLLER_STATE_DIR`, a route around the door.
    - **(2)** The door passes a stray socket of the other kind of instance (T-green's advisory 2).
    - **(3)** `docs/setup-wizard.md` restates apply's creation order, which the brief said to state once.
    - **(4)** `panes.sh`'s `noslash` is an unlisted copy of `strip_slash`.
- **Assumed:**
  - **An earlier pane-config check.** A pane verb builds its Herdr configuration before acting, so a user's
    bare `holler pane close` with only `HOLLER_STATE_DIR` stops with `usage` and touches nothing. I read
    this from `env.rs:66-74`; I did not run it.
  - **The phase number.** Phase 8 follows this run's own handoffs (T-red 4, F 6, T-green 7) and the
    pipeline doc's line 96. The doc also calls this gate Phase 9 elsewhere.
- **Hedged:**
  - **Warn 2's severity.** It needs an agent mistake (a literal of the other instance typed into
    `<herdr_env>`), so it stays a warn. It is the same class as W1, and the fix is a guard clause.
  - **Per-module test helpers.** I counted them as the target's established pattern (W7c), not as
    duplication: `lib`, `out`, `err`, `stage`, `teardown`, and the two small `panes.sh` runners.
- **Evidence:**
  - **The diff:** `git diff 6eff9cc 2698960`, all 39 files listed. I read in full `panes.sh`, the
    `herdr.sh` diff, `proof_fakes/holler`, `proof_803.rs`, `herdr_803.rs` and the `proof.rs`, `herdr.rs`
    and `skilltext.rs` diffs. I read the import blocks and helper lists of `panes_803.rs`,
    `panes_803_plan.rs` and `skilltext_803.rs`.
  - **The skill and docs:** `SKILL.md` lines 1-1587 in full. In `docs/setup-wizard.md`, the section "How
    the wizard builds panes" and the user-shell lines. The ADR-0021, `docs/pane-control.md`, README,
    CHANGELOG, `docs/README.md` and two-instances diffs.
  - **The code:** `instance.sh` in full, and the helper lists of every `lib/*.sh`. `tx_apply.rs` lines
    1-260 and `cell_key`. The headers of `profile/apply.rs`, `pane/list.rs`, `pane/get.rs`,
    `profile/list.rs` and `pane/doctor.rs`. `pane/wiring/env.rs` lines 53-95 and 160-200.
  - **Grep and size checks:** grep checks for bare `holler pane|profile` in code blocks, and for duplicated
    helpers across `tests/wizard/*.rs` and `lib/*.sh`. `wc -l` against `main` for `proof.rs` (825 → 825),
    `herdr.rs` (874 → 751) and `skilltext.rs` (656 → 636).

## S, spec audit: 2026-10-12T10:37-06:00 (MDT)

- **Decided:** REWORK (`reworkKind: production`), four items (`docs/handoffs/803/handoff-S.md`).
  - **Coverage.** Every Acceptance line 1 to 24 has a proving test. A and T preconditions are met. D1 to D22 are built as
    decided, with A's warns and F's declared deviations.
  - **Item 1.** Stage 11's note (`SKILL.md:1510-1513`) and the docs (`setup-wizard.md:645-647`) give the user
    `holler pane close` / `relaunch` with only `HOLLER_STATE_DIR`, or with no environment.
    - A pane verb keeps its Herdr configuration error until Herdr's first use (`wiring.rs:148-162`).
    - `close_live` stops the host's processes first (`close.rs:165-186`).
    - `stop_owned` signals the owned processes of the same-named session on tmux's default server.
    - So, for a named instance, the call can stop another instance's `r1c1` before it fails with `usage`.
  - **Item 2.** `panes.sh` parses a multi-line `layout`, which the unchanged `instance.sh:69-81` reads as a table header.
    A `workspace` after it is dropped:
    - `check` exits 0 with `workspace holler, profile holler`;
    - `workspace = "a=b"` passes too, which bypasses Acceptance 6.
  - **Item 3.** `holler pane reset PANE` on the orchestrator is refused `orchestrator-pane` without `--as-operator`
    (`tx_switch.rs:345-350`). See `SKILL.md:1466-1468` and `setup-wizard.md:282-284`.
  - **Item 4.** `setup-wizard.md:159-161` restates the creation order, against Amendment (d).
- **Assumed:**
  - T's and F's recorded runs stand. I re-ran no build or test.
  - My checks were read-only:
    - `git fetch`, `diff`, `show`, `log`, `diff --check`; `grep` and `wc -l`;
    - `gh issue view` of 803, 839, 842, 850, 829 and 848, and `gh pr list`;
    - `panes.sh` and `instance.sh` on scratch configs (they only read a file and print).
  - The script commits this handoff and this entry at the phase commit.
- **Hedged:**
  - **Item 1's severity.** A-dup kept it a warn, on the reading that the call "touches nothing". The wiring and
    `close_live` show the host acts before the Herdr error surfaces. For a named instance it is the cross-instance case
    that D8 and W1b exist to refuse, so it is REWORK.
  - **Item 2 needs one test.** F changes `panes.sh`. T adds the `panes_803` case, which is a mixed REWORK, so it is routed
    `production`.
  - **Not REWORK:** the one-sided socket guard, the `[ext.*]` multi-line refusal, the fake's missing exit-1 path, the
    `--spec-only` offer for a live pane, and the default instance's tmux environment. All are advisory in handoff-S.
- **Evidence:**
  - **Live:** `origin/main` is `6eff9cc`, which is the merge base. #803 has 4 comments, the last at 04:27 MDT, all covered
    by the brief. #839, #842, #850, #829 and #848 are open, so the REFRESH rows stand.
  - **Read:**
    - the brief's decision, Scope, Amendments, Acceptance, Blast radius, Depends on, Clashes, Rules and U1 to U8 sections;
    - every handoff, `decisions.md` and `evidence.md`;
    - `panes.sh`, the `herdr.sh` diff and file, `SKILL.md` in full, and `docs/setup-wizard.md` in full;
    - the README, ADR-0021, `pane-control.md`, `docs/README.md`, two-instances and CHANGELOG diffs;
    - every new test module, and the diffs of `herdr.rs`, `skilltext.rs`, `skilltext_772a/b.rs`, `instance.rs`, `main.rs`,
      `skilltext_838.rs` and `skilltext_w5_stage0.rs`;
    - `proof_fakes/holler`.
  - **Code checked:**
    - `pane/reset.rs`, `pane/switch.rs`, `tx_switch.rs`, `pane/relaunch.rs` (no orchestrator guard), `pane/close.rs`,
      `pane/doctor.rs`, `profile/apply.rs`, `profile/show.rs` and `profile/create.rs`;
    - `tx_apply.rs` (actions, outcomes, `dry`), `pane/wiring.rs`, `wiring/env.rs` and `reconcile/drift.rs`;
    - in `holler-adapter-host`: `lib.rs` (`stop_owned`) and `tmux.rs`.
  - **Diff checks:**
    - `proof.rs` with `pub(super) ` stripped equals `main` (825 lines);
    - privacy grep of all 8625 added lines: clean;
    - `diff --check`: whitespace only inside the brief's verbatim evidence.

## F, rework of S's REWORK (Phase 6, rework 1): 2026-10-12T10:54-06:00 (MDT)

- **Decided:**
  - **Item 1.** The user's route to a pane verb is the door. Stage 10's report prints the instance's `<herdr_env>`
    and the absolute `lib` path, with every value written out. Stage 11's note and the docs' "Basic Herdr commands"
    give `<herdr_env> bash <lib>/herdr.sh holler pane close NAME --profile <profile>` and `... pane relaunch NAME`,
    and say what a bare pane verb does. The two "own shells" sentences (Stage 1, "Running an instance beside
    another") now say a pane or profile call needs the door. Printing the five `HOLLER_*` values instead was
    rejected: a second spelling of the door's literals that skips its scrub (D8).
  - **Item 2, S's option (a).** `panes.sh` refuses a `layout` left open on its own line (exit 1, naming `layout`,
    "on one line"), skipping its continuation lines so the refusal is one message. (b) was rejected: it patches
    one key and keeps a parser that differs from `instance.sh`'s. The header, the two `LAYOUT` comments, Stage 1's
    refusal list and the CHANGELOG's Breaking Changes item state the rule.
  - **Item 3.** Stage 10 and the docs run `holler pane reset PANE --as-operator` through the door, with the user's
    yes, and say what the reset leaves: an OpenCode orchestrator's new session of record (derive Stage 8's body
    config again and restart the body), the old session as `stray-session` (Stage 9's gate shows it; no exception
    added, D14), and nothing of either for a Claude Code orchestrator.
  - **Item 4.** The docs drop the restated creation order; the section 6 pointer names it.
  - **Also taken: A-dup warn 4.** `noslash` is renamed `strip_slash`, byte-identical to `instance.sh:43-44`, and
    listed in the header.
  - **Not taken: S's advisories 1 to 6** (the one-sided socket guard needs `herdr.sh` and two T cases; the others
    are test-only, observations, or fail closed). Recorded in handoff-F-rework.md.
- **Assumed:**
  - No user relies on a multi-line `layout`: no doc example, fixture or test writes one (grep of every tracked
    file outside `docs/handoffs/`), and the wizard writes the config itself in Stage 1.
  - The user's shell does not export `WIZARD_*_SOCKET`, so the door line printed for one instance carries only
    that instance's socket (S's advisory 1 stays open).
- **Hedged:**
  - Item 1's description of a bare close is read from code (`wiring.rs`, `env.rs`, `close.rs`), not run: no test here
    starts a real tmux or Herdr.
  - The `stray-session` that a reset leaves will stop a later Stage 9 run as a Gate; the text tells the agent what
    it is, rather than changing D14's gate.
  - **A flaky `holler-hub` test, not this change.** `profile_rename_faults_test`'s inode assertion (`:250-254`)
    fails when ext4 hands the freed inode straight back. This session's `TMPDIR` is ext4: the test fails 10 times of
    10 run alone, failed in my first full run, and passed in the second. The hub crate is identical to `origin/main`,
    and `main`'s CI at `6eff9cc` is green. A follow-up for #665's test is the orchestrator's call; not filed.
- **Evidence:** `wizard_scripts` 303 passed (also with mawk and busybox awk first on `PATH`); `docs_cli_test` 3 and
  `docs_pane_control_test` 19 passed; the CI workspace command with `--no-fail-fast`: 167 binaries, 2624 passed,
  0 failed (handoff-F-rework.md); `lint.sh` and `changelog-check.sh` pass; `bash -n` and `shellcheck` clean; S's
  repro and four variants run by hand under gawk, mawk and busybox awk; seven evidence.md entries for the unchanged
  code relied on.

## T, verify / GREEN after F's rework (Phase 7, rework 1): 2026-10-12T11:17-06:00 (MDT)

- **Decided:**
  - Added S item 2's test, `panes_803::a_layout_over_several_lines_is_refused_so_a_later_workspace_is_never_dropped`.
    It asserts `layout` and "one line" (F's option (a)), not `workspace`. It also asserts that `plan` exits 1 with an
    empty stdout, so no step is planned under a label the user never named. Its positive control is the same panes
    with a one-line `layout`.
  - Added three text pins for S's items 1, 3 and 4, which no test covered:
    - the door verbs, and no bare `holler pane VERB`, in Stage 11's first note and the docs' basics;
    - `--as-operator` on every `pane reset PANE`, plus `stray-session`;
    - no restated creation order in the docs, and the pointer to section 6.
  - The bare-verb scan is a whole-text rule over the two user-facing notes, not a single-phrase check. That way a
    reworded bare verb is still caught.
- **Assumed:** the agent-run mentions of `holler pane ...` elsewhere in the skill (Stage 3, "If a stage fails") are
  the agent's calls through the door, as the stage text says. So the bare-verb rule is scoped to the text the user
  runs from their own shell, not the whole skill.
- **Hedged:** the flaky `profile_rename_faults_test` (ext4 inode reuse) passed in my one full run. It is unrelated to
  #803, but a later run here may hit it.
- **Evidence:**
  - `wizard_scripts`: 307 passed under gawk, mawk and busybox awk.
  - The CI command with `--no-fail-fast`: 167 binaries, 2628 passed, 0 failed.
  - Clippy, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test`, `docs_pane_control_test` and
    `wire_selftest` pass.
  - Each new test fails against the pre-rework file from `7a9dbec`, plus one bare-verb mutation (handoff-T-green.md,
    rework 1).

## A, anti-duplication gate (Phase 8, rework 1): 2026-10-12T11:23-06:00 (MDT)

- **Decided:**
  - **PASS, with 0 blocks and 2 warns** (handoff-A-dup.md, section "rework 1").
  - **The rework extends what exists:**
    - `panes.sh` refuses a multi-line `layout` inside its own `layout` handling, and reads `workspace` from nowhere new
      (W6).
    - The user's pane verbs go through the unchanged door, `herdr.sh holler`, with no new script, verb or variable.
    - `strip_slash` is byte-identical to `instance.sh`'s and listed in the header, which closes my warn 4.
    - The new tests reuse `panes_803`'s and `skilltext.rs`'s helpers.
  - **The warns:**
    - **(1)** The skill and docs describe a bare close's CLI ordering in three places. The root is the CLI's deferred
      configuration errors (`wiring.rs:148-160`, `close.rs:181-183`). That is a follow-up for O, not this story.
    - **(2)** The door's one-sided socket guard (my first pass's warn 2) is carried forward. The user's own shell is now
      a caller of the door.
- **Corrected:** my first pass assumed that a bare `holler pane close` with only `HOLLER_STATE_DIR` "stops with `usage`
  and touches nothing". S showed that was wrong, and this pass confirmed it in the code:
  - configuration errors wait for their port's first use;
  - `close_live` stops the host's processes before its first Herdr call.
- **Assumed:** the recorded runs in F's and T's handoffs stand. I re-ran no build or test.
- **Hedged:**
  - **Finding 1 is a warn, not a block.** It is not a parallel path, the story cannot change Rust, and the text is right
    for the CLI as it is.
  - **Finding 2 stays a warn.** A leak needs an exported `WIZARD_*_SOCKET` of the other instance kind.
- **Evidence:**
  - **The diff:** `git diff f2ecfae..HEAD`, all 11 files, read in full. The diff of the frozen paths is empty (Rust
    `src`, `holler-proto`, ADRs, `docs/protocol`, the six other `lib/*.sh`, `proof.rs`, `proof_fakes/`).
  - **The handoffs:** `handoff-S.md`, `handoff-F-rework.md`, `handoff-T-green.md` (rework 1), and `decisions.md` from
    my first entry on.
  - **The scripts:**
    - `herdr.sh`: the header, `base_env`, `holler_guard` and the `holler` verb;
    - `panes.sh`: the layout state (196-232, 362-366, 448-480) and `env` (495-525);
    - `instance.sh` 25-50.
  - **The tests:** `skilltext.rs` 1-185, `skilltext_803.rs` 1-175 and the new tests, and the helper list of
    `panes_803.rs`.
  - **The CLI and its docs:** `wiring.rs` 135-175, `wiring/env.rs` 60-100, `close.rs` 155-195, `reset.rs`'s flags,
    ADR-0021 344-370 and `pane-control.md` 118-134.
  - **Checks:** a byte compare of the copied helpers across `lib/*.sh`, a grep for bare `holler pane|profile` mentions
    in the skill, the docs and the README's wizard section, `wc -l`, and `git diff --check`.
