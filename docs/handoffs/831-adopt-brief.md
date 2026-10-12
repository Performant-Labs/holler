# Brief: #831 adopt (adopt the fleet's existing Claude Code panes)

Repo: Performant-Labs/holler. Issue: #831 (reopened; PR #857 built the harness, this is the step it left). Rigor: in-session. UI surface: no. Kind: feature.
Branch `issue-831-implementation` off `origin/main` (ef881fc at writing). Depends on #835 (merged as PR 860): its `harness-claude-code` classification and the refused flag are on `main`.

## Decided (operator, 2026-10-12)

No item of this brief awaits an operator decision. The operator answered the open points (D1, D2 on 2026-10-12, then D3 and D4 after the architecture review `docs/handoffs/831/handoff-A.md` BLOCKed the first version of this brief; D5 follows from them); everything else follows ADR-0021 section 15 item 17 and decision 23 ("a re-run of the import with `--adopt-claude-code`, no new verb"). This is the amended brief: it resolves A's 3 blocks, 7 warns and 3 questions, finding by finding, in the sections named below.

- **D1 = A.** With `--adopt-claude-code` and an existing `fleet`, the run adds every pane it records (Claude Code panes and any new OpenCode pane) by one compare-and-swap. The flag's meaning grows from "adopt Claude Code panes" to "extend the existing `fleet`"; the ADR and the guide say so. Without the flag a run with panes to record is still `profile-exists`. Rejected: (B) Claude Code panes only, an OpenCode pane stays `profile-exists`; (C) skip each new OpenCode pane as a finding (adds a wire code).
- **D2 = A.** A new required, read-only `HostPort` method, **`foreground_pids(name) -> Result<Vec<u32>, PaneError>`** (working name; the amendment fixes it), answers the pids of the foreground process group of each live pane of the session (pids only, never an argv, I7). Holler tries each against `agents --json` (matched by pid) and takes the one entry; zero or more than one match is a `problem`, never a guess. No `--claude-pid` flag. Rejected: (B) operator-named `--claude-pid PANE=PID`; (C) adopt only a pane whose `pane_pid` is itself `claude`. Why a method: `HostPort::ps` answers tmux's `#{pane_pid}` (`crates/holler-adapter-host/src/tmux.rs:29`, `PS_FORMAT`), the pane's first process, which for the fleet's typed-in `claude` is the shell (the test rig says so: `crates/holler-cli/tests/pane_verbs/import/harness.rs:52`). The opt-in real-tmux test must verify that the foreground group of a typed `claude` includes its pid (the spike note does not settle it).
- **D3 = copy as read, warn, never an agent (Q1; A's block 1).** An adopted Claude record takes `model.provider`/`model.model_id` from `OC_MODEL` and `model.effort` from `OC_EFFORT` as the file declares them, and `context` from `OC_CONTEXT_SOFT`/`OC_CONTEXT_HARD` (the record needs a ceiling); the other `pane.env` variables are kept by name as for any record. When the provider (the half of `OC_MODEL` before the first `/`) is not `anthropic` (compared case-insensitively), the pane gets a **warning** finding `claude-model-not-anthropic` naming the pane and the variable `OC_MODEL`, never a value (I7); the pane is still adopted. **`opencode_agent` is always `None` on a Claude record**, whatever `OC_DEFAULT_AGENT` says (the file's agent is ignored without a finding: `relaunch` refuses any agent for Claude, `argv.rs:70`, `agent-unsupported`, so copying it would make the adopted pane fail the one step AC8 recommends). Why a warning and not a refusal: `relaunch` passes `--model <model_id> --effort <effort>` to `claude` (`argv.rs:95-98`), so a non-Anthropic id would fail there, and the operator wants the pane adopted with the report saying so. T pins: a pane with `OC_DEFAULT_AGENT` set is adopted with `opencode_agent = None` and a `profile apply --dry-run` plans no `differs` for it; a pane with `OC_MODEL=vllm/x` gets the warning and a record; one with `anthropic/claude-opus-4` gets neither.
- **D4 = `ps -t <tty>` in the host adapter (Q3; A's block 2).** The mechanism of `foreground_pids` is fixed in the next paragraph.
- **D5 = an extension run's undo is record deletes only (Q2; A's block 3).** See AC10.

**`foreground_pids`: mechanism (D2 + D4).** tmux 3.7c has `pane_pid` (the first process), `pane_tty` and `pane_current_command`, no foreground pid or group format, so the adapter takes a second source. Verified at ef881fc: the adapter spawns only tmux (`crates/holler-adapter-host/src/lib.rs:249`, `run_tmux`) and the `kill` binary (`with_kill_binary`, `lib.rs:219`; `exec::kill_group`, `exec.rs:126`), both through `exec::run`, which runs `holler_pane::bounded::run` (#696). The call, in order:
1. One tmux call: `list-panes -s -t =<name>: -F '#{pane_dead} #{pane_tty}'` (the listing `foreground_commands` makes, `tmux.rs:151`, with a new format constant beside `FOREGROUND_FORMAT`, `tmux.rs:35`, and an `OP_FOREGROUND_PIDS` beside `OP_FOREGROUND_COMMANDS`, `lib.rs:157`). Parsed as strictly as `parse_foreground` (`tmux.rs:271`): `dead` is `0` or `1`, a dead pane is dropped, and a live pane's tty must be `/dev/` followed by a non-empty relative path of `[A-Za-z0-9._/-]` with no `..` segment and not starting with `-` (an argument that a `ps` could read as an option); anything else fails the whole answer `unavailable`, never a guess.
2. For each live pane, in tmux order, one `ps -t <tty without the "/dev/" prefix> -o pid= -o pgid= -o tpgid=` (Linux spelling `pts/3`, macOS `ttys004`; both are the tty name `ps -t` takes), run through `exec::run` (so the shared bounded runner, one deadline for the whole port call shared with the tmux call, `Call.deadline`), with `LC_ALL=C`. A `ps` that exits non-zero with empty stdout and empty stderr (procps's "no process on this tty") is an empty pane, not a failure; any other non-zero exit is `unavailable` carrying `ps`'s first stderr line (like `kill_group`). Output is numbers only, never an argv (I7: `-o args`/`command` is never requested).
3. **The answer is every member of the foreground group, not just its id.** A process whose `pgid` equals the tty's `tpgid` (`tpgid` is the foreground group of the controlling terminal; `-1` means no controlling terminal and matches nothing) is in the answer. So a `claude` that is not the group leader still matches. The pids are unique, sorted ascending across all live panes, so the result is deterministic. A line that is not three decimal fields fails the whole answer.
4. **A `ps` seam** like `with_kill_binary`: field `ps: PathBuf` (default `ps` from `PATH`) and `with_ps_binary(path)` on `TmuxHost`, a `ps_group(ps, tty, deadline)` function beside `kill_group` in `exec.rs`. The adapter test uses a script in place of `ps` (as `Fake` does for tmux in `common/mod.rs`), never the real one.
Tests of the seam (in the files list and in the first commit's text as "what the conformance case and adapter tests pin"): `crates/holler-adapter-host/tests/foreground_pids_test.rs` (fake tmux + fake `ps`: the typed-in shell case, where `pane_pid` is the shell and the foreground group is another pid; a non-leader member; two panes; a dead pane dropped; a malformed tty, `..`, `-x` and an empty tty each `unavailable`; a Linux `/dev/pts/3` and a macOS `/dev/ttys004` each reach `ps` as `pts/3` and `ttys004`; an empty `ps` answer; a non-zero `ps`; `tpgid` -1), the bound and missing-binary cases added to `fake_tmux_test.rs`'s `METHODS` (`:25`, `[HostOp; 6]` becomes 7) (and a missing `ps`), and the opt-in real-tmux test (`real_tmux/`, `#[ignore]`) that a typed `claude`-named program's pid is in the answer.

**The new method is amend-first.** The FIRST commit amends ADR-0021 (section 2 `HostPort` row, line 110, and its prose at line 124; section 15 item 17; a new decision, number next free at merge time) and `docs/pane-control.md` (the port list, section 10) to add `foreground_pids`, before any test or code. Like `foreground_commands` (decision 49 family) it has no default, so every implementer must add it.

**Every place the new method is added** (verified by `grep -rn "HostPort for" crates --include='*.rs'` at ef881fc, which already includes the #846 and #696 merges; re-run it before the first commit):
- the trait: `crates/holler-pane/src/ports.rs:413` (`pub trait HostPort`, next to `foreground_commands` at 439), plus its two `compile_fail,E0046` doctests (`PidsOnly`, impl at 369, fence at 364; `NoClientCount`, impl at 395, fence at 390): give each the new method except the one it exists to omit, and add a third, minimal doctest for `foreground_pids` (the file is 750 lines and `scripts/lint.sh` refuses 900; keep the three additions under about 40 lines in all);
- the host adapter: `crates/holler-adapter-host/src/lib.rs:333` (`impl HostPort for TmuxHost`; next to `foreground_commands` at 416; the `ps` field, `with_ps_binary` and `OP_FOREGROUND_PIDS`), `tmux.rs` (the `#{pane_dead} #{pane_tty}` listing and its strict parse) and `exec.rs` (`ps_group`, beside `kill_group`, through the shared bounded runner); tests in `crates/holler-adapter-host/tests/` (`foreground_pids_test.rs` beside `foreground_test.rs`, the `METHODS` row in `fake_tmux_test.rs`, the opt-in `real_tmux/` case);
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
Its undo (`Failed::undone`/`undo`/`reconcile`, `writes.rs:107-203`) deletes the failed record, each record written (at the generation its write returned) and then the profile; its `reconcile` text (`writes.rs:167-203`) tells the operator to run `holler profile delete fleet` (with `--keep-panes`), which is right only for a `fleet` this run created.

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

1. **Amend first.** ADR-0021 and `docs/pane-control.md` are edited in the first commit, before any test or code: the new `HostPort::foreground_pids` (section 2 table and prose, the guide's port list) **with its mechanism**: the host adapter's second subprocess, `ps -t <tty>` through the shared bounded runner with its own seam, `#{pane_dead} #{pane_tty}`, and the answer being every member of the tty's foreground process group (see "`foreground_pids`: mechanism"); the section 2 `holler-adapter-host` description and its error table (every subprocess is tmux, `kill` or `ps`) change with it; the five new finding codes (see "Final finding codes") in the section 9 / section 3 vocabulary; item 17 "As built" replaces "not yet", the section 9 `pane import` row, section 13 "The switch" bullet, "Deferred to named stories" and decision 23 are updated; the guide's sections 1, 3, 10 and 12 lose "refused/will be". The new decision numbers (the method; the flag now extending `fleet`) are **next free at merge time** (do not write a number; #850 is amending the same list). `cargo test -p holler-cli --test docs_pane_control_test` passes.
2. `holler pane import ... --adopt-claude-code` no longer answers `usage` naming #831. Test `adopt_claude_code_is_usage_naming_831_and_writes_nothing` is replaced; `grep -rn "not_implemented_message(CLAUDE_CODE_STORY)\|CLAUDE_CODE_STORY" crates/holler-cli/src` prints nothing.
3. **Without the flag nothing changes.** A Claude Code pane is `harness-claude-code`, `skipped`, no record, and no harness call (the existing `harness.rs` tests stay green unedited, except the replaced one).
4. **With the flag, a Claude Code pane gets a record**: `harness.kind = claude`, `port` absent, `pid` = the process `agents --json` lists for that pane (the pid is found through `foreground_pids`, then matched by `shown_session`; see AC9 for the order and the stop rule), `health = unknown`, `session_of_record` = the session that entry holds (read by pid through `shown_session`, never by directory, and the old `last-session` files are not consulted for it), `command` absent (Holler cannot see its argv), `hold = none`, `profile = fleet`, `host.cwd` from the launcher entry, whose `port` is optional for a pane adopted this way (see "Launcher"). **`model`, `env`, `context`, `opencode_agent` follow D3**: `model` from `OC_MODEL`/`OC_EFFORT` as read (warning `claude-model-not-anthropic` when the provider is not `anthropic`), `context` from `OC_CONTEXT_*`, `env` the other variable names, and `opencode_agent = None` always. `shown_session` takes the Herdr pane's id (`ports.rs:589`), and the chain runs before the Herdr join (`assess.rs:116-130`), so the run **resolves the label match first**: from the snapshot already read, a pane whose name is the label of exactly one Herdr pane in the run's Herdr session gets that `PaneId` in `Target` (`observe.rs:37`); a pane with none or several has no `PaneId`, the session read is skipped, and the pane has its existing `pane-unlabelled`/`pane-label-ambiguous` problem and no record, no placeholder id.
5. **An existing `fleet` is extended**: with the flag, a run that has panes to record (Claude Code or OpenCode, D1 = A) and an existing `fleet` writes the records, then adds their specs to `fleet` by one `ProfileStore::cas_put` at the generation just read (change-log actor `holler pane import`), instead of `profile-exists`. Without the flag the same run is still `profile-exists`, nothing written (`an_existing_fleet_with_a_pane_to_record_is_profile_exists` unchanged). **The plan carries the choice** (A warn 6): `Plan.profile: Option<Profile>` becomes `Plan.fleet: FleetPlan` = `None` (no record to write), `Create(Profile)`, or `Extend { profile, generation_read, added }` (`profile` is the read profile with the added specs, `added` the new specs' pane names). `plan` reads `ProfileStore::get(fleet)` **only when the flag is on and there are records** (a new failure mode of `plan`, whose doc and the module docs say so; without the flag `plan` yields `Create` exactly as today and `write` keeps its existing `profile-exists` check, so that path is untouched). `write` matches on `FleetPlan` and reads no flag. The write uses the plan's profile and `generation_read` for its one `cas_put` (a stale one is `generation-conflict`; a `fleet` deleted in between is the store's own error), so a dry run equals the real plan. **When `added` is empty** (every spec of a recorded pane already exists in `fleet`, e.g. a closed pane) **the `fleet` write is skipped**: the records are written and `fleet`'s generation does not move. An existing spec for a recorded pane's name is left unchanged; if its `harness.kind` differs from the new record's, the pane gets the warning finding `spec-kind-differs` (A warn 8), naming the pane and the two kind names, so the permanent `differs` of a later `profile apply --dry-run` is explained; one T test pins it.
6. **Idempotent.** A re-run after an adoption finds the pane with a record: `already-imported`, no store write, exit 0, `fleet` generation unchanged (`a_second_real_run_is_a_no_op` extended to a Claude pane).
7. **Acts on nothing live.** Test asserts the calls: reads only (Herdr snapshot and version, `host.ps`, `host.foreground_commands`, the pid read, `harness.shown_session`); no `run`, `stop_owned`, `send_text`, `send_keys`, `ensure_pane`, `send_prompt`, `create_session`. No source file changes.
8. **The report ends with the next step.** Each adopted pane has one info finding naming `holler pane relaunch NAME` (proposed code `claude-code-adopted`; the amendment lists the final code, and the docs-test pins the vocabulary). The text summary line counts them.
9. **Never recorded on a guess.** No pid found, more than one, `agents --json` unreadable, no session held, or a `pane.env` variable that changes Claude Code's login (`ClaudeCode` kind `login_variables()`) is a `problem` with no record, so a real run is `import-blocked` and writes nothing (the pane's name and the variable's name only, never a value, I7). Codes: a failed `foreground_pids` or `shown_session` is `harness-unreadable` (its existing text, `observe.rs:30`, "what the pane's tmux session runs could not be read", fits a failed read and nothing else); zero pids matched, more than one matched, or the one match holds an empty session is `claude-process-unmatched`; a login variable is `claude-login-env`. See "Final finding codes". **Order and bound (A warn 10):** the pids are tried in the order `foreground_pids` returns them (ascending), each through `shown_session` (one `agents --json` each); a pid for which it answers `None` is not a match; **the walk stops at the second match** (the outcome is already "more than one"). A chain for a Claude pane so makes `host.ps`, `host.foreground_commands`, `host.foreground_pids` (one tmux call and one `ps` per live pane) and then at most one `shown_session` per pid, each a subprocess; it runs in the pane's own thread under the existing per-pane bound. The call-count doc at `observe.rs:9-14` is rewritten for it.
10. **All or nothing, with undo, in two shapes (A block 3; Q2, D5).** The undo of `writes.rs` is generalised (no second copy): `Failed` carries *what failed*, `Write::Record(&PaneName)` (a record's `cas_put`) or `Write::Fleet` (the extension's one `cas_put` of `fleet`, which has no failing pane); `undone` takes the profile to delete as an `Option<(&Profile, &Actor)>`, which is `Some` for `FleetPlan::Create` and `None` for `FleetPlan::Extend`.
   - **Create** (a new `fleet`): unchanged. The profile is written first, then the records; a failure deletes the failed record if it landed, every record written, then the profile; the `reconcile` text keeps `holler profile delete 'fleet'` because this run created `fleet`.
   - **Extend** (an existing `fleet`; `FleetPlan::Extend` with a non-empty `added`): **records first, the one `fleet` `cas_put` last**, so the undo is **record deletes only** and there is no `Stop::Profile` (an extension has no profile delete). A failed record write, or a `generation-conflict` (or any error) on the `fleet` write, deletes the failed record if it landed (a `Write::Fleet` failure has none) and every record written, newest first, each at the generation its write returned; `fleet` is left as it was. The error is the failed write's own code with "nothing was kept (the records written so far were deleted; the profile fleet was not changed)". An undo step that fails is `profile-conflict` whose text names the failing write, the records that remain (`pane_list`), and `holler pane list` (to see them) and `holler profile show 'fleet'` (read-only, to see that `fleet` is unchanged). **It never contains `profile delete`.** The only verb that removes a record, `holler pane close` (`close.rs:1-12`), also stops the pane's processes and closes its Herdr pane, which acts on a live pane (and on an adopted Claude pane would end the operator's session), so the text does not suggest it, and no other verb removes a record alone; the text says a re-run of the import skips the kept records (`already-imported`). The remedy is therefore a statement of what remains and how to look at it; there is no deleting command to name, and the operator's answer ("commands that remove this run's own records") is met by naming none that would also act on a live pane.
   - A test for each failure point with a `FaultSwitch` (the Nth record `cas_put`; the `fleet` `cas_put`; a concurrent `fleet` change through `concurrent_put` between the read and the write), for both shapes, and one with the undo itself faulted: its `profile-conflict` text for an extension **does not contain `profile delete`** (nor `pane close`) and names the kept records; the Create text still names `holler profile delete`.
11. **Dry run** (the plan above; `FleetPlan` is what is printed) with the flag: exit 0, writes nothing, shows each adopted pane's row (`record`, role, pos, session), and for an existing `fleet` a text line `profile fleet (fleet) to extend: N specs added, M total` (instead of "to create") with the added specs listed as `profile show` prints them. In JSON, `profile` keeps its keys and carries the profile as a real run would store it, with the generation it was read at; no key is added. A dry run's output equals the real run's plan.
12. **Mixed kinds** : one run can adopt a Claude Code pane and record OpenCode panes into a new `fleet`; `profile show fleet` lists both with `harness` kinds `opencode` and `claude`, and a later `profile apply --dry-run fleet` plans `adopt` for both (no `differs` from a round-trip).
13. **Gates:** `cargo test --workspace` (the CI command in `.aftersight/pipeline.config.json`), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check -p` for the touched crates only. No test starts a real `claude`, tmux, Herdr or touches the live fleet; the opt-in real-binary test stays `#[ignore]`.

## Files

Production (estimate):
- `crates/holler-cli/src/pane/import.rs` (accept the flag; thread `adopt` into `ImportRequest`; text and JSON for "to extend"), `crates/holler-cli/src/pane/import/writes.rs` (the extension write and the generalised undo of AC10), `crates/holler-cli/src/pane/import/launcher.rs` (see "Launcher").
- **Structure (A warn 9).** `holler-pane/src/ports.rs` is 750 lines and gains the method, its docs and a third doctest (about 40 lines), so near 800; no file may reach 900 (`scripts/lint.sh`). `import/assess.rs` (435) and `import/observe.rs` (245) must not take the Claude code. So **a new sibling module `crates/holler-pane/src/import/adopt.rs`** holds the adopt chain (`foreground_pids`, then `shown_session` per pid with the stop rule of AC9, returning the pid and session or the finding) and the Claude record builder (the env rule of D3, `HarnessInfo { kind: Claude, port: None, pid: Some, health: unknown }`, the login-variable check); `observe.rs` and `assess.rs` gain only the hook (`Target.adopt: Option<PaneId>`, `Observed::ClaudeCode` carrying the adopt answer, and the call into `adopt`). If `adopt.rs` itself nears 600 lines, split the builder (`adopt/record.rs`).
- `crates/holler-pane/src/import.rs` (`ImportRequest.adopt`, `FleetPlan`, `plan`'s one `ProfileStore::get`, `assemble`), `import/findings.rs` (the five new codes, `ALL`/`code()`/`level()` together, the `HarnessClaudeCode` doc reworded: "a switch adopts these panes" becomes what it does now, and the "eighteen kinds" doc becomes twenty-three), `import/files.rs` (`LauncherEntry`).
- `HostPort::foreground_pids`: every site in "Every place the new method is added" above (ports.rs, adapter-host `lib.rs`/`tmux.rs`/`exec.rs`, the testkit `host.rs` and conformance suite, `wiring.rs`, `wiring/bind.rs` x2, and the test implementers).
- `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.

**Launcher (A warn 4).** `checked` (`launcher.rs:115`) refuses a missing port with `source-missing` before any pane is observed, but which entries may omit a port is known only after the classification. So `LauncherEntry.port` becomes `Option<u16>` (`import/files.rs`), `checked` keeps every other check and text (a missing cwd, `no port and no cwd`, port 0, a relative cwd), and an entry with a cwd and no port is accepted with `port: None` plus the phrase it names itself by (`the entry for P in the launcher table F`, the string `checked` already builds as `about`). `assess` emits, for an OpenCode pane (or a Claude pane with the flag off) with `port: None`, the **identical** `source-missing` finding (`<about> has no port`, the same kind, level and message as today). T adds a RED case for it with the flag off and on (the message pinned unedited, as AC3 requires), and the order of the findings is unchanged because findings are sorted. Observation runs for a pane with no port as for one with the launcher entry absent (`Target.port = None`).

Tests: `crates/holler-cli/tests/pane_verbs/import/{harness,writes}.rs` (+ a new `adopt.rs` submodule if `harness.rs` grows past its file's budget), `crates/holler-cli/tests/docs_pane_control_test.rs`, a testkit helper to run a Claude process in a fake host session with a pid both fakes share.

Reuse map (extend, do not duplicate): `HarnessPort::shown_session` for the session (no new harness method; called with the resolved Herdr `PaneId` and `HarnessEndpoint::Process`); `spec_from_pane`/`profile_from_panes` for the specs (the extension merges the new specs into the read profile and builds none by hand); `insert_profile`'s sibling conflict mapping in `crates/holler-cli/src/profile/create.rs`; the existing undo in `writes.rs` (`Failed`/`undone`/`undo`/`reconcile`, generalised by AC10: `Write::{Record, Fleet}` and an optional profile, no second undo); `single_quoted`/`pane_list` (`profile/delete.rs:206,212`) for the remedy text; `HarnessKind::login_variables()` (do not retype the list); `is_claude_code_command` (the one name rule); `EnvValues` (`import/files.rs:37`) for the model, effort and context; the shared bounded runner `holler_pane::bounded` through `exec::run` for the `ps` call (no new runner), and `with_kill_binary`/`kill_group` as the pattern of the `ps` seam; `parse_foreground`'s strictness (`tmux.rs:271`) for the new listing's parse.

**Final finding codes (A warn 7; list fixed here, 23 in all).** The 18 existing kinds stay (wire form and level unchanged; `harness-claude-code` keeps `info` and is what a Claude pane gets without the flag). Five are new, added to `ImportFinding`, `ALL`, `code()`, `level()` together and to `docs/pane-control.md` section 3's table and the `docs_pane_control_test.rs` vocabulary in the same commit:

| code | level | when |
|---|---|---|
| `claude-code-adopted` | info | each adopted pane; the message names `holler pane relaunch NAME` (AC8) |
| `claude-process-unmatched` | problem | zero pids matched `agents --json`, more than one matched, or the match holds an empty session (AC9) |
| `claude-login-env` | problem | `pane.env` sets a variable `login_variables()` names; pane and variable name only |
| `claude-model-not-anthropic` | warning | the `OC_MODEL` provider is not `anthropic` (D3); pane and `OC_MODEL` only |
| `spec-kind-differs` | warning | an existing `fleet` spec for the pane has a `harness.kind` other than the record's (AC5) |

Reused as they are: `harness-unreadable` (a failed `foreground_pids` or `shown_session` only), `source-missing`, `pane-unlabelled`, `pane-label-ambiguous`, `already-imported`.

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
3. Idempotence, dry run, mixed grid, no-live-action call log, and each problem case of item 9 (no pid, two pids and the walk stopping at the second match, a match with an empty session, `foreground_pids` failing, `agents` unreadable, login variable in `pane.env`), a Claude pane with no unique Herdr label (no session read, `pane-unlabelled`), a launcher entry with no port (OpenCode: the identical `source-missing`, flag off and on; Claude with the flag: adopted), D3 (`OC_DEFAULT_AGENT` gives `opencode_agent = None` and no `differs`; `OC_MODEL` non-Anthropic warns; Anthropic does not), `spec-kind-differs`, and an extension whose `added` is empty (records written, `fleet` generation unchanged, no `cas_put` of `fleet`).
4. Undo: a fault on the Nth `pane_store.cas_put`, the `fleet` `cas_put`, and a concurrent `fleet` change between the read and the write (`concurrent_put`), each leaves no new record and `fleet` unchanged; and a faulted undo, whose extension text contains neither `profile delete` nor `pane close` (AC10).
5. The unchanged paths stay green: flag absent leaves a Claude pane skipped with no harness call; flag absent with an existing `fleet` is `profile-exists`.
6. The host conformance case for `foreground_pids` (and the adapter tests of "`foreground_pids`: mechanism", with a fake tmux and a fake `ps`, including a typed-in shell), and a FakeHost/FakeClaude pairing test (the two fakes keep separate pid tables today: the rig needs one pid for both).
Then GREEN, then A's anti-duplication read of the diff (the extension write must reuse the undo of `writes.rs`).

## Risks

- **The pid** (D2 = A). tmux's `pane_pid` is the shell for a typed `claude`; `foreground_pids` exists so the adoption does not adopt nothing in the real fleet while every fake test passes. The fakes cannot prove the real process tree: the opt-in real-tmux check must.
- **The classification limits** (ADR-0021 section 13 "The rule's limits"): a `claude` not in the foreground is imported as OpenCode, not adopted. Unchanged; the dry run is the check.
- **Wire growth**: new finding code(s) and a changed text line; the guide and docs tests pin them, so update `docs/pane-control.md` section 3's table in the same commit as the code.
- **Hot files.** `docs/adr/ADR-0021.md` and `CHANGELOG.md` are edited by every story: rebase onto `main` before merge, keep both sides of every conflict, and never run `cargo fmt` workspace-wide (format the touched files and crates only).
- **Clashes: #850 is running at the same time** (the Claude body driver in `holler-body` and the hub; it amends ADR-0014 and ADR-0021 section 15, and takes the next ADR decision number). Leave every decision number as "next free at merge time" and renumber on rebase. This story touches neither `holler-body` nor the hub; the only shared surface is the ADR text (section 2 and section 15 item 17 here; #850 amends ADR-0014 and section 15, but item 18 is its) and `CHANGELOG.md`. Both stories add decisions: neither writes a number, whoever merges second renumbers. Item 18 of section 15 (the `holler say` step) is #850's, so do not edit it.
- **A second subprocess in the host adapter (D4).** `ps` joins tmux and `kill`; a host without `ps -t` (BusyBox) answers `unavailable`, and a Claude pane there is `harness-unreadable`, never adopted on a guess. `ps` output is numbers only (I7).
- **Cost.** One tmux call and one `ps` per live pane, then at most a `shown_session` per pid until the second match (AC9); all in the pane's own thread under the existing per-pane bound.
- **An undone extension that cannot finish** leaves records without a `fleet` spec and no verb that removes a record alone (AC10); the text says what remains.
- **A restarted or `/clear`ed process** between the dry run and the real run: the session read at the plan may differ at the write; the record keeps the planned session and `pane doctor` shows `shown-driven-mismatch` if so. Acceptable; say so in the guide.
- **Subscription login (I7)**: nothing reads `~/.claude`; the adoption reads names, pids and the session id only.
