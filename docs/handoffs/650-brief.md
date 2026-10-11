# [feat(pane): migration, import today's panes from sessions.toml, last-session, owned-sessions and pane.env (dry-run first)](https://github.com/Performant-Labs/holler/issues/650)

Part of epic #633. This is the complete, final statement of the story (consolidated 2026-10-11 after the architecture review of the first run, the merge of #808 and the operator's decisions). Where an older comment or amendment disagrees, this text is the decision. The reviewers cannot see the repository: every fact the acceptance relies on is quoted in "Evidence (verbatim)" at the end (E1, E2, ...).

## Scope (final)

`holler pane import --from <state-root> --herdr-session NAME [--launcher FILE] [--orchestrator PANE]... [--dry-run] [--format=text|json]`

Read the current fleet's per-pane files and produce registry records, a profile `fleet`, and one report of every disagreement between those files and what Herdr, tmux and the harness show. The dry run writes nothing. A real run writes the records and the profile or writes nothing at all. The verb adopts what is running: it never starts, stops, types into or moves anything, and it deletes nothing from the source files.

### The input (this story fixes these shapes; the repository holds no sample of the real fleet's files)

- **`<state-root>`** is the directory that holds one directory per pane: today `$HOME/.holler-oc/`, whose per-pane directory is `hj-<cCrR>` (E10: `state="$HOME/.holler-oc/$session"`). **The pane's name is the directory's name** (`hj-c1r2`), parsed with `PaneName::parse`. A subdirectory that holds none of the four pane files below (for example `opencode-mo/`) is not a pane directory: it is an information finding `ignored-entry` and nothing else. A subdirectory that holds a pane file but whose name does not parse is a problem finding `source-unreadable`.
- **Per-pane files**, each under the pane's directory:
  - `pane.env`: lines of the form `[export ]NAME=value` (E10, the pattern `^(export[ \t]+)?OC_[A-Z_]+=`). The reader takes the **value** of exactly five names: `OC_MODEL` (`PROVIDER/MODEL_ID`, split at the first `/`, into `ModelSpec.provider` and `model_id`), `OC_EFFORT` (`ModelSpec.effort`), `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD` (unsigned integers, `ContextCeilings.soft` and `hard`), and `OC_DEFAULT_AGENT` (optional, `Pane.opencode_agent` through `AgentKey::parse`). **Every other variable is kept by NAME only** through `EnvVarName::parse`, in file order and de-duplicated; its value is never read into any value, log line or finding. A line that is blank or starts with `#` is skipped. `OC_MODEL`, `OC_EFFORT`, `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD` are required (E2: `ModelSpec` and `ContextCeilings` have no empty form).
  - `last-session`: the whole content, trimmed, is one session id (E10). Absent or empty means "no session recorded".
  - `sessions.toml`: the **`holler-body` attach-config grammar** (confirmed in code, E11: `[[session]]` rows with `harness`, `mode = "attach"`, `endpoint`, `session_id`; the pane-restart script greps `session_id = "<id>"` in it, E10). It is read through `holler_body::config::load`, never through a new parser. Every row's `session_id` is a recorded session; a row whose `harness` is not `opencode` marks a pane that is not an OpenCode pane.
  - `owned-sessions`: one session id per line; blank lines and lines starting with `#` are skipped.
- **The launcher's table** is the file `--launcher FILE`, default `<state-root>/launcher.toml`. Shape (the story's contract; ADR-0021 section 13 names only what it maps to, E9): one table per pane, keyed by the pane's name, with `port` (integer 1 to 65535), `cwd` (an absolute path string) and `command` (an array of strings, an argv). The port is **never derived from the name** (the pane-restart script derives it as `47000 + col * 10 + row`; that is name-as-data and the import does not copy it). The real fleet's own table is read for its shape in the cutover (#654) dry run before anything is written; if its shape differs, the dry run reports `source-unreadable` and the fix is a one-file export, not a code change here.

### What the engine observes (reads only)

Only through `Ports` (E3, E12, E17): `herdr.snapshot()` once, `herdr.version()` once, then per pane `host.ps(name)`, `harness.health(port)` and, only when the health call answered healthy, `harness.list_sessions(port)`. **Nothing else is called**: not `ensure_pane`, `ensure_session`, `run`, `stop_owned`, `serve`, `create_session`, `attach_tui`, `select_session`, `abort`, `send_text`, `send_keys`, `close`, `shown_session`. A `herdr.snapshot()` or `herdr.version()` error ends the verb with that error (its class, its exit code), because nothing can be joined without them; `herdr-version-unsupported` is such an error.

### The join, per pane

1. **The Herdr pane is the snapshot entry whose `label` equals the pane's name** (ADR-0021 section 10, E9, merged by #808, E1). The name's cell is never the join key and never chooses a position. Zero matches: problem `pane-unlabelled`. Two or more: problem `pane-label-ambiguous` (naming the pane ids). The record's `herdr` is that snapshot entry **with `label` set to `None`** (a label is an observation, never recorded state; launch does the same, E12).
2. **The position** is the entry's `grid`. Output prints rowcol (`r2c1`). If the directory name is `hj-` followed by text that `GridPos::parse` accepts (for example `c1r2`) and that cell differs from the observed position, the report carries the information finding `name-cell-differs` (naming both cells); a name that spells no cell produces no finding.
3. **The tmux session** exists when `host.ps(name)` answers; the error `pane-not-found` is the problem `no-tmux-session`; any other error is the warning `observe-failed`, named `host.ps` (reconcile's `<port>.<method>` form, E17).
4. **The session of record.** Let `L` be the content of `last-session`, `S` the set of `session_id`s in `sessions.toml`, `O` the lines of `owned-sessions`. The files **agree** when `L` is present and every other present file names it (`L` is in `S`, and `L` is in `O` when `owned-sessions` exists). Recency is never a tie-break. Results:
   - files name different sessions (`L` missing from a present `S` or `O`): problem `sources-disagree` (naming the files, not the ids' ownership), **no record for the pane**;
   - the files agree and a **healthy** server's `list_sessions` holds `L`: `session_of_record = L` (reconcile's trust rule, E17: only a healthy server's list counts);
   - the files agree but the healthy server's list does not hold `L`: the record is written **without** `session_of_record` and the warning `session-not-found`;
   - the health check or the list failed or the server is not healthy: the record is written without `session_of_record`, `harness.health` is `unknown`, and the warning `observe-failed` names the failed call;
   - `last-session` absent or empty: the record is written without `session_of_record`, warning `no-session-recorded`.
   A session id read from a file is never copied into `last_observed` (I6).
5. **The role** is `orchestrator` for a pane named by `--orchestrator` and `agent` for every other pane. A `--orchestrator` name that matches no pane directory is a usage error (exit 2) before anything is read. The role is never derived from a name or a missing file. A pane named by `--orchestrator` still needs its own `pane.env` (the Pane record requires a model and ceilings and the import invents no defaults): the MO pane, whose config lives in `opencode-mo/` (E10), gets a small `pane.env` from the operator before the cutover, and the dry run says so (`source-missing`, naming the file).
6. **A pane that is not an OpenCode pane** (a `sessions.toml` row whose `harness` is not `opencode`) is the information-level skip `harness-not-opencode`: no record, never blocking (`HarnessKind` is closed to `opencode`, E2).
7. **A stored command** is an argv: `command` as an array becomes `Pane.command`; `command` as a string is the warning `command-not-argv` and the pane is recorded without a command.
8. **A pane that already has a record** (`pane_store.get(name)` answers a record) is `already-imported` (information) and is skipped before any other check.

### The record the import builds (once, in the plan; the real run writes exactly these)

`name`; `generation` 0; `herdr` (item 1); `host { name: tx_launch::HOST_NAME, tmux: name, cwd: launcher cwd, herdr_api_version: Some(herdr.version()) }`; `harness { kind: opencode, port: launcher port, pid: None, health: healthy only when this run's `health` answered healthy, otherwise unknown }`; `session_of_record` (item 4); `role` (item 5); `hold: none`; `last_observed { shown: None, driven: None, at: 0 }` (0 means never observed; the first doctor pass fills it; nothing in it is read from a file); `profile: fleet`; `model`; `opencode_agent`; `env` (names); `context`; `command`; `probe` empty (E2, E12). The profile is `profile_from_panes(fleet, &records)` (E4), so `holler profile show fleet` reports every pane `matches` by construction.

### The real run's writes (the contract of `profile create --from-current`, E5)

1. **Checks first**, before any write: nothing is blocked (below), and `fleet` does not exist unless every candidate pane is already imported.
2. **Write the profile first** through `crate::profile::create::insert_profile` (the one place the create race becomes `profile-exists`, E5), with the actor constant `holler pane import`.
3. **Then each record** with `profile: fleet`, through `cas_put(record, 0)`, in name order.
4. **A failure records nothing** (I3). The undo runs newest first: delete the records this run created (plus the one whose write answered `timeout`, if it landed), then delete the profile at its stored generation, stopping at the first failed step and answering `profile-conflict` with the commands that reconcile (never `holler pane close`, which would act on the live fleet). This undo is deliberate and separate from `create`'s (E5: create detaches existing records; the import deletes the records it created).

### Exit codes and what blocks

- **Levels.** Each finding has a level: `problem` (the pane gets no record), `warning` (the record is written without the thing the warning names), `skipped` (`harness-not-opencode`) and `info` (`ignored-entry`, `already-imported`, `name-cell-differs`). **Only `problem` blocks.** Problems: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`.
- **Dry run:** exit 0 with the whole report, whatever the findings. A `usage` or an adapter error keeps its own class and code.
- **Real run:** if any pane has a problem, nothing is written and the verb answers the refusal **`import-blocked`** (exit 3; a constant in `crates/holler-pane/src/import.rs` through `RefusalCode::from_static`, E16), with the message `import blocked: N pane(s) have problems; run with --dry-run for the report` (one line, nothing on stdout in text mode; the failure envelope in JSON). If `fleet` already exists and at least one candidate pane is not yet imported, the verb answers `profile-exists` (exit 3). If every candidate pane is `already-imported` (or skipped) there is nothing to write: exit 0, the report lists `already-imported`, no profile is created, no store write is made. With no candidate pane at all: exit 0, nothing written.
- **Missing `<state-root>`** (not a directory): the `usage` error, exit 2, one line.

### The output

`--format=text|json` is the global flag (`VerbCtx.format`), one `output::emit` call (E14). The verb flattens no `ProfileOpt` (`import` is in neither `--profile` class, E9); the existing test that `holler pane import --profile demo` is a usage error stays true. The JSON `data` (documented in `PaneImport`'s doc, versioned with the envelope: adding a key later is not breaking) is:
`{"dry_run": bool, "state_root": string, "recorded": integer, "profile": Profile or null, "panes": [{"name", "action": "record"|"already-imported"|"skipped"|"blocked", "role": "agent"|"orchestrator"|null, "pos": GridPos or null, "session_of_record": string or null}], "findings": [{"pane": name or null, "level", "code", "message"}]}`. Positions go through `GridPos` serde (row first). `recorded` is the number of records written (0 in a dry run). The text form prints one line per pane (`NAME  ACTION  ROLE  POS  SESSION`, `-` for absent), then each finding as `  <level> <code>: <message>`, then (dry run) the profile as `profile show` prints a spec (each spec field through `SpecField::ALL` and `FieldValue`). Finding messages carry untrusted text (paths, ids, adapter messages) only through `findings::quoted` / `findings::embedded` (E6); a message about `pane.env` names the file, the line number and the variable, never the line.

### The finding vocabulary

A new single-source enum `ImportFinding` in `crates/holler-pane/src/import/findings.rs`, copying `FindingKind`'s pattern (E6): `code()` is the one source of each code, `Serialize` is built from it, there is an `ALL` list, and a code-table test (unique, kebab-case, `is_valid_code`) lives in `crates/holler-pane/tests/import_test.rs`. A condition that reconcile already names keeps reconcile's spelling through `FindingKind::ObserveFailed.code()` and `FindingKind::HerdrVersionUnsupported.code()`; `command-not-argv` is the closed `PaneError` code. The new codes are exactly: `pane-unlabelled`, `pane-label-ambiguous`, `sources-disagree`, `source-unreadable`, `source-missing`, `no-tmux-session`, `session-not-found`, `no-session-recorded`, `harness-not-opencode`, `name-cell-differs`, `already-imported`, `ignored-entry`. `findings.rs` and `reconcile.rs` are #647's files and are not edited.

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
5. **Disagreement.** `last-session` names A and `sessions.toml` names B: `sources-disagree`, no record, real run refused. A stale file naming A while the fake server lists only B: no record anywhere holds A (I6).
6. **Agreement but the server does not hold the session**, and a failed or unhealthy server: the record is written without `session_of_record`, the finding is `session-not-found` or `observe-failed` (naming `harness.list_sessions` or `harness.health`), and `harness.health` is `unknown`.
7. **Hand-edited and missing files.** An unparsable `sessions.toml`, a `pane.env` line that is not `[export ]NAME=value`, and a non-numeric ceiling each give `source-unreadable` naming the file and, for `pane.env`, the line number and variable but never the line; a missing `pane.env`, a missing launcher entry (port or cwd) give `source-missing`; none has a record. A missing `<state-root>` gives exit 2.
8. **Not OpenCode and command shapes.** A `harness = "claude"` row: `harness-not-opencode`, no record, the other panes still import. `command` as an array imports as `Pane.command`; as a string it gives `command-not-argv` and the pane imports without a command.
9. **Idempotence and `fleet`.** A second real run exits 0, reports `already-imported`, makes no store write. With `fleet` present and a new pane to record: `profile-exists`, exit 3, nothing written.
10. **Failure records nothing.** A fault injected on the second `cas_put` leaves the pane store empty and the profile deleted; a fault that also fails an undo step answers `profile-conflict` with the reconcile commands and never names `pane close`.
11. **Roles and flags.** An unknown `--orchestrator` name: exit 2. `--herdr-session` is required. `holler pane import --profile demo` is still a usage error. The name-cell information: `hj-c1r2` observed at row 1, column 1 gives `name-cell-differs` and the record uses row 1, column 1.
12. **Vocabulary and docs.** The code-table test passes; `cli_surface_test` and `docs_cli_test` pass with the new `cli-surface.txt` lines and the ADR 0003 row; ADR-0021 section 9's import row lists `profile-exists`, `import-blocked` and `usage`, and section 13 and section 10 say what this story decided; `CHANGELOG.md` has an `## [Unreleased]` entry linking this issue.

## Blast radius

Edit: `crates/holler-cli/src/pane/import.rs` (args, run, render, the real run's writes) and its submodules `crates/holler-cli/src/pane/import/*.rs` (for example `sources.rs` for reading and parsing; the CLI side passes `scripts/lint.sh`'s 600-line warning, and the lint fails at 900, so the split is named now); `crates/holler-pane/src/import.rs` and `crates/holler-pane/src/import/*.rs` (the pure engine, the plan, the finding enum, `IMPORT_BLOCKED`); `crates/holler-cli/tests/pane_verbs/import.rs` and `import/**`; `crates/holler-cli/tests/pane_verbs/doctor/rig.rs` (an unrecorded seed only); `crates/holler-cli/tests/pane_verbs/process/stub.rs` (delete its `STUBS` entry and the `// #650` line); `crates/holler-cli/tests/fixtures/cli-surface.txt` (its two lines); `docs/adr/ADR-0003.md` (its row); `docs/adr/ADR-0021.md` (sections 9's import row, 10 and 13 text); `crates/holler-pane/tests/import_test.rs`; `CHANGELOG.md`. `holler-pane` takes no `toml` and no `holler-body` dependency (ADR-0021 section 5): all file-format knowledge stays in `holler-cli`.

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
