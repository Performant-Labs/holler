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
