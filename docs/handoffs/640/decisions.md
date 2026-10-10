The decision journal of #640 part 3 of 3. It replaces part 2's journal, as the brief's Handoffs line says; part 2's is
in git at `0ad2d8a:docs/handoffs/640/decisions.md`.

## A (Phase 3, up-front plan review) — 2026-10-09T20:29:58-06:00
- **Decided:** PASS, with 0 block and 6 warn findings (`docs/handoffs/640/handoff-A.md`). The plan extends the objects
  its Reuse map names: `protocol::excerpt` made `pub(crate)` (one copy), `Tapped::Fail` on the one interceptor,
  `run_herdr_conformance` run rather than copied, and `Timeout.op` renamed once at the port boundary with the transport
  untouched. That is the OpenCode adapter's `Call` shape. It adds no dependency and puts every rule it sets into ADR-0021
  in the same change. Warns:
  - (1) `origin/main` moved to `e327569` (#641, 8:06 PM MDT) after the brief's baseline `dc300ab`.
  - (2) #641's merged `real_tmux_test.rs` uses plain `#[ignore]` and skips when the tool is missing. This plan adds
    `HOLLER_HERDR_SCRATCH=1` and fails when the tool is missing. The divergence is reasoned but not named in the brief or
    in `docs/testing.md`.
  - (3) The Reuse map does not name the helpers the harness re-creates: `holler-cli/tests/support` `wait_for`, `Hub` and
    `StateDir`, the host and OpenCode `exec::run`, and #696. None is reachable from this crate. The harness should hold
    one bounded runner and one poll helper.
  - (4) D4 and A4 state the `op` rule with no room for AC 16's `herdr.connect`. A4's example `harness.health` is never
    returned by the real adapter, and §2 is the better home for the rule.
  - (5) AC 19's grep also matches `adapter.rs:338`, a returned label, not a message.
  - (6) Three things stay stale: the `snapshot` doc versus A9, A7's new PROPOSED item missing from the Deferred list, and
    the test kit's `ASSUMPTION (#640)` comments, whose follow-up is unfiled.
- **Assumed:**
  - The Reuse map embedded in the brief is the survey, since this run has no `survey.md`.
  - On the automated path the run continues to T on PASS without a brief amendment, so the warns are advisory.
  - I started a fresh journal because the brief says part 3's handoffs overwrite part 2's and the conventions keep one
    journal per run.
  - #641's test, merged after the brief was written, is the one sibling opt-in convention on `main`. No other merged
    real-instance test exists.
- **Hedged:**
  - Finding 2 sits between warn and block. One merged instance is not a dominant pattern, and Decision 6 gives a written
    reason to differ that fits Herdr's risk (a socket resolved from `HOME`, `XDG_*` and `HERDR_*`), so I kept it at warn.
  - Finding 3 would become a Phase 7 rejection under this repo's overlay ("a new near-copy ... is a rejection") if its
    justification were not written. This handoff now holds that justification, which assumes Phase 7 reads it.
  - Finding 4 ships slightly wrong contract and ADR text unless the brief is reopened. F may not change D4's meaning
    or touch other ADR lines.
- **Evidence:**
  - Read: the brief in full, the three brief-gate results (r1 and r2 BLOCK, r3 PASS), and part 2's `handoff-A.md`,
    `handoff-A-dup.md` and `decisions.md`.
  - Read in full: `crates/holler-adapter-herdr/src/{adapter,protocol,transport,lib}.rs`, `tests/wire_herdr/{mod,serve}.rs`,
    `tests/{adapter,adapter_conformance}_test.rs`, `tests/common/mod.rs` and `Cargo.toml`;
    `crates/holler-pane-testkit/src/herdr.rs` and `conformance/herdr.rs`; `crates/holler-adapter-opencode/src/{lib,exec}.rs`
    and `tests/support/stub.rs`.
  - Read in part: `crates/holler-pane/src/{ports,error,pane,reconcile,probe}.rs`, `reconcile/observe.rs:155-185`,
    `holler-cli/tests/support/mod.rs:1-260`, and `holler-adapter-opencode/tests/hermetic_test.rs:1-140`;
    `docs/adr/ADR-0021.md` §1-§2, §5, §9-§12 and Consequences through Decisions taken; `docs/testing.md` from "Beyond
    loopback" on; `CHANGELOG.md`; `scripts/lint.sh`, `clippy.toml` and the workspace lints.
  - Other branches and issues: `origin/main` at `e327569`, namely `crates/holler-adapter-host/tests/real_tmux_test.rs`,
    `src/exec.rs` and `src/lib.rs:134-137`; `origin/issue-644-implementation:docs/handoffs/644-brief.md`; and
    `gh issue view 633, 641, 649, 663, 667, 696`.
  - Greps: E-5's and E-12's baselines, AC 8, 9, 19, 21, 24 and 25 at the brief gate, `Timeout {` constructions, uses of
    `op` (`detail()`, `Display`), `#[ignore` across `crates/`, `HOLLER_*` variable names, `Command::new`, `try_wait` and
    `wait_for` sites, and matches on `Tapped`.

## T (Phase 4, author / RED) — 2026-10-09T22:59:00-06:00
- **Decided:**
  - RED is valid (`docs/handoffs/640/handoff-T-red.md`). AC 16 fails on all 14 rows with the wire `op`; the four AC 18
    tests fail quoting the whole `LONG`, each in the arm it names. AC 1-7 and AC 17 pass at RED by design (test code, and
    a pass-through guard for F's rename). Part 1 and part 2 targets stay green.
  - AC 9-11 run and pass. AC 12-13 run opt-in against the merged adapter and real `herdr 0.9.1-preview.2026-09-21-0ff0f27e2226`:
    both pass, four runs in a row; AC 14's cleanup checks print nothing. No real-Herdr gap, so AC 15 adds no test and the
    wire fake is unchanged except `Tapped::Fail`.
  - The scratch base limit is 29 bytes, not the spike's 40 (Decision 2). `SOCKET_PATH_LIMIT` (100) minus the fixed tail
    `/h640.XXXXXX/home/.config/herdr/sessions/holler640-XXXXXXXX/herdr.sock` (70) leaves 29 for the base. A 40-byte base
    would give a 110-byte socket that `check_socket` refuses. Same rule as Decision 2's intent (fall back to `/tmp`),
    stricter number, derived in code as `BASE_LIMIT`.
  - Applied A's finding 3: one bounded runner and one bounded poll helper in the harness. `poll` is `pub` so the test
    file's read polls reuse it rather than adding a second loop.
  - AC 16 takes its expected strings from `HerdrOp::as_str`, so the adapter's ops are pinned equal to the test kit's
    rather than to a second spelled copy.
- **Assumed:**
  - This T run is a restart; an earlier T attempt had written the same four files, uncommitted. I re-read them in full
    against the brief, kept them, and re-ran every check above myself; nothing below rests on the earlier attempt's output.
  - `herdr --version` with an empty environment plus `PATH` and a throwaway `HOME`/`XDG_*` touches no session, so it is
    within Decision 7's limits for AC 14's version line.
- **Hedged:**
  - The pane pass takes about 66 ms including server start; fast, but every step asserts its outcome (the `printf`
    output line, the typed text, the snapshot ids, the two `pane-not-found`s), so a pass cannot be vacuous.
  - Real Herdr's behaviour is checked only on Linux; macOS stays unchecked (spike §15), as the brief's Risks say.
- **Evidence:**
  - `CARGO_BUILD_JOBS=4 cargo test -p holler-adapter-herdr --no-fail-fast` (RED output in the handoff).
  - `cargo test ... --test scratch_herdr_test -- --list --ignored`; `env -i <binary> --ignored --nocapture`;
    `env -i HOLLER_HERDR_SCRATCH=1 <binary> --ignored`; `HOLLER_HERDR_SCRATCH=1 cargo test ... -- --ignored --test-threads=1`
    and three repeats; `pgrep -af 'h640\.'`, `pgrep -a herdr`, `find /tmp -maxdepth 1 -name 'h640.*'`.
  - The doc-AC greps 19, 21-26; `cargo clippy -p holler-adapter-herdr --all-targets -- -D warnings`;
    `cargo fmt --check -p holler-adapter-herdr`; `bash scripts/lint.sh` (exit 0); the AC 8 and AC 29 greps.

## F (Phase 5, implementation; the Workflow script's phase 6) — 2026-10-09T23:16:47-06:00
- **Decided:**
  - The `op` rule (Decision 9) is applied at the call boundary. A new private `HerdrAdapter::run_as(op, |deadline| ..)`
    takes the call's one deadline and renames any `Timeout` from its body to the call's `op`. All seven port methods
    and `connect_with` (`herdr.connect`) run through it, so no `Timeout` can escape unrenamed and the transport stays
    untouched (AC 20). This is the brief's own example, and it changes no helper signature. I rejected the siblings'
    `Call { op, deadline }` struct: in this adapter the transport, not the adapter, makes the timeout.
  - `ensure_pane`'s body moved unchanged into a private `ensure`, because rustfmt re-wrapped it badly inside the
    closure.
  - The four Herdr-sent quotes go through `protocol::excerpt`, now `pub(crate)` (Decision 10). The caller's values keep
    `{:?}`, and `snapshot`'s returned label (`adapter.rs:383`) stays whole (A finding 5).
  - D1-D6 and A1-A9 are written as the brief words them, reflowed only. D4 and A4 keep no constructor exception, and
    A4 keeps `harness.health` (A finding 4). The rule's subject is a port method, `connect` is not one, and Decision 11
    and AC 25 bar changing meaning or other lines. The `herdr.connect` case is stated in the adapter's own docs.
  - `docs/testing.md` gets Decision 13's section plus one sentence on the host adapter's tmux convention (A finding 2).
    `CHANGELOG.md` gets Decision 14's entry right after part 2's.
  - Part 2's `evidence.md` is replaced with part 3's twelve facts. Its AC numbers would have meant other criteria here.
- **Assumed:**
  - "Perform Phase 6's work" is F's phase: the script numbers F as phase 6 (`coding-pipeline.workflow.mjs`, "Phase 6
    (F) has no gate of its own").
  - The brief's Handoffs rule (part 3's files replace part 2's) covers `evidence.md` too, though it names only
    `handoff-<phase>.md` and `decisions.md`.
  - Citing `crates/holler-adapter-host/tests/real_tmux_test.rs` in `docs/testing.md` is fine although this branch's
    base (`dc300ab`) lacks it, because the PR merges into `main`, which has it (`e327569`).
- **Hedged:**
  - A finding 4: keeping D4 and A4 verbatim leaves a reader to infer that a constructor is outside the "port method"
    rule. If the operator or S wants it explicit, it is one clause in each.
  - Not done, and left to the operator: D7 for the `snapshot` port doc (A 6a, which would pre-empt A9's contract
    amendment), a Deferred-list line for A7's PROPOSED owner (A 6b; AC 25 allows none), and filing the #638 follow-up
    (A 6c; outward-facing).
  - F did not merge `origin/main` (`d9eabbb`), since that would be a commit. A scratch three-way merge of every file F
    edited is clean, but CI on the PR's merge ref is the real check.
- **Evidence:**
  - Read: the brief (all of it), `handoff-A.md`, `handoff-T-red.md`, this journal, T's `adapter_messages_test.rs`,
    `scratch_herdr/mod.rs` and `scratch_herdr_test.rs` (the ignored tests), and every file edited. Also
    `holler-adapter-opencode/src/lib.rs` (`Call`, `OP_*`), `origin/main:crates/holler-adapter-host/src/lib.rs`
    (`OP_*`, `Call`) and `origin/main:.../real_tmux_test.rs`, `holler-pane-testkit/src/{herdr,harness}.rs`
    (`HerdrOp`, `HarnessOp::Health`), `ci.yml`, `scripts/lint.sh` and `scripts/changelog-check.sh`.
  - Ran, all with `CARGO_BUILD_JOBS=4`, none with the gate variable: `cargo test -p holler-adapter-herdr --no-fail-fast`
    (all green; `adapter_messages_test` 6/6); `cargo test -p holler-pane` (86 passed);
    `cargo test -p holler-cli --test docs_cli_test` (3 passed); `cargo clippy -p holler-adapter-herdr --all-targets` and
    `cargo clippy --workspace --all-targets -- -D warnings` (both clean); `cargo fmt --check -p holler-adapter-herdr
    -p holler-pane`; `cargo doc` with `-D warnings`; `bash scripts/lint.sh`; `bash scripts/changelog-check.sh`;
    `cargo machete`; `cargo tree -p holler-adapter-herdr -e normal --depth 1`; the AC 8, 9, 19-27, 29 and 30 greps;
    and `git merge-file` against `origin/main` for the nine edited files.

## T (Phase 7, verify / GREEN) — 2026-10-09T23:25:00-06:00

- **Decided:**
  - GREEN is valid with no test change: F flagged no test as wrong, changed no test file, and T's read of the diff found
    nothing to repair. `handoff-T-green.md` replaces part 2's.
  - The Test plan's two mutants were run and reverted: (a) `EXCERPT_LIMIT` 65 fails exactly the four AC 18 tests;
    (b) the `op` rename removed fails exactly AC 16. An extra mutant (rename every error) fails AC 17 and AC 18, so
    AC 17's guard is not vacuous.
  - AC 14's version line is taken from the scratch server's own `version()` reply, not a hand-run `herdr --version`,
    because the brief's Operating rules let only the harness run `herdr`.
- **Assumed:**
  - AC 25's "no other line of the ADR changes" is S's read; T ran only its greps and the diff stat (+18 / -11).
  - `pgrep -af 'h640[.]'` is the faithful form of AC 14's `pgrep -f h640\.`: the bracket keeps the check from matching
    its own shell, which made the unbracketed form print the checking shell at RED and here.
- **Hedged:**
  - The opt-in tests are not in CI; their evidence is four local runs on this machine against Herdr
    `0.9.1-preview.2026-09-21-0ff0f27e2226`.
- **Evidence:**
  - Ran, all with `CARGO_BUILD_JOBS=4`: `cargo test -p holler-adapter-herdr --no-fail-fast` (all green, 2 ignored);
    the two mutants and the extra one; AC 9-11 (`--list --ignored`, `env -i` without and with the gate); AC 12-14
    opt-in four times (all `ok`, no `h640.` process or directory left); `cargo test --workspace --no-fail-fast`
    (exit 0, 1519 passed); `cargo test -p holler-pane` (86); `docs_cli_test` (3); `wire_selftest` (3); clippy
    workspace `-D warnings`; `cargo fmt --check`; `lint.sh`; `changelog-check.sh`; `cargo machete`; `cargo doc`
    `-D warnings`; `git merge-tree` against `origin/main` `d9eabbb` (clean); the AC 8, 19-27, 29-31 greps.
