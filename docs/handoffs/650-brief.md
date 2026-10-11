# [feat(pane): migration, import today's panes from sessions.toml, last-session, owned-sessions and pane.env (dry-run first)](https://github.com/Performant-Labs/holler/issues/650)

Part of epic #633. This is the complete, final statement of the story (consolidated 2026-10-11 after the architecture review of the first run, the merge of #808 and the operator's decisions). Where an older comment or amendment disagrees, this text is the decision. The reviewers cannot see the repository: every fact the acceptance relies on is quoted in "Evidence (verbatim)" at the end (E1, E2, ...).

## Scope (final)

`holler pane import --from <state-root> --herdr-session NAME [--launcher FILE] [--orchestrator PANE]... [--dry-run] [--format=text|json]`

Read the current fleet's per-pane files and produce registry records, a profile `fleet`, and one report of every disagreement between those files and what Herdr, tmux and the harness show. The dry run writes nothing. A real run writes the records and the profile or writes nothing at all. (A failed write is undone; when an undo step itself fails, the undo stops at that step and the verb answers `profile-conflict` with the commands that reconcile, as `profile create --from-current` does: item 4 of "The real run's writes", acceptance 10, E5.) The verb adopts what is running: it never starts, stops, types into or moves anything, and it deletes nothing from the source files.

### The input (this story fixes these shapes; the repository holds no sample of the real fleet's files)

- **`<state-root>`** is the directory that holds one directory per pane: today `$HOME/.holler-oc/`, whose per-pane directory is `hj-<cCrR>` (E10: `state="$HOME/.holler-oc/$session"`). **The pane's name is the directory's name** (`hj-c1r2`), parsed with `PaneName::parse`. That name is the record's key (`Pane.name`, E2) and nothing more: it is neither the pane's identity in Herdr nor its position, which come from the label join (item 1 of "The join, per pane" below; ADR-0021 section 10, E9). A subdirectory that holds none of the four pane files below (for example `opencode-mo/`) is not a pane directory: it is an information finding `ignored-entry` and nothing else. A subdirectory that holds a pane file but whose name does not parse is a problem finding `source-unreadable`.
- **Per-pane files**, each under the pane's directory:
  - `pane.env`: lines of the form `[export ]NAME=value` (E10, the pattern `^(export[ \t]+)?OC_[A-Z_]+=`). The reader takes the **value** of exactly five names: `OC_MODEL` (`PROVIDER/MODEL_ID`, split at the first `/`, into `ModelSpec.provider` and `model_id`), `OC_EFFORT` (`ModelSpec.effort`), `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD` (unsigned integers, `ContextCeilings.soft` and `hard`), and `OC_DEFAULT_AGENT` (optional, `Pane.opencode_agent` through `AgentKey::parse`). **Every other variable is kept by NAME only** through `EnvVarName::parse`, in file order and de-duplicated; its value is never read into any value, log line or finding. A line that is blank or starts with `#` is skipped. `OC_MODEL`, `OC_EFFORT`, `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD` are required (E2: `ModelSpec` and `ContextCeilings` have no empty form). A `pane.env` whose lines parse but that lacks one of these four is `source-missing`, naming the file and the missing variable (a name from this list, never text read from the file), as a launcher entry without its port or cwd is (acceptance 7).
  - `last-session`: the whole content, trimmed, is one session id (E10). Absent or empty means "no session recorded".
  - `sessions.toml`: the **`holler-body` attach-config grammar** (confirmed in code, E11: `[[session]]` rows with `harness`, `mode = "attach"`, `endpoint`, `session_id`; the pane-restart script greps `session_id = "<id>"` in it, E10). It is read through `holler_body::config::load`, never through a new parser. Every row's `session_id` is a recorded session; a row whose `harness` is not `opencode` marks a pane that is not an OpenCode pane. Every row has a `harness`: `RawSession.harness` has no default (E34), so `load` refuses a row without one as `ConfigError::Toml`, which is `source-unreadable`. `load` refuses an `attach` row that has no `session_id` (or no `endpoint`), so a file holding one is unparsable: `source-unreadable` (E24, acceptance 7). `session_id` is optional only on a `spawn` row (E24); a row without one adds nothing to `S` (item 4 of the join).
  - `owned-sessions`: one session id per line; blank lines and lines starting with `#` are skipped. A file that exists but holds no session id (only blank and `#` lines) is present with `O` empty, not absent (only `last-session` reads empty as absent), so a present `L` is not in `O` and the result is `sources-disagree` (item 4 of the join: "`L` is in `O` when `owned-sessions` exists").
- **The launcher's table** is the file `--launcher FILE`, default `<state-root>/launcher.toml`. Shape (the story's contract; ADR-0021 section 13 names only what it maps to, E9): one table per pane, keyed by the pane's name, with `port` (integer 1 to 65535), `cwd` (an absolute path string) and `command` (an array of strings, an argv). The port is **never derived from the name** (the pane-restart script derives it as `47000 + col * 10 + row`; that is name-as-data and the import does not copy it). The real fleet's own table is read for its shape in the cutover (#654) dry run before anything is written; if its shape differs, the dry run reports `source-unreadable` and the fix is a one-file export, not a code change here.

### What the engine observes (reads only)

Only through `Ports` (E3, E12, E17, E23): `herdr.snapshot()` once, `herdr.version()` once, then per pane `host.ps(name)`, `harness.health(port)` and `harness.list_sessions(port)` (after `Ok(true)` to read the sessions; after `Ok(false)` only to tell a wedged server from a down one, its list never trusted, as reconcile does; `health` returns `Result<bool, PaneError>`, E23). The panes are observed **concurrently**, one scoped thread per pane as reconcile does (`reconcile.rs:7-8`, and its `observe_all`, `reconcile.rs:251-278`, E31), and the report lists them in name order, so a dry run over wedged servers costs about one bound, not N. Every method named here exists on the frozen ports as they are (E3, E23). **Nothing else is called**: not `ensure_pane`, `ensure_session`, `run`, `stop_owned`, `serve`, `create_session`, `attach_tui`, `select_session`, `abort`, `send_text`, `send_keys`, `close`, `shown_session`. A `herdr.snapshot()` or `herdr.version()` error ends the verb with that error (its class, its exit code), because nothing can be joined without them; `herdr-version-unsupported` is such an error (`PaneError::HerdrVersionUnsupported`, a refusal, exit 3, E32), never a finding of the report ("The finding vocabulary" below, acceptance 14).

### The join, per pane

1. **The Herdr pane is the snapshot entry whose `session` equals `--herdr-session` and whose `label` equals the pane's name** (the snapshot is filtered by `entry.session` first, as launch's occupancy check does, `tx_launch.rs:456`; a same-labelled pane in another Herdr session is not a candidate) (ADR-0021 section 10, E9, merged by #808, E1). The name's cell is never the join key and never chooses a position. Zero matches: problem `pane-unlabelled`. Two or more: problem `pane-label-ambiguous` (naming the pane ids). The record's `herdr` is that snapshot entry **with `label` set to `None`** (a label is an observation, never recorded state; launch does the same, E12).
2. **The position** is the entry's `grid`. Output prints rowcol (`r2c1`). If the directory name is `hj-` followed by text that `GridPos::parse` accepts (for example `c1r2`) and that cell differs from the observed position, the report carries the information finding `name-cell-differs` (naming both cells); a name that spells no cell produces no finding.
3. **The tmux session** exists when `host.ps(name)` answers; the error `pane-not-found` is the problem `no-tmux-session`; any other error is the warning `observe-failed`, named `host.ps` (reconcile's `<port>.<method>` form, E17).
4. **The session of record.** Let `L` be the content of `last-session`, `S` the set of `session_id`s in `sessions.toml`, `O` the lines of `owned-sessions`. The files **agree** when `L` is present and every other present file names it (`L` is in `S`, and `L` is in `O` when `owned-sessions` exists). Recency is never a tie-break. Results:
   - files name different sessions (`L` missing from a present `S` or `O`): problem `sources-disagree` (naming the files, not the ids' ownership), **no record for the pane**;
   - the files agree and a **healthy** server's `list_sessions` holds `L`: `session_of_record = L` (reconcile's trust rule, E17: only a healthy server's list counts);
   - the files agree but the healthy server's list does not hold `L`: the record is written **without** `session_of_record` and the warning `session-not-found`;
   - a `health` or `list_sessions` call that **returned an error** (`Err`): the record is written without `session_of_record`, `harness.health` is `unknown`, and the warning `observe-failed` names the failed call (the stable meaning of `observe-failed`: a port call returned an error, `findings.rs:46-50`);
   - the server **answers and is not healthy** (`health` is `Ok(false)`): classified exactly as reconcile does (`reconcile/observe.rs:167-181`, `observe_server`): one `list_sessions` call, whose list is never trusted; an `Err(PaneError::Timeout { .. })` is the warning `server-wedged` (`FindingKind::ServerWedged.code()`, message "the harness server on port N accepts a connection and does not answer"), anything else the warning `server-down` (`FindingKind::ServerDown.code()`, "does not pass its health check"). The record is written without `session_of_record` and with `harness.health` = `Unhealthy(<that code>)` (reconcile's `ObservedHealth::recorded`, `reconcile.rs:153-161`, copied as deliberate: `reconcile.rs` is frozen);
   - `last-session` absent or empty: the record is written without `session_of_record`, warning `no-session-recorded`.
   A session id read from a file is never copied into `last_observed` (I6).
5. **The role** is `orchestrator` for a pane named by `--orchestrator` and `agent` for every other pane. A `--orchestrator` name that matches no pane directory is a usage error (exit 2) before anything is read. The role is never derived from a name or a missing file. A pane named by `--orchestrator` still needs its own `pane.env` (the Pane record requires a model and ceilings and the import invents no defaults): the MO pane, whose config lives in `opencode-mo/` (E10), gets a small `pane.env` from the operator before the cutover, and the dry run says so (`source-missing`, naming the file).
6. **A pane that is not an OpenCode pane** (a `sessions.toml` row whose `harness` is not `opencode`) is the information-level skip `harness-not-opencode`: no record, never blocking (`HarnessKind` is closed to `opencode`, E2).
7. **A stored command** is an argv: `command` as an array becomes `Pane.command`; `command` as a string is the warning `command-not-argv` and the pane is recorded without a command.
8. **A pane that already has a record** (`pane_store.get(name)` answers a record) is `already-imported` (information) and is skipped before any other check.

### The record the import builds (once, in the plan; the real run writes exactly these)

`name`; `generation` 0; `herdr` (item 1); `host { name: tx_launch::HOST_NAME, tmux: name, cwd: launcher cwd, herdr_api_version: Some(herdr.version()) }`; `harness { kind: opencode, port: launcher port, pid: None, health: healthy only when this run's `health` answered healthy, otherwise unknown }`; `session_of_record` (item 4); `role` (item 5); `hold: none`; `last_observed { shown: None, driven: None, at: 0 }` (0 means never observed; the first doctor pass fills it; nothing in it is read from a file); `profile: fleet`; `model`; `opencode_agent`; `env` (names); `context`; `command`; `probe` empty (E2, E12). The profile is `profile_from_panes(fleet, &records)` (E4), so `holler profile show fleet` reports every pane `matches` by construction: `profile show` compares each spec with its member's record (a member is a record whose `profile` is `fleet`, `is_member`) through `diff_spec`, which compares the spec with `spec_from_pane(record)`, so a spec built from a record never differs from it (`diff_spec(&spec_from_pane(p), p)` is always empty, E25).

### The real run's writes (the contract of `profile create --from-current`, E5)

1. **Checks first**, before any write: nothing is blocked (below), and `fleet` does not exist unless every candidate pane is already imported.
2. **Write the profile first** through `crate::profile::create::insert_profile` (the one place the create race becomes `profile-exists`, E5), with the actor constant `holler pane import`.
3. **Then each record** with `profile: fleet`, through `cas_put(record, 0)` (E22), in name order (ascending `PaneName`, whose derived `Ord` is the order of the name's string, E30).
4. **A failure records nothing** (I3). The undo runs newest first: delete the records this run created (plus the one whose write answered `timeout`, if it landed), then delete the profile at its stored generation, stopping at the first failed step and answering `profile-conflict` with the commands that reconcile (never `holler pane close`, which would act on the live fleet). This undo is deliberate and separate from `create`'s (E5: create detaches existing records; the import deletes the records it created).
   The import's undo is **new code in this story** (in `crates/holler-cli/src/pane/import.rs` or one of its submodules), not a call to `create`'s `undo`, which is private to `create.rs` and detaches (E5). E5 is cited only for what the two share: the profile written first through `insert_profile`, then the members; an undo that runs newest first, stops at its first failed step and deletes the profile last at the generation it was stored at; and `profile-conflict` when an undo step fails. The one step that differs is the one operator decision 5 states: a record is **deleted**, not detached, since every record this run writes is new (`cas_put(record, 0)`; a pane that already has a record is `already-imported` and skipped, item 8 of the join). Each step uses calls the ports already have: the read `PaneStore::get` and the compare-and-swap deletes `PaneStore::delete` and `ProfileStore::delete` (E22). When a record's `cas_put` fails, the undo calls, in this order: (a) for the pane whose write failed (a `timeout` may have landed), `pane_store.get(name)`; when it answers a record whose `profile` is `fleet` (`is_member`, the check E5's undo makes, E25), that write landed, and `pane_store.delete(name, <the generation get answered>)` removes it; (b) for each record this run stored, newest first, `pane_store.delete(name, <the generation its cas_put returned>)`; (c) `profile_store.delete(fleet, <the generation insert_profile returned>, <the actor of item 2>)`.

### Exit codes and what blocks

- **Levels.** Each finding has a level: `problem` (the pane gets no record), `warning` (the record is written without the thing the warning names), `skipped` (`harness-not-opencode`) and `info` (`ignored-entry`, `already-imported`, `name-cell-differs`). **Only `problem` blocks.** Problems: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`.
- **Dry run:** exit 0 with the whole report, whatever the findings. A `usage` or an adapter error keeps its own class and code.
- **Real run:** if any pane has a problem, nothing is written and the verb answers the refusal **`import-blocked`** (exit 3; a constant in `crates/holler-pane/src/import.rs` through `RefusalCode::from_static`, E16), with the message `import blocked: N pane(s) have problems; run with --dry-run for the report` (one line, nothing on stdout in text mode; the failure envelope in JSON). If `fleet` already exists and at least one candidate pane is not yet imported, the verb answers `profile-exists` (exit 3). If every candidate pane is `already-imported` (or skipped) there is nothing to write: exit 0, the report lists `already-imported`, no profile is created, no store write is made. With no candidate pane at all: exit 0, nothing written.
- **Missing `<state-root>`** (not a directory): the `usage` error, exit 2, one line.

### The output

`--format=text|json` is the global flag (`VerbCtx.format`), one `output::emit` call (E14). The verb flattens no `ProfileOpt` (`import` is in neither `--profile` class, E9); the existing test that `holler pane import --profile demo` is a usage error stays true. The JSON `data` (documented in `PaneImport`'s doc, versioned with the envelope: adding a key later is not breaking) is:
`{"dry_run": bool, "state_root": string, "recorded": integer, "profile": Profile or null, "panes": [{"name", "action": "record"|"already-imported"|"skipped"|"blocked", "role": "agent"|"orchestrator"|null, "pos": GridPos or null, "session_of_record": string or null}], "findings": [{"pane": name or null, "level", "kind", "message"}]}`. Positions go through `GridPos` serde (row first): a pane's `pos` is the object `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order, not a string (ADR-0021 section 10, E9; E30); the text form prints `r2c1`. `recorded` is the number of records written (0 in a dry run). In a dry run `data.profile` is the `Profile` that `profile_from_panes(fleet, &records)` builds (E4, E27), the value a real run hands to `insert_profile`; its `generation`, `created` and `updated` are 0 because the store stamps them (E4). The finding key is `kind`, the name doctor's report already uses for the same field (`findings.rs:214`), because renaming it after merge would be a breaking change. The text form prints one line per pane (`NAME  ACTION  ROLE  POS  SESSION`, `-` for absent), then each finding as `  <level> <code>: <message>` (`<code>` is the finding's `kind`, the same string its JSON `kind` holds), then (dry run) the profile as `profile show` prints a spec (each spec field through `SpecField::ALL` and `FieldValue`, E26: `spec PANE`, then one `  FIELD: VALUE` line per field). The import reuses `profile show`'s own spec text: `push_spec` and `text` in `show.rs` widen from private to `pub(crate)` (visibility only, as `insert_profile`, `detach`, `single_quoted`, `pane_list` and `count` already are) and the import calls them; it copies nothing (E26). Finding messages carry untrusted text (paths, ids, adapter messages) only through `findings::quoted` / `findings::embedded` (E6), with one exception that is a rule: **a `source-unreadable` message never carries text read from a file or from a parser's error**, because `toml` 0.8.23 / `toml_edit` 0.22.27 error text quotes the offending source line verbatim (`toml_edit` `src/error.rs:101-110`) and `holler_body::config`'s `ConfigError::Toml` keeps only that text (`config.rs:273`); the line can be an `env` table of values or a key of the real launcher table (ADR-0021 :298-300, :513). It names the file and a position only: for the launcher table, the line and column from `toml::de::Error::span()` and no message; for `sessions.toml`, `ConfigError::Invalid`'s row and field, and for `ConfigError::Toml` only "not valid TOML"; for `pane.env`, the line number alone (a line that does not parse has no safe variable to name). For the launcher table, `holler-cli` parses the file itself with `toml::from_str`, which answers a `toml::de::Error` (E33), and keeps that error value, never its `to_string()` or `message()`: those are the parser's error text the rule above keeps out of the message. `span()` is the byte range of the error in the text parsed, or `None` (E33), so the reader counts the line and column of the span's start in that text, and with no span the message names the file alone. For `sessions.toml`, `ConfigError::Invalid`'s `row` is the 0-based index of the `[[session]]` table, not a line number, and its `field` is a fixed key name (`&'static str`); the message never carries the variant's `name`, which is the row's own value, or its `reason`, which can quote a value of the file (`parse_mode`'s `unknown mode {other:?}`, E34).

### The finding vocabulary

A new single-source enum `ImportFinding` in `crates/holler-pane/src/import/findings.rs`, copying `FindingKind`'s pattern (E6, E29): `code()` is the one source of each code, `Serialize` is built from it, there is an `ALL` list, and a code-table test (unique, kebab-case, `is_valid_code`) lives in `crates/holler-pane/tests/import_test.rs`. A condition that reconcile already names keeps reconcile's spelling through `FindingKind::ObserveFailed.code()` and `FindingKind::HerdrVersionUnsupported.code()`; `command-not-argv` is the closed `PaneError` code (`PaneError::CommandNotArgv`, E28). Of reconcile's codes, the import's **report** carries `observe-failed` (items 3 and 4 of the join), `server-wedged` and `server-down` (`FindingKind::ServerWedged.code()` and `FindingKind::ServerDown.code()`, item 4). **`herdr-version-unsupported` is never a finding of the import**, in a dry run or a real run: it is the error `herdr.version()` answers (`PaneError::HerdrVersionUnsupported`, a closed code of class refusal, exit 3, E32), and a `herdr.version()` error ends the verb with that error ("What the engine observes"; an adapter error keeps its own class and code, "Exit codes and what blocks"). The verb answers that error in place of a report and writes nothing, so no `ImportFinding` variant, no `data.findings` entry and no text finding line carries it, and ADR-0021 section 9's import row lists it among the verb's error codes (acceptance 12; acceptance 14 pins the behaviour). Its spelling is still reconcile's: the error's code is the same string as `FindingKind::HerdrVersionUnsupported.code()` (E29, E32). Doctor reports it as a finding because a reconcile pass goes on without the version (`reconcile.rs:300-305`, E32); the import does not go on. The new codes are exactly: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`, `session-not-found`, `no-session-recorded`, `harness-not-opencode`, `name-cell-differs`, `already-imported`, `ignored-entry`. `findings.rs` and `reconcile.rs` are #647's files and are not edited.

## Operator decisions (2026-10-11)

1. **Join key: the Herdr label.** The name's cell and the foreground process are forbidden as keys. (#808, merged; ADR-0021 section 10.)
2. **Role: a repeatable `--orchestrator PANE` flag**; every other pane is an agent; the dry run prints each pane's role; never from a name. (Reason: the role gates the orchestrator protections, so it must be stated, not guessed.)
3. **A label that matches no pane or two panes is a finding and nothing is recorded for that pane** (fail closed). (Reason: guessing which pane is which re-creates the incident.)
4. **Files that disagree about the session produce a finding and no record for that pane**; the real run is then refused until the operator fixes the drift with the old tools, as the cutover runbook says. This narrows ADR-0021 section 13 step 2's wording (a disagreement used to import the pane without a session of record); section 13 is edited to say so. Files that **agree** while the server does not hold the session still import the pane without a session of record (warning). (Reason: a record that is wrong in a way the operator has not looked at is worse than no record.)
5. **Exit codes:** dry run 0 with findings; a real run is all-or-nothing and exits 3 (`import-blocked`) when any pane has a problem, following `profile create --from-current` (checks first, the profile through `insert_profile`, then the members, an undo that deletes what the run created).
6. **`pane.env`: names only, never values**, except the five named variables whose values become record fields. (Reason: I7, no secret in any record, log or finding.)
7. **A pane that does not run OpenCode** is a skipped finding and no record, and does not block. (Reason: `HarnessKind` is closed.)
8. **Re-running after an import is a no-op that exits 0** (`already-imported`); it creates no profile and writes nothing. (Reason: safe to repeat during the cutover.)
9. **`--herdr-session` is required**, as on `launch`. (Reason: the same flag, the same meaning.)
10. **`sessions.toml` is read through `holler_body::config`**, which is the attach-config grammar (E11); no second parser.
11. **The record never holds a label** and `last_observed` is empty (`at: 0`): observations come from the first doctor pass.

## Acceptance (each is a test; fakes from the test kit only; fixtures use made-up values, no real host, account, path, key or session id)

Tests go in `crates/holler-cli/tests/pane_verbs/import.rs` (replacing the stub's test) with submodules under `crates/holler-cli/tests/pane_verbs/import/` (a rig that builds a `<state-root>` in a temporary directory and a live world on the fakes, reusing doctor's world-building and adding an unrecorded seed to `doctor/rig.rs`, E19), and in `crates/holler-pane/tests/import_test.rs` (the code table and pure engine cases). The fakes' `faults().calls()` pin "reads only" and "writes nothing"; `check_envelope` (E21) pins the envelope.

1. **Agreement.** Three panes whose files agree and whose fake servers hold the session import with `session_of_record` set, `herdr` equal to the fake snapshot entry with `label` `None`, roles from the flags, and `model`, `effort`, `context`, `env` names and `opencode_agent` from `pane.env`. The pane `hj-c1r2` placed by the fake Herdr at row 2, column 1 (label `hj-c1r2`) imports with `GridPos { row: 2, col: 1 }` and prints `r2c1`.
2. **Dry run writes nothing.** The fake pane store's and profile store's call logs hold none of `cas_put`, `delete`, `rename` (reads allowed); every file under `<state-root>` has the same bytes after; JSON passes `check_envelope`; `data.profile` is the profile it would create and `data.recorded` is 0; the adapters' call logs hold only the allowed reads.
3. **Real run.** The profile `fleet` exists with one spec per imported pane (env NAMES only), each pane's `profile` is `fleet`, `holler profile show fleet` reports every row `matches`, a secret-looking value placed in `pane.env` under a non-listed variable appears in no record, no output and no finding, and no source file changed.
4. **Join failures.** No labelled pane: `pane-unlabelled`; two panes with the label: `pane-label-ambiguous`; each leaves no record, the dry run exits 0 listing it, and the real run writes nothing and exits 3 with the pinned `import-blocked` message.
5. **Disagreement.** `last-session` names A and `sessions.toml` names B: `sources-disagree`, no record, real run refused. A stale file naming A while the fake server lists only B: no record anywhere holds A (I6). An `owned-sessions` that holds only blank and `#` lines while `last-session` names A: `sources-disagree`, no record.
6. **Agreement but the server does not hold the session**: the record is written without `session_of_record` and the finding is `session-not-found`. A `health` or `list_sessions` call that returns an error: `observe-failed` (naming the call), `harness.health` `unknown`. A server whose `health` answers `Ok(false)`: `server-down`, or `server-wedged` when `list_sessions` times out; the record is written without `session_of_record` and `harness.health` is `Unhealthy("server-down")` or `Unhealthy("server-wedged")`; `observe-failed` is not used for it.
7. **Hand-edited and missing files.** An unparsable `sessions.toml`, a `pane.env` line that is not `[export ]NAME=value`, and a non-numeric ceiling each give `source-unreadable` naming the file and a position only (see The output): a secret placed on the offending line of `sessions.toml`, of the launcher table and of `pane.env` appears in no finding, no output and no log line (assert its absence in all three); a missing `pane.env`, a `pane.env` without one of its four required variables (the finding names the variable), a missing launcher entry (port or cwd) give `source-missing`; none has a record. A missing `<state-root>` gives exit 2.
8. **Not OpenCode and command shapes.** A `harness = "claude"` row: `harness-not-opencode`, no record, the other panes still import. `command` as an array imports as `Pane.command`; as a string it gives `command-not-argv` and the pane imports without a command.
9. **Idempotence and `fleet`.** A second real run exits 0, reports `already-imported`, makes no store write. With `fleet` present and a new pane to record: `profile-exists`, exit 3, nothing written.
10. **Failure records nothing.** A fault injected on the second `cas_put` leaves the pane store empty and the profile deleted; a fault that also fails an undo step answers `profile-conflict` with the reconcile commands and never names `pane close`. The message names the remedy steps: after a failure at the profile delete, `holler profile delete fleet` and run the import again; with records kept, `holler profile delete fleet --keep-panes`, run the import again (the kept records are `already-imported` and are skipped), then `holler profile delete fleet --keep-panes` and `holler profile create fleet --from-current` (a pane's record must join `fleet` for the profile to describe it). A follow-up issue is filed for a verb that removes only a record (no verb does today).
11. **Roles and flags.** An unknown `--orchestrator` name: exit 2. `--herdr-session` is required. `holler pane import --profile demo` is still a usage error. The name-cell information: `hj-c1r2` observed at row 1, column 1 gives `name-cell-differs` and the record uses row 1, column 1.
12. **Vocabulary and docs.** The code-table test passes; `cli_surface_test` and `docs_cli_test` pass with the new `cli-surface.txt` lines and the ADR 0003 row; ADR-0021 section 9's import row lists `profile-exists`, `profile-conflict`, `generation-conflict`, `herdr-version-unsupported`, `import-blocked` and `usage`, the open-codes row (`:584`) lists `import-blocked`, section 13 gives the shape of the launcher table (a new on-disk format this story defines), and section 10 says what this story decided; `CHANGELOG.md` has an `## [Unreleased]` entry linking this issue.
13. **Herdr session.** A fake Herdr with the label `hj-c1r2` in two sessions: the import joins only the pane in `--herdr-session`; with the label only in another session the pane is `pane-unlabelled`.
14. **Herdr's version.** A fake Herdr that reports an unsupported build (`FakeHerdr::set_version(HerdrVersion::Unsupported)`, E32): the dry run and the real run each end with the error `herdr-version-unsupported`, exit 3, with no report (in JSON the failure envelope, which passes `check_envelope`), no finding that carries the code, and no store write (the fake pane store's and profile store's call logs hold none of `cas_put`, `delete`, `rename`).

## Blast radius

Edit: `crates/holler-cli/src/pane/import.rs` (args, run, render, the real run's writes) and its submodules `crates/holler-cli/src/pane/import/*.rs` (for example `sources.rs` for reading and parsing; the CLI side passes `scripts/lint.sh`'s 600-line warning, and the lint fails at 900, so the split is named now); `crates/holler-pane/src/import.rs` and `crates/holler-pane/src/import/*.rs` (the pure engine, the plan, the finding enum, `IMPORT_BLOCKED`); `crates/holler-cli/tests/pane_verbs/import.rs` and `import/**`; `crates/holler-cli/tests/pane_verbs/doctor/rig.rs` (an unrecorded seed only); `crates/holler-cli/tests/pane_verbs/process/stub.rs` (delete its `STUBS` entry and the `// #650` line); `crates/holler-cli/tests/fixtures/cli-surface.txt` (its two lines); `crates/holler-cli/src/profile/show.rs` (visibility of `push_spec` and `text` only); `docs/adr/ADR-0003.md` (its row); `docs/adr/ADR-0021.md` (sections 9's import row, 10 and 13 text); `crates/holler-pane/tests/import_test.rs`; `CHANGELOG.md`. `holler-pane` takes no `toml` and no `holler-body` dependency (ADR-0021 section 5): all file-format knowledge stays in `holler-cli`.

**New code, not yet in the repository** (so no excerpt can show it; the acceptance tests named pin it), among it: the `PaneImport` arguments with the repeatable `--orchestrator` and its usage check (acceptance 11); the plan and its name order, the real run's writes and the import's undo (acceptance 3, 9 and 10); and `ImportFinding` with its code-table test, which copy `FindingKind`'s pattern (E29; acceptance 12); and the readers of `pane.env`, `last-session`, `owned-sessions` and the launcher table, with the positions their `source-unreadable` findings name (the `pane.env` line number, the launcher table's line and column from `span()`), in `crates/holler-cli/src/pane/import/` (for example `sources.rs`, as the Blast radius names it; acceptance 7). What they use already exists and is quoted: the ports (E3, E22, E23), `insert_profile` (E5), `profile_from_panes` (E4), `profile show`'s comparison and spec fields (E25, E26) and `holler_body::config::load` (E11, E24).

**Must not edit** (frozen or other stories'): `crates/holler-cli/src/pane/mod.rs`, `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/pane.rs`, `crates/holler-pane/src/profile_snapshot.rs`, `crates/holler-pane/src/findings.rs`, `crates/holler-pane/src/reconcile.rs` and `reconcile/**`, `crates/holler-pane/src/tx_launch.rs`, the Herdr adapter, the test kit's `src/`, and any hub or protocol file. No protocol row, golden file or hub state file changes.

## Depends on

#808 (merged, `263a2d7`: the label), #637, #639, #662 (merged). Does not depend on #802, #642 or #644.

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-650-implementation`.

## Rules for this story

Issue first, tests first, Holler's pipeline and hooks apply (fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`, every `#[allow]` with a `// #NNN` link). Edit only the files in the Blast radius. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes and temporary directories. Never print or commit a credential. This is a public repository: no real hostnames, accounts, paths, key files or session ids in code, tests or docs.

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.

## Evidence (verbatim)

Each excerpt is cut by command from `origin/main` at commit `263a2d7` (never retyped): path and line range are in its heading.

### E1: HerdrPane with its label (merged by #808)
`crates/holler-pane/src/pane.rs:88-110`
```
    }
}

/// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists, and
/// what `HerdrPort::snapshot` lists; a pane Herdr reports also carries its label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
    /// The pane's label as Herdr reports it, read by the Herdr adapter through its
    /// `label_of`: an observation (I6), and the join key from a pane name to a live Herdr
    /// pane (#808). `launch` and `relaunch` never write one, so a record's `herdr.label`
    /// is `None` and its JSON has no `label` key. `None` stands both for a pane Herdr
    /// sent no label for and for one it sent an empty label for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The machine side of a pane.
```

### E2: the Pane record and its parts (HostInfo, HarnessKind, Health, HarnessInfo, PaneRole, Hold, LastObserved, ModelSpec, ContextCeilings, PaneProbe, Pane)
`crates/holler-pane/src/pane.rs:109-269`
```

/// The machine side of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInfo {
    /// The host's name.
    pub name: String,
    /// The tmux session name (equal to the pane's name).
    pub tmux: String,
    /// The project directory or worktree the pane works in.
    pub cwd: String,
    /// The Herdr API version, as `HerdrPort::version()` reports it, recorded by a verb
    /// (the Herdr adapter writes no record; ADR-0021 section 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub herdr_api_version: Option<String>,
}

/// The harness a pane runs. Only OpenCode exists; the `HarnessPort` is the seam for
/// another.
///
/// This is a closed enum on purpose (epic #633: no new harness; a stored record with
/// an unknown harness fails to load rather than being driven blindly). It is a second
/// list beside `holler_proto::vocab::HARNESS_IDS`; every serde name of this enum must
/// be in that list, which a test pins. Adding a harness to panes adds a variant here
/// and goes through the epic's amend-first rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessKind {
    Opencode,
}

/// What the harness server last reported about its health.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    /// Unhealthy, with the reason.
    Unhealthy(String),
    Unknown,
}

/// The harness server of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessInfo {
    pub kind: HarnessKind,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub health: Health,
}

/// The role of a pane (and of a profile spec): an agent or the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneRole {
    Agent,
    Orchestrator,
}

/// A pane's hold state **as a field of the pane record**: whether `park`/`unpark`
/// has parked it, or it is drained. It is **not** the prompt hold of `holler hold`
/// (`holler_proto::SessionHold`). Any refusal of a prompt that is derived from pane
/// state belongs at `send_prompt`, the one choke point every prompt passes through,
/// not in a verb (#646's brief states this).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Hold {
    None,
    Parked {
        reason: String,
        /// When the park ends, in the verb's own words (a time or a condition).
        release_when: String,
        /// When the pane was parked (milliseconds since the Unix epoch).
        since: i64,
    },
    Drained,
}

/// What reconcile last observed about which session the pane shows and which the hub
/// drives. Written by reconcile, never inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastObserved {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driven: Option<String>,
    /// When it was observed (milliseconds since the Unix epoch).
    pub at: i64,
}

/// The model a pane runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    pub provider: String,
    pub model_id: String,
    pub effort: String,
}

/// The context ceilings a watchdog reads (a local model's window is smaller than a
/// hosted one's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCeilings {
    pub soft: u32,
    pub hard: u32,
}

/// The health probe of a pane (B1): the stored `check` argv, the strings its output
/// must contain, and the last result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneProbe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Argv>,
    #[serde(default)]
    pub expect: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<ProbeResult>,
}

/// One record per pane, owned by the hub. The key is [`Pane::name`]. Every write is
/// a compare-and-swap on [`Pane::generation`] (see [`crate::generation`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub name: PaneName,
    /// Bumped on every change.
    pub generation: u64,
    pub herdr: HerdrPane,
    pub host: HostInfo,
    pub harness: HarnessInfo,
    /// THE session: the TUI is attached to it and the hub drives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_of_record: Option<String>,
    pub role: PaneRole,
    pub hold: Hold,
    pub last_observed: LastObserved,
    /// The profile the pane belongs to; a pane belongs to at most one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Recorded by launch/relaunch (#644) and import (#650), so
    /// `profile create --from-current` copies them from the store.
    pub model: ModelSpec,
    /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
    /// server's default agent (#700). A name, never a secret (I7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_agent: Option<AgentKey>,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
    pub env: Vec<EnvVarName>,
    pub context: ContextCeilings,
    /// The launch command: an argv array, never a shell string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Argv>,
    pub probe: PaneProbe,
}

/// One change to the pane store, as `PaneStore::watch` yields it.
```

### E3: HerdrSpec, HerdrSnapshot and the HerdrPort methods the import reads
`crates/holler-pane/src/ports.rs:87-102`
```
/// Where `HerdrPort::ensure_pane` should put a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSpec {
    pub session: String,
    pub workspace: String,
    pub grid: GridPos,
}

/// What Herdr reports about every pane it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSnapshot {
    pub panes: Vec<HerdrPane>,
}

```
`crates/holler-pane/src/ports.rs:121-155`
```
/// Herdr, reached over its local socket (the adapter is `holler-adapter-herdr`,
/// #640). **Provisional** until spike #636 reports.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// Only the adapter converts a [`GridPos`] to Herdr's own order and base.
pub trait HerdrPort: Send + Sync {
    /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail
    /// loudly; never relocates a healthy pane. An occupied cell answers its pane and
    /// makes none. A cell outside the workspace's rows and columns (Herdr has none, so
    /// they are the implementation's configuration) is `grid-out-of-range`; a cell that
    /// no single right or down split reaches is the open code `grid-unreachable`. The
    /// answer carries the pane's label as `snapshot` reports it (#808).
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;

    /// Type `text` into the pane.
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;

    /// Press `keys` in the pane.
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError>;

    /// The last `max_lines` lines of the pane's screen.
    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError>;

    /// Close the pane.
    fn close(&self, pane: &PaneId) -> Result<(), PaneError>;

    /// Every pane Herdr has, with its position and its label (`HerdrPane.label`, #808).
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError>;

    /// Herdr's API version. An unknown version is `herdr-version-unsupported`.
    fn version(&self) -> Result<String, PaneError>;
}
```

### E4: spec_from_pane and profile_from_panes
`crates/holler-pane/src/profile_snapshot.rs:41-84`
```
/// `herdr.pane_id`, `host.name`, `host.tmux`, `host.herdr_api_version`, `harness.pid`,
/// `harness.health`, `session_of_record`, `hold`, `last_observed`, `profile` and
/// `probe.last`.
///
/// The port is copied as recorded: a record at port 0 gives `fixed:0`, which #644's
/// `port_of_policy` refuses (`usage`).
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec {
    ProfileSpec {
        pane: pane.name.as_str().to_owned(),
        herdr: SpecHerdr {
            workspace: pane.herdr.workspace.clone(),
            grid: pane.herdr.grid,
        },
        host: SpecHost {
            cwd: pane.host.cwd.clone(),
        },
        harness: SpecHarness {
            kind: pane.harness.kind,
            port_policy: fixed_port_policy(pane.harness.port),
        },
        model: pane.model.clone(),
        opencode_agent: pane.opencode_agent.clone(),
        role: pane.role,
        env: pane.env.clone(),
        context: pane.context,
        command: pane.command.clone(),
        check: pane.probe.check.clone(),
        expect: pane.probe.expect.clone(),
    }
}

/// A new profile named `name` (slug `name.slug()`, generation 0, `created` and `updated` 0: the store
/// stamps them) with one `spec_from_pane` per entry of `panes`, in the order given.
pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile {
    Profile {
        name: name.clone(),
        slug: name.slug(),
        generation: 0,
        panes: panes.iter().map(spec_from_pane).collect(),
        created: 0,
        updated: 0,
    }
}

```

### E5: profile create --from-current: the contract, insert_profile and the undo
`crates/holler-cli/src/profile/create.rs:1-24`
```
//! `holler profile create NAME [--from-current | --from PROFILE]` (story #662, epic #633): make a
//! profile that is empty, a detached copy of another profile's specs (`--from`), or a snapshot
//! of every pane in the pane registry that makes each pane a member (`--from-current`).
//!
//! - **Checks first.** Every check runs before the first write (#662 B3): the names parse
//!   (`usage`), NAME is not taken by name or slug (`profile-exists`), PROFILE exists
//!   (`profile-not-found`), and with `--from-current` no pane belongs to another profile
//!   (`pane-in-other-profile`, every such pane at once).
//! - **One create write.** Every form stores its record through `insert_profile`, the one
//!   place that reports the create race as `profile-exists` (B7). Every new record is built by
//!   `holler_pane::profile_snapshot::profile_from_panes`, so no `Profile` is built by hand, and
//!   `--from-current`'s specs come from each pane's record (its position from `herdr.grid`,
//!   never from its name), with no Herdr call.
//! - **Write order.** No transaction spans the two registries (ADR-0021 section 8), so
//!   `--from-current` writes the profile first (a pane can name only a profile that exists),
//!   then each pane joins in `list()` order, its `Pane.profile` set by compare-and-swap. A
//!   failed join undoes everything, since a failure records nothing (I3, #662 B2): the pane
//!   whose join failed leaves if its join landed after all (a `timeout` may have), then each
//!   joined pane leaves, newest first, then the profile is deleted. The undo stops at its first
//!   failed step and reports `profile-conflict` with the commands that reconcile, so the
//!   profile still exists whenever a member may remain.
//!
//! The verb reads and writes the two registries and nothing else: no adapter, no probe, and
//! nothing from the process environment.
```
`crates/holler-cli/src/profile/create.rs:202-220`
```
///
/// A `generation-conflict` means another writer created the name between the caller's read and
/// this write (the hub's store answers `next_generation(1, 0)`): to the operator that is a
/// taken name, `profile-exists`. Every other error passes through, the store's own
/// `profile-exists` (the slug is filed under another spelling) included. It takes the port and
/// answers a `PaneError`, with no CLI type, so a caller outside this verb can use it.
pub(crate) fn insert_profile(
    store: &dyn ProfileStore,
    profile: &Profile,
    actor: &Actor,
) -> Result<Profile, PaneError> {
    store
        .cas_put(profile, 0, actor)
        .map_err(|error| match error {
            PaneError::Conflict => taken(&profile.name),
            other => other,
        })
}

```
`crates/holler-cli/src/profile/create.rs:303-331`
```

/// Undo a failed join of `failed` (#662 B2), stopping at the first step that fails: `failed`
/// leaves if its record shows it joined after all (a write that answered `timeout` or
/// `unavailable` may have landed); each pane in `joined` leaves, newest first; then the
/// profile is deleted at the generation it was stored at. Stopping at the first failure keeps
/// the profile whenever a member may remain, so the remedy the error names can always run.
fn undo(
    pane_store: &dyn PaneStore,
    profile_store: &dyn ProfileStore,
    profile: &Profile,
    failed: &PaneName,
    joined: &[Pane],
    actor: &Actor,
) -> Result<(), PaneError> {
    if let Some(record) = pane_store
        .get(failed)?
        .filter(|record| is_member(record, &profile.name))
    {
        detach(pane_store, &record)?;
    }
    for pane in joined.iter().rev() {
        detach(pane_store, pane)?;
    }
    profile_store.delete(&profile.name, profile.generation, actor)
}

/// The text form: `created profile NAME (SLUG): N specs, generation G`, then the form's own
/// line. A profile name holds no control character (`ProfileName` refuses one) and a slug is
/// ASCII letters, digits and `-`, so both print as they are.
```

### E6: the FindingKind pattern and the sanitizers
`crates/holler-pane/src/findings.rs:36-87`
```
const DOCTOR: &str = "holler pane doctor";
const RELAUNCH: &str = "holler pane relaunch";
const RESET: &str = "holler pane reset";

/// What a reconcile pass found about a pane, a harness server, a session or Herdr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingKind {
    /// Herdr reports an API version the adapter does not know; the message names the
    /// supported ones. No Holler verb upgrades Herdr.
    HerdrVersionUnsupported,
    /// A port call of the pass failed, so nothing was concluded from what it would have
    /// observed. The message names the call as `<port>.<method>` and the error's code. It is
    /// a read (`herdr.snapshot`, `host.ps`, `harness.health`, ...) or the record write
    /// (`pane_store.cas_put`), a write that lost a race with another writer included.
    ObserveFailed,
    /// A Herdr pane that no record names: a process with no record. Whole-fleet runs only.
    UnregisteredHerdrPane,
    /// Herdr has no pane where the record says the pane is.
    HerdrPaneMissing,
    /// The pane's tmux session does not exist.
    TmuxSessionMissing,
    /// The pane's harness server accepts a connection and never answers.
    ServerWedged,
    /// The pane's harness server fails its health check, and is not wedged.
    ServerDown,
    /// The record names no session of record. Doctor never picks one.
    NoSessionOfRecord,
    /// The pane's server answered, and it does not have the session of record.
    SessionOfRecordMissing,
    /// The TUI shows a session its pane's own server does not have: a harness the record
    /// does not know is in the pane.
    TuiForeignSession,
    /// The TUI shows another session than the session of record, or its home screen. The
    /// one kind `--fix` repairs: it selects the session of record.
    ShownDrivenMismatch,
    /// A session on a pane's server that is the session of record of no pane.
    StraySession,
}

impl FindingKind {
    /// Every kind, in declaration order.
    pub const ALL: &'static [FindingKind] = &[
        FindingKind::HerdrVersionUnsupported,
        FindingKind::ObserveFailed,
        FindingKind::UnregisteredHerdrPane,
        FindingKind::HerdrPaneMissing,
        FindingKind::TmuxSessionMissing,
        FindingKind::ServerWedged,
        FindingKind::ServerDown,
        FindingKind::NoSessionOfRecord,
        FindingKind::SessionOfRecordMissing,
        FindingKind::TuiForeignSession,
```
`crates/holler-pane/src/findings.rs:324-352`
```
fn reset_command(pane: &PaneName) -> String {
    format!("{RESET} {pane}")
}

/// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
/// control character and quote in it is escaped, and cut to 64 characters
/// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
/// included.
pub fn quoted(text: &str) -> String {
    excerpt(text)
}

/// An adapter's `message`, to embed in a finding's message: one line, with every control
/// character escaped (`\n`, `\u{1b}`), and cut to 200 characters.
pub(crate) fn embedded(message: &str) -> String {
    let mut out = String::with_capacity(message.len().min(EMBED_LIMIT));
    for c in message.chars().take(EMBED_LIMIT) {
        if c.is_control() {
            out.extend(c.escape_debug());
        } else {
            out.push(c);
        }
    }
    if message.chars().nth(EMBED_LIMIT).is_some() {
        out.push_str("...");
    }
    out
}

```

### E7: the two stub files this story replaces
`crates/holler-cli/src/pane/import.rs:1-20`
```
//! `holler pane import`: a stub (story #670). Story #650 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 650;

/// Import the existing fleet into the pane registry.
#[derive(Args, Debug)]
pub struct PaneImport {}

/// Run `holler pane import`: refuse, naming the story that owns it.
pub fn run(_args: &PaneImport, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}

```
`crates/holler-pane/src/import.rs:1-4`
```
//! The migration import: read the existing registrations and create the `fleet`
//! profile and its panes. Empty stub declared by #637 so that no two stories edit
//! `lib.rs`; story #650 fills it.

```

### E8: the import lines of ADR 0003 and of the CLI surface fixture
`docs/adr/ADR-0003.md:61-65`
```
holler pane doctor [PANE] [--fix] [--profile NAME]                #647

holler pane import                                                #650

holler profile create NAME [--from-current | --from PROFILE]      #662
```
`crates/holler-cli/tests/fixtures/cli-surface.txt:165-168`
```
# #650
pane import |
pane import | --format=json

```

### E9: ADR-0021: the --profile classes, the pane import row of section 9, section 10 (the label) and section 13 (the migration)
`docs/adr/ADR-0021.md:161-165`
```
  and `answer`, and the roster): act on P's panes. A named pane outside P is `pane-not-in-profile`. The read verbs and
  `doctor`, `park` and `unpark` with no pane name mean every pane of P; `say`, `interrupt`, `answer`, `switch` and `reset`
  still need a pane name.
- **Neither**: `pane import` takes no `--profile`; a real run creates the profile `fleet` itself (#650).

```
`docs/adr/ADR-0021.md:524-528`
```
| `pane park`, `pane unpark` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
| `pane close` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-not-in-profile` (with `--profile`, P holds no spec for the pane) |
| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
| `pane import` | `profile-exists` (a profile `fleet` already exists); `command-not-argv` and disagreements are findings, not errors (#650) |
| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found` (no session of record); open (#646), as the record says: `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; all exit 3, the code in the message. The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"` |
```
`docs/adr/ADR-0021.md:598-628`
```
### 10. Grid notation (decision 7)

`GridPos { row: u16, col: u16 }`, 1-based, defined once in `holler-pane/src/grid.rs`.

- **Accepted input:** `r<row>c<col>`, `c<col>r<row>` (labelled, either order) and a bare `<row>,<col>` (always row first,
  never column first). Upper case and surrounding ASCII whitespace are accepted; `R2C1`, `c1r2` and `2,1` are all row 2,
  column 1.
- **`grid-ambiguous`:** interior whitespace, a missing half (`r2`, `c1`), a repeated label or a mix (`r2c1c3`, `c1r2c3`), more
  than two numbers (`2,1,3`), unlabelled digits (`21`), the empty string, anything unreadable, and a JSON `pos` that
  disagrees with its `row` and `col`.
- **`grid-out-of-range`:** a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535; or, from `HerdrPort::ensure_pane`,
  a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort`
  conformance suite pins (#638 amendment 2026-10-08, grid; #683). Herdr has no grid, so a workspace's extent is the Herdr
  adapter's configuration: rows by columns per workspace label (`HerdrConfig.workspaces`, #640), which the wiring supplies
  (#649). A cell inside the extent that no single right or down split reaches is the open code `grid-unreachable` (#640).
- **Output** is always row first: text `r2c1`; JSON `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order. On input `pos`
  is optional.
- The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base. `HerdrPort::ensure_pane`
  reaches a position by right and down splits (B4, `plan_splits`, #640) or fails loudly, and never relocates a healthy pane.
  `relaunch` moves a pane only when `--grid` is given. A pane's position comes from Herdr's snapshot, never from its name:
  `hj-c1r2` sits at `r2c1`.
- A pane's name is not its position and not its identity in Herdr: the importer (#650) finds a pane's live Herdr pane by
  `HerdrPane.label == name`. A pane with no cell has no place in the snapshot and is a finding of the importer, never
  recorded.
- Herdr API versions (B5): the Herdr adapter checks Herdr's protocol when it connects, so no adapter exists for an unknown
  one, and `HerdrPort::version()` checks it again on every call; the other methods trust the check made at connect (#640).
  An unknown version is `herdr-version-unsupported`, and the message names the supported ones (#640, from #636's list). The
  string `version()` returns is recorded as `host.herdr_api_version` by a verb, never by the adapter (decided, operator, 2026-10-11:
  #644's launch and relaunch record it, so a pane's recorded version is the one seen at its last launch), and `doctor`
  shows it (#647).

```
`docs/adr/ADR-0021.md:685-710`
```
### 13. Migration from the launcher's files

Each file below is replaced by registry state:

| File | What it held | Replaced by |
|---|---|---|
| `sessions.toml` | the session the launcher recorded for each pane | `Pane.session_of_record` |
| `last-session` | the last session a pane used | `Pane.session_of_record`, with `last_observed` for what was seen |
| `owned-sessions` | the sessions the launcher created for a pane | `session_of_record`; any other session on the pane's server is a stray-session finding from reconcile (#647) |
| `pane.env` | a pane's model, effort and environment | `Pane.model`, `Pane.env` (names only; values stay in the operator's own environment), `Pane.context`, and the spec |
| the launcher's port and directory table | each pane's port, directory and launch command | `harness.port`, `host.cwd`, `command` (an argv; a shell-string command is a finding and is imported without a command) |

The hub's own registration, the fifth record of the incident, is replaced by `session_of_record` through the wiring of #649.

The order:

1. `pane import --from <state-root> --dry-run` (#650) reports every disagreement between these files and what Herdr, tmux
   and the harness show, and writes nothing.
2. The real run writes one record per pane. It sets `session_of_record` only where the sources agree and the session exists;
   otherwise the record has none and the report has a finding. It takes each position from Herdr's snapshot, never from the
   name. It records model, effort, env names and ceilings (C1), creates the profile `fleet` with one spec per imported pane, and
   sets each pane's `profile` to `fleet`. It deletes nothing from the source files.
3. The operator cuts over (#654): from then on only pane verbs change panes, and the launcher's files are no longer read.
   Retiring the launcher's writers is pfleet's follow-up (#653), not Holler's. After seven days on the live fleet (#654) the
   operator recreates the Herdr workspace from the stored profiles (#666) and watches it for 24 hours (C4).

```

### E10: the old launcher: the files it reads and the state directory
`scripts/pane-restart:5-18`
```
#   pane-restart PANE --model PROVIDER/ID [--effort E] [--key-file PATH] [--project DIR] [--agent KEY] [--dry-run]
#   pane-restart PANE [--model ALIAS|NAME] [--effort E] [--fresh] [--dry-run]            (a Claude Code pane, see below)
#
# PANE is cCrR (the tmux session is hj-PANE, derived, never typed). Steps, in order; the first failure stops the
# script and names the step:
#   1 refuse unless the pane is idle and reachable   2 back up pane.env   3 set the OC_ lines in pane.env
#   4 stop the TUI (/exit, then one Ctrl-C)          5 start it fresh      6 wait for the server
#   7 wait for the new session to register (and repair pfleet's record with --set-registration)        8 the pane answers on the new model   9 pane-ready says READY
# Every text typed into a pane is followed by Enter: in the same `tmux send-keys` call for the shell line, as a
# separate call after a short pause for `/exit` (the TUI's autocomplete popup swallows an Enter sent with it). Only the key file's PATH is written to
# pane.env; the file is never read or printed. Exit: 0 ok, 1 refused (nothing changed), 2 failed after a change.
# --mo restarts c4r2, the MO pane: it edits opencode-mo/opencode.json and agents/mo.md instead of pane.env, launches
# with OC_SESSION_NAME=mo and no --agent, needs --key-file, accepts a busy pane, and skips pane-ready.
# A pane that is already fully down is started without the stop step; that needs --project.
```
`scripts/pane-restart:262-266`
```
  [ -n "$project" ] || refuse "cannot read the project directory from $base/path; pass --project"
fi
if [ "$mo" = 0 ] && [ -z "$agent" ]; then
  agent=$(sed -n 's/^\(export[[:space:]]\{1,\}\)\{0,1\}OC_DEFAULT_AGENT=\([A-Za-z0-9_-]*\).*/\2/p' "$penv" | head -1)
fi
```
`scripts/pane-restart:276-284`
```
BEGIN { n = split("OC_MODEL OC_EFFORT OC_KEY_FILE OC_ROLE_MODEL", V, " "); for (i = 1; i <= n; i++) val[V[i]] = ENVIRON["PR_" V[i]] }
{
  if (match($0, /^(export[ \t]+)?OC_[A-Z_]+=/)) {
    name = $0; sub(/^(export[ \t]+)?/, "", name); sub(/=.*/, "", name)
    if ((name in val) && val[name] != "") { if (seen[name]++) next; print "export " name "=" val[name]; next }
  }
  print
}
END { for (i = 1; i <= n; i++) if (V[i] != "OC_ROLE_MODEL" && val[V[i]] != "" && !seen[V[i]]) print "export " V[i] "=" val[V[i]] }
```
`scripts/pane-restart:226-231`
```
fi

session="hj-$pane"
state="$HOME/.holler-oc/$session"
penv="$state/pane.env"
col=${pane:1:1}; row=${pane:3:1}
```
`scripts/pane-restart:266-269`
```
fi
old_session=$(cat "$state/last-session" 2>/dev/null || true)

# The new pane.env, built beside the old one: a replaced line keeps its place, an absent one is appended, a
```
`scripts/pane-restart:352-355`
```
followed() { # oc-holler's watcher wrote the new session to last-session and sessions.toml
  local last; last=$(cat "$state/last-session" 2>/dev/null || true)
  [ -n "$last" ] && [ "$last" != "$old_session" ] && grep -q "session_id = \"$last\"" "$state/sessions.toml" 2>/dev/null
}
```

### E11: holler-body's config: the grammar and the load/parse functions (sessions.toml is this grammar)
`crates/holler-body/src/config.rs:1-30`
```
//! The body's session config file (issue #187): named sessions, each a
//! harness + transport (`spawn` a fresh process or `attach` to one already
//! running), validated fail-closed at parse time.
//!
//! Grammar (the issue's own spec):
//!
//! '''text
//! [[session]]
//! name = "alpha"                       # SessionName grammar (ADR 0005); unique in file
//! harness = "opencode"                 # any holler_proto::HARNESS_IDS id; unknown ids are
//!                                       # legal on the wire but `support` says ok:false
//! mode = "spawn"                       # "spawn" (default) | "attach"
//! command = ["opencode", "acp"]        # spawn only; argv, no shell
//! cwd = "/path"                        # optional, spawn only
//! env = { KEY = "value" }              # optional, spawn only
//! interrupt = "acp"                    # optional: "acp" (default) | "http"; http needs endpoint
//! endpoint = "http://127.0.0.1:4096"   # attach: required; spawn: optional (HTTP interrupt fallback)
//! session_id = "ses_..."               # attach: required
//! auth_method = "api-key"              # optional, spawn only: ACP auth method id sent in `authenticate`
//!                                       # when session/new answers auth-required (ACP v1 only, #439)
//! '''
//!
//! Sharing one file with other tools (issue #436). Unknown keys are still
//! refused, but two things are accepted and **ignored** by the body:
//!
//! - an explicit, namespaced extension point: an optional top-level
//!   `[ext.<namespace>]` and per-session `[session.ext.<namespace>]`, each
//!   namespace an arbitrary TOML table the body never interprets;
//! - the setup wizard's master-file keys, type-checked only: top-level
//!   `hub_host` (string), `layout` (array of arrays of strings) and
```
`crates/holler-body/src/config.rs:94-111`
```

/// One validated `[[session]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    /// The session's name (unique within the file; [`SessionName`] grammar).
    pub name: SessionName,
    /// The harness id (`holler_proto::HARNESS_IDS`; unknown ids are legal on
    /// the wire, so this is not validated against the vocabulary here).
    pub harness: String,
    /// `spawn` or `attach`.
    pub mode: SessionMode,
    /// The spawn argv (`command[0]` resolved on `PATH` or as a path). `None`
    /// for `attach` (or dropped with a warning if the file set one anyway).
    pub command: Option<Vec<String>>,
    /// The spawned process's working directory (spawn only).
    pub cwd: Option<String>,
    /// Extra environment variables for the spawned process (spawn only).
    pub env: Option<BTreeMap<String, String>>,
```
`crates/holler-body/src/config.rs:269-273`
```
/// named" — one refusal, not an accumulated list). The non-fatal cases —
/// `attach` with a `command` or an `auth_method` set — are dropped and
/// recorded in [`ParsedConfig::warnings`] rather than refused.
pub fn parse(contents: &str) -> Result<ParsedConfig, ConfigError> {
    let raw: RawFile = toml::from_str(contents).map_err(|e| ConfigError::Toml(e.to_string()))?;
```
`crates/holler-body/src/config.rs:433-438`
```
}

/// Read + parse + validate the file at `path`.
pub fn load(path: &Path) -> Result<ParsedConfig, ConfigError> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| ConfigError::Io(format!("{}: {e}", path.display())))?;
```

### E12: launch: the constants, and the record it builds
`crates/holler-pane/src/tx_launch.rs:47-63`
```
/// `pane-exists`: `launch` of a name that already has a record (relaunch it instead). A
/// refusal, exit 3.
pub const PANE_EXISTS: RefusalCode = RefusalCode::from_static("pane-exists");
/// `grid-occupied`: the target cell already holds a Herdr pane (never adopted, recorded or
/// not). A refusal, exit 3.
pub const GRID_OCCUPIED: RefusalCode = RefusalCode::from_static("grid-occupied");
/// `port-in-use`: a harness server already answers on the port before this verb started one
/// (never adopted). A refusal, exit 3.
pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
/// The budget of one whole verb run (I5's default bound), checked before every step.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
/// The timeout handed to `Prober::run_probe`.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// `HostInfo.name` of every pane recorded here (epic decision 3: Herdr, tmux and the harness
/// run on the hub's machine).
pub const HOST_NAME: &str = "localhost";
/// The `op` of the `timeout` a launch answers when its budget runs out.
```
`crates/holler-pane/src/tx_launch.rs:313-364`
```
    /// profile, its profile.
    fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
        let (hold, driven, profile) = match self.old {
            Some(old) => (
                old.hold.clone(),
                old.last_observed.driven.clone(),
                self.profile.or(old.profile.as_ref()).cloned(),
            ),
            None => (Hold::None, None, self.profile.cloned()),
        };
        let spec = self.spec;
        Pane {
            name: self.name.clone(),
            generation: self.old.map_or(0, |old| old.generation),
            // The label Herdr reports is observed, never recorded (#808, constraint (c)).
            herdr: HerdrPane {
                label: None,
                ..live.herdr
            },
            host: HostInfo {
                name: HOST_NAME.to_owned(),
                tmux: self.name.as_str().to_owned(),
                cwd: spec.host.cwd.clone(),
                herdr_api_version: Some(seen.herdr_api_version.clone()),
            },
            harness: HarnessInfo {
                kind: spec.harness.kind,
                port: self.port,
                pid: Some(live.pid),
                health: Health::Healthy,
            },
            session_of_record: Some(live.session),
            role: spec.role,
            hold,
            last_observed: LastObserved {
                shown: live.shown,
                driven,
                at,
            },
            profile,
            model: spec.model.clone(),
            opencode_agent: spec.opencode_agent.clone(),
            env: spec.env.clone(),
            context: spec.context,
            command: spec.command.clone(),
            probe: PaneProbe {
                check: spec.check.clone(),
                expect: spec.expect.clone(),
                last: seen.probe.clone(),
            },
        }
    }
```

### E13: the fake Herdr's label hook
`crates/holler-pane-testkit/src/herdr.rs:241-260`
```

    /// The outside world labels the pane `label`, replacing any label it had, as Herdr's
    /// `pane.rename` does (the setup wizard's `herdr.sh label` sets one). `snapshot` and
    /// `ensure_pane` then report it, except that `""` is reported as no label (#808). It
    /// bypasses the faults and the call log. `pane-not-found` when no such pane exists.
    pub fn rename(&self, pane: &PaneId, label: &str) -> Result<(), PaneError> {
        self.lock().pane_mut(pane)?.label = Some(label.to_owned());
        Ok(())
    }

    /// The state, locked. A poisoned lock is taken over: a test that panicked while
    /// holding it must not wedge every later call.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HerdrPort for FakeHerdr {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        self.faults.enter(HerdrOp::EnsurePane)?;
```

### E14: doctor's verb shape and output::emit
`crates/holler-cli/src/pane/doctor.rs:38-64`
```
}

/// Run `holler pane doctor`: one reconcile pass, printed in the run's format.
pub fn run(args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    let report = pass(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, report, render)
}

/// Type the arguments (a bad pane or profile name is `usage`) and run the pass now.
fn pass(args: &PaneDoctor, ports: Ports<'_>) -> Result<Report, PaneError> {
    let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
    let profile = args
        .profile
        .profile
        .as_deref()
        .map(ProfileName::parse)
        .transpose()?;
    let request = ReconcileRequest {
        profile: profile.as_ref(),
        pane: pane.as_ref(),
        fix: args.fix,
        now_ms: now_millis(),
    };
    reconcile(ports, &request)
}

/// The text form of `report`, one line each.
```
`crates/holler-cli/src/output.rs:192-221`
```
///
/// `ports` is held by value: [`Ports`] is `Copy`. A verb takes `&mut VerbCtx` because writing to
/// the sink needs a mutable borrow.
pub struct VerbCtx<'a> {
    pub format: Format,
    pub ports: Ports<'a>,
    pub sink: Sink<'a>,
}

/// Print one result and return the exit code: 0 ok, else the exit code of the error's class
/// (1 runtime failure, 2 usage, 3 refusal).
///
/// `text` renders the data for text mode (JSON mode serializes the data itself and never calls
/// it). The text ends with one newline, which `emit` adds when it is missing; empty text writes
/// nothing.
pub fn emit<T: Serialize>(
    sink: &mut Sink<'_>,
    format: Format,
    result: Result<T, ErrorBody>,
    text: impl FnOnce(&T) -> String,
) -> i32 {
    match format {
        Format::Text => emit_text(sink, result, text),
        Format::Json => emit_json(sink, result),
    }
}

/// Print a stream of results, one line each, and return the exit code.
///
/// Text mode writes `text(item)` per item; JSON mode writes one envelope per line (NDJSON, which is
```

### E15: EnvVarName::parse
`crates/holler-pane/src/argv.rs:88-116`
```
///
/// Parsing is the guard (see the module docs). It serializes as a plain string and
/// reads back through [`EnvVarName::parse`], so a record that holds a value does not
/// load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Check `text` and keep it as a name.
    ///
    /// - a `=` anywhere: `profile-secret-refused` (the entry carries a value);
    /// - empty, or any whitespace or control character: `env-name-invalid`.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        if text.contains('=') {
            return Err(PaneError::ProfileSecretRefused);
        }
        if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(PaneError::EnvNameInvalid);
        }
        Ok(Self(text.to_owned()))
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

```

### E16: RefusalCode (the channel for a code a story owns) and how the open codes are classed
`crates/holler-pane/src/error.rs:301-335`
```
        | PaneCode::ProfileDrift => ErrorClass::Failure,
    }
}

/// A code an adapter or a verb owns: well-formed ([`is_valid_code`]) and not one
/// of the closed codes ([`ALL_CODES`]), so each code has exactly one representation.
///
/// The field is private and there is no `From<&str>`, `From<String>` or `Default`:
/// the only ways to get one are [`RefusalCode::from_static`] (for a `const`) and
/// [`RefusalCode::parse`] (for a code read off the wire). A [`PaneError::Refused`]
/// therefore cannot carry an unvalidated code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefusalCode(Cow<'static, str>);

impl RefusalCode {
    /// A code declared as a constant, checked when the constant is evaluated:
    /// a literal that is not kebab-case, or is a closed code, fails the build.
    /// A verb or an adapter declares the code it owns with
    /// `const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");`,
    /// so the call site needs no `Result`.
    ///
    /// An invalid literal in a `const` does not compile:
    ///
    /// '''compile_fail,E0080
    /// use holler_pane::error::RefusalCode;
    /// const BAD: RefusalCode = RefusalCode::from_static("Not A Code");
    /// '''
    ///
    /// Call it from a `const` item. Called at run time with a literal that is not
    /// an open code it panics, which only a programming error can cause.
    pub const fn from_static(code: &'static str) -> Self {
        assert!(
            is_valid_code(code) && !is_closed_code(code),
            "RefusalCode::from_static: not a kebab-case code, or a closed code"
        );
```

### E17: reconcile's trust rule: only a healthy server's list counts
`crates/holler-pane/src/reconcile/observe.rs:16-80`
```
/// The calls of a chain, named as `<port>.<method>` in an `observe-failed` message.
const HOST_PS: &str = "host.ps";
const HARNESS_HEALTH: &str = "harness.health";
const HARNESS_LIST_SESSIONS: &str = "harness.list_sessions";
const HARNESS_SHOWN_SESSION: &str = "harness.shown_session";
const PANE_STORE_CAS_PUT: &str = "pane_store.cas_put";

/// What one pane's chain found and observed.
pub(super) struct Checked {
    pub(super) findings: Vec<Finding>,
    pub(super) summary: PaneSummary,
    /// The sessions of the pane's server, when it passed its health check and its list was
    /// read: the only list the stray rule trusts.
    pub(super) listed: Option<Vec<String>>,
}

/// The pane's harness server, as the chain observed it.
enum Server {
    /// It passed its health check; `sessions` is its list, `None` when reading it failed.
    Healthy { sessions: Option<Vec<String>> },
    /// It failed its health check, and its list timed out: it accepts and never answers.
    Wedged,
    /// It failed its health check, and its list did anything but time out.
    Down,
    /// The health check itself failed: nothing is known of the server.
    Unknown,
}

impl Server {
    /// The sessions of a healthy server, when they were read: the only list a rule trusts
    /// (an unhealthy server's answer is not).
    fn sessions(&self) -> Option<&[String]> {
        match self {
            Server::Healthy { sessions } => sessions.as_deref(),
            Server::Wedged | Server::Down | Server::Unknown => None,
        }
    }

    /// Whether the server's trusted list holds `session`.
    fn has(&self, session: &str) -> bool {
        self.sessions()
            .is_some_and(|ids| ids.iter().any(|id| id == session))
    }

    fn health(&self) -> ObservedHealth {
        match self {
            Server::Healthy { .. } => ObservedHealth::Healthy,
            Server::Wedged => ObservedHealth::ServerWedged,
            Server::Down => ObservedHealth::ServerDown,
            Server::Unknown => ObservedHealth::Unknown,
        }
    }

    /// The trusted list, owned.
    fn into_sessions(self) -> Option<Vec<String>> {
        match self {
            Server::Healthy { sessions } => sessions,
            Server::Wedged | Server::Down | Server::Unknown => None,
        }
    }
}

/// What the chain knows of the TUI's screen.
enum Screen {
    /// The session it shows; `None` is its home screen, or no TUI in the pane.
```

### E19: doctor's rig, which the import's rig extends
`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:1-16`
```
//! The doctor rig (#647, brief Decision 12): a live world on the test kit's fakes, with one
//! record per pane that matches it.
//!
//! For each seeded pane it makes a Herdr pane (`ensure_pane`), a tmux session
//! (`ensure_session`), a harness server on its own port (`serve`, 48100 + index), a
//! session on it (`create_session`) and a TUI showing that session (`attach_tui`), then
//! stores a `sample_pane` naming all of them, with that session as its session of record.
//! A freshly built rig is a healthy fleet: doctor reports nothing about it.
//!
//! The setup goes through the ports, so it is in the fakes' call logs: a test that asserts
//! on the calls doctor made takes a [`Rig::mark`] first and reads [`Rig::calls_since`].

use std::collections::BTreeMap;
use std::sync::Arc;

use holler_cli::output::Format;
```

### E21: check_envelope
`crates/holler-pane-testkit/src/envelope.rs:188-210`
```
            EnvelopeFault::NotLastFailure => {
                f.write_str("a line before the last of the NDJSON stream is not a success")
            }
        }
    }
}

impl std::error::Error for EnvelopeFault {}

/// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
/// code of the verb that wrote it.
///
/// Returns the envelope, or the fault of the first rule it breaks, in the order the
/// module doc lists.
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
    check_exit_code(exit_code)?;
    let Value::Object(object) = frame(stdout)? else {
        return Err(EnvelopeFault::NotAnObject);
    };
    let [schema_version, ok, data, error] = exact_members(object, ENVELOPE_KEYS, "")?;
    if schema_version.as_u64() != Some(SCHEMA_VERSION) {
        return Err(EnvelopeFault::SchemaVersion);
    }
```

### E22: PaneStore and ProfileStore: the compare-and-swap writes and deletes the real run and its undo call
`crates/holler-pane/src/ports.rs:57-85`
```
/// The registry of panes, kept by the hub (#639) and faked by the test kit (#638).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// Every write is a compare-and-swap on the pane's `generation` (see
/// [`crate::generation`]); a stale one is `generation-conflict`.
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;

    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;

    /// Remove a pane's record if it is still at `expected_generation`. `close`
    /// uses it, so a closed pane's record does not outlive the pane. A stale
    /// generation is `generation-conflict`; a record that does not exist is
    /// `pane-not-found`, whatever `expected_generation` is (a missing record is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;

    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
}
```
`crates/holler-pane/src/profile.rs:323-357`
```
/// The registry of profiles, kept by the hub (#661) and faked by the test kit (#638).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// **Writes** are compare-and-swap on the profile's `generation` (see
/// [`crate::generation`]) and carry the [`Actor`] that made them; each appends one
/// [`ProfileLogEntry`]. A stale generation is `generation-conflict`.
pub trait ProfileStore: Send + Sync {
    /// The profile named `name`, or `None`.
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;

    /// Every profile.
    fn list(&self) -> Result<Vec<Profile>, PaneError>;

    /// Store `profile` if the stored one is still at `expected_generation` (0 for a
    /// new profile); returns the stored record with its bumped generation.
    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError>;

    /// Delete the profile `name` if it is still at `expected_generation`. A stale
    /// generation is `generation-conflict`; a profile that does not exist is
    /// `profile-not-found`, whatever `expected_generation` is (a missing profile is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError>;
```

### E23: HostPort::ps, HarnessPort::health and list_sessions, and Ports: the reads the import makes
`crates/holler-pane/src/ports.rs:157-175`
```
/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;

    /// Run `argv` (never through a shell) in the session `name`.
    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>;

    /// Stop the processes the session `name` owns.
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;

    /// The process ids running in the session `name`.
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
}
```
`crates/holler-pane/src/ports.rs:177-195`
```
/// The harness server of a pane and its sessions (the adapter is
/// `holler-adapter-opencode`, #642). **Final:** the spike #635 confirmed it unchanged;
/// ADR-0021 section 2, "`HarnessPort` as built (#642)", records what the adapter decided.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HarnessPort: Send + Sync {
    /// Start the harness server for `name` on `port`; returns its process id.
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>;

    /// Whether the server on `port` answers.
    fn health(&self, port: u16) -> Result<bool, PaneError>;

    /// Create a session on the server at `port`; returns its id.
    fn create_session(&self, port: u16) -> Result<String, PaneError>;

    /// The session ids on the server at `port`.
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError>;
```
`crates/holler-pane/src/ports.rs:270-281`
```
/// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
/// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
#[derive(Clone, Copy)]
pub struct Ports<'a> {
    pub pane_store: &'a dyn PaneStore,
    pub profile_store: &'a dyn ProfileStore,
    pub herdr: &'a dyn HerdrPort,
    pub host: &'a dyn HostPort,
    pub harness: &'a dyn HarnessPort,
    pub scope: &'a dyn ProfileScope,
    pub prober: &'a dyn Prober,
}
```

### E24: holler-body's SessionConfig (the rest of the struct) and the attach rule (an attach row needs session_id)
`crates/holler-body/src/config.rs:112-125`
```
    /// How to interrupt an in-flight turn.
    pub interrupt: Interrupt,
    /// The attach endpoint (required for `attach`; an optional HTTP-interrupt
    /// fallback for `spawn`).
    pub endpoint: Option<String>,
    /// The existing harness session id being attached to (attach only).
    pub session_id: Option<String>,
    /// The ACP auth method id the driver sends in `authenticate` when the
    /// adapter answers `session/new` with auth-required (spawn only, ACP v1
    /// only; issue #439). Only an id: the credential itself stays in the
    /// spawned process's environment. Kept exactly as written (compared
    /// case-sensitively, untrimmed); `None` for `attach`.
    pub auth_method: Option<String>,
}
```
`crates/holler-body/src/config.rs:304-340`
```
        let mut command = r.command.clone();
        match mode {
            SessionMode::Spawn => {
                if command.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "command",
                        reason: "spawn mode requires `command`".to_string(),
                    });
                }
            }
            SessionMode::Attach => {
                if r.endpoint.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "endpoint",
                        reason: "attach mode requires `endpoint`".to_string(),
                    });
                }
                if r.session_id.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "session_id",
                        reason: "attach mode requires `session_id`".to_string(),
                    });
                }
                if command.take().is_some() {
                    warnings.push(format!(
                        "session {:?} (row {row}): attach mode ignores `command` — never spawned",
                        r.name
                    ));
                }
            }
        }
```

### E25: profile show: the members it compares, and why a spec built from a pane's own record matches it
`crates/holler-cli/src/profile/show.rs:74-103`
```
/// The profile named in `args` and its comparison with its members. A bad name is
/// `usage`, a profile that does not exist is `profile-not-found`, and a store error passes
/// through with its own code.
fn view(args: &ProfileShow, ports: Ports<'_>) -> Result<ProfileView, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let profile = ports
        .profile_store
        .get(&name)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", name.as_str()),
        })?;
    let members: Vec<Pane> = ports
        .pane_store
        .list()?
        .into_iter()
        .filter(|pane| is_member(pane, &profile.name))
        .collect();
    let comparison = diff_profile(&profile, &members)
        .into_iter()
        .map(|diff| ComparisonRow {
            probe: last_probe(&diff, &members),
            diff,
        })
        .collect();
    Ok(ProfileView {
        live: members.len(),
        profile,
        comparison,
    })
}
```
`crates/holler-pane/src/profile_diff.rs:254-285`
```
/// Whether `pane` is a member of `profile`: its `profile` has `profile.slug()`. Slugs are
/// compared on both sides, since the slug is a profile's identity (a pane whose profile
/// is `SOME-PROFILE` is a member of `Some Profile`), like the test kit's fakes do.
///
/// This is what "a live pane of a profile" means (ADR-0021 section 3): `profile list`'s
/// live count, `profile show`'s comparison and `profile delete`'s refusal
/// `profile-has-live-panes` all use it.
pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
    pane.profile
        .as_ref()
        .is_some_and(|own| own.slug() == profile.slug())
}

/// Every field where `live` differs from `spec`, in `SpecField::ALL` order; empty when
/// they match. The names are not compared (the caller pairs them). `env` and `expect`
/// are compared as sets (sorted, duplicates dropped); every other field by equality.
/// Pure: it compares `spec` with `spec_from_pane(live)`, so
/// `diff_spec(&spec_from_pane(p), p)` is always empty.
pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff> {
    let observed = spec_from_pane(live);
    SpecField::ALL
        .into_iter()
        .filter_map(|field| {
            let (want, got) = (field.value(spec), field.value(&observed));
            field.differs(&want, &got).then_some(FieldDiff {
                field,
                spec: want,
                live: got,
            })
        })
        .collect()
}
```
`crates/holler-pane/src/profile_diff.rs:311-324`
```
/// The row of `spec` compared with `pane`, the live pane of its name.
fn compared(spec: &ProfileSpec, pane: &Pane) -> PaneDiff {
    let differences = diff_spec(spec, pane);
    let status = if differences.is_empty() {
        PaneStatus::Matches
    } else {
        PaneStatus::Differs
    };
    PaneDiff {
        pane: spec.pane.clone(),
        status,
        differences,
    }
}
```

### E26: how profile show prints a spec: SpecField::ALL, SpecField::as_str, SpecField::value and FieldValue
`crates/holler-cli/src/profile/show.rs:134-142`
```
/// `spec PANE`, then one `FIELD: VALUE` line per field.
fn push_spec(lines: &mut Vec<String>, spec: &ProfileSpec) {
    lines.push(format!("spec {}", text(&spec.pane)));
    lines.extend(
        SpecField::ALL
            .into_iter()
            .map(|field| format!("  {}: {}", field.as_str(), field.value(spec))),
    );
}
```
`crates/holler-pane/src/profile_diff.rs:52-94`
```
impl SpecField {
    /// Every field, in the order `profile show` lists a spec and [`diff_spec`] compares
    /// it. `harness.port_policy` is compared like every other field: the live side is
    /// the policy of the port the pane's harness uses (ADR-0021 section 3).
    pub const ALL: [SpecField; 15] = [
        SpecField::Workspace,
        SpecField::Grid,
        SpecField::Cwd,
        SpecField::HarnessKind,
        SpecField::PortPolicy,
        SpecField::Provider,
        SpecField::ModelId,
        SpecField::Effort,
        SpecField::Role,
        SpecField::Env,
        SpecField::ContextSoft,
        SpecField::ContextHard,
        SpecField::Command,
        SpecField::Check,
        SpecField::Expect,
    ];

    /// The dotted path of the field in a `ProfileSpec`'s JSON (`"herdr.grid"`), which is
    /// also its serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecField::Workspace => "herdr.workspace",
            SpecField::Grid => "herdr.grid",
            SpecField::Cwd => "host.cwd",
            SpecField::HarnessKind => "harness.kind",
            SpecField::PortPolicy => "harness.port_policy",
            SpecField::Provider => "model.provider",
            SpecField::ModelId => "model.model_id",
            SpecField::Effort => "model.effort",
            SpecField::Role => "role",
            SpecField::Env => "env",
            SpecField::ContextSoft => "context.soft",
            SpecField::ContextHard => "context.hard",
            SpecField::Command => "command",
            SpecField::Check => "check",
            SpecField::Expect => "expect",
        }
    }
```
`crates/holler-pane/src/profile_diff.rs:96-118`
```
    /// The value of this field in `spec`. `harness.kind` and `role` take their text from
    /// their serde form, so no variant name is written a second time.
    pub fn value(self, spec: &ProfileSpec) -> FieldValue {
        match self {
            SpecField::Workspace => FieldValue::Text(spec.herdr.workspace.clone()),
            SpecField::Grid => FieldValue::Grid(spec.herdr.grid),
            SpecField::Cwd => FieldValue::Text(spec.host.cwd.clone()),
            SpecField::HarnessKind => FieldValue::Text(serde_text(&spec.harness.kind)),
            SpecField::PortPolicy => FieldValue::Text(spec.harness.port_policy.clone()),
            SpecField::Provider => FieldValue::Text(spec.model.provider.clone()),
            SpecField::ModelId => FieldValue::Text(spec.model.model_id.clone()),
            SpecField::Effort => FieldValue::Text(spec.model.effort.clone()),
            SpecField::Role => FieldValue::Text(serde_text(&spec.role)),
            SpecField::Env => {
                FieldValue::List(spec.env.iter().map(|n| n.as_str().to_owned()).collect())
            }
            SpecField::ContextSoft => FieldValue::Number(spec.context.soft),
            SpecField::ContextHard => FieldValue::Number(spec.context.hard),
            SpecField::Command => FieldValue::Argv(spec.command.clone()),
            SpecField::Check => FieldValue::Argv(spec.check.clone()),
            SpecField::Expect => FieldValue::List(spec.expect.clone()),
        }
    }
```
`crates/holler-pane/src/profile_diff.rs:155-193`
```
/// One field's value, typed so that it serializes exactly as the field does inside a
/// `ProfileSpec` (a grid through `GridPos`'s own `Serialize`, row first) and prints for a
/// person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// `herdr.workspace`, `host.cwd`, `harness.kind`, `harness.port_policy`, the three
    /// `model` fields and `role`.
    Text(String),
    /// `herdr.grid`.
    Grid(GridPos),
    /// `context.soft`, `context.hard`.
    Number(u32),
    /// `env` (the names) and `expect`, in stored order.
    List(Vec<String>),
    /// `command`, `check`; `None` serializes as `null`.
    Argv(Option<Argv>),
}

impl fmt::Display for FieldValue {
    /// The text form:
    ///
    /// - `Text` with every control character escaped by `char::escape_default`, so ESC
    ///   prints as `\u{1b}` and a newline as `\n` (the rule Holler's text log lines
    ///   escape a field value with, in `holler_proto::log`); every other character as is.
    /// - `Grid` as `r2c1`, and `Number` in decimal.
    /// - `List` and `Argv(Some)` as a compact JSON array of strings, `["opencode","serve"]`,
    ///   never a line joined for a shell; `Argv(None)` as `none`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Text(text) => write_escaped(f, text),
            FieldValue::Grid(grid) => write!(f, "{grid}"),
            FieldValue::Number(number) => write!(f, "{number}"),
            FieldValue::List(items) => write_json_strings(f, items),
            FieldValue::Argv(Some(argv)) => write_json_strings(f, argv.as_slice()),
            FieldValue::Argv(None) => f.write_str("none"),
        }
    }
}
```

### E27: Profile, the type of data.profile
`crates/holler-pane/src/profile.rs:206-224`
```
/// A named, stored set of panes. Every write is a compare-and-swap on
/// [`Profile::generation`] (see [`crate::generation`]).
///
/// On read, the stored `slug` must equal the slug derived from the name: a record
/// where they disagree is corrupt or forged and does not load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawProfile")]
pub struct Profile {
    pub name: ProfileName,
    /// The unique id derived from the name ([`ProfileName::slug`]).
    pub slug: String,
    /// Bumped on every change.
    pub generation: u64,
    pub panes: Vec<ProfileSpec>,
    /// When the profile was created (milliseconds since the Unix epoch).
    pub created: i64,
    /// When it last changed (milliseconds since the Unix epoch).
    pub updated: i64,
}
```

### E28: command-not-argv is a closed PaneError code
`crates/holler-pane/src/error.rs:70-77`
```
impl PaneCode {
    /// Every closed code, in the epic's order.
    pub(crate) const ALL: [PaneCode; CODE_COUNT] = [
        PaneCode::NotImplemented,
        PaneCode::Usage,
        PaneCode::GridAmbiguous,
        PaneCode::GridOutOfRange,
        PaneCode::CommandNotArgv,
```
`crates/holler-pane/src/error.rs:97-104`
```
    /// The kebab-case wire string of this code.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            PaneCode::NotImplemented => "not-implemented",
            PaneCode::Usage => "usage",
            PaneCode::GridAmbiguous => "grid-ambiguous",
            PaneCode::GridOutOfRange => "grid-out-of-range",
            PaneCode::CommandNotArgv => "command-not-argv",
```
`crates/holler-pane/src/error.rs:140-146`
```
/// Every closed code, as its kebab-case wire string, in the epic's order.
///
/// The set is closed: the invariants of this crate (one variant per code, unique,
/// valid under [`is_valid_code`]) are tested against it, and no story edits it.
/// `profile-drift` is listed for convenience: it is a reconcile finding kind
/// (#647/#665 put it in `findings.rs`), not an error a port returns.
pub const ALL_CODES: &[&str] = &CLOSED_CODES;
```
`crates/holler-pane/src/error.rs:419-421`
```
    /// `command-not-argv`: a command given where an argv array is required, such as
    /// a shell string. (#637, `Argv`.)
    CommandNotArgv,
```

### E29: FindingKind's code(), its Serialize and its code-table test: the pattern ImportFinding copies
`crates/holler-pane/src/findings.rs:92-109`
```
    /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
    /// once merged (ADR-0021 section 9).
    pub const fn code(self) -> &'static str {
        match self {
            FindingKind::HerdrVersionUnsupported => "herdr-version-unsupported",
            FindingKind::ObserveFailed => "observe-failed",
            FindingKind::UnregisteredHerdrPane => "unregistered-herdr-pane",
            FindingKind::HerdrPaneMissing => "herdr-pane-missing",
            FindingKind::TmuxSessionMissing => "tmux-session-missing",
            FindingKind::ServerWedged => "server-wedged",
            FindingKind::ServerDown => "server-down",
            FindingKind::NoSessionOfRecord => "no-session-of-record",
            FindingKind::SessionOfRecordMissing => "session-of-record-missing",
            FindingKind::TuiForeignSession => "tui-foreign-session",
            FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
            FindingKind::StraySession => "stray-session",
        }
    }
```
`crates/holler-pane/src/findings.rs:146-151`
```
impl Serialize for FindingKind {
    /// The kind's [`code`](FindingKind::code), so the JSON form cannot drift from it.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}
```
`crates/holler-pane/tests/findings_test.rs:27-44`
```
#[test]
fn finding_kind_codes_are_stable() {
    let codes: Vec<&str> = FindingKind::ALL.iter().map(|k| k.code()).collect();
    assert_eq!(codes, CODES, "the kinds, in declaration order");

    let unique: BTreeSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len(), "a duplicate code: {codes:?}");

    for kind in FindingKind::ALL {
        assert!(
            is_valid_code(kind.code()),
            "{:?} is not kebab-case",
            kind.code()
        );
        let json = serde_json::to_value(kind).unwrap();
        assert_eq!(json, kind.code(), "{kind:?} serializes as its code");
    }
}
```

### E30: GridPos's JSON form, and the order of PaneName (a SessionName, a String)
`crates/holler-pane/src/grid.rs:129-138`
```
impl Serialize for GridPos {
    /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("GridPos", 3)?;
        out.serialize_field("row", &self.row)?;
        out.serialize_field("col", &self.col)?;
        out.serialize_field("pos", &self.to_string())?;
        out.end()
    }
}
```
`crates/holler-pane/src/pane.rs:25-32`
```
/// The name of a pane, e.g. `hj-c1r1` (the tmux session name). It is a
/// `holler_proto::vocab::SessionName`: the ADR 0005 name grammar is reused, not
/// copied, so a pane name is exactly a valid session name. Serde goes through
/// [`PaneName::parse`]; an invalid name is `usage`.
///
/// Pane names such as `hj-c1r2` are names, not positions, and are not renamed.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneName(SessionName);
```
`crates/holler-proto/src/vocab.rs:73-74`
```
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionName(String);
```

### E31: reconcile observes every pane at once, one scoped thread per pane
`crates/holler-pane/src/reconcile.rs:7-8`
```
//! One pass, over every pane in scope at once (a thread per pane, so a pass takes about as
//! long as its slowest pane's chain, not the pane count times it: I5):
```
`crates/holler-pane/src/reconcile.rs:251-278`
```
/// Observe Herdr and every pane of `scope` at once: one thread for the two Herdr calls and
/// one per pane, each pane's in `scope` order. A thread that cannot start or does not finish
/// leaves `None` for Herdr, and a lone `observe-failed` for a pane.
fn observe_all(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    scope: &[Pane],
) -> (Option<HerdrSeen>, Vec<Checked>) {
    thread::scope(|threads| {
        let herdr = thread::Builder::new().spawn_scoped(threads, move || HerdrSeen {
            version: ports.herdr.version(),
            snapshot: ports.herdr.snapshot(),
        });
        let chains: Vec<_> = scope
            .iter()
            .map(|pane| {
                thread::Builder::new()
                    .spawn_scoped(threads, move || observe::check_pane(ports, request, pane))
            })
            .collect();
        let checked: Vec<Checked> = scope
            .iter()
            .zip(chains)
            .map(|(pane, chain)| joined(chain).unwrap_or_else(|| observe::lost(pane)))
            .collect();
        (joined(herdr), checked)
    })
}
```

### E32: `herdr-version-unsupported`: a closed error of class refusal (exit 3), which doctor reports as a finding; the fake Herdr's unsupported build
`crates/holler-pane/src/error.rs:453-455`
```
    /// `herdr-version-unsupported`: Herdr reports an API version the adapter does not
    /// know; `message` names the version and the supported ones. (#640.)
    HerdrVersionUnsupported { message: String },
```
`crates/holler-pane/src/error.rs:115`
```
            PaneCode::HerdrVersionUnsupported => "herdr-version-unsupported",
```
`crates/holler-pane/src/error.rs:276-291`
```
        // Understood and declined: a guard, a policy or a gate said no, the name is
        // taken, or the request named something that does not exist.
        PaneCode::GridAmbiguous
        | PaneCode::GridOutOfRange
        | PaneCode::CommandNotArgv
        | PaneCode::EnvNameInvalid
        | PaneCode::ProfileSecretRefused
        | PaneCode::ProfileExists
        | PaneCode::ProfileHasLivePanes
        | PaneCode::PaneNotInProfile
        | PaneCode::PaneInOtherProfile
        | PaneCode::ProbeFailed
        | PaneCode::HerdrVersionUnsupported
        | PaneCode::ProfileNotFound
        | PaneCode::PaneNotFound
        | PaneCode::SessionNotFound => ErrorClass::Refusal,
```
`crates/holler-pane/src/error.rs:235-245`
```
impl ErrorClass {
    /// The exit code of a verb that ends with an error of this class, the same in
    /// text and JSON mode: 2 usage, 3 refusal, 1 runtime failure (ADR 0003's exit
    /// codes; success is 0 and has no class).
    pub const fn exit_code(self) -> i32 {
        match self {
            ErrorClass::Usage => 2,
            ErrorClass::Refusal => 3,
            ErrorClass::Failure => 1,
        }
    }
```
`crates/holler-pane/src/reconcile.rs:300-305`
```
    match &herdr.version {
        Ok(_) => {}
        Err(PaneError::HerdrVersionUnsupported { message }) => findings.push(Finding::new(
            FindingKind::HerdrVersionUnsupported,
            format!("{}; no holler verb upgrades Herdr", embedded(message)),
        )),
```
`crates/holler-pane-testkit/src/herdr.rs:92-100`
```
/// Which Herdr build `version()` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HerdrVersion {
    /// Protocol 22: `version()` is `Ok(PROTOCOL_22_VERSION)` (the default).
    #[default]
    Protocol22,
    /// A build the adapter does not know: `version()` is `herdr-version-unsupported`.
    Unsupported,
}
```
`crates/holler-pane-testkit/src/herdr.rs:208-211`
```
    /// Which build `version()` reports from now on.
    pub fn set_version(&self, version: HerdrVersion) {
        self.lock().version = version;
    }
```

### E33: `toml` 0.8.23, the version `Cargo.lock` pins: `toml::from_str` is `de::from_str` and answers a `de::Error`, which keeps its message apart from its span, a byte range (the excerpts of `lib.rs` and `de.rs` are cut by command from the crates.io source of `toml` 0.8.23, not from the repository)
`Cargo.lock:2741-2742`
```
name = "toml"
version = "0.8.23"
```
`toml-0.8.23/src/lib.rs:167`
```
pub use crate::de::{from_str, Deserializer};
```
`toml-0.8.23/src/de.rs:40-52`
```
#[cfg(feature = "parse")]
pub fn from_str<T>(s: &'_ str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(Deserializer::new(s))
}

/// Errors that can occur when deserializing a type.
#[derive(PartialEq, Eq, Clone)]
pub struct Error {
    inner: crate::edit::de::Error,
}
```
`toml-0.8.23/src/de.rs:63-72`
```
    /// What went wrong
    pub fn message(&self) -> &str {
        self.inner.message()
    }

    /// The start/end index into the original document where the error occurred
    #[cfg(feature = "parse")]
    pub fn span(&self) -> Option<std::ops::Range<usize>> {
        self.inner.span()
    }
```

### E34: holler-body's ConfigError, the raw `[[session]]` row (`harness` has no default) and a `reason` that quotes a value of the file
`crates/holler-body/src/config.rs:136-154`
```
/// Why a config file was refused. Every variant is a fail-closed validation
/// refusal the CLI maps to exit 3 (ADR 0003) — there is no "warning that
/// still exits 0" case here (that's [`ParsedConfig::warnings`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file failed to parse as TOML, or violated a `deny_unknown_fields`
    /// gate (an unknown top-level or per-session key). The message is the
    /// `toml` crate's own (it already names the offending key/line).
    Toml(String),
    /// A per-session validation failure: `row` is the 0-based index into the
    /// file's `[[session]]` array, `name` is that row's (possibly invalid)
    /// `name` value, `field` is the offending key, `reason` is human-
    /// readable.
    Invalid { row: usize, name: String, field: &'static str, reason: String },
    /// No config file was found via any discovery source.
    NoConfigFound,
    /// The file could not be read (permissions, not a file, etc).
    Io(String),
}
```
`crates/holler-body/src/config.rs:209-216`
```
/// One raw `[[session]]` table, before validation. `deny_unknown_fields`
/// catches a typo'd key (e.g. `harnes`) as a parse error rather than a
/// silently-ignored field.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSession {
    name: String,
    harness: String,
```
`crates/holler-body/src/config.rs:409-420`
```
fn parse_mode(row: usize, raw_name: &str, mode: Option<&str>) -> Result<SessionMode, ConfigError> {
    match mode {
        None | Some("spawn") => Ok(SessionMode::Spawn),
        Some("attach") => Ok(SessionMode::Attach),
        Some(other) => Err(ConfigError::Invalid {
            row,
            name: raw_name.to_string(),
            field: "mode",
            reason: format!("unknown mode {other:?} (expected \"spawn\" or \"attach\")"),
        }),
    }
}
```
