# [docs: the operator guide for `holler pane`, the deprecation of hand-managed session files, and the runbook entries](https://github.com/Performant-Labs/holler/issues/652)

Part of epic #633. This is the complete statement of story #652, written as one text against `main`: its facts were re-checked at `5d366b9` (#650 and #664 merged) and its evidence was cut at `7284001` (#802 merged; "Facts that wait for other stories" says which excerpts moved). It supersedes every earlier version of this brief and, where it disagrees, the text of issue #652 as first filed (the differences are listed in "Where this brief is the decision", below). The story is **docs plus one docs test, one comment line and a CHANGELOG entry**: it changes no behaviour.

It was briefed ahead of the stories it documents, and most have merged. The text states what is on `main` for #650 (`holler pane import`), #664 (`holler profile apply`), #644 part 2 (the agent in the output) and #642 part 3 (`agent-unusable`). Only a line marked **REFRESH AFTER MERGE of #NNN** states the *expected* contract of a story that has not merged: today that is #649 alone (R1 to R3); the build run re-cuts the excerpt named on that line from the merged code and patches the line before it writes the guide. "Facts that wait for other stories" lists them all. An excerpt cut before a merge (E29, E34, E36, E41, E47 and the R excerpts of merged stories) shows the text as it was then; the build run re-cuts it from `main`.

## Where this brief is the decision

1. **Which panes `holler pane` manages (operator decision (a), 2026-10-11).** `holler pane` and `holler profile` manage the panes in the registry, and the registry holds OpenCode panes only. A pane that runs Claude Code is outside it and keeps using `scripts/pane-restart`. After the 7-day sign-off only the script's OpenCode path is retired (in a later story, not this one).
2. **The agent in the output** is `, agent KEY` in the text result line and an additive JSON key `agent` (the contract of #644 part 2, R11). The issue as first filed said `agent=KEY`; that spelling is superseded.
3. **The test file** is `crates/holler-cli/tests/docs_pane_control_test.rs`, target `docs_pane_control_test` (the repository's `docs_<subject>_test` cadence, E59, E57). The issue as first filed said `pane_control_doc_test`; that name is superseded.
4. **SHOWN against DRIVEN** is read as `pane list`'s SYNC reads it: SHOWN against the session of record, which invariant I2 makes the session the hub drives (E8, E40). The DRIVEN column prints what the record holds and is not compared (E6).
5. **Additions to the issue's scope as first filed** (each is listed here so a reviewer need not diff the old text): `scripts/pane-restart` is in the blast radius for one comment line (line 3); the first-filed acceptances 1 and 11 are narrowed to what a test can check (the line rule for PROPOSED, the twelve headings); acceptance 3 pins the class word and number per error code, built from `class_of`; #650 is added to "Depends on" because the cutover command is a documented example that must parse; acceptance 13 (the envelope examples) is required, where the filed text made it optional; acceptances 16 (the CHANGELOG entry) and 17 (content the audit found missing) are new; the procedure of section 10 supersedes the body of issue #654 where they differ (point 6).
6. **The cutover procedure of section 10 supersedes #654's body where they differ**: SYNC is read from `pane list` and `doctor`, not from `holler roster` (decision 1), and the first pane is watched through a full overnight, not a day (decision 6). Issue #654's body is edited by the operator, not by this story. Issue #655 (the operations story) is independent of this one and is not changed by it.

## Scope (final)

Write `docs/pane-control.md` (new), rewrite one section of `docs/orchestrating.md`, add the guide to `docs/README.md` under "Guidance" (E48), change one comment line of `scripts/pane-restart`, and add the CHANGELOG entry. The agent guidance for orchestrators is `docs/orchestrating.md` and the `docs/README.md` entry; `AGENTS.md` and `agent-skills/` are out of scope.

### The boundary: registry panes

The record's `harness.kind` is the closed enum `HarnessKind`, which has one variant, `opencode` (E27, E38). So:

- **A registry pane** is an OpenCode pane with a record. Every `holler pane` and `holler profile` verb acts on registry panes.
- **A pane that does not run OpenCode** (a Claude Code pane is the case in the fleet) has no record and can have none. `holler pane import` creates no record for a pane whose `sessions.toml` names another harness (the finding `harness-not-opencode`, level `skipped`, never blocking: `ImportFinding` in `holler-pane`'s `import/findings.rs`; R5, R8, items 6 and 7); the "Known facts" bullet below says what the files cannot tell. A verb that names it as an existing pane (`get`, `relaunch`, `switch`, `reset`, `park`, `close`) answers `pane-not-found` (exit 3; E41). A whole-fleet `doctor` pass reports it as `unregistered-herdr-pane`, because the finding is "a Herdr pane that no record names" whatever the pane runs (E12, E16, E17), and a pass keeps no memory, so the finding appears in every pass (E46). That is expected and is not a failure.
- **`scripts/pane-restart` is that pane's only restart path** (its Claude Code path, issue #784; E50, lines 6 and 19-30; E49, lines 125-145), now and after the sign-off. The guide and `docs/orchestrating.md` say so.
- **Known facts about the live fleet.** (1) The live fleet has Claude Code panes whose old per-pane files look exactly like OpenCode panes: `sessions.toml` says `harness = "opencode"`, `mode = "attach"`, and `pane.env` holds an `anthropic/...` model. `holler pane import` reads only those files and the live observation, and `harness-not-opencode` fires only for a row whose `harness` is not `opencode`, so import would adopt those panes as OpenCode panes with a record. No exclusion mechanism exists yet; the brief names none and the guide invents none. (2) The dry run cannot run on the live fleet until #649 merges, because the binary's ports are stubs and every port call answers `not-implemented`. The guide states both facts in section 10, before step 1, as facts and not as procedure.

The guide states this in section 1 and repeats the consequence where it matters (sections 3, 5, 10 and 11, and the rewritten section of `docs/orchestrating.md`).

### The guide's shape

`docs/pane-control.md` has exactly twelve `##` headings, numbered, in this order, so the test slices a section from `\n## N. ` to the next `\n## ` (the way `docs_errors_test.rs` slices its section, E57) and the numbers do not move when a later story adds a `###` subsection:

- `## 1. The rule (I1)`
- `## 2. What a pane runs`
- `## 3. The verbs`
- `## 4. Reading the table`
- `## 5. doctor`
- `## 6. Profiles`
- `## 7. Grid notation`
- `## 8. Probe, argv, layout, Herdr version`
- `## 9. JSON output reference`
- `## 10. Migration and cutover`
- `## 11. Rollback`
- `## 12. Later`

Each section's content follows. Where a list of verbs, flags, findings, codes or columns is needed, the guide takes it from the code's own table (the ALL arrays, `COLUMNS`, the clap tree, `--help`), not from this brief, and the test pins the lists that can be pinned (Acceptance).

**1. The rule (I1).** Quote I1 in ADR-0021's words: "Only pane verbs change a pane or its registration" (E40). Then: the `holler profile` verbs change panes only through the same transactions (`profile apply` creates and relaunches through the launch transaction and adopts by a compare-and-swap write of `Pane.profile`, R9; `profile create --from-current` and `delete --keep-panes` write only `Pane.profile`, E32, E33), so the rule is not widened. The registry is the one source of truth for what a registry pane is; Herdr, tmux and the harness are observed (I6, E40), never inferred from files. Then the boundary above: the rule is about registry panes; a pane that does not run OpenCode is outside it and `scripts/pane-restart` still restarts it.

**2. What a pane runs.** One Herdr pane shows one tmux session (E38). The harness server (OpenCode) runs *inside that tmux session*: the host adapter starts it from the argv `HarnessPort::serve_command` builds and `HostPort::stop_owned` stops it (E28, E29, E39), by `TERM` to every owned process group, one shared grace (default 2 s), then `KILL` of every group that still has a member (E30). The launch is a transaction that creates the session of record (E65).
- `--agent KEY` sets the server's default agent when the server starts. A key the running server cannot use is refused with the OpenCode adapter's code `agent-unusable` (#642 part 3, merged: the adapter sets the key as the server's `default_agent` and checks that the server lists it; R12, R13, ADR-0021 section 8; E29 is the cut from before the merge, and the build run re-cuts it). The launch is rolled back: the server is stopped and no record is written. **A `relaunch` meets the refusal after its old server was already stopped, so the pane is left with no server and its record unchanged; `holler pane relaunch PANE` without `--agent` brings it back on the recorded agent** (ADR-0021 section 8). The guide says this here and in section 11. `agent-unusable` is **not** one of `ALL_CODES` (E19, E20): it is an *open* code, a constant of the adapter's own file, and every open code is a refusal, exit 3 (E22, E18). The guide says so where it names the code.
- `relaunch` without `--agent` keeps the agent of its base spec: the recorded pane without `--profile`, and the profile's spec for the pane with `--profile P` (the unflagged value comes from that base, `launch.rs` lines 176-179); with `--agent` it replaces it (E64; `relaunch` shares the function, E65 lines 17-18, E31); there is no flag to clear an agent (ADR-0021 section 8, decision 17). A key that is empty or holds anything but ASCII letters, digits, `-` and `_` is refused `agent-key-invalid` before anything is done; the key is never echoed. `agent-key-invalid` is also an open code (a refusal, exit 3), named beside `agent-unusable`.
- The agent is printed as `, agent KEY` after `harness port N` in the text result of `launch` and `relaunch` and as the additive JSON key `agent` of their data (#644 part 2, merged: R11 is the cut from before the merge, and the build run re-cuts it). `pane list` has no agent column and `pane get` has no agent line in its text; `pane get --format=json` carries the agent as `opencode_agent` in `data.pane` (the record, when it is set) and in `data.spec` (the profile's spec for the pane, when it holds one), and the `spec` line of the text form prints that spec as JSON. `profile show` prints the field as `opencode_agent: KEY`, or `opencode_agent: (default)` for a spec with no agent (`none` is a valid key, so it is never used for "no agent"); in JSON the value is the key or `null`.
- What a verb needs. Every `pane` and `profile` verb needs the hub to be running; without one it answers `unavailable` (exit 1) naming the missing hub, and a verb that never touches Herdr (`pane list`, `pane get`, `pane watch`, `profile list`, `profile show`) works with no Herdr running (REFRESH AFTER MERGE of #649: R1). The Herdr session, the tmux socket and the OpenCode binary come from environment variables; the guide names `HOLLER_HERDR_SESSION` and points to the module doc of `crates/holler-cli/src/pane/wiring.rs` for the full list and does not copy it (REFRESH AFTER MERGE of #649: R2). **`--herdr-session NAME` stays a required flag** of every verb that reaches Herdr (`launch`, `pane import`, and `profile apply` when its plan has a `create`; `relaunch` has no such flag and acts in the session its record names) even when `HOLLER_HERDR_SESSION` is set: the variable configures the adapter, and the flag names the session the verb acts on; the two must name the same session or the verb answers `unavailable`. The guide says so in this section and in section 10 (REFRESH AFTER MERGE of #649: R2; defaulting the flag from the wiring is not built).

**3. The verbs.** `holler pane` `list`, `get`, `watch`, `launch`, `relaunch`, `switch`, `reset`, `park`, `unpark`, `close`, `doctor`, `import` (E2); `holler profile` `create`, `delete`, `list`, `show`, `apply` (E3); and `rename`, `export`, `import`, which are **PROPOSED** (#665, not built; E3 line 11, E35, R14) and marked as such. One short subsection per verb with a heading of the form ``### `holler pane list` ``: its purpose, its flags, an example, and the exit codes it can end with. **A heading span carries the verb's required arguments**, because `docs_cli_test` parses every one-backtick span that starts `holler ` and rejects a missing required argument (`MissingRequiredArgument` is not among the kinds it tolerates). On `main` that gives: ``holler pane list``, ``holler pane watch``, ``holler pane unpark``, ``holler pane close``, ``holler pane doctor`` and ``holler profile list`` bare (their `PANE` is optional); ``holler pane get PANE``, ``holler pane launch PANE``, ``holler pane relaunch PANE``, ``holler pane switch PANE``, ``holler pane reset PANE``; ``holler pane park PANE --reason TEXT --release-when WHEN`` (the two flags are required, E4); ``holler profile create NAME``, ``holler profile delete NAME``, ``holler profile show NAME``; and ``holler pane import --from STATE_ROOT --herdr-session NAME`` (#650, merged: `--from` is a required flag; `--herdr-session` is declared optional and is required at run time, `usage` without it, so the heading names it; R4) and ``holler profile apply NAME`` (#664, merged: `NAME` is a positional; R9). The build run checks each heading form against the clap tree, and the heading text is the form that parses. The heading of a PROPOSED verb ends `(PROPOSED)`, as in ``### `holler profile export` (PROPOSED) ``.
- **Flags** are the ones in `--help` and in the verb's `Args` (E4, E5, E10, E11, E32, E33); the guide does not hand-copy a flag list longer than it describes. `--profile` has two classes: spec-editing (`launch`, `relaunch`, `close`) and scoping (the rest of the `pane` verbs); `pane import` takes no `--profile`, and no `profile` verb takes it (E3 line 7, E4). `--spec-only` exists only on `launch`, `relaunch` and `close` and requires `--profile` (E4). `--take-over` is `profile apply`'s own flag (E3 line 8).
- **Exit codes** are the common ones (`usage` 2, and `unavailable`, `timeout`, `store-corrupt` 1) and the verb's own row of ADR-0021 section 9 (E41), which the guide cites by link and does not restate in full. `doctor` exits 0 whenever a pass completes (section 5). `profile apply` follows its own rule (the reference below). `pane import` follows its own row of ADR-0021 section 9 (#650, merged): a dry run exits 0 with the whole report, whatever it found; a real run exits 0 when it wrote the plan or had nothing to write, 3 with `import-blocked` or `profile-exists`, and otherwise by the class of the error that stopped it (R6).
- **`pane import`** (#650, merged: R4 to R8; `crates/holler-cli/src/pane/import.rs`): the synopsis is `holler pane import --from STATE_ROOT --herdr-session NAME` with `[--launcher FILE]` (the launcher table, default `STATE_ROOT/launcher.toml`; section 10), `[--orchestrator PANE]` (repeatable; the dry run prints each pane's role, and the orchestrator is named with this flag, never read from a name) and `[--dry-run]`. It adopts what is running: it never starts, stops, types into or moves anything and deletes nothing from the source files. The guide lists the import findings from `ImportFinding::ALL` (R7: sixteen kinds, each with a level, `problem`, `warning`, `skipped` or `info`; only a `problem` blocks a real run) and the refusal `import-blocked` (exit 3). **Six finding codes are also spelled as something else, and the guide says in one sentence which is which:** `session-not-found` and `command-not-argv` are closed error codes (section 9), and `tmux-session-missing`, `observe-failed`, `server-wedged` and `server-down` are doctor kinds (section 5); in the import's report each is a *finding* (a `warning`, except `tmux-session-missing`, a `problem`), never the error of the verb (ADR-0021's import row: they are "findings of the report, not errors"). `herdr-version-unsupported` is the one code that is not a finding of the import: it is the error that ends the run (section 5).
- **PROPOSED verbs** (`rename`, `export`, `import` under `profile`) appear **bare**, each on a line that says `PROPOSED`, with their planned arguments described in prose (R14): their argument structs are empty today (E35), so any example with an argument fails the docs test.

**4. Reading the table.** The columns of `holler pane list`, all nine of `COLUMNS` (E7), and what each means. HEALTH and SHOWN are what reconcile last recorded (E6); the empty cell is `-`. DRIVEN is printed as the record holds it and is never compared (E6, E8); until the cutover wires a DRIVEN source it prints `-` (REFRESH AFTER MERGE of #649: R3). SYNC is the comparison: SHOWN against the session of record, `ok`, `MISMATCH` or `-` when nothing was observed (E8). A `pane watch` line prints the same cells as `key=value` with the column name in lower case (E7). Then:
- a line that says **the roster extension (the PANE, POS, PROFILE, HEALTH, SHOWN and DRIVEN columns on `holler roster`, #648) is later**: this guide documents `pane list` and `doctor` as the places to read SHOWN against DRIVEN, and `holler roster` does not show them yet (R15; operator decisions 1 and 7). That one line contains `holler roster`, `#648` and `later`.
- one sentence on the Herdr display plugin (#651, `plugins/herdr-holler`): it shows project, SHOWN against DRIVEN and hold in Herdr's sidebar; it only reports metadata and reads, and its actions run the `holler pane` verbs (E37).

**5. `doctor`.** What one reconcile pass reports: every finding kind of `FindingKind::ALL` (E12), each in backticks with what it means and the command it names (`FindingKind::remedy`, E13; text mode prints it, E11). Then:
- `--fix` repairs **only** `shown-driven-mismatch`: it selects the session of record in the pane's TUI so the TUI shows it, and **never changes** which session is the session of record and never writes `session_of_record` (E12 lines 68-69, E14, E15, E11 line 33). It leaves the orchestrator's pane alone unless the run names it (E14, `skipped`). The guide words this so it cannot be read as editing the record.
- It will not fix every other kind; each finding names the command to run instead (E13). A `shown-driven-mismatch` is also `not-fixable` when its server is not healthy or does not hold the session of record (`FixState::NotFixable`, `findings.rs` lines 156-159; the text line then carries no `[...]` state); the guide says so.
- A pass that completes exits 0 with findings in the report; a script reads `data.findings` (E11).
- The periodic pass: a scheduler runs `holler pane doctor --format=json` without `--fix`; there is no hub timer, and a pass keeps no memory, so every tick reports every standing finding again (E11, E46).
- `herdr-version-unsupported` is one wire string with two roles: a *finding* in a pass that still exits 0, and a closed *error code* of `ALL_CODES` (refusal, exit 3) that `pane launch`, `pane relaunch` and `profile apply` end with (E12, E19, E41, E43) and that `pane import` ends with (it comes from `herdr.version()`, which ends the run with no report; it is never a finding of the import, ADR-0021's import row and `import/findings.rs`). The guide says so and points to section 8.
- **Panes outside the registry.** Each Herdr pane that no record names (a Claude Code pane, a plain shell) is reported as `unregistered-herdr-pane` in every whole-fleet pass, and no Holler verb adopts it (E16, E17, E46). A pass that names a pane or a profile does not report it, because a pass is whole-fleet only when it names neither (E16, E66). The finding has no remedy command (E13).
- `profile-drift` is a PROPOSED finding (#665, R14): it is a code of `ALL_CODES` and not yet a kind in `FindingKind::ALL` (E12, E19, E43). It appears only on a line that says `PROPOSED`.

**6. Profiles.** What a profile is: a named set of pane specs, kept in the registry; a pane belongs to at most one profile (ownership policy C3, E43 `pane-in-other-profile`); a profile holds names and non-secret values, never a secret, so an env var is a NAME only (I7, E40, `profile-secret-refused`). `create` (with `--from-current`: a snapshot of every pane, each becomes a member; `--from PROFILE` is a detached copy; E32), `delete` (refused while panes are live with `profile-has-live-panes`; with `--keep-panes` each member is detached first and keeps running; E33), `list`, `show` (specs, live panes and, per pane, every field that differs from its spec; reads only), and `apply`, whose reference is "The `profile apply` reference" below.

**7. Grid notation.** Positions are rowcol and 1-based: text `r2c1`, JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first (E25, E45). Input accepts `r<row>c<col>`, `c<col>r<row>` and a bare `<row>,<col>` (always row first, never column first); upper case and surrounding whitespace are accepted. The refusals `grid-ambiguous` and `grid-out-of-range` (E25, E45). **Pane names such as `hj-c1r2` predate the standard and list the column first, so `hj-c1r2` sits at `r2c1`; no pane is renamed and a name is never read as a position** (E38, E45). The section carries the example block of acceptance 5, including the pane name itself as a refused form, `refused grid-ambiguous hj-c1r2`: `GridPos::parse` refuses it as unreadable, because it holds no comma and a labelled cell must start with `r` or `c` (E26, lines 107-108).

**8. Probe, argv, layout, Herdr version.** The health probe, with the example `check ["curl","-s","http://127.0.0.1:8095/v1/models"]`, `expect ["qwen38"]` and the refusal `probe-failed`; the argv rule (`--command-arg` / `--command-json`, `--check-arg` / `--check-json`, never a shell string, `command-not-argv`; E5); layout reached by splits, a position inside the workspace's extent and never a relocation of a healthy pane (E45); the Herdr API version, which `doctor` shows, and the finding and error code `herdr-version-unsupported` (section 5).

**9. JSON output reference.** `--format=json` (and `--json`, which means `--format=json` under `pane` and `profile`; `--json --format=text` is a usage error) prints exactly one envelope per result with `schema_version` 1 (E23, E42, E44): `{"schema_version":1,"ok":true,"data":{},"error":null}` and `{"schema_version":1,"ok":false,"data":null,"error":{"code":...,"message":...}}`; `error` is `null` exactly when `ok` is true and `data` is `null` exactly when it is false (E24). `pane watch` is the one exception: its JSON output is a stream, one envelope per line (NDJSON), resumable with `--since CURSOR` (E9, E23). Adding a field does not change `schema_version`; removing, renaming or retyping one does (E44). Then:
- **The exit codes**, a table with one row each for 0 (ok), 1 (runtime failure), 2 (usage) and 3 (refusal), the same in text and JSON mode (E21, E23, E42). A row begins `| 0 |`, `| 1 |`, `| 2 |` or `| 3 |`.
- **Every error code** of `ALL_CODES` (E20), one table row each, beginning `` | `code` | `` and carrying the class word and number exactly as `class_of` gives it: `usage (2)`, `refusal (3)` or `failure (1)` (E21, E22, E43). `profile-drift` is the one row that also says `PROPOSED`.
- The code list is the *closed* set. A verb or an adapter owns further **open** codes, which are not in `ALL_CODES` and are always a refusal, exit 3 (E18, E22, E43); `agent-unusable` and `agent-key-invalid` of section 2, `orchestrator-not-relaunched` and `spec-not-launchable` of `profile apply` and, for the import, `import-blocked` are examples (R6, R9, R13; #642 part 3, #664 and #650 are merged on `main`).

**10. Migration and cutover.** The procedure of #654 (R16) as this guide states it; where it differs from #654's body (SYNC from `pane list` and `doctor` and not from `holler roster`; an overnight watch of the first pane and not a day) this guide is the standing text, as "Where this brief is the decision" point 6 says. In order:
1. **Have the launcher table.** `holler pane import` reads `STATE_ROOT/launcher.toml`, or the file `--launcher FILE` names: one TOML table per pane, named as the pane, with `port` (1 to 65535), `cwd` (an absolute path) and `command` (an array of strings, an argv; a string is the finding `command-not-argv`); no other key is accepted (ADR-0021 section 13, which #650 defined; R5). No file the old launcher wrote has this shape, so the operator provides it, and the guide shows a two-line example in a `toml` fence. **Without it every pane is `source-missing` (a problem), so the dry run reports it for every pane and a real run is refused with `import-blocked`.** If the live fleet's table has another shape, the dry run says so (a `source-unreadable` for every pane).
   Then run `holler pane import --from STATE_ROOT --herdr-session NAME --dry-run` on the live fleet and read every finding (#650, merged: R4, R6; the form must parse against the clap tree, because the bare `holler pane import --dry-run` fails `docs_cli_test` with a missing required argument). Name `--orchestrator PANE` for the orchestrator (MO) pane, which needs its own `pane.env`. Set `HOLLER_HERDR_SESSION` and pass the same name as `--herdr-session` (section 2): the flag stays required.
2. Fix drift with the old tools. Files that disagree about a pane's session give a finding and no record for that pane, and the real run is refused (`import-blocked`) until they agree (#650, merged: R8, decision 4).
3. Import. **A second real run changes nothing and says why:** a pane that already has a record is `already-imported` and is skipped before any other check, and when every pane is `already-imported` the run has nothing to write and exits 0 (it never reaches the check of the profile `fleet`). `profile-exists` (refusal, exit 3, nothing written) ends a real run only when there is a pane to record while the profile `fleet` already exists, that is, when a *new* pane directory has appeared since the first import (ADR-0021 section 9's import row; `pane/import/writes.rs`; R5, R6). A real run writes the records and the profile `fleet` (from today's panes and their layout), or nothing at all; the layout is each pane's position as Herdr's snapshot shows it (the entry whose label is the pane's name), never a position read from a name; and a session id read from a file becomes the session of record only when a healthy server's session list holds it, so the import reads the old files as the source of the records it creates and never as observed state (I6; R5, R8). A real run is refused with `import-blocked` (exit 3, nothing written) while any pane has a problem; a dry run exits 0 whatever it found.
4. Run `holler pane doctor` until it reports no `shown-driven-mismatch` and `pane list` shows SYNC `ok` for every pane (SHOWN equals DRIVEN, as section 4 reads it).
5. Cut **one** pane over to `launch` / `reset` and watch it **through a full overnight**, then move the rest.
6. Seven days with no invisible session: every day `doctor` reports no `shown-driven-mismatch` and no stray session older than an hour (R16). A pass keeps no memory (E46), so the guide says to read "older than an hour" by comparing the `session` of each `stray-session` finding between passes an hour apart. The operator signs off.
7. Record each step in `docs/handoffs/<#654>-cutover/decisions.md`.

The files the old launcher wrote are `sessions.toml`, `last-session`, `owned-sessions` and `pane.env` (the guide names each); the launcher table of step 1 is the import's own input and ADR-0021 section 13 lists it beside them. They are **replaced by registry state** and are no longer read after the cutover; ADR-0021 section 13 has the table of what replaces each (E47) and the guide links it and does not restate it. Retiring the launcher's writers is pfleet's follow-up (#653, in another repository; R17). The files are kept, read-only, for one release, with one exception: the fallback script still writes `pane.env` when it is used as a rollback (section 11). A pane that does not run OpenCode gets no record from the import; after the import a whole-fleet `doctor` reports it as `unregistered-herdr-pane` in every pass, which is expected and is not counted against the seven days (section 5).

**11. Rollback.** **`scripts/pane-restart` stays as the fallback until the 7-day sign-off**; the guide says so in these words and shows its OpenCode usage line (E50 line 5), `scripts/pane-restart PANE --model PROVIDER/ID [--effort E] [--key-file PATH]`, in a `bash` fence (a line that starts `scripts/`, not `holler `). Then:
- **After the sign-off** only the script's OpenCode path is retired, in a later story and not this one. Its Claude Code path (issue #784) stays, because a Claude Code pane is not in the registry and no verb restarts it (the boundary above). Claude Code panes are not part of the 7-day criterion.
- **What using the script on a registry pane does.** It takes that pane out of the new regime: it rewrites `pane.env` and starts a TUI on a new session that is no pane's session of record, and the record's model and session of record go stale (E50 lines 8-15). The next `doctor` reports what it observes about the pane; the kinds it can report are in section 5, and the guide does not predict which one (it depends on where the script started the new session): a `shown-driven-mismatch` when the TUI shows a session on the recorded server that is not the session of record, a `tui-foreign-session` when the TUI is on another server, and a `stray-session` for the new session (E12). `--fix` repairs only the mismatch by selecting the *old* session of record, which undoes the script's switch, so **do not run `--fix` on a pane that was restarted with the script**.
- **The way back** is `holler pane relaunch PANE --model PROVIDER/ID`, which restarts the server, keeps the session of record the restarted server still lists (and creates a new one only when it does not), and records the new model (E65, R11; `tx_launch.rs` lines 27 and 680-688); then `doctor` is clean for that pane again. The same command is the way back from an `agent-unusable` refusal of a `relaunch --agent KEY`: that refusal comes after the old server was stopped, so the pane has no server and its record is unchanged until `holler pane relaunch PANE` runs again without `--agent`.
- The use of the fallback is a rollback and is recorded as one in `docs/handoffs/<#654>-cutover/decisions.md`.

**12. Later.** Deferred, so not documented as existing: the roster extension (#648), the shared subprocess runner (#696), the real-adapter scenario for `profile apply` (#667), the workspace recreation from profiles (#666) and the PROPOSED profile verbs and finding (#665). The guide names them under this heading, and describes none of them as available. Where another section needs one of them (#648 with its `later` line in section 4; #665 in the PROPOSED clauses of sections 3, 5 and 9), that section names it once there.

### `docs/orchestrating.md`

The section "Restarting a pane on another model (interim)" (E49, lines 108-145) is rewritten, not deleted; the rest of the page is unchanged. The new section:
1. Opens with `holler pane relaunch PANE --model PROVIDER/ID [--effort E]` as the way to restart a registry (OpenCode) pane on another model, in a `bash` fence, with an example that parses against the clap tree, and links `pane-control.md`.
2. Names `scripts/pane-restart` as the **fallback until the 7-day sign-off**, replacing both the sentence "This section is removed when #644 and #653 have merged" and the opening "Until holler pane relaunch with --model (#644) and the pfleet adoption (#653) land" (E49 lines 110-111: #644 has merged, and #653 is in another repository and has not happened, R17), keeps its OpenCode usage and its notes (E49 lines 110-123), and says that a pane restarted with it leaves the registry's control until it is relaunched (guide section 11).
3. Keeps the **Claude Code subsection** (E49 lines 125-145) and adds that a Claude Code pane is not in the registry, so `holler pane` cannot restart it, `scripts/pane-restart` is its way, and the sign-off does not retire that path.
4. Keeps the heading text free of "(interim)" only if no page links the anchor; none does (C2), so the heading is `## Restarting a pane on another model`.

### `scripts/pane-restart`

Line 3 (E50) says "A stop-gap (issue #746) until `holler pane relaunch --model` (#644) and the pfleet adoption (#653) land." It contradicts the retention rule. It becomes one comment line (or two) saying that for an OpenCode pane the script is the fallback for `holler pane relaunch --model` until the 7-day sign-off of #654, and that its Claude Code path (issue #784) has no replacement. **A comment only; no behaviour changes.** Nothing but line 3 refers to the sentence (C1), and the script's own CI test (E51) is run unchanged.

### `CHANGELOG.md`

One entry under `## [Unreleased]` / `### Enhancements` (E52; the structure is `docs/releasing.md`'s) that links issue #652, names the guide, the rewritten section and the retained fallback, and says no behaviour changes.

## The `profile apply` reference

#664 is merged: the text below is what it built, checked against `main` at `5d366b9`. The standing spec is the paragraph "Apply as built (#664)" of ADR-0021 section 8 with its definition of "adopt", section 9's apply row, `crates/holler-pane/src/tx_apply.rs` and `crates/holler-cli/src/profile/apply.rs`; the guide cites ADR-0021 for the contract, and the build run checks each wording against the merged ADR and code. R9 and R10 are the decided contract of the #664 brief, which the operator took on 2026-10-11 (the brief is also on `main` as `docs/handoffs/664-brief.md`); E34 shows the stub that #664 replaced.

- One step per spec pane in (workspace, row, col) order; the first matching row applies: **a** another profile and no `--take-over`: `refused` (`pane-in-other-profile`); **b** no record: `create` through the launch transaction; **c** healthy and matches its spec: `adopt` (sets `Pane.profile`; with a take-over from another profile, leave then join, two writes); **d** healthy and differs: `reported` with the pane's diff, nothing live touched; **e** not healthy, role orchestrator: `refused` (`orchestrator-not-relaunched`), no write even under `--take-over`; **f** not healthy, role agent: `relaunch`, and under `--take-over` from another profile the leave and join happen **first**, then the relaunch.
- A step is `{pane, action, outcome, code, diff, detail}`. **Actions:** `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name) and `observe` (a failed observation). **Outcomes:** `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`, and `planned` (a dry-run step that would act; row d's is `reported`). `refused` and `reported` do not stop the run or change the exit code. A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the run; the later steps are `skipped`, but a failed run prints no steps, so `skipped` appears only in the failure's message count. A `usage` answer of the launch for a step's spec is refused with the open code `spec-not-launchable`. An absent `code`, `diff` or `detail` prints `null` (it is not omitted). A step refused or failed after its launch or relaunch had begun carries the reconcile step (R9, R10: the #664 brief's C22 and C25, built as written). Exit code: 1 if any step failed, 2 for the up-front usage case (a plan with a `create` and no `--herdr-session`), else 0.
- A run that ends with a `failed` step prints **only the failure**: in JSON the failure envelope (`ok` false, `data` null, `error.code` the failed step's error code, `error.message` is `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`), in text one `error:` line on stderr and nothing on stdout; a dry run with a failed step prints no plan. This is the same failure envelope every `pane` and `profile` verb prints (`data` null when `ok` is false, E24).
- `--dry-run` prints the same table as the plan (for a run that does not fail), runs no probe, issues no split and writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code, diff, detail}]}` (the keys always present, `null` when absent). Flags: `--herdr-session NAME`, `--take-over`, `--dry-run`, `--format`.
- "Healthy" is a live observation (the harness answers its health call, Herdr's snapshot lists the recorded pane, and a readable shown session equals the session of record), never the stored health; apply never kills a healthy pane; a shown session that cannot be read does not make a pane unhealthy.
- `orchestrator-not-relaunched` and `spec-not-launchable` are *open* codes of `apply` (refusals, exit 3; not in `ALL_CODES`), named beside `agent-unusable` where section 9 lists open codes.

## Operator decisions (2026-10-11)

Decisions 1 to 7 are the filed ones (R18); decision 8 is new and amends 2 and 5.

1. **The guide documents `pane list` and `doctor` for SHOWN/DRIVEN and marks the roster extension "later".** Reason: #648 is deferred until after the cutover, and the guide must not describe a column that does not exist.
2. **`scripts/pane-restart` stays as the rollback until the 7-day sign-off.** Reason: it is the only restart path that does not depend on the new registry, and removing it before sign-off removes the way back. (Amended by decision 8 for what remains after the sign-off.)
3. **The grep tests derive their lists from the code's own tables**: the verbs from the clap tree (`Cli::command()`), the finding kinds from `FindingKind::ALL`, the error codes from `ALL_CODES` and their classes from `class_of`, the columns from `COLUMNS`. Reason: a hand-kept list in the test drifts from the code (the repo's single-source rule). The one hand-kept list is the PROPOSED set, which the code has no table for, and its doc names #665 as the story that changes it.
4. **The `profile apply` reference is the decided shape of the #664 brief, as built**, with the O1 to O6 decisions. Reason: one contract, stated once.
5. **`orchestrating.md`'s interim section is rewritten, not deleted**, and names the script as a fallback until the sign-off. Reason: the section's own deletion condition (#653) is in another repository and has not happened.
6. **The cutover procedure keeps the overnight watch on the first pane.** Reason: a day-scale defect (idle drift, timeouts, restarts) shows only over time, and a staged cutover exists to find it on one pane.
7. **Deferred after the cutover:** #648, #696, #667, #666. Reason: none gates #654.
8. **Decision (a): `holler pane` manages only OpenCode panes; after the 7-day sign-off only `scripts/pane-restart`'s OpenCode path is retired.** Claude Code panes keep using the script. Reason: `HarnessKind` is closed to `opencode`, the import creates no record for any other pane, and the script's Claude Code path is the only restart path this repository has for such a pane. Option (b), requiring every fleet pane to run OpenCode before the cutover, would change #654's procedure and was not chosen.

## How to write the guide so `docs_cli_test` passes

`docs_cli_test` reads `README.md` and `docs/**/*.md` except `docs/handoffs/` (E67), takes every line inside a fence tagged `bash`, `sh`, `shell`, `console`, `text` or untagged that starts `holler ` and every one-backtick span that starts `holler `, normalises placeholders, and parses each against the clap tree; it tolerates only a bare namespace such as `holler pane` (E53 to E55). So:

1. **Fences are three backticks and use an allowed tag.** A `~~~` fence or a fence tagged `zsh` or any other tag is never scanned, so the guide has no `~~~` line at all and no fence outside the allowed tags that contains a line starting `holler ` (the test pins this, acceptance 12). A fence tagged `json` holds output examples, never commands.
2. **Typed values take a number or sit inside `[...]`:** `holler pane watch --since 42`, not `--since CURSOR` (a `u64`, E10); `--ctx-soft 800000` and `--ctx-hard 900000`, not `N` (`u32`, E5).
3. **Ellipses are dropped, so they hide an incomplete command.** The normaliser removes both `…` and `...` (`docs_cli_test.rs` line 149), so a span that ends `--model ...` parses and checks nothing. The guide writes the full form, `holler pane relaunch PANE --model PROVIDER/ID`, and uses no ellipsis inside a `holler` span.
3a. **Quotes and leading hyphens.** The splitter honours only double quotes (`fixture::split_args`): `"a b"` is one token and a single quote is a literal. A value that starts with `-` is read by clap as a flag, so write it in the `=` form, `--check-arg=-s`, never `--check-arg -s` (the flag has no `allow_hyphen_values`, E5).
4. **The PROPOSED profile verbs appear bare**: `holler profile export`, on a line that says `PROPOSED`; their planned arguments are prose (E35). This story cannot list them in `cli-surface.pending.txt` (it is #670's file, which holds only comments today, E60).
5. **Every command example parses against the merged tree**: `holler pane import` needs `--from STATE_ROOT` (and the guide's form adds `--herdr-session NAME`, R4), and `holler profile apply` needs `NAME`. Both verbs are real on `main` (E34 and E36 show the stubs they replaced), so the bare `holler pane import --dry-run` and the bare `holler profile apply` fail with a missing required argument, and the guide writes the full forms.
6. **No bare command fragment appears in backticks** (E63). A bare namespace such as `holler pane` alone parses as a prefix of the tree and is allowed.
7. **The words `profile rename`, `profile export` and `profile import` appear only on a line that says `PROPOSED`**, and so does `profile-drift` (acceptance 1). Prose such as "the profile import" is not allowed anywhere else.
8. **A verb span carries its required arguments** (section 3 lists the forms). A span without them fails `MissingRequiredArgument`.

## Acceptance (each is a test)

All new tests live in `crates/holler-cli/tests/docs_pane_control_test.rs`, declared as `[[test]] name = "docs_pane_control_test"` with `path = "tests/docs_pane_control_test.rs"` in `crates/holler-cli/Cargo.toml` (the crate sets `autotests = false`, E58; the entry follows `docs_cli_test`'s, E59). The test reads `docs/pane-control.md`, `docs/orchestrating.md`, `docs/README.md` and the head of `scripts/pane-restart` from the workspace root, the way `docs_cli_test.rs` does (E53). Its shape:

- It starts with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149`, the line `docs_cli_test.rs` carries (lint check 1 needs the trailing `// #NNN`, E62).
- A helper slices section N of the guide from `\n## N. ` to the next `\n## ` (E57). Every test collects all of its failures into a list and asserts once at the end, so one run shows every missing item (the pattern of `docs_cli_test.rs`, E55, lines 206-211).
- The clap tree is walked with `holler_cli::Cli::command()` and `get_subcommands()`, filtering clap's own `help` (E56).
- The PROPOSED set is one constant, `PROPOSED_PROFILE_VERBS = ["rename", "export", "import"]`, whose doc names #665 as the story that changes it (it removes a verb when it builds it, and then edits the guide's PROPOSED clauses and this test).

The tests:

1. **Verbs.** Every subcommand of `pane` and of `profile` (`find_subcommand("pane")` and `("profile")`, `get_subcommands()`, `help` filtered) appears in the guide as `holler pane <verb>` / `holler profile <verb>` inside a heading line or a one-backtick code span that starts with that text and carries the verb's required arguments (section 3), so `docs_cli_test` parses it. **One PROPOSED rule, the line rule:** for each verb `v` of `PROPOSED_PROFILE_VERBS`, every line of the guide that contains `profile v` also contains `PROPOSED`, and at least one such line exists; and the same for every line that contains `profile-drift`. The guide does not contain `holler roster --profile`.
2. **Finding kinds.** Every `FindingKind::ALL[i].code()` appears in backticks in section 5.
3. **Error codes.** Every string of `holler_pane::error::ALL_CODES` appears in backticks in section 9, on a table row that begins `` | `code` | `` and contains the class word and number of `class_of(code)`: `usage (2)`, `refusal (3)` or `failure (1)`, built from `ErrorClass::exit_code`, not typed. The section has a row beginning `| 0 |` and one for each value of `ErrorClass::{Usage, Refusal, Failure}.exit_code()`. A code that is both a finding kind and an error code (computed as the intersection of `FindingKind::ALL[i].code()` and `ALL_CODES`, never typed; at `7284001` it is `herdr-version-unsupported` alone) appears in backticks in section 5 **and** in section 9.
4. **Columns.** Every name in `holler_cli::pane::list::COLUMNS` appears in section 4.
5. **Grid examples parse.** Section 7 carries a fenced block tagged `text` with one example per line, `ok <form>` or `refused <code> <form>`. The test parses each `ok` form with `holler_pane::GridPos::parse` and asserts each `refused` form fails with the named code (`grid-ambiguous` or `grid-out-of-range`). The block contains at least the forms `r2c1`, `c1r2`, `2,1` and `R2C1` (all four parse to row 2, column 1), `refused grid-ambiguous hj-c1r2`, and one refused form of each code.
6. **The existing docs test still passes.** Every `holler ...` command in the new guide parses against the clap tree; `docs_cli_test` already scans `docs/**/*.md` and is part of the verify command; the new test does not repeat its extraction. (A bare command fragment is the check of E63.)
7. **Roster is marked later.** Section 4 contains a line with `holler roster`, `#648` and the word `later`.
8. **Rollback and the boundary.** The guide contains `scripts/pane-restart` and `until the 7-day sign-off`; section 11 contains `holler pane relaunch` and `--fix`; section 1 contains `Claude Code`; the guide contains `harness-not-opencode` and a line that contains both `Claude Code` and `scripts/pane-restart`. `docs/orchestrating.md` contains `holler pane relaunch`, `scripts/pane-restart` and `Claude Code`, and no longer contains `removed when #644 and #653 have merged`. The head of `scripts/pane-restart` (its first ten lines) contains `until the 7-day sign-off` and no longer contains `A stop-gap`.
9. **Replaced files.** Section 10 names each of `sessions.toml`, `last-session`, `owned-sessions` and `pane.env`, names the launcher table (`launcher.toml` and `--launcher`) with its finding `source-missing`, and contains `ADR-0021.md`.
10. **Index.** `docs/README.md` links `pane-control.md` under "Guidance" (the text from `\n## Guidance` to the next `\n## ` contains `](pane-control.md)`).
11. **Headings.** The guide has the twelve `\n## N. ` headings, N = 1 to 12, in order.
12. **Nothing escapes the docs test.** No line of the guide starts with `~~~`, and no fence whose tag is outside `""`, `bash`, `sh`, `shell`, `console`, `text` contains a line that starts `holler ` (E54).
13. **Envelope examples.** Each fence tagged `json` in section 9 that starts `{"schema_version"` passes `holler_pane_testkit::envelope::check_envelope(block, exit)` with `exit` 0 for an `ok` envelope and `class_of(error.code).exit_code()` for a failure (E61; `holler-cli` dev-depends on the test kit). If the checker rejects an example, the example is wrong and is fixed (an example must be a true envelope); the test is not dropped.
14. **The import (#650, merged: R7).** Every code of `holler_pane::import::findings::ImportFinding::ALL`, and `import-blocked` (`holler_pane::import::IMPORT_BLOCKED`), appear in backticks in section 3 or section 10. Six of those codes are also closed error codes or doctor kinds (section 3 says which is which), so this test pins only that each is named, never in which section it also appears.
15. **`profile apply` (#664, merged: R9, R10).** The guide's apply reference contains each outcome word (`created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`, `skipped`, `planned`), each action word (`create`, `adopt`, `report`, `relaunch`, `refuse`, `observe`), `detail`, `orchestrator-not-relaunched` and `spec-not-launchable`; `tx_apply` exposes no table of its actions or outcomes (`Action` and `Outcome` each have only `as_str`), so the lists are one constant in the test whose doc names ADR-0021 section 9's apply row.
16. **CHANGELOG.** `CHANGELOG.md` has, between `## [Unreleased]` and the next `## [`, a line that links `/issues/652` and contains `pane-control.md`.
17. **Content the audit found missing.** Section 2 contains `agent-key-invalid`, `agent-unusable` and `--herdr-session`; section 5 contains `not-fixable`; section 10 contains `profile-exists`, `already-imported`, `--herdr-session`, `launcher.toml` and `source-missing`; section 3 contains each verb's heading form listed in "The verbs" (computed from the clap tree's required arguments, not typed: the test builds the argv `holler pane <verb>` plus, for each required positional and required flag in the clap tree's argument order, the flag name (for a flag) and a placeholder, and asserts that string appears in the guide). **The placeholder is the argument's first `value_name` when it has one (`PANE`, `TEXT`, `WHEN`, `STATE_ROOT`, `NAME`), else its id upper-cased**; a `<pane>` or the bare id does not match the headings of section 3.

The verify command is the workspace test of `.aftersight/pipeline.config.json` (the CI workspace test with its one skipped timing test), which includes `docs_cli_test`, `cli_surface_test` and the new target, plus `bash scripts/tests/pane-restart-test.sh`.

## Blast radius

- `docs/pane-control.md` (new), `docs/orchestrating.md`, `docs/README.md`
- `crates/holler-cli/tests/docs_pane_control_test.rs` (new) and its `[[test]]` entry in `crates/holler-cli/Cargo.toml`
- `scripts/pane-restart` (line 3 only, a comment)
- `CHANGELOG.md` (an `## [Unreleased]` entry linking this issue)

No source file outside tests changes, and no behaviour changes. `docs/adr/ADR-0021.md` is read, not edited. Markdown files under `docs/` are scanned by `docs_cli_test`, so the guide obeys the rules above.

## Facts that wait for other stories

On `main` at `7284001` all of these were OPEN (C4), except #802. Since the cut, #644 part 2 (`b8242d7`), #642 part 3 (`3286486`), #664 and #650 have merged (`main` at `5d366b9`): their lines are patched from the merged code, and the build run re-cuts R4 to R13 and E29, E34, E36 from `main`. Of the stories below only #649 is still open (#665 and #648 stay deferred). For each, the build run re-cuts the excerpt (the same `sed -n` command, against merged `main`) and patches the line before it writes the guide.

| Story | What the brief expects | Where it appears | Excerpt to re-cut |
|---|---|---|---|
| #649 (open) | no hub: `unavailable`, exit 1; read verbs work without Herdr; the environment variables live in the module doc of `wiring.rs`; DRIVEN prints `-` until #654 | guide sections 2 and 4 | R1, R2, R3 (its body was edited after the cut: re-cut) |
| #650 (merged) | the import as built: synopsis (`--from`, `--herdr-session`, `--orchestrator`, `--launcher`, `--dry-run`); the launcher table; join by label; exit codes, `import-blocked` and the no-op re-run; the sixteen findings; `harness-not-opencode`; ADR-0021 section 13 and the import row | guide sections 3 and 10, acceptances 9, 14 and 17 | R4 to R8; E36, E41, E47 |
| #664 (merged) | the whole `profile apply` reference as built; its flags and example; `orchestrator-not-relaunched`; ADR-0021 "Apply as built (#664)" and section 9's apply row | guide sections 3 and 6, the reference, acceptance 15 | R9, R10; E34, E41 |
| #644 part 2 (merged) | the agent in the output: `, agent KEY` and the JSON key `agent` | guide section 2 | R11 |
| #642 part 3 (merged) | `agent-unusable`, the open code, which the OpenCode adapter answers instead of `agent-unsupported` | guide sections 2 and 9 | R12, R13; E29 |
| #665 | stays PROPOSED; if it merges first, remove the PROPOSED clauses and the constant | guide sections 3, 5, 9; acceptance 1 | R14; E3, E12, E35 |
| #648 | stays "later" | guide section 4, acceptance 7 | R15 |

Excerpts of `main` that the merged stories edited, or that a later story edits, and that the build run therefore re-cuts: E1 and E2 if a verb changes; E3 and E35 (#665); E12 (a new `FindingKind` such as `profile-drift`); E4, E5, E29, E31, E64 (#644 part 2, #642 part 3: doc comments and line shifts); E34 (#664); E36 (#650); E52 (the CHANGELOG head now carries the #650 and #664 entries); E38 to E47 (ADR-0021, edited by #650, #664, #644 part 2 and #642 part 3: every line range moved).

## Files named by this brief

Checked by `git cat-file -e main:PATH` on `main` at `7284001` (the cut); the same files exist and the two new ones do not at `5d366b9`.

| File | Role in this story | On `main` at the cut |
|---|---|---|
| `docs/pane-control.md` | NEW (this story writes it) | **no** |
| `crates/holler-cli/tests/docs_pane_control_test.rs` | NEW (this story writes it) | **no** |
| `docs/orchestrating.md` | edit (one section) | yes |
| `docs/README.md` | edit (one index line) | yes |
| `scripts/pane-restart` | edit (line 3, a comment) | yes |
| `CHANGELOG.md` | edit (one entry) | yes |
| `crates/holler-cli/Cargo.toml` | edit (one [[test]] entry) | yes |
| `docs/adr/ADR-0021.md` | read (the standing spec; edited by #650, #664, #644 part 2, #642 part 3, not by this story) | yes |
| `docs/releasing.md` | read (the CHANGELOG entry structure) | yes |
| `README.md` | read (scanned by docs_cli_test) | yes |
| `crates/holler-cli/tests/docs_cli_test.rs` | read (the rules the guide obeys) | yes |
| `crates/holler-cli/tests/cli_surface_test.rs` | read (the clap-tree walk) | yes |
| `crates/holler-cli/tests/fixtures/cli-surface.pending.txt` | read (names no command; not edited) | yes |
| `crates/holler-proto/tests/docs_errors_test.rs` | read (the section-slicing precedent) | yes |
| `scripts/tests/pane-restart-test.sh` | read (run unchanged) | yes |
| `scripts/lint.sh` | read (lint check 1) | yes |
| `.github/workflows/ci.yml` | read (the script's CI step) | yes |
| `.aftersight/pipeline.config.json` | read (the verify command) | yes |
| `plugins/herdr-holler/herdr-plugin.toml` | read (the display plugin) | yes |
| `docs/agent-overlays/tester.md` | read ("bare command fragment") | yes |
| `crates/holler-cli/src/pane/wiring.rs` | read (module doc lists the environment; rewritten by #649) | yes |
| `crates/holler-cli/src/profile/apply.rs` | read (the merged #664 verb; a stub at the cut) | yes |
| `crates/holler-pane/src/tx_apply.rs` | read (the merged #664 engine; a stub at the cut) | yes |
| `AGENTS.md` | out of scope (not edited) | yes |
| `agent-skills/` | out of scope (not edited) | yes (directory) |
| `docs/handoffs/<#654>-cutover/decisions.md` | created by the operator during #654; not by this story | no (created later) |

## Depends on

- **To write and run the guide's tests:** #649 (the hub wiring), the one story still open. Merged: #650 (the import's arguments, which the docs test needs to parse) and #664 (the `profile apply` arguments and contract). The issue as first filed named #649 and #664; #650 was added because the cutover command is a documented example and `holler pane import` was a stub with no arguments until #650 merged (E36, E60).
- **To state the behaviour accurately (merged):** #644 part 2 (the agent in the output) and #642 part 3 (`agent-unusable`). The brief states their contracts (R11 to R13), checked against `main`.
- **Already merged:** #802 (the server runs in the pane's tmux session; E28, E29, E30, E39), #651 (the display plugin; E37), and #650, #664, #644 part 2 and #642 part 3 above.
- If #649 (or any story after it) changes a behaviour stated above, the brief is patched and the review re-run before the build.

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-652-implementation`.

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it without waiting for another story's code. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius; the shared hot spots in the epic are owned by one story each. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.

## Evidence (verbatim)

Cut on 2026-10-11 11:14 MDT from `main` at `7284001`. Every excerpt below was produced by a command (shown under its heading) that pipes the source through `sed -n 'A,Bp'`; the line numbers on the left are added by `awk` after the cut. Nothing is retyped. An `E<n>` excerpt is code or docs on `main`. An `R<n>` excerpt is the expected contract of a story that has not merged (an issue body or a branch), cut by the same `sed -n` and re-cut after that story merges (see "Facts that wait for other stories"); an `E<n>` that appears inside an `R<n>` excerpt names that issue's own evidence list, not this one. A `C<n>` is the output of a command.

**E1: `crates/holler-cli/src/cli.rs:101-127`** — The top-level command enum: `Pane` and `Profile` are subcommands, and `Roster` is the one verb that #648 would extend.

Command: `git show main:crates/holler-cli/src/cli.rs | sed -n '101,127p'`

~~~~text
101	#[derive(Subcommand, Debug)]
102	pub enum Command {
103	    /// Operate a hub (serve, tokens, status, caps, support, query).
104	    Hub(Hub),
105	    /// Operate a body (join, run, detach, status, caps, support, query, attach).
106	    Body(Body),
107	    /// List the roster. (hub-only, top-level daily verb)
108	    Roster(Roster),
109	    /// Send a one-shot prompt to a session and print the reply. (hub-only)
110	    Say(Say),
111	    /// Interrupt the session's in-flight turn. (hub-only)
112	    Interrupt(Interrupt),
113	    /// Resolve a held permission/elicitation. (hub-only)
114	    Answer(Answer),
115	    /// Block until a session settles/blocks/fails/vanishes. (hub-only)
116	    Wait(Wait),
117	    /// Refuse new work to a session until it is released. (hub-only)
118	    Hold(Hold),
119	    /// Lift a hold set with `hold`. (hub-only)
120	    Release(Release),
121	    /// Control panes: launch, switch, park, close, doctor, import. (epic #633)
122	    #[command(subcommand)]
123	    Pane(PaneCmd),
124	    /// Manage profiles, the named sets of pane specs. (epic #633)
125	    #[command(subcommand)]
126	    Profile(ProfileCmd),
127	}
~~~~

**E2: `crates/holler-cli/src/pane/mod.rs:34-49`** — The `holler pane` verbs (acceptance 1 reads them from the clap tree, never from a list in the test).

Command: `git show main:crates/holler-cli/src/pane/mod.rs | sed -n '34,49p'`

~~~~text
34	/// The `holler pane` verbs.
35	#[derive(Subcommand, Debug)]
36	pub enum PaneCmd {
37	    List(list::PaneList),
38	    Get(get::PaneGet),
39	    Watch(watch::PaneWatch),
40	    Launch(launch::PaneLaunch),
41	    Relaunch(relaunch::PaneRelaunch),
42	    Switch(switch::PaneSwitch),
43	    Reset(reset::PaneReset),
44	    Park(park::PanePark),
45	    Unpark(unpark::PaneUnpark),
46	    Close(close::PaneClose),
47	    Doctor(doctor::PaneDoctor),
48	    Import(import::PaneImport),
49	}
~~~~

**E3: `crates/holler-cli/src/profile/mod.rs:1-37`** — The `holler profile` verbs. `rename`, `export` and `import` are PROPOSED (#665, line 11); `--take-over` is `apply`'s own flag (line 8).

Command: `git show main:crates/holler-cli/src/profile/mod.rs | sed -n '1,37p'`

~~~~text
1	//! `holler profile ...` (epic #633): a profile is a named set of pane specs, kept in the same
2	//! registry as the panes.
3	//!
4	//! **Frozen by story #670.** This file lists the verbs and dispatches them; it never changes
5	//! again. One verb is one file in this directory with its own clap `Args` struct and its own
6	//! `run`, owned by one story. Until then each verb is a stub that refuses with
7	//! `not implemented (story #NNN)`. No profile verb takes `--profile`: the profile is the verb's
8	//! own argument, so there is no `args.rs` here; `--take-over` is `apply`'s own flag.
9	//!
10	//! The verb modules are named after the verb; their `Args` structs are `Profile<Verb>`.
11	//! `rename`, `export` and `import` are proposed (#665) and wait for the operator to confirm them.
12	
13	pub mod apply;
14	pub mod create;
15	pub mod delete;
16	pub mod export;
17	pub mod import;
18	pub mod list;
19	pub mod rename;
20	pub mod show;
21	
22	use clap::Subcommand;
23	
24	use crate::output::VerbCtx;
25	
26	/// The `holler profile` verbs.
27	#[derive(Subcommand, Debug)]
28	pub enum ProfileCmd {
29	    Create(create::ProfileCreate),
30	    Delete(delete::ProfileDelete),
31	    List(list::ProfileList),
32	    Show(show::ProfileShow),
33	    Apply(apply::ProfileApply),
34	    Rename(rename::ProfileRename),
35	    Export(export::ProfileExport),
36	    Import(import::ProfileImport),
37	}
~~~~

**E4: `crates/holler-cli/src/pane/args.rs:1-33`** — The shared flag groups: `--profile` is on every `pane` verb but `import` (line 7), `--spec-only` is on `launch`, `relaunch` and `close` only and requires `--profile` (lines 9 and 31).

Command: `git show main:crates/holler-cli/src/pane/args.rs | sed -n '1,33p'`

~~~~text
1	//! The flag groups the `holler pane` verbs share (story #670, epic #633), defined once:
2	//!
3	//! - [`SpecFlags`]: one flag per field of a pane spec, used by `launch` and `relaunch` (and by
4	//!   the profile spec). Every value stays a string at clap time; [`SpecFlags::validate`] types
5	//!   them, with the guards of `holler-pane` and their stable error codes, so a bad `--grid` is a
6	//!   refusal with a code a script can match (exit 3) and not a clap usage error.
7	//! - [`ProfileOpt`]: `--profile NAME`, on every `pane` verb but `import`, and on `roster`,
8	//!   `say`, `interrupt` and `answer`.
9	//! - [`SpecOnly`]: `--spec-only`, on `launch`, `relaunch` and `close`.
10	//!
11	//! This is all that #670 declares for the verbs. The positionals and flags that belong to one
12	//! verb (`pane get PANE`, `pane doctor --fix`) are declared in that verb's own file by its owning
13	//! story.
14	
15	use clap::Args;
16	use holler_pane::pane::PaneRole;
17	use holler_pane::{AgentKey, Argv, EnvVarName, GridPos, PaneError};
18	
19	/// `--profile NAME`: scope the verb to a profile.
20	#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
21	pub struct ProfileOpt {
22	    /// Act on the panes of this profile; a named pane must belong to it.
23	    #[arg(long, value_name = "NAME")]
24	    pub profile: Option<String>,
25	}
26	
27	/// `--spec-only`: edit a profile's spec and change nothing live.
28	#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
29	pub struct SpecOnly {
30	    /// Edit the profile's spec for the pane and change nothing live.
31	    #[arg(long, requires = "profile")]
32	    pub spec_only: bool,
33	}
~~~~

**E5: `crates/holler-cli/src/pane/args.rs:35-91`** — The spec flags (`launch` and `relaunch`): `--agent`, `--env` (a name only), the argv rule (`--command-arg` / `--command-json`, `--check-arg` / `--check-json`), `--expect`. Typed values: `--ctx-soft`, `--ctx-hard` are `u32`.

Command: `git show main:crates/holler-cli/src/pane/args.rs | sed -n '35,91p'`

~~~~text
35	/// One flag per field of a pane spec.
36	///
37	/// The text flags keep the user's string, [`SpecFlags::validate`] types it. `--env` names a
38	/// variable and never carries its value; a command or a health check is given as repeated `ARG`
39	/// flags or as one JSON array, never both and never as a shell string.
40	#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
41	pub struct SpecFlags {
42	    /// The project directory or worktree the pane works in.
43	    #[arg(long, value_name = "DIR")]
44	    pub project: Option<String>,
45	    /// The Herdr workspace the pane lives in.
46	    #[arg(long, value_name = "NAME")]
47	    pub workspace: Option<String>,
48	    /// Where the pane sits in the Herdr grid, from 1: `r<row>c<col>`, `c<col>r<row>` or `<row>,<col>`.
49	    #[arg(long, value_name = "POS")]
50	    pub grid: Option<String>,
51	    /// The model the pane runs, as PROVIDER/ID.
52	    #[arg(long, value_name = "PROVIDER/ID")]
53	    pub model: Option<String>,
54	    /// The model's effort level.
55	    #[arg(long, value_name = "LEVEL")]
56	    pub effort: Option<String>,
57	    /// The pane's role: agent or orchestrator.
58	    #[arg(long, value_name = "ROLE")]
59	    pub role: Option<String>,
60	    /// The OpenCode agent the pane's messages run as: letters, digits, - and _. Without it, the
61	    /// server's default agent.
62	    #[arg(long, value_name = "KEY")]
63	    pub agent: Option<String>,
64	    /// An environment variable NAME the pane gets (a name only, never NAME=value). Repeatable.
65	    #[arg(long, value_name = "NAME")]
66	    pub env: Vec<String>,
67	    /// The soft context ceiling a watchdog reads.
68	    #[arg(long, value_name = "N")]
69	    pub ctx_soft: Option<u32>,
70	    /// The hard context ceiling a watchdog reads.
71	    #[arg(long, value_name = "N")]
72	    pub ctx_hard: Option<u32>,
73	    /// How the harness server's port is chosen.
74	    #[arg(long, value_name = "POLICY")]
75	    pub port_policy: Option<String>,
76	    /// One element of the launch command, program first. Repeatable; not with --command-json.
77	    #[arg(long, value_name = "ARG", conflicts_with = "command_json")]
78	    pub command_arg: Vec<String>,
79	    /// The launch command as a JSON array of strings; not with --command-arg.
80	    #[arg(long, value_name = "JSON")]
81	    pub command_json: Option<String>,
82	    /// One element of the health check, program first. Repeatable; not with --check-json.
83	    #[arg(long, value_name = "ARG", conflicts_with = "check_json")]
84	    pub check_arg: Vec<String>,
85	    /// The health check as a JSON array of strings; not with --check-arg.
86	    #[arg(long, value_name = "JSON")]
87	    pub check_json: Option<String>,
88	    /// A string the health check's output must contain. Repeatable.
89	    #[arg(long, value_name = "STR")]
90	    pub expect: Vec<String>,
91	}
~~~~

**E6: `crates/holler-cli/src/pane/list.rs:1-16`** — What the read verbs read: the record and nothing else. "SHOWN and health are what reconcile last recorded. DRIVEN is printed as the record holds it and never compared."

Command: `git show main:crates/holler-cli/src/pane/list.rs | sed -n '1,16p'`

~~~~text
1	//! `holler pane list` (story #643, epic #633): the pane registry as a table, one row per
2	//! pane, and the view code the three read verbs share.
3	//!
4	//! The shared items are `pub` and live here because the frozen `pane/mod.rs` admits no new
5	//! module. [`PaneRow`] is one pane in summary: a `list` row, and the `pane` of a `watch`
6	//! put. [`SessionSync`] is SYNC, read from a record through [`shown_differs`], the one
7	//! SHOWN/DRIVEN rule (`pane doctor` uses it too). [`text_value`] and [`json_text`] make a
8	//! stored string safe to print on a terminal, and [`observed_at`] prints a record
9	//! timestamp. `get.rs` and `watch.rs` use them through `super::list`, and a later consumer
10	//! (the roster, #648) can use them through `crate::pane::list`.
11	//!
12	//! The read verbs read the record and nothing else. They call `PaneStore::{get, list,
13	//! watch}`, `ProfileStore::get` and `ProfileScope::resolve`, never an adapter or a probe,
14	//! and they write nothing. SHOWN and health are what reconcile last recorded. DRIVEN is
15	//! printed as the record holds it and never compared: nothing records it before the hub
16	//! wiring (#649).
~~~~

**E7: `crates/holler-cli/src/pane/list.rs:28-38`** — `COLUMNS`, the table's columns, and the empty-cell marker.

Command: `git show main:crates/holler-cli/src/pane/list.rs | sed -n '28,38p'`

~~~~text
28	/// The table's columns, in order. A `pane watch` line prints the same cells after the
29	/// pane's name, each as `key=value` with the column name in lower case.
30	pub const COLUMNS: [&str; 9] = [
31	    "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
32	];
33	
34	/// What a text cell or field holds when the record has no value.
35	pub const NO_VALUE: &str = "-";
36	
37	/// What separates two columns of the table.
38	const GAP: &str = "  ";
~~~~

**E8: `crates/holler-cli/src/pane/list.rs:205-250`** — SYNC: SHOWN against the session of record through `shown_differs`; `last_observed.driven` is printed, never compared; the words `ok`, `MISMATCH` and `-`.

Command: `git show main:crates/holler-cli/src/pane/list.rs | sed -n '205,250p'`

~~~~text
205	/// SYNC: whether a pane's TUI shows the session the hub drives, read from its record. The
206	/// comparison is [`shown_differs`], the one SHOWN/DRIVEN rule (`pane doctor` uses it too):
207	/// SHOWN (`last_observed.shown`) against the pane's session of record, which ADR-0021's I2
208	/// makes the session the hub drives. `last_observed.driven` is printed, never compared.
209	#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
210	#[serde(rename_all = "snake_case")]
211	pub enum SessionSync {
212	    /// The TUI showed the pane's session of record.
213	    Ok,
214	    /// The TUI showed another session, or its home screen: the pane is not on the session
215	    /// the hub drives.
216	    Mismatch,
217	    /// There is nothing to compare: reconcile has never observed the pane, or the pane has
218	    /// no session of record (`pane doctor` reports that as `no-session-of-record`). Not
219	    /// flagged, since nothing was checked.
220	    Unobserved,
221	}
222	
223	impl SessionSync {
224	    /// SYNC of the record `pane`, in the form [`shown_differs`]'s doc asks of a reader:
225	    /// [`Unobserved`](Self::Unobserved) unless reconcile has observed the pane
226	    /// (`last_observed.at > 0`; [`observed_at`] prints `never` otherwise) and it has a
227	    /// session of record. Then [`Mismatch`](Self::Mismatch) when `shown_differs` answers
228	    /// yes for `last_observed.shown` (`None` is the home screen), and [`Ok`](Self::Ok) when
229	    /// it answers no.
230	    pub fn of(pane: &Pane) -> Self {
231	        let observed = &pane.last_observed;
232	        let record = pane.session_of_record.as_deref();
233	        if observed.at <= 0 || record.is_none() {
234	            Self::Unobserved
235	        } else if shown_differs(record, observed.shown.as_deref()) {
236	            Self::Mismatch
237	        } else {
238	            Self::Ok
239	        }
240	    }
241	
242	    /// The word text mode prints: `ok`, `MISMATCH` (loud on purpose) or `-`.
243	    pub fn text(self) -> &'static str {
244	        match self {
245	            Self::Ok => "ok",
246	            Self::Mismatch => "MISMATCH",
247	            Self::Unobserved => NO_VALUE,
248	        }
249	    }
250	}
~~~~

**E9: `crates/holler-cli/src/pane/watch.rs:1-9`** — `pane watch`: the store's change feed, NDJSON in JSON mode, `--since N`.

Command: `git show main:crates/holler-cli/src/pane/watch.rs | sed -n '1,9p'`

~~~~text
1	//! `holler pane watch` (story #643, epic #633): the pane store's change feed as a stream,
2	//! one line per change, NDJSON in JSON mode.
3	//!
4	//! "Each change once" is the port's contract, not a dedupe here: the verb prints every
5	//! change the stream yields, in order. From cursor 0 that is the current state (one put
6	//! per pane) and then every later change; from `--since N`, every change after N
7	//! (`holler_pane::ports::Watch`). The verb reads only `ProfileScope::resolve` (with
8	//! `--profile`, before the stream opens) and `PaneStore::watch`, and it writes nothing.
9	
~~~~

**E10: `crates/holler-cli/src/pane/watch.rs:45-58`** — `pane watch`'s arguments: `--since` is a `u64` (so a bare placeholder fails `docs_cli_test`), and `--until-idle`.

Command: `git show main:crates/holler-cli/src/pane/watch.rs | sed -n '45,58p'`

~~~~text
45	#[derive(Args, Debug)]
46	pub struct PaneWatch {
47	    /// Print only this pane's changes.
48	    #[arg(value_name = "PANE")]
49	    pub pane: Option<String>,
50	    #[command(flatten)]
51	    pub profile: ProfileOpt,
52	    /// Print the changes after this cursor (the "cursor" of a line already seen) instead of
53	    /// the current state.
54	    #[arg(long, value_name = "CURSOR")]
55	    pub since: Option<u64>,
56	    /// Stop as soon as nothing more is owed, instead of waiting for the next change.
57	    #[arg(long)]
58	    pub until_idle: bool,
~~~~

**E11: `crates/holler-cli/src/pane/doctor.rs:1-38`** — `pane doctor`: exit codes (a completed pass exits 0 with findings), the periodic pass (no hub timer), text mode, and `--fix`.

Command: `git show main:crates/holler-cli/src/pane/doctor.rs | sed -n '1,38p'`

~~~~text
1	//! `holler pane doctor [PANE] [--fix] [--profile NAME]` (story #647): run one reconcile pass
2	//! (`holler_pane::reconcile`) over the panes in scope and print its report.
3	//!
4	//! - **Exit codes.** A pass that completes exits 0, with or without findings: findings are
5	//!   kinds in the report, not errors (ADR-0021 section 9), and an envelope cannot carry data
6	//!   with a failure, so a script reads `data.findings`. The verb fails only when it cannot
7	//!   run the pass: a bad `PANE` or `--profile` value is `usage` (2); a pane or profile that
8	//!   does not exist, or a pane outside the profile, is a refusal (3); a pane store or profile
9	//!   scope that cannot be read is a failure (1).
10	//! - **The periodic pass.** There is no hub timer (ADR-0021 section 12): a scheduler runs
11	//!   `holler pane doctor --format=json`, without `--fix`, as often as wanted, and each run
12	//!   records what changed.
13	//! - **Text mode** prints one line each: Herdr's version, each host's recorded Herdr API
14	//!   version, each finding (`<pane> <pos> <kind>: <message>`, then what `--fix` did and the
15	//!   command to run), and a summary. Untrusted values are quoted (`findings::quoted`), so no
16	//!   control sequence reaches the terminal; JSON carries the raw values.
17	
18	use clap::Args;
19	use holler_pane::findings::{quoted, Finding, FixState};
20	use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
21	use holler_pane::{PaneError, PaneName, Ports, ProfileName};
22	use holler_proto::clock::now_millis;
23	
24	use super::args::ProfileOpt;
25	use crate::output::{emit, ErrorBody, VerbCtx};
26	
27	/// Check panes against Herdr, tmux and the harness and report what differs.
28	#[derive(Args, Debug)]
29	pub struct PaneDoctor {
30	    /// Check only this pane (default: every pane in scope).
31	    #[arg(value_name = "PANE")]
32	    pub pane: Option<String>,
33	    /// Repair what the record decides (selects the session of record; never changes it).
34	    #[arg(long)]
35	    pub fix: bool,
36	    #[command(flatten)]
37	    pub profile: ProfileOpt,
38	}
~~~~

**E12: `crates/holler-pane/src/findings.rs:40-109`** — `FindingKind`, its `ALL` table and the stable `code()` strings. `UnregisteredHerdrPane` (line 51-52) is "A Herdr pane that no record names ... Whole-fleet runs only".

Command: `git show main:crates/holler-pane/src/findings.rs | sed -n '40,109p'`

~~~~text
40	/// What a reconcile pass found about a pane, a harness server, a session or Herdr.
41	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
42	pub enum FindingKind {
43	    /// Herdr reports an API version the adapter does not know; the message names the
44	    /// supported ones. No Holler verb upgrades Herdr.
45	    HerdrVersionUnsupported,
46	    /// A port call of the pass failed, so nothing was concluded from what it would have
47	    /// observed. The message names the call as `<port>.<method>` and the error's code. It is
48	    /// a read (`herdr.snapshot`, `host.ps`, `harness.health`, ...) or the record write
49	    /// (`pane_store.cas_put`), a write that lost a race with another writer included.
50	    ObserveFailed,
51	    /// A Herdr pane that no record names: a process with no record. Whole-fleet runs only.
52	    UnregisteredHerdrPane,
53	    /// Herdr has no pane where the record says the pane is.
54	    HerdrPaneMissing,
55	    /// The pane's tmux session does not exist.
56	    TmuxSessionMissing,
57	    /// The pane's harness server accepts a connection and never answers.
58	    ServerWedged,
59	    /// The pane's harness server fails its health check, and is not wedged.
60	    ServerDown,
61	    /// The record names no session of record. Doctor never picks one.
62	    NoSessionOfRecord,
63	    /// The pane's server answered, and it does not have the session of record.
64	    SessionOfRecordMissing,
65	    /// The TUI shows a session its pane's own server does not have: a harness the record
66	    /// does not know is in the pane.
67	    TuiForeignSession,
68	    /// The TUI shows another session than the session of record, or its home screen. The
69	    /// one kind `--fix` repairs: it selects the session of record.
70	    ShownDrivenMismatch,
71	    /// A session on a pane's server that is the session of record of no pane.
72	    StraySession,
73	}
74	
75	impl FindingKind {
76	    /// Every kind, in declaration order.
77	    pub const ALL: &'static [FindingKind] = &[
78	        FindingKind::HerdrVersionUnsupported,
79	        FindingKind::ObserveFailed,
80	        FindingKind::UnregisteredHerdrPane,
81	        FindingKind::HerdrPaneMissing,
82	        FindingKind::TmuxSessionMissing,
83	        FindingKind::ServerWedged,
84	        FindingKind::ServerDown,
85	        FindingKind::NoSessionOfRecord,
86	        FindingKind::SessionOfRecordMissing,
87	        FindingKind::TuiForeignSession,
88	        FindingKind::ShownDrivenMismatch,
89	        FindingKind::StraySession,
90	    ];
91	
92	    /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
93	    /// once merged (ADR-0021 section 9).
94	    pub const fn code(self) -> &'static str {
95	        match self {
96	            FindingKind::HerdrVersionUnsupported => "herdr-version-unsupported",
97	            FindingKind::ObserveFailed => "observe-failed",
98	            FindingKind::UnregisteredHerdrPane => "unregistered-herdr-pane",
99	            FindingKind::HerdrPaneMissing => "herdr-pane-missing",
100	            FindingKind::TmuxSessionMissing => "tmux-session-missing",
101	            FindingKind::ServerWedged => "server-wedged",
102	            FindingKind::ServerDown => "server-down",
103	            FindingKind::NoSessionOfRecord => "no-session-of-record",
104	            FindingKind::SessionOfRecordMissing => "session-of-record-missing",
105	            FindingKind::TuiForeignSession => "tui-foreign-session",
106	            FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
107	            FindingKind::StraySession => "stray-session",
108	        }
109	    }
~~~~

**E13: `crates/holler-pane/src/findings.rs:111-143`** — `FindingKind::remedy`: the one table of the commands a finding tells the operator to run.

Command: `git show main:crates/holler-pane/src/findings.rs | sed -n '111,143p'`

~~~~text
111	    /// The command a finding of this kind tells the operator to run, for the pane `pane`
112	    /// (`None` when it is about no pane) once `--fix` left it at `fix`; `None` when no Holler
113	    /// verb repairs it. The one table of remedies:
114	    ///
115	    /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
116	    /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
117	    /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
118	    ///   one (choosing an existing session is the operator's, never a guess);
119	    /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
120	    ///   it cannot or failed, nothing once it is fixed.
121	    pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
122	        match self {
123	            FindingKind::HerdrVersionUnsupported
124	            | FindingKind::UnregisteredHerdrPane
125	            | FindingKind::StraySession => None,
126	            FindingKind::ObserveFailed => Some(doctor_command(pane, false)),
127	            FindingKind::HerdrPaneMissing
128	            | FindingKind::TmuxSessionMissing
129	            | FindingKind::ServerWedged
130	            | FindingKind::ServerDown
131	            | FindingKind::TuiForeignSession => pane.map(relaunch_command),
132	            FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
133	                pane.map(reset_command)
134	            }
135	            FindingKind::ShownDrivenMismatch => match fix {
136	                FixState::Fixable | FixState::Skipped => {
137	                    pane.map(|pane| doctor_command(Some(pane), true))
138	                }
139	                FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
140	                FixState::Fixed => None,
141	            },
142	        }
143	    }
~~~~

**E14: `crates/holler-pane/src/findings.rs:153-182`** — `FixState`: what `--fix` did or could do (`not-fixable`, `fixable`, `skipped`, `fixed`, `failed`).

Command: `git show main:crates/holler-pane/src/findings.rs | sed -n '153,182p'`

~~~~text
153	/// What `--fix` did, or could do, about a finding.
154	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
155	pub enum FixState {
156	    /// `--fix` does not repair it: every kind but the mismatch, and a mismatch whose server
157	    /// is not healthy or does not have the session of record.
158	    NotFixable,
159	    /// `--fix` would repair it, and this run did not ask to.
160	    Fixable,
161	    /// `--fix` was asked for and left it alone: the orchestrator's pane is switched only by a
162	    /// run that names it.
163	    Skipped,
164	    /// `--fix` repaired it: the TUI was seen showing the session of record afterwards.
165	    Fixed,
166	    /// `--fix` tried and the TUI was not seen showing the session of record; the finding's
167	    /// `fix_error` says why.
168	    Failed,
169	}
170	
171	impl FixState {
172	    /// The state's kebab-case name, which is also its JSON form.
173	    pub const fn as_str(self) -> &'static str {
174	        match self {
175	            FixState::NotFixable => "not-fixable",
176	            FixState::Fixable => "fixable",
177	            FixState::Skipped => "skipped",
178	            FixState::Fixed => "fixed",
179	            FixState::Failed => "failed",
180	        }
181	    }
182	}
~~~~

**E15: `crates/holler-pane/src/reconcile.rs:10-22`** — What one reconcile pass does: observe, compare, and repair one thing with `--fix` (never writes `session_of_record`, never types into a pane).

Command: `git show main:crates/holler-pane/src/reconcile.rs | sed -n '10,22p'`

~~~~text
10	//! 1. **Observe** each pane: `host.ps`, `harness.health`, `harness.list_sessions` and
11	//!    `harness.shown_session`; and once per pass `herdr.version` and `herdr.snapshot`. A
12	//!    healthy server's list is trusted. An unhealthy server's list only tells a wedged
13	//!    server (the list times out) from a down one. A rule that needs a value that was not
14	//!    observed is skipped, and the call that failed is an `observe-failed` finding: nothing
15	//!    is guessed.
16	//! 2. **Compare** (each [`FindingKind`] says when it is reported). Stray sessions are judged
17	//!    against every record, not only the scope, so another profile's session of record is
18	//!    never a stray; unregistered Herdr panes are reported by whole-fleet passes only.
19	//! 3. **Repair**, with `--fix`, one thing: a shown/driven mismatch, by selecting the session
20	//!    of record when the pane's healthy server has it, then observing the TUI again (I3). It
21	//!    never creates, starts, attaches, aborts or deletes anything, never types into a pane
22	//!    (I4) and never writes `session_of_record`. The orchestrator's pane is repaired only by
~~~~

**E16: `crates/holler-pane/src/reconcile.rs:285-324`** — `herdr_findings`: `unregistered-herdr-pane` is raised in whole-fleet passes only.

Command: `git show main:crates/holler-pane/src/reconcile.rs | sed -n '285,324p'`

~~~~text
285	/// The findings about Herdr itself, and those that need its snapshot: `herdr-pane-missing`
286	/// for a pane in scope, and (whole-fleet passes only) `unregistered-herdr-pane`.
287	fn herdr_findings(
288	    herdr: Option<&HerdrSeen>,
289	    scope: &[Pane],
290	    records: &[Pane],
291	    whole_fleet: bool,
292	) -> Vec<Finding> {
293	    let Some(herdr) = herdr else {
294	        let message = format!(
295	            "{HERDR_VERSION} and {HERDR_SNAPSHOT} were not observed: the check did not finish"
296	        );
297	        return vec![Finding::new(FindingKind::ObserveFailed, message)];
298	    };
299	    let mut findings = Vec::new();
300	    match &herdr.version {
301	        Ok(_) => {}
302	        Err(PaneError::HerdrVersionUnsupported { message }) => findings.push(Finding::new(
303	            FindingKind::HerdrVersionUnsupported,
304	            format!("{}; no holler verb upgrades Herdr", embedded(message)),
305	        )),
306	        Err(error) => findings.push(Finding::new(
307	            FindingKind::ObserveFailed,
308	            failed_message(HERDR_VERSION, error),
309	        )),
310	    }
311	    match &herdr.snapshot {
312	        Ok(snapshot) => {
313	            findings.extend(missing_herdr_panes(snapshot, scope));
314	            if whole_fleet {
315	                findings.extend(unregistered_herdr_panes(snapshot, records));
316	            }
317	        }
318	        Err(error) => findings.push(Finding::new(
319	            FindingKind::ObserveFailed,
320	            failed_message(HERDR_SNAPSHOT, error),
321	        )),
322	    }
323	    findings
324	}
~~~~

**E17: `crates/holler-pane/src/reconcile.rs:349-369`** — `unregistered_herdr_panes`: every pane of Herdr's snapshot that no record names is a finding, whatever it runs.

Command: `git show main:crates/holler-pane/src/reconcile.rs | sed -n '349,369p'`

~~~~text
349	/// `unregistered-herdr-pane` for each pane of the snapshot that no record names.
350	fn unregistered_herdr_panes(snapshot: &HerdrSnapshot, records: &[Pane]) -> Vec<Finding> {
351	    let named: BTreeSet<(&str, &PaneId)> = records
352	        .iter()
353	        .map(|pane| (pane.herdr.session.as_str(), &pane.herdr.pane_id))
354	        .collect();
355	    snapshot
356	        .panes
357	        .iter()
358	        .filter(|herdr| !named.contains(&(herdr.session.as_str(), &herdr.pane_id)))
359	        .map(|herdr| {
360	            let message = format!(
361	                "Herdr pane {} (session {}, workspace {}) is in no pane record; no holler verb adopts a pane",
362	                quoted(herdr.pane_id.as_str()),
363	                quoted(&herdr.session),
364	                quoted(&herdr.workspace)
365	            );
366	            Finding::new(FindingKind::UnregisteredHerdrPane, message).with_herdr_pane(herdr)
367	        })
368	        .collect()
369	}
~~~~

**E18: `crates/holler-pane/src/error.rs:1-23`** — The error taxonomy: closed codes, open codes carried by `Refused`, one exit class per code.

Command: `git show main:crates/holler-pane/src/error.rs | sed -n '1,23p'`

~~~~text
1	//! The error taxonomy of pane control (epic #633, decision 2 of #637).
2	//!
3	//! One closed set of kebab-case codes, one validator ([`is_valid_code`], the only
4	//! one in the workspace), and an open [`PaneError::Refused`] variant for the codes
5	//! an adapter or a verb owns.
6	//!
7	//! - **Closed codes** have one [`PaneError`] variant each. The list is closed:
8	//!   a later story fills behaviour, never the list ([`ALL_CODES`] is exactly the
9	//!   set below). Each variant's doc names the story that owns it.
10	//! - **Open codes** travel in [`PaneError::Refused`]. A verb or an adapter declares
11	//!   the code it owns as a `const` ([`RefusalCode::from_static`]), so an invalid
12	//!   literal fails the build and the call site needs no `Result`. A code read off
13	//!   the wire that is not closed lands in `Refused` too (see
14	//!   [`crate::PaneReply::into_result`]).
15	//! - **One representation per code**: a closed code is never carried by `Refused`
16	//!   ([`RefusalCode`] refuses it), so a `match` on the closed variants cannot be
17	//!   bypassed.
18	//! - **One exit class per code** (ADR-0021 section 9): [`class_of`] sorts any code
19	//!   into an [`ErrorClass`] (a usage error, a refusal or a runtime failure), and
20	//!   [`ErrorClass::exit_code`] is the exit code a `pane` or `profile` verb ends
21	//!   with (2, 3 or 1). It is the one place that decides which code is a refusal
22	//!   and which a failure; the CLI calls it, and so will the test kit (#638).
23	//!
~~~~

**E19: `crates/holler-pane/src/error.rs:97-123`** — The closed error codes and their wire strings.

Command: `git show main:crates/holler-pane/src/error.rs | sed -n '97,123p'`

~~~~text
97	    /// The kebab-case wire string of this code.
98	    pub(crate) const fn as_str(self) -> &'static str {
99	        match self {
100	            PaneCode::NotImplemented => "not-implemented",
101	            PaneCode::Usage => "usage",
102	            PaneCode::GridAmbiguous => "grid-ambiguous",
103	            PaneCode::GridOutOfRange => "grid-out-of-range",
104	            PaneCode::CommandNotArgv => "command-not-argv",
105	            PaneCode::EnvNameInvalid => "env-name-invalid",
106	            PaneCode::GenerationConflict => "generation-conflict",
107	            PaneCode::ProbeFailed => "probe-failed",
108	            PaneCode::ProfileConflict => "profile-conflict",
109	            PaneCode::ProfileNotFound => "profile-not-found",
110	            PaneCode::ProfileExists => "profile-exists",
111	            PaneCode::ProfileHasLivePanes => "profile-has-live-panes",
112	            PaneCode::PaneNotInProfile => "pane-not-in-profile",
113	            PaneCode::PaneInOtherProfile => "pane-in-other-profile",
114	            PaneCode::ProfileSecretRefused => "profile-secret-refused",
115	            PaneCode::HerdrVersionUnsupported => "herdr-version-unsupported",
116	            PaneCode::Timeout => "timeout",
117	            PaneCode::PaneNotFound => "pane-not-found",
118	            PaneCode::SessionNotFound => "session-not-found",
119	            PaneCode::StoreCorrupt => "store-corrupt",
120	            PaneCode::Unavailable => "unavailable",
121	            PaneCode::ProfileDrift => "profile-drift",
122	        }
123	    }
~~~~

**E20: `crates/holler-pane/src/error.rs:140-146`** — `ALL_CODES`, the public list acceptance 3 reads.

Command: `git show main:crates/holler-pane/src/error.rs | sed -n '140,146p'`

~~~~text
140	/// Every closed code, as its kebab-case wire string, in the epic's order.
141	///
142	/// The set is closed: the invariants of this crate (one variant per code, unique,
143	/// valid under [`is_valid_code`]) are tested against it, and no story edits it.
144	/// `profile-drift` is listed for convenience: it is a reconcile finding kind
145	/// (#647/#665 put it in `findings.rs`), not an error a port returns.
146	pub const ALL_CODES: &[&str] = &CLOSED_CODES;
~~~~

**E21: `crates/holler-pane/src/error.rs:217-246`** — `ErrorClass` and its exit codes (usage 2, refusal 3, failure 1; success 0).

Command: `git show main:crates/holler-pane/src/error.rs | sed -n '217,246p'`

~~~~text
217	/// What kind of error a code stands for, which decides the exit code of a `pane` or
218	/// `profile` verb that ends with it (ADR-0021 section 9). [`class_of`] gives the
219	/// class of a code.
220	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
221	pub enum ErrorClass {
222	    /// The request is malformed: a missing or conflicting argument, a bad name, a
223	    /// params object that does not decode (`usage`). Exit 2.
224	    Usage,
225	    /// The request was understood and declined, working as designed, so nothing was
226	    /// wrong with the system (`pane-in-other-profile`, `grid-ambiguous`, every open
227	    /// code). Exit 3.
228	    Refusal,
229	    /// Something went wrong while doing the work: a timeout, an unreachable hub or
230	    /// adapter, a conflict between writers (`timeout`, `unavailable`,
231	    /// `generation-conflict`). Exit 1.
232	    Failure,
233	}
234	
235	impl ErrorClass {
236	    /// The exit code of a verb that ends with an error of this class, the same in
237	    /// text and JSON mode: 2 usage, 3 refusal, 1 runtime failure (ADR 0003's exit
238	    /// codes; success is 0 and has no class).
239	    pub const fn exit_code(self) -> i32 {
240	        match self {
241	            ErrorClass::Usage => 2,
242	            ErrorClass::Refusal => 3,
243	            ErrorClass::Failure => 1,
244	        }
245	    }
246	}
~~~~

**E22: `crates/holler-pane/src/error.rs:248-303`** — `class_of`, the one place that gives a code its class: an open code is always a refusal (exit 3), `profile-drift` is a failure (exit 1).

Command: `git show main:crates/holler-pane/src/error.rs | sed -n '248,303p'`

~~~~text
248	/// The class of the error coded `code`: the one place that decides which code is a
249	/// refusal and which a runtime failure. ADR-0021 section 9 has the table, with the
250	/// reason for every row. The CLI's output module calls this, and so will the test
251	/// kit (#638); neither keeps a table of its own.
252	///
253	/// It takes the code, not a [`PaneError`], because a code is what an envelope
254	/// carries. A caller that holds a `PaneError` passes [`PaneError::code`]:
255	/// `class_of(error.code())`. (It is not `PaneError::classify`, a crate-private
256	/// helper that only tells a closed code from an open one.)
257	///
258	/// - A **closed** code has the class of its arm below. The `match` names every
259	///   closed code and has no catch-all arm, so a closed code added without a class
260	///   does not compile.
261	/// - Any other **well-formed** code is an open code, carried by
262	///   [`PaneError::Refused`]: a refusal, always. A verb that reports a runtime
263	///   failure uses a closed failure code.
264	/// - A **malformed** code is a failure: it cannot be trusted, the same rule that
265	///   turns a garbled wire reply into `unavailable`.
266	pub fn class_of(code: &str) -> ErrorClass {
267	    let Some(closed) = PaneCode::parse(code) else {
268	        return if is_valid_code(code) {
269	            ErrorClass::Refusal
270	        } else {
271	            ErrorClass::Failure
272	        };
273	    };
274	    match closed {
275	        PaneCode::Usage => ErrorClass::Usage,
276	        // Understood and declined: a guard, a policy or a gate said no, the name is
277	        // taken, or the request named something that does not exist.
278	        PaneCode::GridAmbiguous
279	        | PaneCode::GridOutOfRange
280	        | PaneCode::CommandNotArgv
281	        | PaneCode::EnvNameInvalid
282	        | PaneCode::ProfileSecretRefused
283	        | PaneCode::ProfileExists
284	        | PaneCode::ProfileHasLivePanes
285	        | PaneCode::PaneNotInProfile
286	        | PaneCode::PaneInOtherProfile
287	        | PaneCode::ProbeFailed
288	        | PaneCode::HerdrVersionUnsupported
289	        | PaneCode::ProfileNotFound
290	        | PaneCode::PaneNotFound
291	        | PaneCode::SessionNotFound => ErrorClass::Refusal,
292	        // Went wrong while doing the work: a race between writers, a bound that ran
293	        // out, something unreachable or unreadable, live state that disagrees with
294	        // its spec, or work the verb cannot do yet.
295	        PaneCode::GenerationConflict
296	        | PaneCode::ProfileConflict
297	        | PaneCode::Timeout
298	        | PaneCode::Unavailable
299	        | PaneCode::StoreCorrupt
300	        | PaneCode::NotImplemented
301	        | PaneCode::ProfileDrift => ErrorClass::Failure,
302	    }
303	}
~~~~

**E23: `crates/holler-cli/src/output.rs:1-58`** — The envelope, its `schema_version`, the exit codes and the `--format` / `--json` rule.

Command: `git show main:crates/holler-cli/src/output.rs | sed -n '1,58p'`

~~~~text
1	//! What the `holler pane` and `holler profile` verbs print (story #670, epic #633): one module
2	//! decides it, so every verb answers in one shape and each verb supplies only its data.
3	//!
4	//! - **Text mode** (the default): the data of a successful verb goes to `out`; an error goes to
5	//!   `err` as `error: <message>`, with nothing on `out`.
6	//! - **JSON mode** (`--format=json`, or `--json`): exactly one envelope goes to `out` per result,
7	//!   an error included, and nothing goes to `err`. `pane watch` is the one stream: one envelope
8	//!   per line ([`emit_stream`]).
9	//!
10	//! The exit code is the same in both modes: `0` ok, `1` runtime failure, `2` usage, `3` refusal
11	//! (ADR-0021 section 9). The class of the error's code decides it, through
12	//! `holler_pane::error::class_of`, and this module keeps no table of its own. An error coded
13	//! `usage` exits 2 however it got here, so a run-time guard of `holler-pane` (a bad pane name,
14	//! `--command-json` that is not JSON) is a usage error and not a runtime failure; a request
15	//! understood and declined (`pane-in-other-profile`, `grid-ambiguous`) exits 3; something that
16	//! went wrong while doing the work (`timeout`, `unavailable`) exits 1.
17	//!
18	//! Writing goes through an injected [`Sink`], so a test captures both streams. Nothing here exits
19	//! the process: every function returns the code and `main.rs`, the one file allowed to exit,
20	//! applies it (`scripts/lint.sh` check 2).
21	//!
22	//! A write that fails (a closed pipe) turns an exit 0 into 1 and ends a stream, so a verb never
23	//! reports success for output nobody received.
24	
25	use std::io::{self, Write};
26	
27	use clap::error::ErrorKind;
28	use clap::ValueEnum;
29	use holler_pane::error::{class_of, is_valid_code};
30	use holler_pane::{PaneError, Ports};
31	use serde::Serialize;
32	
33	/// The envelope's `schema_version` (epic #633, "Output"). A change to the envelope's shape bumps it.
34	pub const SCHEMA_VERSION: u32 = 1;
35	
36	/// The two subcommands whose usage errors are envelopes in JSON mode (decision 1(d) of #670).
37	/// Every other verb keeps its own usage-error output, as ADR 0003 requires.
38	const ENVELOPE_NAMESPACES: [&str; 2] = ["pane", "profile"];
39	
40	/// The output format: the global `--format`, and what `--json` means.
41	#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
42	pub enum Format {
43	    /// Human-readable text (the default).
44	    Text,
45	    /// One JSON envelope per result.
46	    Json,
47	}
48	
49	/// How the global `--format` and `--json` resolved.
50	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
51	pub struct FormatChoice {
52	    /// The format every verb prints in.
53	    pub format: Format,
54	    /// `--format=json` was given itself (`--json` alone does not set it). `roster` prints the
55	    /// envelope only for this, so `roster --json` keeps its legacy shape (ADR 0003: the `--json`
56	    /// shape is part of the surface); the `pane` and `profile` verbs treat either flag the same.
57	    pub json_explicit: bool,
58	}
~~~~

**E24: `crates/holler-cli/src/output.rs:125-182`** — The `error` member (`code`, `message`) and the envelope of every `pane` and `profile` result: `data` is `null` when `ok` is false.

Command: `git show main:crates/holler-cli/src/output.rs | sed -n '125,182p'`

~~~~text
125	/// The `error` member of a failed envelope.
126	#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
127	pub struct ErrorBody {
128	    /// The stable code a script matches on.
129	    pub code: ErrorCode,
130	    /// One line for a person. In JSON mode [`emit`] puts it on one line.
131	    pub message: String,
132	}
133	
134	impl From<&PaneError> for ErrorBody {
135	    fn from(error: &PaneError) -> Self {
136	        Self {
137	            code: ErrorCode::from(error),
138	            message: error.to_string(),
139	        }
140	    }
141	}
142	
143	/// An `ErrorBody` coded `usage`.
144	fn usage_body(message: &str) -> ErrorBody {
145	    ErrorBody::from(&PaneError::Usage {
146	        message: message.to_owned(),
147	    })
148	}
149	
150	/// The one envelope of every `holler pane` and `holler profile` result in JSON mode.
151	///
152	/// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
153	/// `ok` is false and `error` is `null` when it is true.
154	#[derive(Debug, Serialize)]
155	pub struct Envelope<T> {
156	    pub schema_version: u32,
157	    pub ok: bool,
158	    pub data: Option<T>,
159	    pub error: Option<ErrorBody>,
160	}
161	
162	impl<T> Envelope<T> {
163	    /// A successful result.
164	    pub fn success(data: T) -> Self {
165	        Self {
166	            schema_version: SCHEMA_VERSION,
167	            ok: true,
168	            data: Some(data),
169	            error: None,
170	        }
171	    }
172	
173	    /// A failed result.
174	    pub fn failure(error: ErrorBody) -> Self {
175	        Self {
176	            schema_version: SCHEMA_VERSION,
177	            ok: false,
178	            data: None,
179	            error: Some(error),
180	        }
181	    }
182	}
~~~~

**E25: `crates/holler-pane/src/grid.rs:1-55`** — `GridPos::parse`: the accepted forms and the two refusal codes.

Command: `git show main:crates/holler-pane/src/grid.rs | sed -n '1,55p'`

~~~~text
1	//! `GridPos`: the one grid-position type of pane control (epic #633, decision 7).
2	//!
3	//! Grid notation is ROWCOL and 1-based. The type, its parser, the `rRcC` display
4	//! and the serde form are defined here once; the Herdr adapter (#640) is the only
5	//! place that converts a `GridPos` to Herdr's own order and base.
6	//!
7	//! - **Input** ([`GridPos::parse`]): `r<row>c<col>` or `c<col>r<row>` (labelled, in
8	//!   either order) or a bare `<row>,<col>`, which is always row first and never
9	//!   col,row. Upper case and surrounding ASCII whitespace are accepted; interior
10	//!   whitespace is refused. A missing half, a repeated label, a mix such as `r2c1c3`
11	//!   or anything unreadable is `grid-ambiguous`; a zero, or a number above
12	//!   `u16::MAX`, is `grid-out-of-range`.
13	//! - **Output**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first.
14	
15	use std::fmt;
16	
17	use serde::de::{self, Deserializer};
18	use serde::ser::{SerializeStruct, Serializer};
19	use serde::{Deserialize, Serialize};
20	
21	use crate::error::{excerpt, PaneError};
22	
23	/// What every refusal of a grid position tells the user to write instead.
24	const FORMS: &str = "write r<row>c<col>, c<col>r<row> or <row>,<col>";
25	
26	/// What every out-of-range refusal says about the bounds.
27	const BOUNDS: &str = "rows and columns run from 1 to 65535";
28	
29	/// A cell of the layout grid, 1-based: `row` counts down, `col` counts across.
30	///
31	/// The fields are public so a cell can be written `GridPos { row: 2, col: 1 }`; a
32	/// zero there is not a cell, and [`GridPos::parse`] and the serde reader both
33	/// refuse it.
34	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
35	pub struct GridPos {
36	    /// The row, from 1.
37	    pub row: u16,
38	    /// The column, from 1.
39	    pub col: u16,
40	}
41	
42	impl GridPos {
43	    /// Parse `r2c1`, `c1r2` or `2,1` (see the module docs for the exact rules).
44	    pub fn parse(text: &str) -> Result<GridPos, PaneError> {
45	        let lower = text.trim_ascii().to_ascii_lowercase();
46	        let (row, col) = split_cells(&lower).ok_or_else(|| PaneError::GridAmbiguous {
47	            what: format!("{} ({FORMS})", excerpt(text)),
48	        })?;
49	        match (coordinate(row), coordinate(col)) {
50	            (Some(row), Some(col)) => Ok(GridPos { row, col }),
51	            _ => Err(PaneError::GridOutOfRange {
52	                what: format!("{} ({BOUNDS})", excerpt(text)),
53	            }),
54	        }
55	    }
~~~~

**E26: `crates/holler-pane/src/grid.rs:75-116`** — Why `GridPos::parse("hj-c1r2")` is `grid-ambiguous`: it holds no comma, and a labelled cell must start with `r` or `c` (line 108).

Command: `git show main:crates/holler-pane/src/grid.rs | sed -n '75,116p'`

~~~~text
75	/// The digit strings of the row and the column of `text` (already trimmed and
76	/// lower-cased), or `None` when its structure is wrong.
77	fn split_cells(text: &str) -> Option<(&str, &str)> {
78	    if text.contains(',') {
79	        split_pair(text)
80	    } else {
81	        split_labelled(text)
82	    }
83	}
84	
85	/// `<row>,<col>`: exactly two digit runs around one comma, row first.
86	fn split_pair(text: &str) -> Option<(&str, &str)> {
87	    let (row, col) = text.split_once(',')?;
88	    (is_number(row) && is_number(col)).then_some((row, col))
89	}
90	
91	/// `r<row>c<col>` or `c<col>r<row>`: two labelled cells with different labels.
92	fn split_labelled(text: &str) -> Option<(&str, &str)> {
93	    let (first_label, first, rest) = take_cell(text)?;
94	    let (second_label, second, tail) = take_cell(rest)?;
95	    if !tail.is_empty() || first_label == second_label {
96	        return None;
97	    }
98	    if first_label == 'r' {
99	        Some((first, second))
100	    } else {
101	        Some((second, first))
102	    }
103	}
104	
105	/// One labelled cell at the start of `text`: its label (`r` or `c`), its digits and
106	/// what follows.
107	fn take_cell(text: &str) -> Option<(char, &str, &str)> {
108	    let label = text.chars().next().filter(|c| matches!(c, 'r' | 'c'))?;
109	    let body = text.strip_prefix(label)?;
110	    let end = body
111	        .bytes()
112	        .position(|b| !b.is_ascii_digit())
113	        .unwrap_or(body.len());
114	    let (digits, rest) = body.split_at(end);
115	    (!digits.is_empty()).then_some((label, digits, rest))
116	}
~~~~

**E27: `crates/holler-pane/src/pane.rs:128-138`** — `HarnessKind` is a closed enum with one variant, `Opencode`.

Command: `git show main:crates/holler-pane/src/pane.rs | sed -n '128,138p'`

~~~~text
128	///
129	/// This is a closed enum on purpose (epic #633: no new harness; a stored record with
130	/// an unknown harness fails to load rather than being driven blindly). It is a second
131	/// list beside `holler_proto::vocab::HARNESS_IDS`; every serde name of this enum must
132	/// be in that list, which a test pins. Adding a harness to panes adds a variant here
133	/// and goes through the epic's amend-first rule.
134	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
135	#[serde(rename_all = "snake_case")]
136	pub enum HarnessKind {
137	    Opencode,
138	}
~~~~

**E28: `crates/holler-pane/src/ports.rs:203-219`** — `HostPort`: `run` starts a program in the pane's tmux session, `stop_owned` stops what the session owns, the pane's harness server among it (#802).

Command: `git show main:crates/holler-pane/src/ports.rs | sed -n '203,219p'`

~~~~text
203	pub trait HostPort: Send + Sync {
204	    /// Make the tmux session `name` exist, working in `cwd`.
205	    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;
206	
207	    /// Run `argv` (never through a shell) in the session `name`, starting it in `cwd`,
208	    /// which need not be the directory the session was made in (#802: the launch engine
209	    /// passes the pane spec's directory, for the spec's command and for the harness
210	    /// server alike).
211	    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError>;
212	
213	    /// Stop the processes the session `name` owns: what `run` started there, the pane's
214	    /// harness server among them (#802).
215	    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;
216	
217	    /// The process ids running in the session `name`.
218	    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
219	}
~~~~

**E29: `crates/holler-pane/src/ports.rs:221-265`** — `HarnessPort`: `serve_command` builds the server's argv and starts nothing; `verify_serving` asks the running server whether it honours the spec; an agent the adapter cannot apply is refused `agent-unsupported` today.

Command: `git show main:crates/holler-pane/src/ports.rs | sed -n '221,265p'`

~~~~text
221	/// The harness server of a pane and its sessions (the adapter is
222	/// `holler-adapter-opencode`, #642). **Final:** the spike #635 confirmed it unchanged;
223	/// ADR-0021 section 2, "`HarnessPort` as built (#642)", records what the adapter decided,
224	/// and #802 amended how a server starts: the port builds its argv, and [`HostPort::run`]
225	/// starts it in the pane's tmux session, so [`HostPort::stop_owned`] stops it.
226	///
227	/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
228	/// thread) in async code. Every method returns within I5's bound (default 10 s) or
229	/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
230	pub trait HarnessPort: Send + Sync {
231	    /// The argv that starts the harness server of `name` on `port` with `spec` applied,
232	    /// for [`HostPort::run`] to start. Pure: it starts no process, binds no port, writes
233	    /// no file and sends nothing.
234	    ///
235	    /// It never returns an argv that would start a server without what `spec` asks for.
236	    /// When `spec.agent` is `Some(key)`, some element of the argv contains the key's
237	    /// text; an implementation that cannot apply the agent refuses with
238	    /// [`AGENT_UNSUPPORTED`].
239	    fn serve_command(
240	        &self,
241	        name: &PaneName,
242	        port: u16,
243	        spec: &ServeSpec,
244	    ) -> Result<Argv, PaneError>;
245	
246	    /// Whether the server running on `port` honours `spec`. The launch engine asks once
247	    /// the server is healthy, before any session is created on it (#802). `Ok` only when
248	    /// the server honours every field of `spec`; a server that does not is a refusal with
249	    /// an open code the implementation owns. The engine answers every error by rolling
250	    /// back (the host then stops the server). An implementation that cannot check a field
251	    /// it cannot apply refuses it, as [`HarnessPort::serve_command`] does
252	    /// ([`AGENT_UNSUPPORTED`] for an agent).
253	    ///
254	    /// It answers for the spec, not for whether the server is up: the engine's health wait
255	    /// has just seen the server answer, and a server gone since then fails the engine's
256	    /// next call to it (the session of record's), which rolls back the same way. So an
257	    /// implementation sends a request only for what it must check on the server, and that
258	    /// request answers an unreachable server as any call does (`unavailable`, `timeout`).
259	    /// A spec that the implementation's own argv already settles is answered `Ok(())` with
260	    /// nothing sent. That is the OpenCode adapter's answer to a spec without an agent: its
261	    /// argv applies none, so the server keeps its own default.
262	    ///
263	    /// Required, with no default: a harness that starts servers must say whether the
264	    /// running one honours the spec.
265	    fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError>;
~~~~

**E30: `crates/holler-adapter-host/src/lib.rs:34-42`** — How `stop_owned` stops: `TERM` to every owned process group, one shared grace (default 2 s), `KILL` to every group that still has a member.

Command: `git show main:crates/holler-adapter-host/src/lib.rs | sed -n '34,42p'`

~~~~text
34	//! **Ownership, and how `stop_owned` stops.** A pane is owned when its `@holler-pid`
35	//! equals its live `pane_pid`, and that pid is in `2..=i32::MAX`. `stop_owned` sends
36	//! `TERM` to the process group of every owned pane (the pane's process leads its own
37	//! group), waits one shared grace (default 2 s), sends `KILL` to every group that still
38	//! has a member, whether its pane is still listed or not, and waits for the rest of the
39	//! bound. A group is stopped once its pane is gone (not listed, or listed dead) and
40	//! `kill -s 0` finds no member. Every signal to a pane's processes goes through the
41	//! `kill` binary with `LC_ALL=C` (so the crate needs no `libc`), and only to the group of
42	//! an owned pane or to the pid `run` just started when tagging it failed. Nothing is
~~~~

**E31: `crates/holler-cli/src/pane/launch.rs:130-142`** — The effective spec: a flag replaces its field of the profile's base spec, and an omitted `--agent` keeps the base's agent (`relaunch` shares this function).

Command: `git show main:crates/holler-cli/src/pane/launch.rs | sed -n '130,142p'`

~~~~text
130	/// Overlay the typed flags on `base` (with `None`, every required field must come from the
131	/// flags). A flag given replaces its field; a repeatable flag given at least once (`--env`,
132	/// `--expect`, `--command-arg`, `--check-arg`) replaces the whole list; `--role` defaults to
133	/// `agent` and the harness to OpenCode for a new spec. `usage`, in one line naming every flag
134	/// that is missing or bad: with no base, a missing `--project`, `--workspace`, `--grid`,
135	/// `--model`, `--effort`, `--ctx-soft`, `--ctx-hard` or `--port-policy`; a `--model` that is not
136	/// `PROVIDER/ID` with both halves non-empty; `--ctx-soft` above `--ctx-hard`; a port policy
137	/// `port_of_policy` refuses; and an `--expect` with no check.
138	pub(crate) fn effective_spec(
139	    base: Option<&ProfileSpec>,
140	    name: &PaneName,
141	    values: &SpecValues,
142	) -> Result<ProfileSpec, PaneError> {
~~~~

**E32: `crates/holler-cli/src/profile/create.rs:1-18`** — `profile create NAME [--from-current | --from PROFILE]`: what it checks and writes, and the order.

Command: `git show main:crates/holler-cli/src/profile/create.rs | sed -n '1,18p'`

~~~~text
1	//! `holler profile create NAME [--from-current | --from PROFILE]` (story #662, epic #633): make a
2	//! profile that is empty, a detached copy of another profile's specs (`--from`), or a snapshot
3	//! of every pane in the pane registry that makes each pane a member (`--from-current`).
4	//!
5	//! - **Checks first.** Every check runs before the first write (#662 B3): the names parse
6	//!   (`usage`), NAME is not taken by name or slug (`profile-exists`), PROFILE exists
7	//!   (`profile-not-found`), and with `--from-current` no pane belongs to another profile
8	//!   (`pane-in-other-profile`, every such pane at once).
9	//! - **One create write.** Every form stores its record through `insert_profile`, the one
10	//!   place that reports the create race as `profile-exists` (B7). Every new record is built by
11	//!   `holler_pane::profile_snapshot::profile_from_panes`, so no `Profile` is built by hand, and
12	//!   `--from-current`'s specs come from each pane's record (its position from `herdr.grid`,
13	//!   never from its name), with no Herdr call.
14	//! - **Write order.** No transaction spans the two registries (ADR-0021 section 8), so
15	//!   `--from-current` writes the profile first (a pane can name only a profile that exists),
16	//!   then each pane joins in `list()` order, its `Pane.profile` set by compare-and-swap. A
17	//!   failed join undoes everything, since a failure records nothing (I3, #662 B2): the pane
18	//!   whose join failed leaves if its join landed after all (a `timeout` may have), then each
~~~~

**E33: `crates/holler-cli/src/profile/delete.rs:1-11`** — `profile delete NAME [--keep-panes]`: refused while any pane belongs to it; with `--keep-panes` each member is detached first.

Command: `git show main:crates/holler-cli/src/profile/delete.rs | sed -n '1,11p'`

~~~~text
1	//! `holler profile delete NAME [--keep-panes]` (story #662, epic #633): delete a profile, refused
2	//! while any pane belongs to it; with `--keep-panes`, each such pane is detached first and keeps
3	//! running.
4	//!
5	//! A profile's live panes are its members, the pane records whose `profile` has its slug
6	//! (`holler_pane::profile_diff::is_member`, ADR-0021 section 3). No transaction spans the two
7	//! registries (ADR-0021 section 8), so `--keep-panes` writes in the order that never leaves a
8	//! pane naming a profile that is gone: each member leaves first (its `Pane.profile` becomes none
9	//! by compare-and-swap), then the profile is deleted. A failed detach stops the verb and nothing
10	//! is re-attached: detaching is what was asked for, running the delete again converges, and a
11	//! re-attach could fail too (#662 Decision 8).
~~~~

**E34: `crates/holler-cli/src/profile/apply.rs:1-17`** — On `main`, `profile apply` is #670's stub: its only argument is `--take-over`.

Command: `git show main:crates/holler-cli/src/profile/apply.rs | sed -n '1,17p'`

~~~~text
1	//! `holler profile apply`: a stub (story #670). Story #664 owns the real verb: it replaces this
2	//! file and adds the verb's own positionals and flags to the arguments below, edits its own
3	//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
4	
5	use clap::Args;
6	
7	use crate::output::{emit_error, not_implemented, VerbCtx};
8	
9	/// The story that owns this verb.
10	const STORY: u32 = 664;
11	
12	/// Converge the live panes on a profile.
13	#[derive(Args, Debug)]
14	pub struct ProfileApply {
15	    /// Take over a pane that belongs to another profile.
16	    #[arg(long)]
17	    pub take_over: bool,
~~~~

**E35: `crates/holler-cli/src/profile/export.rs:1-20`** — On `main`, `profile export` (and `rename`, `import`) have empty argument structs, so any example with an argument fails `docs_cli_test`.

Command: `git show main:crates/holler-cli/src/profile/export.rs | sed -n '1,20p'`

~~~~text
1	//! `holler profile export`: a stub (story #670). Story #665 owns the real verb: it replaces this
2	//! file and adds the verb's own positionals and flags to the arguments below, edits its own
3	//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
4	//!
5	//! Proposed, not confirmed: #665 waits for the operator, so this verb and its ADR 0003 row
6	//! may be dropped.
7	
8	use clap::Args;
9	
10	use crate::output::{emit_error, not_implemented, VerbCtx};
11	
12	/// The story that owns this verb.
13	const STORY: u32 = 665;
14	
15	/// Write a profile to a file. (proposed, #665: the operator confirms it first)
16	#[derive(Args, Debug)]
17	pub struct ProfileExport {}
18	
19	/// Run `holler profile export`: refuse, naming the story that owns it.
20	pub fn run(_args: &ProfileExport, ctx: &mut VerbCtx<'_>) -> i32 {
~~~~

**E36: `crates/holler-cli/src/pane/import.rs:1-19`** — On `main`, `pane import` is #670's stub with no arguments.

Command: `git show main:crates/holler-cli/src/pane/import.rs | sed -n '1,19p'`

~~~~text
1	//! `holler pane import`: a stub (story #670). Story #650 owns the real verb: it replaces this
2	//! file and adds the verb's own positionals and flags to the arguments below, edits its own
3	//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
4	
5	use clap::Args;
6	
7	use crate::output::{emit_error, not_implemented, VerbCtx};
8	
9	/// The story that owns this verb.
10	const STORY: u32 = 650;
11	
12	/// Import the existing fleet into the pane registry.
13	#[derive(Args, Debug)]
14	pub struct PaneImport {}
15	
16	/// Run `holler pane import`: refuse, naming the story that owns it.
17	pub fn run(_args: &PaneImport, ctx: &mut VerbCtx<'_>) -> i32 {
18	    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
19	}
~~~~

**E37: `plugins/herdr-holler/herdr-plugin.toml:1-37`** — The Herdr display plugin (#651): metadata only; its actions run the `holler pane` verbs.

Command: `git show main:plugins/herdr-holler/herdr-plugin.toml | sed -n '1,37p'`

~~~~text
1	# The Holler display plugin's manifest (issue #651), loaded by Herdr. The
2	# plugin's library and binary live in this crate; the shapes follow the spike
3	# probe (the spike doc sections 12-13 and its plugin probe script).
4	#
5	# Display only: the plugin reports metadata (pane.report_metadata and
6	# workspace.report_metadata) and reads; it never changes a pane, a session or
7	# a layout. tests/display_only_allowlist.rs fails the crate if a source or
8	# this manifest names any other Herdr method.
9	#
10	# Every command is an argv array, never a shell string. The actions run the
11	# holler pane verbs; the pane an action acts on is resolved at run time from
12	# the HERDR_PANE_ID the action process receives, through the hub registry
13	# (resolve_action_pane). The event hooks re-run the plugin's refresh entry.
14	
15	id = "holler"
16	name = "Holler"
17	version = "0.4.0"
18	min_herdr_version = "0.9.1"
19	description = "Holler's pane state in the sidebar: project, SHOWN vs DRIVEN, hold"
20	
21	[[actions]]
22	id = "switch"
23	title = "Switch this pane to its session of record"
24	contexts = ["pane"]
25	command = ["holler", "pane", "switch"]
26	
27	[[actions]]
28	id = "reset"
29	title = "Reset this pane"
30	contexts = ["pane"]
31	command = ["holler", "pane", "reset"]
32	
33	[[actions]]
34	id = "doctor"
35	title = "Check this pane (read only)"
36	contexts = ["pane"]
37	command = ["holler", "pane", "doctor"]
~~~~

**E38: `docs/adr/ADR-0021.md:35-44`** — The pane record: the name is the tmux session name, `hj-c1r2` is a name and not a position, `HarnessKind` is "only `opencode`" (line 41).

Command: `git show main:docs/adr/ADR-0021.md | sed -n '35,44p'`

~~~~text
35	| Field | Type (merged) | Notes |
36	|---|---|---|
37	| `name` | `PaneName` | A newtype over `holler_proto::vocab::SessionName`, so a pane name obeys the [ADR 0005](ADR-0005.md) grammar. It is also the tmux session name. Names such as `hj-c1r2` are names, not positions. |
38	| `generation` | `u64` | See section 8. |
39	| `herdr` | `HerdrPane { session, workspace, pane_id, grid: GridPos, label? }` | `pane_id` is Herdr's opaque id (`PaneId`). `label` is observed, never recorded: launch and relaunch write none (section 2; amended 2026-10-11, #808). |
40	| `host` | `HostInfo { name, tmux, cwd, herdr_api_version? }` | `herdr_api_version` is the string `HerdrPort::version()` reports, recorded by a verb, never by the Herdr adapter, which writes no record (section 10). |
41	| `harness` | `HarnessInfo { kind, port, pid?, health }` | `kind` is the closed enum `HarnessKind` (only `opencode`); `health` is `"healthy"`, `{"unhealthy": reason}` or `"unknown"`. Launch writes no `pid` since #802 (the server is a process of the pane's tmux session, which the host stops); a record written earlier still reads with one. |
42	| `session_of_record` | `Option<String>` | THE session: the TUI shows it and the hub drives it. The epic wrote `Option<SessionId>`; the merged type is a plain string. |
43	| `role` | `PaneRole` | `agent` or `orchestrator`. |
44	| `hold` | `Hold` | `"none"`, `{"parked": {reason, release_when, since}}` or `"drained"`. This is the park state of a pane, **not** the prompt hold of the hold and release verbs (`holler_proto::SessionHold`). |
~~~~

**E39: `docs/adr/ADR-0021.md:134-141`** — Decision (operator, 2026-10-11, #802): the harness server runs inside the pane's tmux session, started through the host adapter.

Command: `git show main:docs/adr/ADR-0021.md | sed -n '134,141p'`

~~~~text
134	4. **Decided (operator, 2026-10-11): the harness server runs inside the pane's tmux session, started through the host
135	   adapter.** `HostPort` (`run`, and `stop_owned`: TERM, a shared grace, then KILL) is the one place that starts and stops a
136	   pane's processes. So `HarnessPort::serve` became `serve_command(name, port, &ServeSpec) -> Argv`, which builds the
137	   server's argv and starts nothing (no process, no bound port, no file); the OpenCode adapter no longer spawns `opencode
138	   serve`; and `harness.pid` stops being a second source of truth: launch no longer writes it. This closes #695 (a wedged
139	   server could not be stopped). When the spec has an agent, some element of the argv contains the key's text; a harness
140	   that cannot apply it refuses `agent-unsupported` (an open code, exit 3: the harness understood the request and
141	   declines it, as `prompt-unsupported` does; the OpenCode adapter answers it for every agent until #642 part 3).
~~~~

**E40: `docs/adr/ADR-0021.md:277-289`** — The invariants I1 to I8, in the ADR's words ("Only pane verbs change a pane or its registration").

Command: `git show main:docs/adr/ADR-0021.md | sed -n '277,289p'`

~~~~text
277	### 4. Ratified: the invariants, each with a test idea
278	
279	| | Invariant | One-line test idea |
280	|---|---|---|
281	| I1 | Only pane verbs change a pane or its registration. | Every adapter call in a scenario is made from a verb's code path: the fakes record their caller, and the hub's `pane/*` handlers hold no adapter (no adapter crate in `holler-hub`'s manifest). |
282	| I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
283	| I3 | Every verb is plan, act, observe, record; a mismatch fails loudly and records nothing. | Make the fake TUI show a different session after `act`: the verb exits 1 with a code, and the stored record and its generation are unchanged. |
284	| I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the switch and reset tests fail if any keystroke was sent (#645). |
285	| I5 | A verb returns in bounded time (default 10 s); long work returns an operation id. | A wedged fake port makes the verb exit with `timeout` within the bound plus a margin; the operation-id half is not built (section 12, decided 2026-10-11). |
286	| I6 | State is observed from Herdr, tmux and the harness, never inferred from files. | Give the importer and reconcile a stale registration file that names session A while the fake harness shows B: no record gets A as observed state. |
287	| I7 | A profile holds names and non-secret values, never a secret. | A `ProfileSpec` or `Pane` JSON with `"env": ["TOKEN=x"]` fails to decode with `profile-secret-refused`, and the refusal text does not contain `x`. |
288	| I8 | An edit through `--profile` and the live change are one transaction. | Fail `act` in `edit_spec` on the fake: P's specs are equal to before and no pane record changed; succeed it: P holds the edit and the pane record names P. |
289	
~~~~

**E41: `docs/adr/ADR-0021.md:592-621`** — The closed and open pane codes, and the codes each verb can end with (the source for "the exit codes it can end with" of each verb).

Command: `git show main:docs/adr/ADR-0021.md | sed -n '592,621p'`

~~~~text
592	**Decided: closed and open pane codes.** The closed set (`ALL_CODES`, one `PaneError` variant each) is: `not-implemented`,
593	`usage`, `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `generation-conflict`, `probe-failed`,
594	`profile-conflict`, `profile-not-found`, `profile-exists`, `profile-has-live-panes`, `pane-not-in-profile`,
595	`pane-in-other-profile`, `profile-secret-refused`, `herdr-version-unsupported`, `timeout`, `pane-not-found`,
596	`session-not-found`, `store-corrupt`, `unavailable`, and `profile-drift` (a reconcile finding kind, listed for convenience; it
597	is not an error a port returns). No story edits that list (ruling 3). A verb or adapter declares any other code it owns as a
598	constant in its own file through `RefusalCode::from_static`, which refuses a malformed or closed code at build time, and
599	raises it as `PaneError::Refused`. **Stable** means a code is never renamed or reused for another meaning once it has
600	merged; a code whose condition disappears is retired, not recycled. Every message is one line and never echoes a secret.
601	
602	**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
603	story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
604	
605	| Verb | Codes beyond the common ones |
606	|---|---|
607	| `pane list`, `pane watch` | `profile-not-found`, `pane-not-in-profile` |
608	| `pane get` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile` |
609	| `pane launch`, `pane relaunch` | `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `profile-secret-refused`, `probe-failed`, `herdr-version-unsupported`, `session-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-in-other-profile`; `relaunch` also `pane-not-found`; open (#644) for a name that already has a record, an occupied target cell and a port a server already answers on (`pane-exists`, `grid-occupied`, `port-in-use`); open (#640) for a cell that no single split reaches, `grid-unreachable`; open (#802) for a harness that cannot apply the spec's agent, `agent-unsupported` (the OpenCode adapter's answer to every agent until #642 part 3) |
610	| `pane switch`, `pane reset` | `pane-not-found`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645): `orchestrator-pane`, `server-unhealthy`, for `switch` `session-of-other-pane`, and for `reset` a session of record that is running a turn or holds a question (`session-busy`, `session-holds-question`) and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default) |
611	| `pane park`, `pane unpark` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
612	| `pane close` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-not-in-profile` (with `--profile`, P holds no spec for the pane) |
613	| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
614	| `pane import` | `profile-exists` (a profile `fleet` already exists); `command-not-argv` and disagreements are findings, not errors (#650) |
615	| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found` (no session of record); open (#646), as the record says: `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; all exit 3, the code in the message. The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"` |
616	| `profile create` | `profile-exists`, `profile-not-found` (for `--from`), `pane-in-other-profile`, `generation-conflict`, `profile-conflict` |
617	| `profile delete` | `profile-not-found`, `profile-has-live-panes`, `generation-conflict`, `profile-conflict` |
618	| `profile list` | none |
619	| `profile show` | `profile-not-found` |
620	| `profile apply` | `profile-not-found`, `pane-in-other-profile` (without `--take-over`), `probe-failed` (per pane; the others proceed), `herdr-version-unsupported`, `grid-out-of-range`, `generation-conflict`, `profile-conflict`; open (#640) for a cell that no single split reaches, `grid-unreachable`; open (#802), through the launch engine, `agent-unsupported` |
621	| PROPOSED `profile rename`, `export`, `import` | `profile-not-found`, `profile-exists`, `profile-secret-refused`, `command-not-argv`, `grid-ambiguous`, `grid-out-of-range` |
~~~~

**E42: `docs/adr/ADR-0021.md:623-645`** — The `--format=json` envelope, `pane watch` as NDJSON, and the exit codes (the same in both formats).

Command: `git show main:docs/adr/ADR-0021.md | sed -n '623,645p'`

~~~~text
623	**Decided (decision 6): the `--format=json` envelope.** Every pane and profile verb and the roster take
624	`--format=text|json` (default text). In JSON mode stdout carries exactly one envelope, and every diagnostic goes to stderr:
625	
626	```json
627	{"schema_version": 1, "ok": true, "data": {}, "error": null}
628	{"schema_version": 1, "ok": false, "data": null, "error": {"code": "pane-not-found", "message": "pane not found: hj-c1r1"}}
629	```
630	
631	- `error` is `null` exactly when `ok` is true. `error.code` is the `PaneError` code verbatim and `error.message` its one-line
632	  text; the envelope has no `detail`. Each verb supplies only its `data`. The types and `emit()`, `emit_stream()` and
633	  `emit_usage_error()` are in `holler-cli/src/output.rs` (#670 creates it, #660 completes it).
634	- `pane watch` in JSON mode writes NDJSON: one envelope per line, flushed per line.
635	- **Exit codes are the same in both formats (operator, 2026-10-09):** 0 ok, 1 runtime failure, 2 usage, 3 refusal. A
636	  refusal is a request the system understood and declined, working as designed, so nothing was wrong with the system
637	  (`pane-in-other-profile`, `profile-secret-refused` and the like); this is [ADR 0003](ADR-0003.md)'s exit 3, a fail-closed
638	  policy refusal, carried to the new verbs. A failure is something that went wrong while doing the work (a timeout, an
639	  unreachable hub or adapter, a conflict between writers). A usage error under the `pane` or `profile` subcommands in JSON
640	  mode is an envelope with code `usage` and exit 2. In JSON mode `ok` is false for exit 1, 2 and 3 alike. Which `PaneError`
641	  code is a refusal and which a failure is decided once, in one function in `holler-pane`, `holler_pane::error::class_of`
642	  ([#676](https://github.com/Performant-Labs/holler/issues/676); see "Decisions taken", item 5). The CLI's output module and
643	  the test kit (#638) call it, and `ErrorClass::exit_code` turns its class into the number; the table below is its
644	  classification. An open code is always a refusal (exit 3), so a verb that reports a runtime failure uses a closed
645	  failure code; a malformed code is a failure. This **amends the epic's output contract**, which said refusals exit 1.
~~~~

**E43: `docs/adr/ADR-0021.md:647-672`** — The class of every code (the table `class_of` implements).

Command: `git show main:docs/adr/ADR-0021.md | sed -n '647,672p'`

~~~~text
647	  | Code | Class (exit) | Reason |
648	  |---|---|---|
649	  | `usage` | Usage (2) | The request is malformed; ADR 0003's exit 2. |
650	  | `grid-ambiguous` | Refusal (3) | The `GridPos` guard declined text it cannot read one way. |
651	  | `grid-out-of-range` | Refusal (3) | The `GridPos` guard declined a row or column outside its bounds, or the Herdr port declined a cell outside the workspace. |
652	  | `command-not-argv` | Refusal (3) | Policy: a stored command is an argv array, never a shell string. |
653	  | `env-name-invalid` | Refusal (3) | The `EnvVarName` guard declined a blank or whitespace name. |
654	  | `profile-secret-refused` | Refusal (3) | Policy I7: a profile holds names, never values. |
655	  | `profile-exists` | Refusal (3) | The name is taken; nothing went wrong. |
656	  | `profile-has-live-panes` | Refusal (3) | Delete is declined while panes of the profile are live. |
657	  | `pane-not-in-profile` | Refusal (3) | The `--profile` scope check declined a pane outside the profile. |
658	  | `pane-in-other-profile` | Refusal (3) | Ownership policy: a pane belongs to at most one profile (C3). |
659	  | `probe-failed` | Refusal (3) | The health gate declined to launch or relaunch, working as designed. |
660	  | `herdr-version-unsupported` | Refusal (3) | A fail-closed version gate, like ADR 0003's exit 3. |
661	  | `profile-not-found` | Refusal (3) | The named profile does not exist; the request was understood and declined. |
662	  | `pane-not-found` | Refusal (3) | The named pane does not exist. |
663	  | `session-not-found` | Refusal (3) | The named harness session does not exist. |
664	  | `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
665	  | `profile-conflict` | Failure (1) | The profile moved after the live change (the I8 write order); a race. |
666	  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. Its `op` names the port method that ran out, as `<port>.<method>` (`herdr.ensure_pane`, `harness.health`), in every implementation and fake, except that `launch` and `relaunch` name their own bound, `pane.launch` and `pane.relaunch` (section 12); an adapter's own wire steps do not appear in it. |
667	  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply), or the live state a verb needs is not there after its act (#644). |
668	  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
669	  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
670	  | `profile-drift` | Failure (1) | A finding kind, not an error a port returns; if one surfaces, live state disagrees with its spec. |
671	  | any other well-formed code (open, raised as `Refused`) | Refusal (3) | `Refused` is the channel for the codes a verb or adapter owns; the open codes so far (#640's `grid-unreachable`, #645's `session-busy`, `session-holds-question` and `prompt-unsupported`, #646's `pane-parked`, `pane-unhealthy` and `pane-shown-driven-mismatch`, all merged, and #802's `agent-unsupported`) are all refusals. |
672	  | a malformed code | Failure (1) | It cannot be trusted: the same rule turns a garbled wire reply into `unavailable`. |
~~~~

**E44: `docs/adr/ADR-0021.md:678-683`** — `--json` means `--format=json`; `schema_version` is 1 and only a removal or retyping changes it.

Command: `git show main:docs/adr/ADR-0021.md | sed -n '678,683p'`

~~~~text
678	- `--json` on these verbs means `--format=json`, and `--json --format=text` is a usage error (#670).
679	- `schema_version` is an integer and is 1 now. Adding a field to the envelope or to a verb's `data` does not change it.
680	  Removing, renaming or retyping one does, and is a breaking change under ADR 0003's versioning (the JSON shape).
681	- Grid positions in `data` serialize through `GridPos`.
682	- **Deferred to #648:** how `roster`'s existing `--json` output and the envelope coexist (#670's format resolver reports
683	  whether `--format=json` was given explicitly).
~~~~

**E45: `docs/adr/ADR-0021.md:685-714`** — Grid notation: accepted input, the two refusals, output, and that a position comes from Herdr's snapshot and never from the name.

Command: `git show main:docs/adr/ADR-0021.md | sed -n '685,714p'`

~~~~text
685	### 10. Grid notation (decision 7)
686	
687	`GridPos { row: u16, col: u16 }`, 1-based, defined once in `holler-pane/src/grid.rs`.
688	
689	- **Accepted input:** `r<row>c<col>`, `c<col>r<row>` (labelled, either order) and a bare `<row>,<col>` (always row first,
690	  never column first). Upper case and surrounding ASCII whitespace are accepted; `R2C1`, `c1r2` and `2,1` are all row 2,
691	  column 1.
692	- **`grid-ambiguous`:** interior whitespace, a missing half (`r2`, `c1`), a repeated label or a mix (`r2c1c3`, `c1r2c3`), more
693	  than two numbers (`2,1,3`), unlabelled digits (`21`), the empty string, anything unreadable, and a JSON `pos` that
694	  disagrees with its `row` and `col`.
695	- **`grid-out-of-range`:** a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535; or, from `HerdrPort::ensure_pane`,
696	  a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort`
697	  conformance suite pins (#638 amendment 2026-10-08, grid; #683). Herdr has no grid, so a workspace's extent is the Herdr
698	  adapter's configuration: rows by columns per workspace label (`HerdrConfig.workspaces`, #640), which the wiring supplies
699	  (#649). A cell inside the extent that no single right or down split reaches is the open code `grid-unreachable` (#640).
700	- **Output** is always row first: text `r2c1`; JSON `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order. On input `pos`
701	  is optional.
702	- The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base. `HerdrPort::ensure_pane`
703	  reaches a position by right and down splits (B4, `plan_splits`, #640) or fails loudly, and never relocates a healthy pane.
704	  `relaunch` moves a pane only when `--grid` is given. A pane's position comes from Herdr's snapshot, never from its name:
705	  `hj-c1r2` sits at `r2c1`.
706	- A pane's name is not its position and not its identity in Herdr: the importer (#650) finds a pane's live Herdr pane by
707	  `HerdrPane.label == name`. A pane with no cell has no place in the snapshot and is a finding of the importer, never
708	  recorded.
709	- Herdr API versions (B5): the Herdr adapter checks Herdr's protocol when it connects, so no adapter exists for an unknown
710	  one, and `HerdrPort::version()` checks it again on every call; the other methods trust the check made at connect (#640).
711	  An unknown version is `herdr-version-unsupported`, and the message names the supported ones (#640, from #636's list). The
712	  string `version()` returns is recorded as `host.herdr_api_version` by a verb, never by the adapter (decided, operator, 2026-10-11:
713	  #644's launch and relaunch record it, so a pane's recorded version is the one seen at its last launch), and `doctor`
714	  shows it (#647).
~~~~

**E46: `docs/adr/ADR-0021.md:766-771`** — No hub timer; a pass keeps no memory between runs, so every tick reports every standing finding again.

Command: `git show main:docs/adr/ADR-0021.md | sed -n '766,771p'`

~~~~text
766	**Decided (#647): no hub timer.** Under ruling 1 the hub runs no adapters, so it cannot reconcile: the periodic pass is the
767	CLI verb itself, which a scheduler runs as `holler pane doctor --format=json` (without `--fix`) as often as wanted, each run
768	recording `last_observed` and `harness.health` where they changed. The engine (`holler_pane::reconcile::reconcile`) takes
769	only the ports and a request, so a later daemon or `--every` loop can call it unchanged. None is built now: a pass keeps no
770	memory between runs, so every tick would report every standing finding again, and suppressing them needs stored state that
771	a later story may add.
~~~~

**E47: `docs/adr/ADR-0021.md:773-797`** — Migration from the launcher's files: each file is "replaced by registry state", the order, and what the cutover means.

Command: `git show main:docs/adr/ADR-0021.md | sed -n '773,797p'`

~~~~text
773	### 13. Migration from the launcher's files
774	
775	Each file below is replaced by registry state:
776	
777	| File | What it held | Replaced by |
778	|---|---|---|
779	| `sessions.toml` | the session the launcher recorded for each pane | `Pane.session_of_record` |
780	| `last-session` | the last session a pane used | `Pane.session_of_record`, with `last_observed` for what was seen |
781	| `owned-sessions` | the sessions the launcher created for a pane | `session_of_record`; any other session on the pane's server is a stray-session finding from reconcile (#647) |
782	| `pane.env` | a pane's model, effort and environment | `Pane.model`, `Pane.env` (names only; values stay in the operator's own environment), `Pane.context`, and the spec |
783	| the launcher's port and directory table | each pane's port, directory and launch command | `harness.port`, `host.cwd`, `command` (an argv; a shell-string command is a finding and is imported without a command) |
784	
785	The hub's own registration, the fifth record of the incident, is replaced by `session_of_record` through the wiring of #649.
786	
787	The order:
788	
789	1. `pane import --from <state-root> --dry-run` (#650) reports every disagreement between these files and what Herdr, tmux
790	   and the harness show, and writes nothing.
791	2. The real run writes one record per pane. It sets `session_of_record` only where the sources agree and the session exists;
792	   otherwise the record has none and the report has a finding. It takes each position from Herdr's snapshot, never from the
793	   name. It records model, effort, env names and ceilings (C1), creates the profile `fleet` with one spec per imported pane, and
794	   sets each pane's `profile` to `fleet`. It deletes nothing from the source files.
795	3. The operator cuts over (#654): from then on only pane verbs change panes, and the launcher's files are no longer read.
796	   Retiring the launcher's writers is pfleet's follow-up (#653), not Holler's. After seven days on the live fleet (#654) the
797	   operator recreates the Herdr workspace from the stored profiles (#666) and watches it for 24 hours (C4).
~~~~

**E48: `docs/README.md:34-38`** — The index the guide is added to ("Guidance" section).

Command: `git show main:docs/README.md | sed -n '34,38p'`

~~~~text
34	## Guidance
35	
36	- [**orchestrating.md**](orchestrating.md) — the orchestrator pattern: dispatch with `say`, don't poll `roster`, block on a deterministic `wait`, redirect with `interrupt`, resolve with `answer` (issue #142).
37	- [**guidance-effective-agent-monitoring.md**](guidance-effective-agent-monitoring.md) — how to monitor a body/agent effectively (operating guidance, not a protocol or an ADR).
38	- [**setup-wizard.md**](setup-wizard.md) — watching multiple attach-mode sessions in one local [Herdr](https://herdr.dev) workspace: plain `opencode attach` panes (not `herdr-mirror`), and the config-driven setup wizard (installs Herdr itself, then optionally configures and launches a workspace) that automates it.
~~~~

**E49: `docs/orchestrating.md:108-145`** — The interim section being rewritten: the OpenCode path (108-123) and the Claude Code subsection (125-145).

Command: `git show main:docs/orchestrating.md | sed -n '108,145p'`

~~~~text
108	## Restarting a pane on another model (interim)
109	
110	Until holler pane relaunch with --model (#644) and the pfleet adoption (#653) land, `scripts/pane-restart`
111	restarts one fleet pane's OpenCode TUI on a new model. Run it on the host that owns the pane's tmux session
112	(the fleet host), from any shell except the pane being restarted:
113	
114	```bash
115	scripts/pane-restart c3r1 --model anthropic/claude-sonnet-5-5 --effort medium \
116	  --key-file '$HOME/.holler-oc/keys/anthropic.key'      # add --dry-run first to see the plan
117	```
118	
119	It prints one line: `c3r1: OK model=… session=…`, `c3r1: REFUSED: …` (exit 1, nothing changed) or
120	`c3r1: FAIL step N (name): …` (exit 2, with the `pane.env` backup path). It refuses the MO pane (`c4r2`) and
121	any pane that is not idle. Pause the watchdog yourself first if you do not want it acting on the pane meanwhile.
122	Do not type `tmux send-keys` by hand for this: text sent without `Enter` sits unsubmitted in the TUI prompt
123	(the 2026-10-10 failure that motivated the script). This section is removed when #644 and #653 have merged.
124	
125	A pane running **Claude Code** is detected, not named: the script looks for a live Claude Code process whose
126	record in `~/.claude/sessions/<pid>.json` names the pane's tmux session (`hj-c3r1`). Such a pane takes a Claude
127	model alias or name and an effort level, and none of the OpenCode options:
128	
129	```bash
130	scripts/pane-restart c3r1 --model opus --effort high --dry-run   # alias (opus, sonnet, fable) or full name, optional [1m]
131	scripts/pane-restart c3r1 --effort xhigh                         # an omitted --model or --effort keeps its old value
132	scripts/pane-restart c3r1 --model sonnet --fresh                 # a new session instead of resuming the old one
133	```
134	
135	It refuses unless the record says `idle` and the screen shows neither `esc to interrupt`, a question
136	(`Enter to select`) nor a running agent row (`◯ name`). It stops the process (`/exit`, then one Ctrl-C),
137	starts `claude` again in the same directory with the flags it had, the new model and effort, and
138	`--resume <session id>`, and waits for the new process to report `idle`. A session that has no saved
139	conversation yet cannot be resumed and is refused before anything changes; pass `--fresh`. The model is confirmed
140	from the new process's own arguments; no prompt is sent, so nothing is spent. A running process with a flag the
141	script does not know how to carry over (a prompt, `--mcp-config`) is refused and the flag is named.
142	Keys are typed into the pane id the record names (`hj-c3r1:@6.%6` gives `%6`), not into the session, so a second pane
143	or window in the same tmux session is never touched. Not carried over: environment variables set on the old command
144	line, and a model or effort changed inside the session with `/model` or `/effort` (only the command-line arguments are
145	read). The first Ctrl-C of the fallback only warns in Claude Code, so the script sends two.
~~~~

**E50: `scripts/pane-restart:1-30`** — The script's header: usage lines for OpenCode and Claude Code panes, the exit codes, and line 3, the stop-gap sentence.

Command: `git show main:scripts/pane-restart | sed -n '1,30p'`

~~~~text
1	#!/usr/bin/env bash
2	# pane-restart: restart one fleet pane's OpenCode or Claude Code TUI on a new model, in its existing tmux session.
3	# A stop-gap (issue #746) until `holler pane relaunch --model` (#644) and the pfleet adoption (#653) land.
4	#
5	#   pane-restart PANE --model PROVIDER/ID [--effort E] [--key-file PATH] [--project DIR] [--agent KEY] [--dry-run]
6	#   pane-restart PANE [--model ALIAS|NAME] [--effort E] [--fresh] [--dry-run]            (a Claude Code pane, see below)
7	#
8	# PANE is cCrR (the tmux session is hj-PANE, derived, never typed). Steps, in order; the first failure stops the
9	# script and names the step:
10	#   1 refuse unless the pane is idle and reachable   2 back up pane.env   3 set the OC_ lines in pane.env
11	#   4 stop the TUI (/exit, then one Ctrl-C)          5 start it fresh      6 wait for the server
12	#   7 wait for the new session to register (and repair pfleet's record with --set-registration)        8 the pane answers on the new model   9 pane-ready says READY
13	# Every text typed into a pane is followed by Enter: in the same `tmux send-keys` call for the shell line, as a
14	# separate call after a short pause for `/exit` (the TUI's autocomplete popup swallows an Enter sent with it). Only the key file's PATH is written to
15	# pane.env; the file is never read or printed. Exit: 0 ok, 1 refused (nothing changed), 2 failed after a change.
16	# --mo restarts c4r2, the MO pane: it edits opencode-mo/opencode.json and agents/mo.md instead of pane.env, launches
17	# with OC_SESSION_NAME=mo and no --agent, needs --key-file, accepts a busy pane, and skips pane-ready.
18	# A pane that is already fully down is started without the stop step; that needs --project.
19	# A Claude Code pane (issue #784) is found, not named: a live Claude Code process whose per-process record in
20	# ~/.claude/sessions/<pid>.json names the pane's tmux session. It is restarted through its own command line, with no
21	# pane.env and no oc-holler: refuse unless the record says idle and the screen shows neither 'esc to interrupt', a
22	# question ('Enter to select') nor a running agent row ('◯ name'); stop it (C-u, /exit, then two Ctrl-C); start `claude` again in the same
23	# directory with the flags it had, the new --model and/or --effort (an omitted one keeps its old value), and
24	# `--resume <its session id>` (--fresh starts a new session instead; a session with no saved conversation can only be
25	# restarted with --fresh, and is refused before anything changes without it); wait for the new process and for it to report
26	# idle. The model is confirmed from the new process's own arguments, because no prompt is sent. A process whose command
27	# line has arguments this script does not know how to carry over is refused. --model takes an alias (opus, sonnet,
28	# fable) or a full name, with an optional [1m]; --effort is low, medium, high, xhigh or max.
29	# Keys go to the pane id in the record (hj-c3r1:@6.%6 -> %6), not to the session's active pane. Not carried over: the old
30	# command's environment variables, and a model or effort set inside the session with /model or /effort.
~~~~

**E51: `.github/workflows/ci.yml:380-385`** — The script has its own CI test; line 3 of the script is a comment no test reads.

Command: `git show main:.github/workflows/ci.yml | sed -n '380,385p'`

~~~~text
380	      # scripts/pane-restart (issue #746) is tested against fake tmux, curl and pfo
381	      # on PATH and a temporary HOME; it never touches a live pane. Runs once (ubuntu).
382	      - name: pane-restart (scripts/tests/pane-restart-test.sh)
383	        if: env.SKIP_THIS_LEG != 'true' && (matrix.os == 'ubuntu-latest')
384	        run: bash scripts/tests/pane-restart-test.sh
385	        shell: bash
~~~~

**E52: `CHANGELOG.md:1-9`** — Where the `## [Unreleased]` entry goes.

Command: `git show main:CHANGELOG.md | sed -n '1,9p'`

~~~~text
1	# Changelog
2	
3	All notable changes to `holler` are documented here, per [Keep a
4	Changelog](https://keepachangelog.com/en/1.1.0/). See
5	[`docs/releasing.md`](docs/releasing.md) for the entry structure and the process that
6	fills this file in at release time.
7	
8	## [Unreleased]
9	
~~~~

**E53: `crates/holler-cli/tests/docs_cli_test.rs:1-31`** — The existing docs test whose rules the new guide must obey, and the style the new test copies (`holler_cli::Cli` is public).

Command: `git show main:crates/holler-cli/tests/docs_cli_test.rs | sed -n '1,31p'`

~~~~text
1	#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
2	//! Docs doc-test (issue #155 §6): every `holler …` command the documentation
3	//! shows must parse against the real clap tree.
4	//!
5	//! Two sources, across `README.md` and `docs/**/*.md`:
6	//! 1. lines inside fenced code blocks (`bash`/`sh`/`shell`/`console`/`text`/
7	//!    untagged) that start with `holler ` (an optional `$ ` prompt is
8	//!    stripped),
9	//! 2. inline code spans `` `holler …` `` anywhere in the prose (a few
10	//!    `v2.md` §10.1 wire-mapping rows, e.g. `hub query <target> caps`).
11	//!    The bulk of the surface is the `v2.md` §10 code block (the ADR 0003
12	//!    table reproduced verbatim, #148), which arrives via source (1).
13	//!
14	//! Placeholders are normalised (`<x>` → `x`, `[optional]` dropped, `a|b` →
15	//! `a`, `…` dropped, a trailing annotation after two spaces / ` (` / ` — `
16	//! cut). A command whose leaf verb is in `cli-surface.pending.txt` is
17	//! tolerated (specified, not yet implemented) and reported, never silently
18	//! skipped.
19	
20	mod fixture;
21	
22	use std::path::{Path, PathBuf};
23	
24	use clap::error::ErrorKind;
25	use clap::Parser;
26	use fixture::{pending_leaves, split_args};
27	use holler_cli::Cli;
28	
29	fn workspace_root() -> PathBuf {
30	    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
31	}
~~~~

**E54: `crates/holler-cli/tests/docs_cli_test.rs:90-128`** — What `docs_cli_test` extracts: fences tagged `bash`, `sh`, `shell`, `console`, `text` or untagged (three backticks only), and one-backtick spans that start `holler `.

Command: `git show main:crates/holler-cli/tests/docs_cli_test.rs | sed -n '90,128p'`

~~~~text
90	    let mut out = Vec::new();
91	    let mut in_fence = false;
92	    let mut fence_ok = false;
93	    for (i, line) in text.lines().enumerate() {
94	        let line_no = i + 1;
95	        if let Some(rest) = line.trim_start().strip_prefix("```") {
96	            if in_fence {
97	                in_fence = false;
98	            } else {
99	                in_fence = true;
100	                let lang = rest.trim();
101	                fence_ok = matches!(lang, "" | "bash" | "sh" | "shell" | "console" | "text");
102	            }
103	            continue;
104	        }
105	        if in_fence {
106	            if fence_ok {
107	                let t = line.trim_start();
108	                let t = t.strip_prefix("$ ").unwrap_or(t);
109	                if t.starts_with("holler ") || t == "holler" {
110	                    out.push(Doc { file: file.clone(), line_no, raw: t.to_string() });
111	                }
112	            }
113	            continue;
114	        }
115	        // Inline code spans in prose / tables.
116	        let mut rest = line;
117	        while let Some(start) = rest.find('`') {
118	            let after = &rest[start + 1..];
119	            let Some(end) = after.find('`') else { break };
120	            let span = &after[..end];
121	            if span.starts_with("holler ") {
122	                out.push(Doc { file: file.clone(), line_no, raw: span.to_string() });
123	            }
124	            rest = &after[end + 1..];
125	        }
126	    }
127	    out
128	}
~~~~

**E55: `crates/holler-cli/tests/docs_cli_test.rs:186-201`** — Which parse errors it tolerates (a bare namespace such as `holler pane`); any other error fails it.

Command: `git show main:crates/holler-cli/tests/docs_cli_test.rs | sed -n '186,201p'`

~~~~text
186	            checked += 1;
187	            match Cli::try_parse_from(&argv) {
188	                Ok(_) => {}
189	                // `holler --help` / `--version`, and a bare namespace such as
190	                // `holler hub` or `holler body …` used in prose to name the
191	                // role: a prefix of the real tree is fine, an unknown or
192	                // malformed verb is not.
193	                Err(e) if matches!(
194	                    e.kind(),
195	                    ErrorKind::DisplayHelp
196	                        | ErrorKind::DisplayVersion
197	                        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
198	                        | ErrorKind::MissingSubcommand
199	                ) => {}
200	                Err(e) => failures.push(format!("{}:{}: `{}` → {:?}: {}", doc.file, doc.line_no, doc.raw, argv, e.kind())),
201	            }
~~~~

**E56: `crates/holler-cli/tests/cli_surface_test.rs:32-51`** — The clap-tree walk that filters clap's own `help` subcommand.

Command: `git show main:crates/holler-cli/tests/cli_surface_test.rs | sed -n '32,51p'`

~~~~text
32	/// Every leaf verb path clap knows (`hub token mint`, `roster`, …), built by
33	/// walking the derived command tree. `help` is clap's own, not ours.
34	fn clap_leaves() -> BTreeSet<String> {
35	    fn walk(cmd: &clap::Command, prefix: &str, out: &mut BTreeSet<String>) {
36	        let mut subs = cmd.get_subcommands().filter(|c| c.get_name() != "help").peekable();
37	        if subs.peek().is_none() {
38	            out.insert(prefix.trim().to_string());
39	            return;
40	        }
41	        for sub in subs {
42	            walk(sub, &format!("{prefix} {}", sub.get_name()), out);
43	        }
44	    }
45	    let mut out = BTreeSet::new();
46	    let cmd = Cli::command();
47	    for sub in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
48	        walk(sub, sub.get_name(), &mut out);
49	    }
50	    out
51	}
~~~~

**E57: `crates/holler-proto/tests/docs_errors_test.rs:12-34`** — The precedent for slicing a docs section by its numbered heading and checking one row per code of a closed table.

Command: `git show main:crates/holler-proto/tests/docs_errors_test.rs | sed -n '12,34p'`

~~~~text
12	fn v2_md() -> String {
13	    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol/v2.md")).unwrap()
14	}
15	
16	/// The text of §8 (from its heading up to §9's).
17	fn errors_section(doc: &str) -> &str {
18	    let start = doc.find("\n## 8. Errors").expect("§8 heading");
19	    let rest = &doc[start..];
20	    let end = rest.find("\n## 9.").expect("§9 heading");
21	    &rest[..end]
22	}
23	
24	#[test]
25	fn every_error_code_has_a_row_in_section_8() {
26	    let doc = v2_md();
27	    let section = errors_section(&doc);
28	    for c in Code::ALL {
29	        // The JSON-RPC-standard codes (-327xx / -326xx) are prose-listed by
30	        // number in the table too; every code has one row starting `| `-NNNNN`` .
31	        let needle = format!("| `{}` | `{}` |", c.jsonrpc(), c.data_code());
32	        assert!(section.contains(&needle), "docs §8 has no row for {} ({})", c.data_code(), c.jsonrpc());
33	    }
34	}
~~~~

**E58: `crates/holler-cli/Cargo.toml:11-14`** — The crate sets `autotests = false`.

Command: `git show main:crates/holler-cli/Cargo.toml | sed -n '11,14p'`

~~~~text
11	# on every build. Disabling auto-detection and declaring the real test targets
12	# explicitly (below) keeps the bin the sole owner of the name while still
13	# building/running both integration tests.
14	autotests = false
~~~~

**E59: `crates/holler-cli/Cargo.toml:82-91`** — How a docs test is declared as a `[[test]]` target.

Command: `git show main:crates/holler-cli/Cargo.toml | sed -n '82,91p'`

~~~~text
82	# CLI-surface fixture and docs doc-test (issue #155 §5/§6). Both read
83	# tests/fixtures/cli-surface*.txt via the shared tests/fixture/mod.rs.
84	[[test]]
85	name = "cli_surface_test"
86	path = "tests/cli_surface_test.rs"
87	
88	[[test]]
89	name = "docs_cli_test"
90	path = "tests/docs_cli_test.rs"
91	
~~~~

**E60: `crates/holler-cli/tests/fixtures/cli-surface.pending.txt:1-10`** — The pending list names no command; this story cannot add to it (it is #670's file, its lines are other stories').

Command: `git show main:crates/holler-cli/tests/fixtures/cli-surface.pending.txt | sed -n '1,10p'`

~~~~text
1	# Verbs and flags ADR 0003 specifies that NO story has implemented yet
2	# (issue #155 §5). Same format as cli-surface.txt.
3	#
4	# tests/cli_surface_test.rs asserts every line here does NOT parse — so when
5	# the owning story lands its verb, the test fails until the line is moved to
6	# cli-surface.txt. The docs doc-test (tests/docs_cli_test.rs) tolerates a
7	# documented command whose leaf path is listed here, and nowhere else.
8	#
9	# Each line names its story.
10	
~~~~

**E61: `crates/holler-pane-testkit/src/envelope.rs:197-202`** — `check_envelope`, the test kit's envelope checker (public; `holler-cli` dev-depends on the test kit).

Command: `git show main:crates/holler-pane-testkit/src/envelope.rs | sed -n '197,202p'`

~~~~text
197	/// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
198	/// code of the verb that wrote it.
199	///
200	/// Returns the envelope, or the fault of the first rule it breaks, in the order the
201	/// module doc lists.
202	pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
~~~~

**E62: `scripts/lint.sh:7-25`** — Lint check 1: every `#[allow(...)]` needs a trailing `// #NNN` issue link.

Command: `git show main:scripts/lint.sh | sed -n '7,25p'`

~~~~text
7	# 1. No blanket dead-code allows, and no #[allow(...)] without an issue link
8	#    on the same line. Exception: a forward-contract stub (a helper shipped
9	#    ahead of its first caller, e.g. the test harness's Hub::start/join) may
10	#    allow(dead_code) — but only if that allow carries a trailing `// #NNN`
11	#    issue link naming the story that lands the caller. Otherwise dead code
12	#    stays dead: land helpers with their first caller.
13	#    An `allow(...)` can also arrive wrapped inside `cfg_attr(cond, allow(...))`
14	#    (#248) — that hides the same bypass from a regex anchored on `#!?\[allow\(`,
15	#    so both checks below also match `allow(` reached via `cfg_attr\([^)]*`.
16	if grep -rn --include='*.rs' -E '(#!?\[allow\([^)]*dead_code[^)]*\)|cfg_attr\([^)]*allow\([^)]*dead_code[^)]*\))' crates/ \
17	    | grep -vE '//\s*#[0-9]+'; then
18	  echo "lint: allow(dead_code) without a '// #NNN' link is forbidden — land helpers with their first caller (or link the forward-contract story)"
19	  fail=1
20	fi
21	if grep -rn --include='*.rs' -E '(#!?\[allow\(|cfg_attr\([^)]*allow\()' crates/ \
22	    | grep -vE '//\s*#[0-9]+' \
23	    | grep -v 'allow(clippy::assertions_on_constants)'; then
24	  echo "lint: every #[allow] needs a trailing '// #NNN' issue link"
25	  fail=1
~~~~

**E63: `docs/agent-overlays/tester.md:14-14`** — What "a bare command fragment in backticks" means in this repository.

Command: `git show main:docs/agent-overlays/tester.md | sed -n '14,14p'`

~~~~text
14	- Every documented `holler ...` command must parse (`docs_cli_test`); a bare command fragment in backticks fails it.
~~~~

**E64: `crates/holler-cli/src/pane/launch.rs:170-178`** — The overlay inside `effective_spec`: `.agent.or(base.opencode_agent)` - an omitted `--agent` keeps the base spec's agent, a given one replaces it.

Command: `git show main:crates/holler-cli/src/pane/launch.rs | sed -n '170,178p'`

~~~~text
170	    let command = values
171	        .command
172	        .clone()
173	        .or(base.and_then(|b| b.command.clone()));
174	    let check = values.check.clone().or(base.and_then(|b| b.check.clone()));
175	    let opencode_agent = values
176	        .agent
177	        .clone()
178	        .or(base.and_then(|b| b.opencode_agent.clone()));
~~~~

**E65: `crates/holler-cli/src/pane/launch.rs:1-20`** — `pane launch` as a transaction: arguments, the effective spec, the engine, the output (`relaunch` shares the same functions).

Command: `git show main:crates/holler-cli/src/pane/launch.rs | sed -n '1,20p'`

~~~~text
1	//! `holler pane launch PANE --herdr-session NAME [SPEC FLAGS] [--profile NAME] [--spec-only]`
2	//! (story #644): create a pane, its tmux session, its harness server and its session of record,
3	//! and record them (ADR-0021 section 8, "Launch and relaunch as built").
4	//!
5	//! The verb types its arguments, reads the profile it names, builds the effective spec, runs the
6	//! engine (`holler_pane::tx_launch::launch`) and prints what it answered. In order:
7	//!
8	//! 1. `PANE` is a pane name (`usage`), the spec flags pass `SpecFlags::validate` (each guard's
9	//!    own code) and `--profile` is a profile name (`usage`).
10	//! 2. With `--profile P`, P is read (`profile-not-found`, and nothing else is called); its spec
11	//!    for the pane, when it has one, is the base the flags overlay. The pane's record is not read
12	//!    here: the engine's first live step does that.
13	//! 3. [`effective_spec`]: every flag given replaces its field of the base. With no base the
14	//!    position, directory, model, effort, ceilings and port policy must all be given.
15	//! 4. The engine, then [`emit_outcome`].
16	//!
17	//! `relaunch.rs` shares [`effective_spec`], [`emit_outcome`], [`Verb`], [`stored_profile`] and
18	//! [`spec_of`], because the frozen `pane/mod.rs` admits no new module.
19	
20	use clap::Args;
~~~~

**E66: `crates/holler-pane/src/reconcile.rs:190-193`** — A pass is whole-fleet only when it names neither a pane nor a profile.

Command: `git show main:crates/holler-pane/src/reconcile.rs | sed -n '190,193p'`

~~~~text
190	    let (scope, records) = resolve(ports, request)?;
191	    let (herdr, checked) = observe_all(ports, request, &scope);
192	    let whole_fleet = request.profile.is_none() && request.pane.is_none();
193	    let mut findings = herdr_findings(herdr.as_ref(), &scope, &records, whole_fleet);
~~~~

**E67: `crates/holler-cli/tests/docs_cli_test.rs:32-55`** — Which markdown the docs test scans: README.md and everything under `docs/` except `docs/handoffs/`.

Command: `git show main:crates/holler-cli/tests/docs_cli_test.rs | sed -n '32,55p'`

~~~~text
32	
33	/// Whether a path relative to the workspace root (a markdown file such as
34	/// `docs/testing.md`, or a directory the walk would enter) holds CLI
35	/// documentation the doc-test must scan (#462).
36	///
37	/// Skipped: `docs/handoffs/`, the coding pipeline's per-story scratch
38	/// (briefs, handoffs, decision journals, gate output). It exists only in a
39	/// pipeline worktree while a story is in flight and is deleted before the PR
40	/// is pushed, so CI never sees it, but `cargo test --workspace` in that
41	/// worktree does, and a handoff that quotes a `holler …` fragment (on #420's
42	/// branch, a Rust format string) failed the whole run on text that is not
43	/// CLI documentation.
44	///
45	/// Scanned, deliberately: every other directory under `docs/`. They hold
46	/// tracked, hand-written documents. That includes `docs/reviews/`, which is
47	/// the review battery's runbook, prompts, overlays and pre-flight script, not
48	/// its output: the battery files its findings as GitHub issues and writes
49	/// nothing under `docs/`.
50	///
51	/// `Path::starts_with` compares whole components, so a page whose name merely
52	/// contains `handoffs` (e.g. `docs/research/handoffs-notes.md`) is scanned.
53	fn is_cli_doc(rel: &Path) -> bool {
54	    !rel.starts_with("docs/handoffs")
55	}
~~~~

**R1: issue #649 body, lines 18-19** — REFRESH AFTER MERGE of #649 - D2: `connect` needs the hub and nothing else (no hub: `unavailable`, exit 1, naming the missing hub); a verb that never touches Herdr works with no Herdr running.

Command: `gh issue view 649 -R Performant-Labs/holler --json body -q .body | sed -n '18,19p'`

~~~~text
18	D2. **`connect` needs the hub and nothing else.** It makes one `control/status` exchange (`ControlCall::status()`, E6); with no hub running (`ControlError::NoLiveHub`, E8) it fails `unavailable` and the run exits 1 with that code, whose message names the missing hub (E3 reports it). `Io`, `BadReply` and `Refused` of that exchange are `unavailable` too. Herdr, tmux and OpenCode are not touched at `connect`: `TmuxHost::new` and `OpenCodeHarness::new` do no I/O (E27, E33), and the Herdr adapter pings Herdr when it is built (E23), so it is built on its first use and the result kept (`OnceLock`). A verb that never touches Herdr (`pane list`, `pane get`, `pane watch`, `profile list`, `profile show`) therefore works with no Herdr running.
19	
~~~~

**R2: issue #649 body, lines 24-28** — REFRESH AFTER MERGE of #649 - D5 to D7: the environment that configures Herdr, tmux and OpenCode (`HOLLER_HERDR_SESSION` and the rest). The merged list is the module doc of `crates/holler-cli/src/pane/wiring.rs`; the guide points there and does not copy the list.

Command: `gh issue view 649 -R Performant-Labs/holler --json body -q .body | sed -n '24,28p'`

~~~~text
24	D5. **Herdr** is `HerdrAdapter::connect(HerdrConfig)` (E22, E23) from the environment of the process: the session is `HOLLER_HERDR_SESSION`, else `HERDR_SESSION` (Herdr sets it inside its panes, E25); the socket is `HOLLER_HERDR_SOCKET`, else `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`~/.config` when `XDG_CONFIG_HOME` is unset, E25); the workspaces are `HOLLER_HERDR_WORKSPACES=label=RxC[,label=RxC]...` (rows by columns, E24, E37, E38). No session configured: the Herdr port answers `usage` naming the two variables when it is first used (store-only verbs are unaffected, D2). `--herdr-session` stays required by `launch` as ADR-0021 records it (E37); the adapter refuses a name other than its configured session as `unavailable` (E24). Defaulting the flag from the wiring (ADR-0021 line 522) is **not** done here: it edits `launch.rs`, outside the Blast radius, and stays recorded as deferred.
25	
26	D6. **tmux** is one `HOLLER_TMUX_SOCKET` (an absolute path becomes `TmuxSocket::Path`; unset is `TmuxSocket::Default`), used for **both** `TmuxHost` (E27) and `OpenCodeConfig.tmux` (E30), whose two `TmuxSocket` types the wiring maps from the one value. The binary is `tmux` from `PATH`.
27	
28	D7. **OpenCode** is `OpenCodeConfig` (E30) with: `opencode_bin` = `HOLLER_OPENCODE_BIN`, else `opencode` resolved on `PATH` to an absolute path (E30 says wiring passes one); `serve_args` = `HOLLER_OPENCODE_SERVE_ARGS`, a JSON array of strings, default none; `env` = `ProcessEnv::Inherit` always (E29); the default `Timeouts`. No binary found: the harness answers `unavailable` naming `HOLLER_OPENCODE_BIN` when first used. **No `HostPort` amendment and no credential handling here**: a provider credential reaches the server through the operator's tmux server environment (E29, E39); how production sets it is the cutover's (#654), and nothing in this story puts a credential on an argv, in a file or in a log.
~~~~

**R3: issue #649 body, lines 52-52** — REFRESH AFTER MERGE of #649 - D18: how the hub's DRIVEN session follows `session_of_record` is deferred to #654; `last_observed.driven` stays as stored.

Command: `gh issue view 649 -R Performant-Labs/holler --json body -q .body | sed -n '52,52p'`

~~~~text
52	D18. **Not done here, and not decided here.** (a) *DRIVEN:* how the hub's DRIVEN session follows `session_of_record` stays deferred to #654 (E41); this story wires nothing for it, `last_observed.driven` stays as stored, and the scenario asserts SHOWN against `session_of_record` only. (b) The whole-fleet defaults of ADR-0021 line 522. (c) Moving the first message to the hub's prompt path (#646). (d) Importing the live fleet (#650) and the cutover (#654).
~~~~

**R4: issue #650 body, lines 5-5** — REFRESH AFTER MERGE of #650 - the synopsis of `holler pane import`: `--from` and `--herdr-session` are required.

Command: `gh issue view 650 -R Performant-Labs/holler --json body -q .body | sed -n '5,5p'`

~~~~text
5	`holler pane import --from <state-root> --herdr-session NAME [--launcher FILE] [--orchestrator PANE]... [--dry-run] [--format=text|json]`
~~~~

**R5: issue #650 body, lines 23-38** — REFRESH AFTER MERGE of #650 - the join, per pane: label as the join key, the position from Herdr's snapshot, the session of record only when a healthy server holds it, the role from `--orchestrator`, and `harness-not-opencode` (items 3 to 6).

Command: `gh issue view 650 -R Performant-Labs/holler --json body -q .body | sed -n '23,38p'`

~~~~text
23	### The join, per pane
24	
25	1. **The Herdr pane is the snapshot entry whose `label` equals the pane's name** (ADR-0021 section 10, E9, merged by #808, E1). The name's cell is never the join key and never chooses a position. Zero matches: problem `pane-unlabelled`. Two or more: problem `pane-label-ambiguous` (naming the pane ids). The record's `herdr` is that snapshot entry **with `label` set to `None`** (a label is an observation, never recorded state; launch does the same, E12).
26	2. **The position** is the entry's `grid`. Output prints rowcol (`r2c1`). If the directory name is `hj-` followed by text that `GridPos::parse` accepts (for example `c1r2`) and that cell differs from the observed position, the report carries the information finding `name-cell-differs` (naming both cells); a name that spells no cell produces no finding.
27	3. **The tmux session** exists when `host.ps(name)` answers; the error `pane-not-found` is the problem `no-tmux-session`; any other error is the warning `observe-failed`, named `host.ps` (reconcile's `<port>.<method>` form, E17).
28	4. **The session of record.** Let `L` be the content of `last-session`, `S` the set of `session_id`s in `sessions.toml`, `O` the lines of `owned-sessions`. The files **agree** when `L` is present and every other present file names it (`L` is in `S`, and `L` is in `O` when `owned-sessions` exists). Recency is never a tie-break. Results:
29	   - files name different sessions (`L` missing from a present `S` or `O`): problem `sources-disagree` (naming the files, not the ids' ownership), **no record for the pane**;
30	   - the files agree and a **healthy** server's `list_sessions` holds `L`: `session_of_record = L` (reconcile's trust rule, E17: only a healthy server's list counts);
31	   - the files agree but the healthy server's list does not hold `L`: the record is written **without** `session_of_record` and the warning `session-not-found`;
32	   - the health check or the list failed or the server is not healthy: the record is written without `session_of_record`, `harness.health` is `unknown`, and the warning `observe-failed` names the failed call;
33	   - `last-session` absent or empty: the record is written without `session_of_record`, warning `no-session-recorded`.
34	   A session id read from a file is never copied into `last_observed` (I6).
35	5. **The role** is `orchestrator` for a pane named by `--orchestrator` and `agent` for every other pane. A `--orchestrator` name that matches no pane directory is a usage error (exit 2) before anything is read. The role is never derived from a name or a missing file. A pane named by `--orchestrator` still needs its own `pane.env` (the Pane record requires a model and ceilings and the import invents no defaults): the MO pane, whose config lives in `opencode-mo/` (E10), gets a small `pane.env` from the operator before the cutover, and the dry run says so (`source-missing`, naming the file).
36	6. **A pane that is not an OpenCode pane** (a `sessions.toml` row whose `harness` is not `opencode`) is the information-level skip `harness-not-opencode`: no record, never blocking (`HarnessKind` is closed to `opencode`, E2).
37	7. **A stored command** is an argv: `command` as an array becomes `Pane.command`; `command` as a string is the warning `command-not-argv` and the pane is recorded without a command.
38	8. **A pane that already has a record** (`pane_store.get(name)` answers a record) is `already-imported` (information) and is skipped before any other check.
~~~~

**R6: issue #650 body, lines 51-56** — REFRESH AFTER MERGE of #650 - exit codes and what blocks: a dry run exits 0 with the whole report; a real run is all-or-nothing and answers the refusal `import-blocked` (exit 3).

Command: `gh issue view 650 -R Performant-Labs/holler --json body -q .body | sed -n '51,56p'`

~~~~text
51	### Exit codes and what blocks
52	
53	- **Levels.** Each finding has a level: `problem` (the pane gets no record), `warning` (the record is written without the thing the warning names), `skipped` (`harness-not-opencode`) and `info` (`ignored-entry`, `already-imported`, `name-cell-differs`). **Only `problem` blocks.** Problems: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`.
54	- **Dry run:** exit 0 with the whole report, whatever the findings. A `usage` or an adapter error keeps its own class and code.
55	- **Real run:** if any pane has a problem, nothing is written and the verb answers the refusal **`import-blocked`** (exit 3; a constant in `crates/holler-pane/src/import.rs` through `RefusalCode::from_static`, E16), with the message `import blocked: N pane(s) have problems; run with --dry-run for the report` (one line, nothing on stdout in text mode; the failure envelope in JSON). If `fleet` already exists and at least one candidate pane is not yet imported, the verb answers `profile-exists` (exit 3). If every candidate pane is `already-imported` (or skipped) there is nothing to write: exit 0, the report lists `already-imported`, no profile is created, no store write is made. With no candidate pane at all: exit 0, nothing written.
56	- **Missing `<state-root>`** (not a directory): the `usage` error, exit 2, one line.
~~~~

**R7: issue #650 body, lines 63-65** — REFRESH AFTER MERGE of #650 - the finding vocabulary (`ImportFinding`, with an `ALL` list and `code()`): the new codes, and the conditions that keep reconcile's spelling.

Command: `gh issue view 650 -R Performant-Labs/holler --json body -q .body | sed -n '63,65p'`

~~~~text
63	### The finding vocabulary
64	
65	A new single-source enum `ImportFinding` in `crates/holler-pane/src/import/findings.rs`, copying `FindingKind`'s pattern (E6): `code()` is the one source of each code, `Serialize` is built from it, there is an `ALL` list, and a code-table test (unique, kebab-case, `is_valid_code`) lives in `crates/holler-pane/tests/import_test.rs`. A condition that reconcile already names keeps reconcile's spelling through `FindingKind::ObserveFailed.code()` and `FindingKind::HerdrVersionUnsupported.code()`; `command-not-argv` is the closed `PaneError` code. The new codes are exactly: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`, `session-not-found`, `no-session-recorded`, `harness-not-opencode`, `name-cell-differs`, `already-imported`, `ignored-entry`. `findings.rs` and `reconcile.rs` are #647's files and are not edited.
~~~~

**R8: issue #650 body, lines 69-78** — REFRESH AFTER MERGE of #650 - the operator decisions 1 to 10 (join key, `--orchestrator`, all-or-nothing, re-run is a no-op, names only).

Command: `gh issue view 650 -R Performant-Labs/holler --json body -q .body | sed -n '69,78p'`

~~~~text
69	1. **Join key: the Herdr label.** The name's cell and the foreground process are forbidden as keys. (#808, merged; ADR-0021 section 10.)
70	2. **Role: a repeatable `--orchestrator PANE` flag**; every other pane is an agent; the dry run prints each pane's role; never from a name. (Reason: the role gates the orchestrator protections, so it must be stated, not guessed.)
71	3. **A label that matches no pane or two panes is a finding and nothing is recorded for that pane** (fail closed). (Reason: guessing which pane is which re-creates the incident.)
72	4. **Files that disagree about the session produce a finding and no record for that pane**; the real run is then refused until the operator fixes the drift with the old tools, as the cutover runbook says. This narrows ADR-0021 section 13 step 2's wording (a disagreement used to import the pane without a session of record); section 13 is edited to say so. Files that **agree** while the server does not hold the session still import the pane without a session of record (warning). (Reason: a record that is wrong in a way the operator has not looked at is worse than no record.)
73	5. **Exit codes:** dry run 0 with findings; a real run is all-or-nothing and exits 3 (`import-blocked`) when any pane has a problem, following `profile create --from-current` (checks first, the profile through `insert_profile`, then the members, an undo that deletes what the run created).
74	6. **`pane.env`: names only, never values**, except the five named variables whose values become record fields. (Reason: I7, no secret in any record, log or finding.)
75	7. **A pane that does not run OpenCode** is a skipped finding and no record, and does not block. (Reason: `HarnessKind` is closed.)
76	8. **Re-running after an import is a no-op that exits 0** (`already-imported`); it creates no profile and writes nothing. (Reason: safe to repeat during the cutover.)
77	9. **`--herdr-session` is required**, as on `launch`. (Reason: the same flag, the same meaning.)
78	10. **`sessions.toml` is read through `holler_body::config`**, which is the attach-config grammar (E11); no second parser.
~~~~

**R9: `docs/handoffs/664-brief.md` at commit `3e09a1f` (branch `issue-664-implementation`), lines 47-71** — REFRESH AFTER MERGE of #664 - the step table, the outcomes, `--dry-run`, `data` and the flags of `profile apply` (sections 3 to 8 of the #664 brief, as committed on branch `issue-664-implementation`). The standing spec after the merge is ADR-0021 section 9's apply row, `tx_apply.rs` and `profile/apply.rs`.

Command: `git show 3e09a1f:docs/handoffs/664-brief.md | sed -n '47,71p'`

~~~~text
47	**3. The step table.** One row per spec pane, in `(workspace, row, col)` order; the first row that matches applies:
48	
49	| # | Situation | Step | Live effect | Registry effect |
50	|---|---|---|---|---|
51	| a | the pane has a record that belongs to **another profile**, and no `--take-over` | `refused` (`pane-in-other-profile`) | none | none |
52	| b | **no record** | `create` through the launch transaction (it refuses an occupied cell or an existing session through its own checks: that is a `refused` or `failed` step per 5) | launch's | launch's |
53	| c | record, **healthy**, matches its spec (`diff_spec` empty) | `adopt` | none (never touched) | set `Pane.profile` to P if not already (with `--take-over` from another profile: leave then join, two writes) |
54	| d | record, **healthy**, **differs** from its spec | `reported` with the `PaneDiff` | none (apply never kills or rewrites a healthy pane; the operator uses `launch`/`relaunch --profile P`) | with `--take-over` from another profile: leave then join, otherwise none |
55	| e | record, **not healthy**, role **orchestrator** | `refused` (`orchestrator-not-relaunched`) | none | none, **even under `--take-over`** (no profile writes happen for a refused pane) |
56	| f | record, **not healthy**, role agent | `relaunch` through `tx_launch::relaunch` (`grid_given: false`, `profile: Some(P)`) | relaunch's, **after** the profile move below | with `--take-over` from another profile: **leave then join first (two writes, exactly as rows c and d), then the relaunch**, which then sees a record that already names P. `tx_launch::relaunch` is called unchanged and never refuses `pane-in-other-profile` for this step, because the record no longer names another profile when it runs. If the relaunch then fails, the pane stays in P (the intended end state) and the step is `failed`; apply does not compensate the profile move. Without a take-over (the record already names P, or none) there is no profile move, and the relaunch's own write sets `Pane.profile` to P |
57	
58	Row order matters: row a is checked first and fires only for another profile **without** `--take-over`; with `--take-over` the pane falls to rows b to f, and rows c, d and f add the profile move described in the table, **always before any live effect of the same step**; row e never writes.
59	
60	**4. Per-pane answers outside the table.** A step that the launch transaction answers with a **refusal** (exit class 3, for example `probe-failed`, `pane-exists`) is a `refused` step carrying that code. A step answered with `usage` (exit class 2) for a per-pane reason (a relaunch's refusal to move a pane when the spec's cell or directory differs from the record's, a spec whose port policy is not `fixed:<port>`) is also a `refused` step carrying a named code, not a run failure. **`create` without `--herdr-session`** is checked **before any step runs**: if the plan contains a `create` and the flag is absent, the whole verb exits 2 (`usage`) and no step runs, in a dry run too.
61	
62	**5. The exhaustive outcome set and what each contributes.** Every step ends in exactly one of: `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`.
63	- `refused` and `reported` do **not** stop the run and do **not** change the exit code.
64	- `failed` is any failure-class answer (exit class 1: `unavailable`, `timeout`, a failed `health` call, a store conflict) and **stops the run at that step**; steps after it are not attempted and are listed as `skipped`.
65	- Run exit code: **1** if any step is `failed`; **2** for the up-front `usage` case in 4; otherwise **0**. This is the doctor form (ADR-0021 section 9's apply row is reworded to say so).
66	
67	**6. `--dry-run`** prints the same table as the plan and changes nothing: it runs no probe, issues no split, writes nothing, and performs no health call that has a side effect; it does the read-only health and snapshot observations needed to choose each row.
68	
69	**7. `data`** is the object `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code?, diff?}]}`, embedding `PaneDiff` unchanged.
70	
71	**8. Flags:** `--herdr-session NAME` (as `pane launch` has it), `--take-over`, `--dry-run`, `--format`.
~~~~

**R10: `docs/handoffs/664-brief.md` at commit `3e09a1f` (branch `issue-664-implementation`), lines 748-761** — REFRESH AFTER MERGE of #664 - the decisions O1 to O6 (take-over order, unreadable shown session, a failed join, no run budget, what a run that ends with a `failed` step prints).

Command: `git show 3e09a1f:docs/handoffs/664-brief.md | sed -n '748,761p'`

~~~~text
748	## Decisions on O1 to O5 (final, 2026-10-11, taken under the operator's standing rules; they replace "Not decided" above)
749	
750	- **O1 (row f under `--take-over`): leave then join first, then relaunch.** The profile move (leave Q, join P: two writes, as rows c and d) happens before the relaunch, so `tx_launch::relaunch` (unchanged) sees a record that names P and does not refuse `pane-in-other-profile`. If the relaunch then fails, the pane stays in P (the intended end state), the step is `failed`, and apply writes nothing to compensate. Row f's table text says the same.
751	- **O2 (a shown session that cannot be read): `Ok(None)` and `Err(_)` both mean "cannot tell" and do not make a pane unhealthy.** Only `Ok(Some(s))` with `shown_differs` true counts as a mismatch. Reason: apply never kills a pane because its TUI shows the home screen; an unreadable reading is reported in the step's detail, not acted on.
752	- **O3 (#802 vs this story):** this story is written against the test kit as it is on `main` when the run starts, seeding servers through one rig helper (`profile_verbs/rig.rs`), so that if #802 merges first, adapting is a one-line rename of that helper and nothing else. Both stories edit `docs/adr/ADR-0021.md` in different sections (this one: the definition of "adopt" and section 9's apply row); the merge of main resolves it. *(Resolved: #802 merged first as `7284001` (PR #817); the helper is adapted as this sentence says, and the apply rig's `run` starts the fake server: C25.)*
753	- **O4 (a take-over whose join fails after its leave landed): leave the pane in no profile and say so.** The step is `failed`, its error text says the pane left Q and is in no profile, and the run stops (section 5). Apply does not write a compensating re-join of Q (a further write that can fail too); the operator re-runs apply, which then sees a pane with no profile and takes it through rows b to f.
754	- **O5 (a budget over the whole run): none.** The bound is per step plus one up-front observation, recorded in the module doc of `crates/holler-pane/src/tx_apply.rs` (section 9) and asserted by a test over that file.
755	
756	## Decision on O6 (final, 2026-10-11; overrides every "O6 ... open / NOT DECIDED" sentence above)
757	
758	- **O6: option (a).** A run that ends with a `failed` step exits 1 and the verb prints **only the failure**: in `--format=json` the failure envelope (`ok` false, `data` null, `error.code` the failed step's error code, `error.message` naming the pane, the step's action and how many later steps were skipped); in text mode one `error: <same message>` line on stderr and nothing on stdout. No plan table and no `skipped` list is printed at exit 1, for a dry run too (a dry run with a `failed` step prints no plan). This narrows §6's "prints the same table as the plan" to runs that do not fail, and C2 reads accordingly. ADR-0021's decision 6, `output.rs` and the test kit's envelope checker are **not** changed.
759	- **Where the `failed` / `skipped` steps are asserted:** the tests behind C14's entries 17, 21, 22, 23, 25 and 29 read the steps from `tx_apply`'s own returned result (an in-process test over the fakes), not from the verb's output. The verb-level test for the same situations asserts exit 1, the failure envelope's code and the message text, and that stdout is empty in text mode.
760	- The `error.message` format is pinned: `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`.
761	
~~~~

**R11: issue #644 body, lines 15-16** — REFRESH AFTER MERGE of #644 part 2 - the agent in the output: text gains `, agent KEY` after `harness port N`; JSON `data` gains the additive key `agent`.

Command: `gh issue view 644 -R Performant-Labs/holler --json body -q .body | sed -n '15,16p'`

~~~~text
15	   - **text:** the one-line result gains `, agent KEY` after `harness port N` and the optional `(pid P)`, before any `; profile ...` clause (E7). A pane with no agent prints exactly the line it prints today (a golden test). (Since #802 no launch records a pid, so `(pid P)` appears only for a record written before it; the clause stays as it is.)
16	   - **JSON:** `data` gains the key `agent` after `spec_only` (additive; every existing key and its order are unchanged): the stored record's `opencode_agent` as a string, or `null` when the pane has none and with `--spec-only`. `Report::new` derives it from `launched.pane`, as it derives `spec_only`, and never from the request's spec, so `data.agent` cannot disagree with `data.pane.opencode_agent`.
~~~~

**R12: issue #642 body, lines 7-8** — REFRESH AFTER MERGE of #642 part 3 - `serve_command` applies the agent; `verify_serving` refuses with the adapter's open code `agent-unusable`.

Command: `gh issue view 642 -R Performant-Labs/holler --json body -q .body | sed -n '7,8p'`

~~~~text
7	1. **`serve_command` applies the agent.** With `spec.agent == Some(key)` the argv it returns starts the server with `default_agent` set to `key`; with `None` it returns exactly the argv it returns today. It no longer refuses an agent with `agent-unsupported`.
8	2. **`verify_serving` checks the running server.** It returns `Ok(())` only when the server really runs the agent the spec asks for, and refuses with the adapter's own open code `agent-unusable` otherwise. The launch engine (#802) calls it after the health wait and before `create_session`, and its rollback then stops the server (a process of the pane's tmux session).
~~~~

**R13: issue #642 body, lines 23-23** — REFRESH AFTER MERGE of #642 part 3 - decision D5: `agent-unusable` is an open refusal code, exit 3, a constant of the adapter's own file; it is not one of `ALL_CODES`.

Command: `gh issue view 642 -R Performant-Labs/holler --json body -q .body | sed -n '23,23p'`

~~~~text
23	- **D5. `agent-unusable` is an open refusal code (exit 3), a constant in the adapter's own file** (`pub const AGENT_UNUSABLE: RefusalCode = RefusalCode::from_static("agent-unusable")` in `crates/holler-adapter-opencode/src/server.rs`, built through `PaneError::Refused`, the way the Herdr adapter owns `grid-unreachable`), **re-exported at the crate root** (`pub use server::AGENT_UNUSABLE;` beside `pub use tui::{TmuxConfig, TmuxSocket};`), because `server` is a private module and the Herdr precedent is a `pub const` of a public module (`plan`): a test or the wiring (#649) must be able to name it, and the tests compare `AGENT_UNUSABLE.as_str()`, never the literal (the test kit's rule, `herdr.rs:50`). It is distinct from `AGENT_UNSUPPORTED`, which stays in `holler-pane` for a harness that cannot apply an agent at all. The ADR-0021 code tables list it wherever they list `agent-unsupported`, as the OpenCode adapter's code and not the only possible one: the test kit's `FakeHarness::verify_serving` refuses the same mismatch with its own `agent-mismatch`, because the port lets each implementation own its code (E3). Reason: "this harness cannot" and "this agent cannot run here" are different refusals for the operator.
~~~~

**R14: issue #665 body, lines 1-7** — REFRESH AFTER MERGE of #665 (not planned before the cutover) - `profile rename`, `export`, `import` and the doctor finding `profile-drift` are PROPOSED.

Command: `gh issue view 665 -R Performant-Labs/holler --json body -q .body | sed -n '1,7p'`

~~~~text
1	Part of epic #633. (added 2026-10-08, profiles) **PROPOSED: the operator confirms before any pane picks this up. Not on the critical path; nothing waits for it.**
2	
3	## Scope
4	- `holler profile rename OLD NEW`: one hub-side transaction that changes the name and slug and every member's `Pane.profile`, with one change-log entry (handler `profile/rename`, reserved by #637).
5	- `holler profile export NAME [--out FILE]` and `holler profile import FILE [--name NAME]`: a profile as a file (specs only; no generation, no log). Names and non-secret values only (I7): import refuses a file with an env value (`profile-secret-refused`) and an existing name (`profile-exists`). (amended 2026-10-08, features) `export` writes `command` and `check` as argv arrays, and `import` refuses a shell string with `command-not-argv`. Positions in the file go through the one `GridPos` parser (#637), so `r2c1`, `c1r2` and `2,1` (row,col) all load as row 2, col 1; export writes `rRcC`. (amended 2026-10-08, grid)
6	- All three take `--format=text|json`.
7	- A doctor finding `profile-drift` (a live pane differs from its spec), raised by reconcile through the comparison in `crates/holler-pane/src/profile_diff.rs` (#662).
~~~~

**R15: issue #648 body, lines 7-8** — REFRESH AFTER MERGE of #648 (deferred until after the cutover) - the roster columns `PANE`, `POS`, `PROFILE`, `HEALTH`, `SHOWN`, `DRIVEN`.

Command: `gh issue view 648 -R Performant-Labs/holler --json body -q .body | sed -n '7,8p'`

~~~~text
7	1. **New columns**, appended after the existing ones, in this order: `PANE` (the pane's name, `-` for an agent with no pane: the io body, the alpha and beta agents), `POS` (the pane's grid position), `PROFILE` (the pane's profile, `-` for none), `HEALTH` (the pane's harness health: `healthy`, `unhealthy`, `unknown`, `-` when no pane), `SHOWN` (the session the pane's TUI shows, `-` when unknown or no pane) and `DRIVEN` (the session the hub drives, `-` when no pane).
8	2. **`POS` is rowcol**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}` with `pos` equal to `r{row}c{col}`. A pane named `hj-c1r2` is a name, not a position: it sits at `r2c1` and is not renamed. The roster's help and legend say so.
~~~~

**R16: issue #654 body, lines 3-10** — The cutover procedure and its acceptance (issue #654, operator-run): the order of the steps, the overnight watch, the seven days.

Command: `gh issue view 654 -R Performant-Labs/holler --json body -q .body | sed -n '3,10p'`

~~~~text
3	## Scope
4	Staged and reviewed before anything changes: run `holler pane import --dry-run` on the live fleet and read every finding; fix drift with the old tools; import (a real import also creates the profile `fleet` from today's panes and their layout, #650 (amended 2026-10-08, review)); run `doctor` and the roster until SHOWN equals DRIVEN for every pane; cut one pane over to `launch`/`reset` and watch a day; then the rest. The old per-pane files are kept (read-only) for one release. Record each step in `docs/handoffs/<this issue>-cutover/decisions.md`.
5	
6	## Acceptance
7	- Seven days with no invisible session: every day, `doctor` reports no SHOWN/DRIVEN mismatch and no stray session older than an hour; the operator signs off.
8	
9	Depends on: #649, #650, #652, #655 (the Herdr server moved to the hub's machine).
10	
~~~~

**R17: issue #653 body, lines 3-4** — Issue #653 (the pfleet adoption) is tracked here and the work is in another repository, so its condition for removing the interim section has not happened.

Command: `gh issue view 653 -R Performant-Labs/holler --json body -q .body | sed -n '3,4p'`

~~~~text
3	## Scope
4	A pfleet epic (filed when #643, #644, #645, #646 have merged) that makes `pfleet o` reset, say, park, doctor and roster delegate to `holler pane …`, retires its own session and pane-map code (`o/panedirs.py`, the launcher's registration files), and updates the MO role file to use only the `holler pane` verbs. It must be coordinated with pfleet epic #266 (stories #271 reset, #272 doctor, #273 no-lost-input): build #271 and #272 to call `holler pane` rather than to grow their own session logic, or finish them first and port.
~~~~

**R18: issue #652 body, lines 32-38** — The operator decisions of 2026-10-11 as filed on the issue (decisions 1 to 7), which this brief restates in its own words and amends by decision (a) below.

Command: `gh issue view 652 -R Performant-Labs/holler --json body -q .body | sed -n '32,38p'`

~~~~text
32	1. **The guide documents `pane list` and `doctor` for SHOWN/DRIVEN and marks the roster extension "later".** Reason: #648 is deferred until after the cutover, and the guide must not describe a column that does not exist.
33	2. **`scripts/pane-restart` stays as the rollback until the 7-day sign-off.** Reason: it is the only restart path that does not depend on the new registry, and removing it before sign-off removes the way back.
34	3. **The grep tests derive their lists from the code's own tables**: the verbs from the clap tree (`Cli::command()`), the finding kinds from `FindingKind::ALL`, the error codes from `ALL_CODES`, the columns from `COLUMNS`. Reason: a hand-kept list in the test drifts from the code (the repo's single-source rule).
35	4. **The `profile apply` reference is the decided shape of the #664 brief** (above), with the O1 to O6 decisions. Reason: one contract, stated once.
36	5. **`orchestrating.md`'s interim section is rewritten, not deleted**, and names the script as a fallback until the sign-off. Reason: the section's own deletion condition (#653) is in another repository and has not happened.
37	6. **The cutover procedure keeps the overnight watch on the first pane.** Reason: a day-scale defect (idle drift, timeouts, restarts) shows only over time, and a staged cutover exists to find it on one pane.
38	7. **Deferred after the cutover:** #648, #696, #667, #666. Reason: none gates #654.
~~~~

**C1: command output** — The sentence of `scripts/pane-restart:3` appears nowhere else in the scripts or the CI files, so no test reads it.

Command: `git grep -n 'stop-gap' main -- scripts .github`

~~~~text
main:scripts/pane-restart:3:# A stop-gap (issue #746) until `holler pane relaunch --model` (#644) and the pfleet adoption (#653) land.
~~~~

**C2: command output** — No page links to the interim section's anchor (no output means no match).

Command: `git grep -n -i 'restarting-a-pane' main -- . ':!docs/handoffs'`

~~~~text
~~~~

**C3: command output** — The two new files do not exist on `main` (no output means neither exists).

Command: `git ls-tree -r main --name-only | grep -E '^docs/pane-control\.md$|docs_pane_control_test\.rs$'`

~~~~text
~~~~

**C4: command output** — The state of every story the brief names, at the time of the cut.

Command: `for n in 642 644 648 649 650 664 665 666 667 696 802; do gh issue view $n -R Performant-Labs/holler --json number,state,title -q '"#\(.number) \(.state) \(.title)"'; done`

~~~~text
#642 OPEN feat(adapter-opencode): the OpenCode adapter (serve, health, create and switch sessions, abort, attach a TUI, report the shown session)
#644 OPEN feat(cli): `holler pane launch` and `relaunch`, a transaction that creates the session of record
#648 OPEN feat(cli): the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ
#649 OPEN test(pane): hub wiring and the integration scenario with the real adapters against a scratch Herdr, tmux and OpenCode
#650 OPEN feat(pane): migration, import today's panes from sessions.toml, last-session, owned-sessions and pane.env (dry-run first)
#664 OPEN feat(cli): holler profile apply (plan as JSON, --dry-run, adopt not kill)
#665 OPEN feat(cli): holler profile rename, export and import; doctor code profile-drift [PROPOSED]
#666 OPEN ops: recreate the Herdr workspace in the new form from the profiles [operator-run]
#667 OPEN test(pane): profile apply on scratch Herdr, tmux and OpenCode (the real-adapter scenario for apply)
#696 OPEN refactor(pane): one bounded subprocess runner shared by the adapters and run_probe
#802 CLOSED refactor(pane): HarnessPort::serve takes a ServeSpec that carries the pane's OpenCode agent (step 1 of #642 part 3)
~~~~

