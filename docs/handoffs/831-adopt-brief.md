# Brief: #831 adopt (adopt the fleet's existing Claude Code panes)

Repo: Performant-Labs/holler. Issue: #831 (reopened; PR #857 built the harness, this is the step it left). Rigor: in-session. UI surface: no. Kind: feature.
Branch `issue-831-implementation` off `origin/main` (ef881fc at writing). Depends on #835 (merged as PR 860): its `harness-claude-code` classification and the refused flag are on `main`.

## Decided (operator, 2026-10-12)

No item of this brief awaits an operator decision. The operator answered both open points; everything else follows ADR-0021 section 15 item 17 and decision 23 ("a re-run of the import with `--adopt-claude-code`, no new verb").

- **D1 = A.** With `--adopt-claude-code` and an existing `fleet`, the run adds every pane it records (Claude Code panes and any new OpenCode pane) by one compare-and-swap. The flag's meaning grows from "adopt Claude Code panes" to "extend the existing `fleet`"; the ADR and the guide say so. Without the flag a run with panes to record is still `profile-exists`. Rejected: (B) Claude Code panes only, an OpenCode pane stays `profile-exists`; (C) skip each new OpenCode pane as a finding (adds a wire code).
- **D2 = A.** A new required, read-only `HostPort` method, **`foreground_pids(name) -> Result<Vec<u32>, PaneError>`** (working name; the amendment fixes it), answers the pids of the foreground process group of each live pane of the session (pids only, never an argv, I7). Holler tries each against `agents --json` (matched by pid) and takes the one entry; zero or more than one match is a `problem`, never a guess. No `--claude-pid` flag. Rejected: (B) operator-named `--claude-pid PANE=PID`; (C) adopt only a pane whose `pane_pid` is itself `claude`. Why a method: `HostPort::ps` answers tmux's `#{pane_pid}` (`crates/holler-adapter-host/src/tmux.rs:29`, `PS_FORMAT`), the pane's first process, which for the fleet's typed-in `claude` is the shell (the test rig says so: `crates/holler-cli/tests/pane_verbs/import/harness.rs:52`). The opt-in real-tmux test must verify that the foreground group of a typed `claude` includes its pid (the spike note does not settle it).

**The new method is amend-first.** The FIRST commit amends ADR-0021 (section 2 `HostPort` row, line 110, and its prose at line 124; section 15 item 17; a new decision, number next free at merge time) and `docs/pane-control.md` (the port list, section 10) to add `foreground_pids`, before any test or code. Like `foreground_commands` (decision 49 family) it has no default, so every implementer must add it.

**Every place the new method is added** (verified by `grep -rn "HostPort for" crates --include='*.rs'` at ef881fc, which already includes the #846 and #696 merges; re-run it before the first commit):
- the trait: `crates/holler-pane/src/ports.rs:369-` (`pub trait HostPort`, next to `foreground_commands` at 439), plus its two `compile_fail,E0046` doctests (`PidsOnly` ~369, `NoClientCount` ~395): give each the new method except the one it exists to omit, and add a third doctest for `foreground_pids`;
- the host adapter: `crates/holler-adapter-host/src/lib.rs:333` (`impl HostPort for TmuxHost`; next to `foreground_commands` at 416) and `tmux.rs` (the tmux format and parse); tests in `crates/holler-adapter-host/tests/` (beside `foreground_test.rs`);
- the test kit fake: `crates/holler-pane-testkit/src/host.rs:226` (`FakeHost`, beside 258), the host conformance suite `crates/holler-pane-testkit/src/conformance/host.rs` (a case beside `foreground_commands_names_what_run_started`, line 235), `tests/host_conformance_test.rs:101` (`Mutant`), `tests/fake_host_test.rs`;
- the wiring: `crates/holler-cli/src/pane/wiring.rs:355` (`Unwired`), `crates/holler-cli/src/pane/wiring/bind.rs:171` (`BindingHost`) and `:397` (`Wired<dyn HostPort>`);
- the other implementers in tests: `crates/holler-cli/tests/verb_harness/linked_host.rs:51` (`LinkedHost`, the #846 fold, forwards), `crates/holler-cli/tests/pane_verbs/import/harness.rs:307` (`LostChain`), `crates/holler-cli/tests/pane_integration/forwarding.rs:56` (`Counting`; add a forwarding assertion), `crates/holler-pane/tests/ports_test.rs:207` (`TestHost`).
That is 10 `impl HostPort for` sites outside the doctests; the gate is `cargo build --workspace --all-targets` with no E0046.

---


## Problem

`holler pane import` leaves a pane whose tmux session runs Claude Code alone (`harness-claude-code`, #835) and refuses `--adopt-claude-code` with `usage` until this step. Once the main import has created `fleet`, a second real import with a new pane directory is refused `profile-exists`, so nothing adds the existing Claude Code panes afterwards. The operator decided (2026-10-11) that a re-run of the import with the flag does it, with no new verb (ADR-0021 section 15 item 17, decision 23). This brief builds that: the flag adopts those panes into the registry with the Claude Code harness (`kind: claude`), onto a new or an existing `fleet`, acting on nothing live.

## Evidence (verbatim, as of ef881fc)

The flag is refused today, before any file is read or port called:
```
crates/holler-cli/src/pane/import.rs:176-183
    fn read(args: &PaneImport) -> Result<Self, PaneError> {
        if args.adopt_claude_code {
            return Err(usage(&format!(
                "--adopt-claude-code: {}; without it the import leaves a pane that runs Claude \
                 Code alone",
                not_implemented_message(CLAUDE_CODE_STORY)
            )));
        }
```
The plan drops a Claude Code pane before any record is built (the chain stops after `foreground_commands`):
```
crates/holler-pane/src/import/observe.rs:104-107
    let tmux = match read_tmux(ports, target.name) {
        Ok(true) => return Observed::ClaudeCode,
crates/holler-pane/src/import/assess.rs:135
            Observed::ClaudeCode => Outcome::claude_code(&files.name),
```
The record builder is OpenCode-only and needs a launcher entry with a `port`:
```
crates/holler-pane/src/import/assess.rs:408-413
        harness: HarnessInfo {
            kind: HarnessKind::Opencode,
            port: Some(launcher.port),
            pid: None,
            health: seen.health,
        },
crates/holler-cli/src/pane/import/launcher.rs:117-126
        let (port, cwd) = match (raw.port, &raw.cwd) {
            (Some(port), Some(cwd)) => (port, cwd),
            (port, cwd) => {
                ...
                return Err(Finding::about(pane, ImportFinding::SourceMissing, message));
```
The plan builds the profile from the records only, and the write refuses an existing `fleet`:
```
crates/holler-pane/src/import.rs:213
    let profile = (!records.is_empty()).then(|| profile_from_panes(fleet, &records));
crates/holler-cli/src/pane/import/writes.rs:43-53
    let Some(profile) = &plan.profile else {
        return Ok(None);
    };
    let actor = Actor::parse(ACTOR).map_err(body)?;
    if ports.profile_store.get(&profile.name).map_err(body)?.is_some() {
        return Err(body(exists(&profile.name, &plan.records)));
    }
    let stored = insert_profile(ports.profile_store, profile, &actor).map_err(body)?;
```
Its undo (`writes.rs:107-155`) deletes the failed record, each record written (at the generation its write returned) and then the profile.

What the decision already says (the amendment to build on, not to redo):
```
docs/adr/ADR-0021.md:1577-1596   (section 15, item 17; its text quoted in "Decisions already made")
docs/adr/ADR-0021.md:1743-1745   decision 23
docs/adr/ADR-0021.md:876         the `pane import` row of section 9 ("`usage` (`--adopt-claude-code` until #831's adoption step lands ...")
docs/adr/ADR-0021.md:1252-1256   section 13, "The switch" bullet
docs/adr/ADR-0021.md:1674-1676   "Deferred to named stories"
docs/pane-control.md:28-30, 348-355, 393-395, 1010-1014, 1077-1078
```
The seams the adoption reuses (all already on `main`):
```
crates/holler-pane/src/ports.rs:436-439   fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>;   (required, names only)
crates/holler-pane/src/ports.rs:589-592   fn shown_session(&self, pane: &PaneId, at: &HarnessEndpoint) -> Result<Option<String>, PaneError>;
crates/holler-adapter-claude/src/lib.rs:297-300   shown_session: self.entry(pid)?.map(|entry| entry.session)   (None when `agents --json` lists no such pid; matched by pid only)
crates/holler-pane/src/ports/process.rs:54-60   HarnessEndpoint::{Port(u16), Process { pane, pid }}
crates/holler-pane/src/pane.rs:269-279   HarnessInfo { kind, port, pid, health }; a claude harness needs a pid and no port (`endpoint`, pane.rs:315)
crates/holler-pane/src/profile_snapshot.rs:61-73   spec_from_pane: `port_policy_of` gives `none` for a harness with no port, so a claude spec round-trips
crates/holler-pane/src/tx_launch/rules.rs:91-112   refuse_login_env: a Claude spec naming a login-changing variable is refused (`claude-login-env-refused`)
crates/holler-pane/src/profile.rs:340-346   ProfileStore::cas_put(profile, expected_generation, actor)
crates/holler-pane-testkit/src/claude.rs:253   FakeClaude::run_by_hand(name, session) -> pid  (a process started without Holler's settings)
```
The existing tests this extends:
```
crates/holler-cli/tests/pane_verbs/import/harness.rs:188-215   adopt_claude_code_is_usage_naming_831_and_writes_nothing   (REPLACED by this story)
crates/holler-cli/tests/pane_verbs/import/writes.rs:173-208   a_second_real_run_is_a_no_op, an_existing_fleet_with_a_pane_to_record_is_profile_exists   (stay green without the flag)
crates/holler-cli/tests/docs_pane_control_test.rs:534-545   section_3_names_every_flag_of_pane_import; line 515 pins the section-10 vocabulary
```

## Acceptance criteria

1. **Amend first.** ADR-0021 and `docs/pane-control.md` are edited in the first commit, before any test or code: the new `HostPort::foreground_pids` (section 2 table and prose, the guide's port list), item 17 "As built" replaces "not yet", the section 9 `pane import` row, section 13 "The switch" bullet, "Deferred to named stories" and decision 23 are updated; the guide's sections 1, 3, 10 and 12 lose "refused/will be". The new decision numbers (the method; the flag now extending `fleet`) are **next free at merge time** (do not write a number; #850 is amending the same list). `cargo test -p holler-cli --test docs_pane_control_test` passes.
2. `holler pane import ... --adopt-claude-code` no longer answers `usage` naming #831. Test `adopt_claude_code_is_usage_naming_831_and_writes_nothing` is replaced; `grep -rn "not_implemented_message(CLAUDE_CODE_STORY)\|CLAUDE_CODE_STORY" crates/holler-cli/src` prints nothing.
3. **Without the flag nothing changes.** A Claude Code pane is `harness-claude-code`, `skipped`, no record, and no harness call (the existing `harness.rs` tests stay green unedited, except the replaced one).
4. **With the flag, a Claude Code pane gets a record**: `harness.kind = claude`, `port` absent, `pid` = the process `agents --json` lists for that pane (the pid is found through `foreground_pids`), `health = unknown`, `session_of_record` = the session that entry holds (read by pid through `shown_session`, never by directory, and the old `last-session` files are not consulted for it), `command` absent (Holler cannot see its argv), `hold = none`, `profile = fleet`, `model`/`env`/`context` from `pane.env`, `host.cwd` from the launcher entry, whose `port` is optional for a pane adopted this way.
5. **An existing `fleet` is extended**: with the flag, a run that has panes to record (Claude Code or OpenCode, D1 = A) and an existing `fleet` writes the records, then adds their specs to `fleet` by one `ProfileStore::cas_put` at the generation just read (change-log actor `holler pane import`), instead of `profile-exists`. Without the flag the same run is still `profile-exists`, nothing written (`an_existing_fleet_with_a_pane_to_record_is_profile_exists` unchanged).
6. **Idempotent.** A re-run after an adoption finds the pane with a record: `already-imported`, no store write, exit 0, `fleet` generation unchanged (`a_second_real_run_is_a_no_op` extended to a Claude pane).
7. **Acts on nothing live.** Test asserts the calls: reads only (Herdr snapshot and version, `host.ps`, `host.foreground_commands`, the pid read, `harness.shown_session`); no `run`, `stop_owned`, `send_text`, `send_keys`, `ensure_pane`, `send_prompt`, `create_session`. No source file changes.
8. **The report ends with the next step.** Each adopted pane has one info finding naming `holler pane relaunch NAME` (proposed code `claude-code-adopted`; the amendment lists the final code, and the docs-test pins the vocabulary). The text summary line counts them.
9. **Never recorded on a guess.** No pid found, more than one, `agents --json` unreadable, no session held, or a `pane.env` variable that changes Claude Code's login (`ClaudeCode` kind `login_variables()`) is a `problem` with no record, so a real run is `import-blocked` and writes nothing (the pane's name and the variable's name only, never a value, I7). Reuses `harness-unreadable` where it fits; a new code only where none does, listed in the amendment.
10. **All or nothing, with undo.** A failed record write or a `generation-conflict` on the `fleet` write (another writer changed `fleet`) deletes every record this run wrote and leaves `fleet` as it was; the error is the failed write's own code with "nothing was kept". An undo step that fails is `profile-conflict` naming reconcile commands that do not act on a live pane. **Order: records first, the one `fleet` write last**, so the undo is record deletes only (the profile was not yet touched). A test for each failure point with a `FaultSwitch`.
11. **Dry run** with the flag: exit 0, writes nothing, shows each adopted pane's row (`record`, role, pos, session), and for an existing `fleet` a text line `profile fleet (fleet) to extend: N specs added, M total` (instead of "to create") with the added specs listed as `profile show` prints them. In JSON, `profile` keeps its keys and carries the profile as a real run would store it, with the generation it was read at; no key is added. A dry run's output equals the real run's plan.
12. **Mixed kinds** : one run can adopt a Claude Code pane and record OpenCode panes into a new `fleet`; `profile show fleet` lists both with `harness` kinds `opencode` and `claude`, and a later `profile apply --dry-run fleet` plans `adopt` for both (no `differs` from a round-trip).
13. **Gates:** `cargo test --workspace` (the CI command in `.aftersight/pipeline.config.json`), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check -p` for the touched crates only. No test starts a real `claude`, tmux, Herdr or touches the live fleet; the opt-in real-binary test stays `#[ignore]`.

## Files

Production (estimate):
- `crates/holler-cli/src/pane/import.rs` (accept the flag; thread `adopt` into `ImportRequest`; text and JSON for "to extend"), `crates/holler-cli/src/pane/import/writes.rs` (the extension write and its undo), `crates/holler-cli/src/pane/import/launcher.rs` (`port` optional when the pane is adopted as Claude Code).
- `crates/holler-pane/src/import.rs`, `import/assess.rs`, `import/observe.rs`, `import/findings.rs` (the adopt branch of the chain, the Claude record, the new finding codes, the plan's `profile` for an existing `fleet`).
- `HostPort::foreground_pids`: every site in "Every place the new method is added" above (ports.rs, adapter-host `lib.rs`/`tmux.rs`, the testkit `host.rs` and conformance suite, `wiring.rs`, `wiring/bind.rs` x2, and the test implementers).
- `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.

Tests: `crates/holler-cli/tests/pane_verbs/import/{harness,writes}.rs` (+ a new `adopt.rs` submodule if `harness.rs` grows past its file's budget), `crates/holler-cli/tests/docs_pane_control_test.rs`, a testkit helper to run a Claude process in a fake host session with a pid both fakes share.

Reuse map (extend, do not duplicate): `HarnessPort::shown_session` for the session (no new harness method); `spec_from_pane`/`profile_from_panes` for the specs; `insert_profile`'s sibling conflict mapping in `crates/holler-cli/src/profile/create.rs`; the existing undo in `writes.rs` (generalise it, do not write a second); `HarnessKind::login_variables()` (do not retype the list); `is_claude_code_command` (the one name rule).

## Decisions already made (operator and ADR; copied from item 17, not reopened)

- A re-run of `holler pane import --from ... --herdr-session ...` with `--adopt-claude-code` is the one adoption path; **no new verb** (a separate verb was declined because `relaunch` already puts a pane under Holler).
- A pane that already has a record is never touched and is reported again as already imported. Without the flag an existing profile is `profile-exists`, as today.
- The import acts on nothing live. The record is `kind: claude`, the pid and session from `agents --json` for that pid, health `unknown`. A process Holler did not start lacks `crossSessionInbound: accept` and its socket hook, so delivery would be held (`inbound-held`) and `pane doctor` shows `pane-waiting`; the report therefore ends with `holler pane relaunch NAME`, which restarts it under Holler with `--resume <session>` and keeps the conversation.
- The write extends the existing profile by compare-and-swap; the profile named is the default `fleet` (the import has no `--profile`).

**Follow-on choices (stated decisions for review; made by the brief's author, not the operator):**
- **The finding keeps the name `already-imported`.** Item 17's text says `already-registered`, but `already-imported` is shipped, on the wire and pinned (`holler-pane/src/import.rs:95`, `docs/pane-control.md:366`, `docs_pane_control_test.rs:515`). The amendment corrects the ADR's wording; no wire rename.
- A pane with a spec in `fleet` but no record (a closed pane) is recorded and its existing spec is left unchanged (specs are keyed by pane name, never duplicated).
- `command` is not recorded for an adopted Claude pane; relaunch builds the argv itself (`process_command`).
- `host.cwd` comes from the launcher table (the old files are the source of the records an import creates, section 13); a Claude pane's entry may omit `port`. A missing entry or `cwd` is the existing `source-missing` problem. Check this against the real launcher table in the dry run.
- The orchestrator role works as for any pane (`--orchestrator`).
- Final exit codes need no new row: `usage` (2), `import-blocked`, `profile-exists` (3), `generation-conflict`, `profile-conflict`, as the section 9 row already lists.

## Out of scope

`holler say --pane` reaching a Claude Code pane (#839/#850); running the opt-in real-binary test; answering permission prompts or `--agent`; launching or relaunching anything (the report only names `relaunch`); retiring `scripts/pane-restart`; changing the classification rule (#835's) or the `harness-claude-code` default; a `--profile` flag on the import; the clear-up of `profile apply`'s exit code; any real tmux, Herdr, Claude or the live fleet.

## Test plan

Fakes only: `FakeHost`, `FakeHerdr`, `FakeClaude`, the in-memory stores, through the existing `ImportRig` (`crates/holler-cli/tests/pane_verbs/import/rig.rs`). Never a real `claude`.

**Commit order: the first commit is the ADR-0021 and `docs/pane-control.md` amendment (including `foreground_pids`), before any test.** Then RED first (what fails today and why):
1. Adopt, new `fleet`: `--adopt-claude-code` returns `usage` (exit 2) instead of a record with `kind: claude`.
2. Adopt onto an existing `fleet` (seeded as `an_existing_fleet_with_a_pane_to_record_is_profile_exists` does): fails with the usage refusal today; the target is exit 0, one record, `fleet` at generation+1 with the extra spec, and the records' `profile` = fleet.
3. Idempotence, dry run, mixed grid, no-live-action call log, and each problem case of item 9 (no pid, two pids, `agents` unreadable, login variable in `pane.env`).
4. Undo: a fault on the Nth `pane_store.cas_put`, and a concurrent `fleet` change between the read and the write (`concurrent_put`), each leaves no new record and `fleet` unchanged.
5. The unchanged paths stay green: flag absent leaves a Claude pane skipped with no harness call; flag absent with an existing `fleet` is `profile-exists`.
6. The host conformance case for `foreground_pids` (and an adapter test with a typed-in shell), and a FakeHost/FakeClaude pairing test (the two fakes keep separate pid tables today: the rig needs one pid for both).
Then GREEN, then A's anti-duplication read of the diff (the extension write must reuse the undo of `writes.rs`).

## Risks

- **The pid** (D2 = A). tmux's `pane_pid` is the shell for a typed `claude`; `foreground_pids` exists so the adoption does not adopt nothing in the real fleet while every fake test passes. The fakes cannot prove the real process tree: the opt-in real-tmux check must.
- **The classification limits** (ADR-0021 section 13 "The rule's limits"): a `claude` not in the foreground is imported as OpenCode, not adopted. Unchanged; the dry run is the check.
- **Wire growth**: new finding code(s) and a changed text line; the guide and docs tests pin them, so update `docs/pane-control.md` section 3's table in the same commit as the code.
- **Hot files.** `docs/adr/ADR-0021.md` and `CHANGELOG.md` are edited by every story: rebase onto `main` before merge, keep both sides of every conflict, and never run `cargo fmt` workspace-wide (format the touched files and crates only).
- **Clashes: #850 is running at the same time** (the Claude body driver in `holler-body` and the hub; it amends ADR-0014 and ADR-0021 section 15, and takes the next ADR decision number). Leave every decision number as "next free at merge time" and renumber on rebase. This story touches neither `holler-body` nor the hub; the only shared surface is the ADR text (section 2 and section 15 item 17 here; #850 amends ADR-0014 and section 15, but item 18 is its) and `CHANGELOG.md`. Both stories add decisions: neither writes a number, whoever merges second renumbers. Item 18 of section 15 (the `holler say` step) is #850's, so do not edit it.
- **A restarted or `/clear`ed process** between the dry run and the real run: the session read at the plan may differ at the write; the record keeps the planned session and `pane doctor` shows `shown-driven-mismatch` if so. Acceptable; say so in the guide.
- **Subscription login (I7)**: nothing reads `~/.claude`; the adoption reads names, pids and the session id only.
