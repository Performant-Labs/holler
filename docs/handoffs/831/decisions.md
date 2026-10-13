# Decision journal: #831 adopt, adopting the fleet's existing Claude Code panes

Run handoff directory `docs/handoffs/831` - branch `issue-831-implementation` - rigor in-session - uiSurface false. Append-only.

## A, up-front plan review: 2026-10-12

- **Decided:** BLOCK, 3 `block` and 7 `warn` (`docs/handoffs/831/handoff-A.md`). The plan extends the right objects and keeps the choke point.
  1. AC4 copies `pane.env`'s OpenCode values into a Claude record: `OC_DEFAULT_AGENT` makes the later `relaunch` refuse (`agent-unsupported`), and `OC_MODEL`/`OC_EFFORT` go to `claude --model --effort`.
  2. tmux has no foreground-process-group format, so `foreground_pids` needs `#{pane_tty}` plus a `ps` call: a new subprocess seam the brief does not name.
  3. Reusing the undo for an extension would tell the operator to `profile delete fleet --keep-panes`, which deletes the pre-existing profile.
- **Not blocking:** optional launcher port (where the check moves), the `PaneId` that `shown_session` needs, `Plan` carrying create-or-extend, final finding codes, a differing existing spec, `ports.rs` near 800 lines, call count per pane.
- **Read:** the brief; `import/{observe,assess,files,findings}.rs`, `import.rs`; `writes.rs`, `launcher.rs`, `pane_env.rs`; `ports.rs`, `pane.rs`, `profile_snapshot.rs`; the Claude and host adapters; `lint.sh`; ADR-0021 lines cited by the brief.

## A, up-front plan review (second pass, amended brief): 2026-10-12

- **Decided:** BLOCK, 1 `block` and 6 `warn` (`docs/handoffs/831/handoff-A.md`). The amendment resolves the first pass's 3 blocks (D3 model/effort/agent rule; the `ps -t` mechanism of `foreground_pids`; a record-delete-only undo for an extension). The 10 `impl HostPort for` sites and the cited lines match the tree.
  1. "Launcher" has `assess` emit `source-missing` for a port-less entry "or a Claude pane with the flag off". Today a flag-off Claude pane is `Outcome::claude_code(name)`, which drops `files.problems`, so it is `skipped`; the sentence would make the operator's existing import `import-blocked`, against AC3.
- **Not blocking:** `Target.adopt` cannot encode off / on-labelled / on-unlabelled; the stale "until #831 lands" message and help; the undo's failed-record check would delete a concurrent `fleet` member in an extension; `fake_tmux_test.rs` at 822 lines; the OpenCode helpers the Claude record must skip; partial `ps` answers.
- **Read:** the brief; `import/{observe,assess,files,findings}.rs`, `import.rs`; `writes.rs`, `launcher.rs`, `sources.rs`; `ports.rs`, `ports/router.rs`, `profile_snapshot.rs`; the host and Claude adapters; `lint.sh`; ADR-0021 lines cited by the brief.

## A, up-front plan review (third pass, brief amended twice): 2026-10-12

- **Decided:** PASS, 0 `block` and 5 `warn` (`docs/handoffs/831/handoff-A.md`). The second pass's block is resolved: the port-less `source-missing` is emitted on the OpenCode path only, a flag-off Claude pane keeps `Outcome::claude_code`, and a named test pins it. The second pass's six warns are folded in. The cited tree (12 `HostPort for` hits, `checked`, `write`/`undo`, store behaviour, file sizes) matches.
  1. Warns for F and Phase 7: a second `Pane { .. }` literal and a copied `assess` tail in `adopt.rs` would be a parallel path; share `record()` and the join tail.
  2. Port 0 and a relative cwd are `source-unreadable` in the tree, not `source-missing`; T pins the real codes.
  3. Decide the test file split (`adopt.rs`, `adopt_undo.rs`) up front.
- **Read:** the brief; `import/{assess,observe,files}.rs`, `import.rs`; `writes.rs`, `launcher.rs`; the testkit and hub pane stores; `profile_diff.rs`; `lint.sh`; ADR-0021 lines 110, 1577, 1743.

## T, Phase 4 author / RED: 2026-10-12

- **Decided:** RED is valid (`docs/handoffs/831/handoff-T-red.md`). 49 verb cases in `pane_verbs/import/{adopt,adopt_problems,adopt_fleet,adopt_undo,harness}.rs` (46 RED, 3 green-before by the brief's design), 19 adapter cases for `foreground_pids` against a fake `tmux` and a fake `ps`, one `#[ignore]` real-tmux case, a twelfth host conformance case with four mutants, the port's wrapper forwarding, the five finding codes and two docs tests. The bare tree fails to compile on one cause, `HostPort::foreground_pids`; with the contract temporarily in, every new case fails on the assertion it pins.
  1. Four flag-on files instead of A's two, to stay under the 600-line warning.
  2. `fake_tmux_test.rs` gets the one arm its exhaustive `match` on `HostOp` needs (the brief's "no edit" cannot hold); `METHODS` stays six.
  3. The third `compile_fail` doctest is F's, with the trait edit.
- **Assumed:** the message words listed under "Contract F implements" in the handoff (the call names in `harness-unreadable`, the `harness-claude-code` phrases, "nothing was kept" / "was not changed", `adopted` in the summary line) are the nearest reading of the brief; none is a word the operator gave. A shell is in the background of a typed-in program's tty only with job control, so the real-tmux case asserts the typed pid, not the shell's absence.
- **Hedged:** the prototype of `TmuxHost::foreground_pids` that proved the adapter tests and D2 on a real tmux 3.7c was reverted, not handed over; F writes the adapter. The CLI cases were not run against a full prototype of the adoption (that is F's work), so a case may still carry a wrong expectation that only a correct implementation shows; T repairs such a case in Phase 6 rather than F bending the code to it.
- **Evidence:** `cargo build --workspace --all-targets --keep-going` (E0407 x2, E0599 x2, E0282); with the temporary contract: `cargo test --workspace --no-fail-fast` fails exactly `pane_verbs` (46), `docs_pane_control_test` (2), `import_test` (2) and, unrelated, one load flake of `holler-body --test rebind_test` that passes on re-run; `cargo clippy -p holler-pane -p holler-pane-testkit -p holler-adapter-host --all-targets` and the `pane_verbs`/`docs_pane_control_test`/`pane_integration`/`pane_claude` targets with `-D warnings`: clean; `bash scripts/lint.sh`: exit 0; `foreground_pids_test` + `foreground_pids_bound_test` 15 of 15 runs green; real tmux: `the_tmux_host_passes_the_host_conformance_suite` and the typed-in case 5 of 5 green.
- **Read:** the brief; `handoff-A.md`; `import/{observe,assess,files,findings}.rs`, `import.rs`; the CLI `import.rs`, `import/{writes,launcher}.rs`; `ports.rs`, `ports/router.rs`; the host adapter (`lib.rs`, `exec.rs`, `tmux.rs`); the test kit (`host.rs`, `claude.rs`, `harnesses.rs`, `pane_store.rs`, `profile_store.rs`, the host conformance suite); the import rig and every `import/*.rs` test; `docs_pane_control_test.rs`; ADR-0021 and the guide at the lines the brief cites.

## F, Phase 5 implement: 2026-10-12

- **Decided:** implemented against T's RED with no test edited (`docs/handoffs/831/handoff-F.md`). `HostPort::foreground_pids` and its 10 sites; `TmuxHost` reads `#{pane_tty}` then one `ps -t` per live pane under one deadline; the import's adopt chain in a new `import/adopt.rs`, sharing one `record()` and the join's `begin`/`finish` with the OpenCode path (A's warns 1 and 2); `Plan.fleet: FleetPlan`; the undo of `writes.rs` generalised to `Write::{Record, Fleet}` with the planned-record equality check; the ADR and guide amended.
  1. `FleetPlan::Extend` keeps `generation_read` as the brief names it, though it equals the read profile's generation.
  2. The two new ADR decisions are listed after decision 49 under a sentence that each takes the next free number at merge.
  3. `HarnessPart.keeps_agent` (a bool) instead of the caller passing the agent, so `record()` alone checks `pane.env` was read.
- **Assumed:** the summary wording (`(N adopted)`, `extended profile ... N specs added, M total`) and the finding messages are F's; only the phrases T pinned are fixed by tests.
- **Hedged:** `cargo fmt -p holler-cli` reformats ~100 untouched files under the local rustfmt; they were reverted and only the touched files are formatted. The undo's equality check is proven against the testkit store only.
- **Evidence:** `cargo test --workspace --no-fail-fast` (CI skip) green after one docs fix; `pane_verbs` 455, `docs_pane_control_test` 23, adapter `foreground_pids` 13 + 6, doctests 12; real tmux `--ignored` 10 of 10; clippy `-D warnings` clean on the touched crates' lib and relevant test targets; `scripts/lint.sh` exit 0; `docs/handoffs/831/evidence.md`.
- **Read:** the brief, `handoff-A.md`, `handoff-T-red.md`; every test file T wrote; `import/{observe,assess,files,findings}.rs`, `import.rs`; the CLI `import.rs`, `writes.rs`, `launcher.rs`, `sources.rs`; the host adapter; `ports.rs`; `pane.rs`; `profile_snapshot.rs`; the testkit `host.rs`, `claude.rs`, `harnesses.rs`; ADR-0021 and the guide at the cited lines.
