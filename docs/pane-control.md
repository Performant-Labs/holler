# Pane control: the operator guide for `holler pane` and `holler profile`

This guide is for the operator and the orchestrator who run the fleet's panes with Holler (epic
[#633](https://github.com/Performant-Labs/holler/issues/633)). It says what each verb does, how to read what it prints,
and how the fleet moves from the old launcher's files to the pane registry. The contract it describes is
[ADR-0021](adr/ADR-0021.md); where this guide and the ADR differ, the ADR wins. Every list of verbs, flags, findings,
codes and columns below is taken from the code (`--help`, the clap tree and the code's own tables), and
`crates/holler-cli/tests/docs_pane_control_test.rs` fails when one of them is missing here.

## 1. The rule (I1)

ADR-0021's first invariant, in its words: **"Only pane verbs change a pane or its registration."** No other command, no
script and no hand edit changes a registry pane's processes, its session or its record.

The `holler profile` verbs do not widen the rule. They change panes only through the same transactions:
`holler profile apply NAME` creates and relaunches through the launch transaction that `holler pane launch PANE` runs,
and it adopts a pane by one compare-and-swap write of the record's `profile` field;
`holler profile create NAME --from-current` and `holler profile delete NAME --keep-panes` write only that field.

The registry is the one source of truth for what a registry pane is: its name, its place in Herdr, its harness server
and its **session of record**, the one session its TUI shows and the hub drives (I2). Herdr, tmux and the harness are
observed, never inferred from files (I6).

**The boundary.** The rule is about registry panes. A registry pane is an OpenCode pane with a record: the record's
harness kind is a closed set with one member, `opencode`. A pane that does not run OpenCode (in the fleet, a pane that
runs Claude Code) has no record and can have none, so it is outside the rule: no `holler pane` verb restarts it, and
for a Claude Code pane `scripts/pane-restart` is still the way to restart it, now and after the cutover's sign-off.
The import (`pane import`) creates no record for a pane whose files name another harness (the finding `harness-not-opencode`,
section 10). A Herdr pane that no record names is **unmanaged**, a state that is observed and never stored:
`holler pane list` over the whole registry shows it in a table of its own (section 4), and `holler pane get PANE` shows it
when PANE is its label or its Herdr pane id. A verb that acts on a pane (`relaunch`, `switch`, `reset`, `park`, `unpark`,
`close`) refuses it with `pane-unmanaged` (exit 3), and a name that is neither a record nor an unmanaged pane is
`pane-not-found`. A whole-fleet `doctor` pass reports it as `unregistered-herdr-pane` (section 5). Sections 3, 4, 5, 10 and
11 repeat the consequence where it matters.

## 2. What a pane runs

One Herdr pane shows one tmux session, named as the pane. The harness server (OpenCode) runs **inside that tmux
session**: the host adapter starts it from the argv the harness adapter builds, and stops it with everything else the
session owns, by `TERM` to every owned process group, one shared grace (2 s by default), then `KILL` to every group that
still has a member. A launch is a transaction: it makes the Herdr pane and the tmux session, starts and checks the
server, creates the session of record over the harness API, attaches the TUI to it, observes that the TUI shows it, and
only then writes the record. A step that fails undoes what the run made and writes no record.

**The model is recorded, not applied yet.** A pane's model, effort, environment variable names and context ceilings are
written to its record (and to its profile's spec), but no port carries them to the harness server yet: the server a
launch or relaunch starts receives the agent only (below), and the model it uses is whatever its OpenCode
configuration selects. Applying them is issue [#829](https://github.com/Performant-Labs/holler/issues/829), not built. Until it lands, `--model` and `--effort` change
what the record says, not what the server runs (section 11 says what that means for a restart on another model).

**The agent.** `--agent KEY` on `launch` and `relaunch` sets the OpenCode agent the server starts with as its default
agent. A key that is empty or holds anything but ASCII letters, digits, `-` and `_` is refused with `agent-key-invalid`
before anything is done, and the key is never echoed. A key the running server cannot use is refused with the OpenCode
adapter's code `agent-unusable`: the adapter starts the server with the key as its `default_agent` and checks that the
server lists it, and the launch is rolled back (the server is stopped and no record is written). Both are **open**
codes, not codes of the closed list of section 9, and every open code is a refusal, exit 3.

**A `relaunch` meets `agent-unusable` after its old server was already stopped**, so the pane is left with no server and
its record unchanged. `holler pane relaunch PANE` without `--agent` brings it back on the recorded agent (section 11).

Without `--agent`, `relaunch` keeps the agent of its base spec: the recorded pane's, or with `--profile P` the agent of
P's spec for the pane. With `--agent` it replaces it. There is no flag that clears an agent.

The agent is printed after the harness port in the one-line text result of `launch` and `relaunch`, and as the key
`agent` of their JSON `data` (`null` for a pane with none):

```text
launched demo-alpha at r1c1 in demo: session ses_0a1b2c, harness port 8095, agent build
```

`holler pane list` has no agent column and `holler pane get PANE` has no agent line in its text. With `--format=json`,
`pane get` carries the agent as `opencode_agent` in `data.pane` (the record, when it has one) and in `data.spec` (the
profile's spec for the pane, when it has one); the `spec` line of the text form prints that spec as JSON.
`holler profile show NAME` prints the field as `opencode_agent: KEY`, or `opencode_agent: (default)` for a spec with no
agent (`none` is a valid key, so it never means "no agent"); in JSON the value is the key or `null`.

**What a verb needs.** Every `pane` and `profile` verb needs the hub: the registry lives there. With no hub running a verb
answers `unavailable` (exit 1) and its message names the missing hub; it touches no adapter. The hub is the only thing a
verb needs before it starts: Herdr, tmux and OpenCode are reached when a verb first uses them. So the read verbs
(`holler pane list`, `holler pane get PANE`, `holler pane watch`, `holler profile list`, `holler profile show NAME`) work
with no Herdr running. `pane list` over the whole registry and `pane get` of a name with no record also read Herdr, for
the unmanaged panes (section 4); without Herdr, `list` still lists every record and says on one `warning:` line that the
unmanaged panes are not listed.

The Herdr session, the tmux socket and the OpenCode binary the adapters use come from environment variables. The one to
know is `HOLLER_HERDR_SESSION`, the Herdr session the panes live in (Herdr's own `HERDR_SESSION` is not read); with it
unset, each Herdr call answers `usage` naming it. A variable that is missing or malformed never stops a verb from
starting: the port it configures answers the error at its first use. The full list, with each variable's default, is the
module doc of `crates/holler-cli/src/pane/wiring.rs`, and this guide does not copy it.

**`--herdr-session NAME` stays a required flag** of every verb that reaches Herdr: `launch` (unless `--spec-only`),
`pane import`, and `profile apply` when its plan has a create. `relaunch` has no such flag and acts in the Herdr session
its record names, which must be the adapter's session too. The flag is required even when `HOLLER_HERDR_SESSION` is set, and is not defaulted from it: the
variable configures the adapter, and the flag names the session the verb acts on. **The two must name the same
session**, and a mismatch is not answered the same way by every verb:

- `launch` answers a session other than the adapter's with `unavailable` (exit 1), naming both sessions, and writes
  no record. In `profile apply` the same answer is a create step that fails, which stops the run (exit 1, section 6).
- `pane import` looks for each pane's label only in the session the flag names, and the adapter's snapshot holds only
  its own session, so with a mismatch it finds no Herdr pane at all: **every pane it would record is `pane-unlabelled`**
  (a problem). A dry run exits 0 listing them, and a real run is refused with `import-blocked`. Read a dry run in which
  every pane is `pane-unlabelled` as a session mismatch first (section 10, step 1).

## 3. The verbs

`holler pane` has `list`, `get`, `watch`, `launch`, `relaunch`, `switch`, `reset`, `park`, `unpark`, `close`, `doctor`
and `import`. `holler profile` has `create`, `delete`, `list`, `show` and `apply`, and three verbs that are PROPOSED and
not built (`rename`, `export` and `import` under `profile`, #665), at the end of this section.

Each verb's `--help` is the full reference for its flags; this section names the flags that change what it does.
Two flag groups are shared:

- `--profile NAME` has two classes. On `launch`, `relaunch` and `close` it is **spec-editing**: the verb edits the
  profile's spec for the pane and changes the live pane in one transaction (I8), and `--spec-only` (only on these three,
  and only with `--profile`) edits the spec and changes nothing live. On every other `pane` verb it is **scoping**: the
  verb acts on that profile's panes, and a named pane outside it is `pane-not-in-profile`. `pane import` takes no
  `--profile`, and no `profile` verb takes it: the profile is the verb's own argument.
- `--format=json` (or `--json`) prints one envelope (section 9). The default is text.

**Exit codes.** Every verb ends with 0 on success, or with the class of its error's code: 2 usage, 3 refusal, 1 runtime
failure (section 9). Every verb can answer `usage`, `unavailable`, `timeout` and `store-corrupt`; the codes beyond those
are the verb's row of [ADR-0021 section 9](adr/ADR-0021.md#9-failure-taxonomy-and-the-cli-output), and each subsection
below names the main ones. The verbs that act on an existing pane (`relaunch`, `switch`, `reset`, `park`, `unpark`,
`close`) refuse an unmanaged pane, named by its label or its Herdr pane id, with the open code `pane-unmanaged` (3)
where they would otherwise say `pane-not-found` (section 1). `doctor`, `profile apply` and `pane import` have rules of
their own, given in their subsections.

### `holler pane list`

The pane registry as a table, one row per pane, sorted by name; section 4 says how to read it. `PANE` (optional) lists
one pane, `--profile NAME` the panes of one profile. It writes nothing. A scoped list reads the records only; a list of
the whole registry also reads Herdr's snapshot to show the unmanaged panes in a second table (section 4).

```bash
holler pane list
holler pane list --profile fleet --format=json
```

Ends with `profile-not-found` or `pane-not-in-profile` (3) for a bad scope.

### `holler pane get PANE`

One pane in full, one `key: value` line per field: its record, its profile's spec for it and SYNC (section 4). For a
pane with a record it reads the record only.

```bash
holler pane get demo-alpha
```

When no record answers, PANE may be the label or the Herdr pane id (such as `w1:p3`) of an unmanaged pane, and `get`
reads Herdr and shows it: `state` (always `unmanaged`), `pane-id`, `label`, `session`, `workspace`, `pos`, `project`,
`last-screen-change` (UTC) and `idle`; in JSON, the same object `pane list` prints for it. Anything else with no record
ends with `pane-not-found` (3).

### `holler pane watch`

The registry's change feed: the current state (one line per pane), then each change as it happens. `--since CURSOR`
resumes after a line already seen (pass its `cursor`), and `--until-idle` stops once nothing more is owed. In JSON mode
it prints one envelope per line (NDJSON). A store error ends it (exit 1): start it again with `--since`.

```bash
holler pane watch --until-idle
holler pane watch demo-alpha --since 42 --format=json
```

### `holler pane launch PANE`

Create a pane: its Herdr pane, its tmux session, its harness server and its session of record, then its record
(section 2). Its flags are one per field of a pane spec: `--workspace`, `--grid`, `--project`, `--model`, `--effort`,
`--role`, `--agent`, `--env` (a variable's name, never its value), `--ctx-soft`, `--ctx-hard`, `--port-policy`, the command
and the health check (section 8). Without `--profile` every position, directory, model, effort, ceiling and port-policy
flag is needed; with `--profile P` the profile's spec for the pane fills the ones not given. The one port policy is
`fixed:PORT`. `--herdr-session NAME` is required unless `--spec-only`.

```bash
holler pane launch demo-alpha --herdr-session demo --workspace demo --grid r1c1 --project /srv/demo --model anthropic/claude-sonnet-5-5 --effort medium --ctx-soft 800000 --ctx-hard 900000 --port-policy fixed:8095
```

The checks come first and change nothing when one fails: a name that has a record (`pane-exists`), a failing health
check (`probe-failed`), an unsupported Herdr (`herdr-version-unsupported`), an occupied cell (`grid-occupied`), a port a
server already answers on (`port-in-use`). Then `agent-key-invalid` and `agent-unusable` (section 2), the grid codes
(section 7) and the codes of `--profile` (`profile-not-found`, `pane-in-other-profile`, `profile-conflict`). Once the
live change has started, a failure's message ends with the command that reconciles.

### `holler pane relaunch PANE`

Launch a pane again, replacing its processes. It stops only what the pane owns, fails loudly when the old server still
answers, then brings the pane up as `launch` does, and keeps the session of record when the restarted server still has
it (a new one otherwise). A flag not given keeps the record's value, or with `--profile P` the value of P's spec. The
pane keeps its directory, and moves to another cell only with `--grid`.

```bash
holler pane relaunch demo-alpha --model anthropic/claude-opus-5-5 --effort high
```

A new `--model` or `--effort` is recorded, not yet applied to the server (#829, section 2). Its codes are `launch`'s,
and `pane-not-found` and `pane-unmanaged` (3).

### `holler pane switch PANE SESSION`

Point a pane at an existing session of its own server: its TUI shows it, and the record names it as the session of
record. It never types into the TUI (I4). `--as-operator` allows the orchestrator's own pane.

```bash
holler pane switch demo-alpha ses_0a1b2c
```

Refused (3) with `session-not-found` (the pane's server does not have it), `session-of-other-pane`, `orchestrator-pane`
(without `--as-operator`) or `server-unhealthy` (relaunch it first). A TUI that does not show the session afterwards is
`unavailable` (1) and records nothing.

### `holler pane reset PANE`

Start a pane on a fresh session: create it, show it, record it. `--first TEXT` queues a first message to the new session
once it is recorded. The previous session stays on the server, and `doctor` then reports it as a `stray-session`.

```bash
holler pane reset demo-alpha --first "Read the brief and start."
```

Refused (3) with `session-busy` or `session-holds-question` while the session of record is running a turn or waits on
an answer, and with `orchestrator-pane` and `server-unhealthy` as `switch` is.

**Do not use `--first` on a parked or held pane.** The first message goes to the harness directly, outside the hub's
prompt path, so the gate that refuses a prompt to a parked or held pane does not see it (ADR-0021 section 8, "The prompt
and activity seam"). Sending it through that gate is issue [#839](https://github.com/Performant-Labs/holler/issues/839),
not built.

### `holler pane park --reason TEXT --release-when WHEN`

Take a pane out of service until it is unparked: the record's hold becomes parked, with the reason and the release
condition (one line each, at most 200 characters). It takes one pane, or with `--profile` and no pane every pane of the
profile. It changes the record only: nothing moves in Herdr and no process is touched. A prompt
through the hub to a parked pane's session of record is refused (`pane-parked`); the first message of `reset --first` is
not (see `reset`).

```bash
holler pane park demo-alpha --reason "waiting for the review" --release-when "the review merges"
```

### `holler pane unpark`

Return a parked pane to service. Like `park`, it takes a pane, or with `--profile` every pane of the profile, and leaves
a pane that is not parked as it is (exit 0).

```bash
holler pane unpark demo-alpha
```

### `holler pane close`

Close a pane: stop its processes (its harness server among them), close its Herdr pane and remove its record. With
`--profile P` it also removes P's spec for the pane; `--spec-only` removes the spec and touches nothing live.

```bash
holler pane close demo-alpha
```

Ends with `pane-not-found` or `pane-unmanaged` (3), or with the error of the step it reached, followed by the command
that reconciles.

### `holler pane doctor`

Run one reconcile pass and print what it found; `--fix` repairs one kind, and `--unmanaged-stalled-after SECONDS`
reports the unmanaged panes whose screen has not changed for that long (section 5). Section 5 is its reference. A pass
that completes exits 0, whatever it found.

```bash
holler pane doctor
holler pane doctor demo-alpha --fix
holler pane doctor --unmanaged-stalled-after 1800
```

### `holler pane import --from STATE_ROOT --herdr-session NAME`

Adopt the panes the old launcher runs into the registry, as the profile `fleet` (section 10). Its full synopsis is
`holler pane import --from STATE_ROOT --herdr-session NAME [--launcher FILE] [--orchestrator PANE] [--dry-run]`:
`--launcher FILE` names the launcher table (default `STATE_ROOT/launcher.toml`), `--orchestrator PANE` (repeatable)
names the orchestrator's pane, since a role is never read from a name, and `--dry-run` reports and writes nothing.
`--herdr-session` is declared optional and is required at run time (`usage` without it). The import adopts what is
running: it never starts, stops, types into or moves anything, and deletes nothing from the source files.

```bash
holler pane import --from /srv/fleet-state --herdr-session demo --orchestrator demo-mo --dry-run
```

It reports what it found as findings, one line each, each with a level: `problem` (the pane gets no record, and a real
run is refused), `warning` (the pane is recorded without what the finding names), `skipped` or `info`. The findings are:

| Finding | Level | What it means |
|---|---|---|
| `already-imported` | info | The pane has a record; it is skipped before any other check. |
| `harness-not-opencode` | skipped | Its `sessions.toml` names a harness that is not OpenCode: no record, never blocking. |
| `source-unreadable` | problem | A file could not be read or parsed; the message names the file and a position only. |
| `source-missing` | problem | A file, a required variable or a launcher-table entry (or its `port` or `cwd`) is missing. |
| `pane-unlabelled` | problem | No Herdr pane in the named Herdr session carries the pane's name as its label. |
| `pane-label-ambiguous` | problem | Two or more Herdr panes there carry that label. |
| `name-cell-differs` | info | The name spells a cell other than where Herdr shows the pane; the record takes Herdr's. |
| `tmux-session-missing` | problem | The pane's tmux session does not exist. |
| `observe-failed` | warning | A port call failed, so nothing was concluded from it. |
| `server-wedged` | warning | The pane's server accepts a connection and never answers. |
| `server-down` | warning | The pane's server fails its health check. |
| `sources-disagree` | problem | The files name different sessions for the pane. |
| `no-session-recorded` | warning | `last-session` is absent or empty: no session of record. |
| `session-not-found` | warning | The files agree on a session the pane's healthy server does not have. |
| `command-not-argv` | warning | The launcher table gives the command as a string: the pane is recorded without one. |
| `ignored-entry` | info | A directory of the state root that holds none of the pane files. |

Six of these codes are also spelled as something else: `session-not-found` and `command-not-argv` are closed error codes
(section 9), and `tmux-session-missing`, `observe-failed`, `server-wedged` and `server-down` are doctor kinds (section 5);
in the import's report each is a finding, never the error of the verb. `herdr-version-unsupported` is the one code that
is not a finding of the import: it is the error that ends the run, with no report.

Its exit codes follow its own row of ADR-0021 section 9. A dry run exits 0 with the whole report, whatever it found. A
real run exits 0 when it wrote the plan or had nothing to write; 3 with `import-blocked` (an open code: a pane has a
problem, and nothing is written) or `profile-exists` (section 10); 2 with `usage`; otherwise by the class of the error
that stopped it.

### `holler profile create NAME`

Create a profile: empty, a detached copy of another with `--from PROFILE` (no pane joins), or with `--from-current` a
snapshot of every pane in the registry, each of which becomes a member. A pane that belongs to another profile refuses
the whole create (`pane-in-other-profile`).

```bash
holler profile create fleet --from-current
```

Ends with `profile-exists` (3), or `profile-not-found` (3) for `--from`.

### `holler profile delete NAME`

Delete a profile. Refused with `profile-has-live-panes` (3) while any pane belongs to it; with `--keep-panes` each member
is detached first (it belongs to no profile and keeps running).

```bash
holler profile delete fleet --keep-panes
```

### `holler profile list`

One line per profile: its name, its slug, its number of specs, its live (member) panes and its generation.

```bash
holler profile list
```

### `holler profile show NAME`

A profile's specs, its live panes and, for each pane, every field that differs from its spec. It reads only; the last
probe result it prints is read, never run.

```bash
holler profile show fleet
```

Ends with `profile-not-found` (3).

### `holler profile apply NAME`

Converge the live panes on a profile: create, adopt or relaunch, and never kill a healthy pane. Its flags are
`--dry-run` (print the plan, change nothing), `--take-over` (move a pane that belongs to another profile) and
`--herdr-session NAME` (needed when the plan creates a pane). The full reference is in section 6, which ADR-0021
section 8 ("Apply as built") and its section 9 apply row state as the contract.

```bash
holler profile apply fleet --dry-run
holler profile apply fleet --herdr-session demo
```

Each step is `{pane, action, outcome, code, diff, detail}`. The actions are `create`, `adopt`, `report`, `relaunch`,
`refuse` and `observe`; the outcomes are `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`, `skipped`
(after a failed step) and `planned` (a dry run's step that would act). A `refused` or `reported` step does not stop the
run or change the exit code; a `failed` step stops the run and exits 1. Its own open codes are
`orchestrator-not-relaunched` and `spec-not-launchable`. It exits 2 (`usage`) when the plan has a create and no
`--herdr-session`, 3 for `profile-not-found`, 1 for a failed step, and 0 otherwise.

### The PROPOSED profile verbs

`holler profile rename`, `holler profile export` and `holler profile import` are PROPOSED (#665) and not built: each
answers `not-implemented`.
- `holler profile rename` (PROPOSED) would change a profile's name and every member's `profile` in one transaction.
- `holler profile export` (PROPOSED) would write a profile's specs to a file, named by a planned output flag.
- `holler profile import` (PROPOSED) would read a profile's specs from a file, refusing an env value and an existing name.

## 4. Reading the table

`holler pane list` prints one row per pane with these columns:

| Column | What it holds |
|---|---|
| PANE | The pane's name, which is also its tmux session's name. |
| POS | Its grid cell, row first: `r2c1` is row 2, column 1 (section 7). |
| PROFILE | The profile it belongs to. |
| PROJECT | The directory it works in. |
| HEALTH | `healthy`, `unhealthy` or `unknown`, as the harness server last reported it (`pane get` shows the reason). |
| SHOWN | The session the pane's TUI showed when reconcile last observed it. |
| DRIVEN | The session the hub drives, printed as the record holds it and never compared. |
| SYNC | SHOWN against the session of record: `ok`, `MISMATCH` (loud on purpose) or `-` when nothing was observed. |
| HOLD | `none`, `parked` or `drained`. |

Every value of this table is the registry's. HEALTH and SHOWN are what reconcile (`pane doctor`) last recorded, and
`list` observes no registry pane itself: run `holler pane doctor` to bring them up to date. An empty cell is `-`. A
stored value that holds a space, a quote or a character a terminal could act on is printed quoted and escaped.

**Unmanaged panes.** A list of the whole registry (no PANE and no `--profile`) also shows every pane of Herdr's
snapshot that no record names, today a Claude Code pane set up by hand (section 1). They follow the registry's table
after a blank line, as a second table with the columns PANE-ID, LABEL, POS, PROJECT, STATE and IDLE:

- STATE is always `unmanaged`. It is observed on every run and never stored, and it is not a hold.
- IDLE is how long the pane's screen has not changed, and `-` when Herdr does not say. **Herdr 0.8.2 reports no time of
  the last screen change, so IDLE is `-` for every pane against real Herdr** until a source exists (ADR-0021 section 14).
- In JSON, `data` gains `unmanaged`, an array left out when there is none, of objects
  `{"state": "unmanaged", "pane_id", "session", "workspace", "pos", "label", "cwd", "last_screen_change", "idle_seconds"}`,
  with `null` for what Herdr did not report.
- When Herdr (or the pane store) cannot be read, the registry's rows are still listed: text prints one `warning:` line on
  stderr, and JSON carries `unmanaged_unavailable` with the failed call's code.
- The snapshot lists only the panes in a workspace's grid tab, so an unmanaged pane in another tab is not shown.

**SHOWN against DRIVEN.** The session of record is the session the hub drives (I2), so SYNC compares SHOWN with it. It
is `MISMATCH` when the TUI showed another session or its home screen, and `-` when reconcile has never observed the pane
or the pane has no session of record. DRIVEN is printed as the record holds it and is not compared: nothing records it
yet (the hub wiring, #649, leaves it as stored; making it follow each pane's session of record is
[#839](https://github.com/Performant-Labs/holler/issues/839), not built), so it prints `-`. `pane doctor` uses the same rule for `shown-driven-mismatch` (section 5).

A `holler pane watch` line prints the same cells as `key=value`, the column name in lower case, after the pane's name:

```text
cursor=4 put demo-alpha pos=r1c1 profile=fleet project=/srv/demo health=healthy shown=ses_0a1b2c driven=- sync=ok hold=none
```

**The roster.** `holler roster` appends six columns after HOLD: PANE (the pane's name), POS (`r2c1`, row first; a pane
named `hj-c1r2` is a name, and it sits at `r2c1`), PROFILE, HEALTH, SHOWN and DRIVEN. They are the registry's values,
the same cells `pane list` prints for the same record, and an empty cell is `-`; a session with no pane (the io body, an
alpha or beta agent) shows `-` in all six. A roster row is joined to a pane when the part of its name before the first
`/` is the pane's name. The roster calls no adapter and shows no unmanaged pane (`pane list` does), and it reads the
registry of the local hub, so with `--server` the six columns are `-`. When at least one pane's SHOWN and DRIVEN are
both known and differ, the first line of the output says so:
`2 pane(s) show a different session than the hub drives: demo-alpha, demo-beta`.
`holler roster --profile fleet` lists only that profile's panes, and a profile that does not exist is refused
`profile-not-found` (exit 3). `holler roster --format=json` prints the envelope of section 9 with `data` =
`{"rows": [...], "mismatches": ["demo-alpha", "demo-beta"]}`; `holler roster --json` keeps its shape and gains the keys
`pane`, `pos`, `profile`, `health`, `shown` and `driven` (`null` for a row with no pane). DRIVEN is `-` for every pane
until the hub's DRIVEN session is recorded ([#839](https://github.com/Performant-Labs/holler/issues/839)), so the
summary line does not appear before then; read SYNC from `holler pane list` and `holler pane doctor`.

The Herdr display plugin (`plugins/herdr-holler`, #651) shows each pane's project, SHOWN against DRIVEN and hold in
Herdr's sidebar; it only reports metadata and reads, and its actions run the `holler pane` verbs.

## 5. doctor

`holler pane doctor` runs one reconcile pass over the panes in scope (one pane, a profile's panes, or every pane) and
reports what differs between the records and what Herdr, tmux and the harness show. A pass first prints Herdr's version
and each host's recorded Herdr API version, then one line per finding, `<pane> <pos> <kind>: <message>`, followed by what
`--fix` did or could do in brackets and the command to run, then a summary. JSON mode carries the same as `data`.

The finding kinds:

| Kind | What it means | The command it names |
|---|---|---|
| `herdr-version-unsupported` | Herdr reports an API version the adapter does not know; no Holler verb upgrades Herdr. | none |
| `observe-failed` | A port call of the pass failed, so nothing was concluded from it; the message names the call. | `holler pane doctor` again |
| `unregistered-herdr-pane` | A Herdr pane that no record names. Whole-fleet passes only. | none |
| `herdr-pane-missing` | Herdr has no pane where the record says the pane is. | `holler pane relaunch PANE` |
| `tmux-session-missing` | The pane's tmux session does not exist. | `holler pane relaunch PANE` |
| `server-wedged` | The pane's harness server accepts a connection and never answers. | `holler pane relaunch PANE` |
| `server-down` | The pane's harness server fails its health check, and is not wedged. | `holler pane relaunch PANE` |
| `no-session-of-record` | The record names no session of record; doctor never picks one. | `holler pane reset PANE` |
| `session-of-record-missing` | The pane's server answered, and it does not have the session of record. | `holler pane reset PANE` |
| `tui-foreign-session` | The TUI shows a session its pane's own server does not have. | `holler pane relaunch PANE` |
| `shown-driven-mismatch` | The TUI shows another session than the session of record, or its home screen. | `holler pane doctor PANE --fix` |
| `stray-session` | A session on a pane's server that is the session of record of no pane. | none |
| `unmanaged-stalled` | An unmanaged pane whose screen has not changed for the threshold or longer. Whole-fleet passes with `--unmanaged-stalled-after SECONDS` only. | none |

**`--fix` repairs only `shown-driven-mismatch`.** It selects the session of record in the pane's TUI, so the TUI shows
the session the record already names, and observes the TUI again. It never changes which session is the session of
record, never writes the record's session, and never creates, starts, stops or types into anything. It leaves the
orchestrator's pane alone unless the run names that pane (the bracket then says `skipped`).

`--fix` does not repair any other kind: each of those names the command to run instead, and four name none. A
`shown-driven-mismatch` is also `not-fixable` when its server is not healthy or does not have the session of record; its
line then carries no bracket, and the command it names is `holler pane relaunch PANE`. The bracket is `fixable` (a run
without `--fix` would repair it), `skipped`, `fixed` or `fix failed: CODE`.

**Exit codes.** A pass that completes exits 0, with or without findings: findings are kinds in the report, not errors.
A script reads `data.findings`. The verb fails only when it cannot run the pass: `usage` (2) for a bad pane or profile
name, a refusal (3) for a pane or profile that does not exist or a pane outside the profile, a failure (1) for a store
that cannot be read.

**The periodic pass.** There is no hub timer. A scheduler runs `holler pane doctor --format=json`, without `--fix`, as
often as wanted; each run records what it observed. A pass keeps no memory between runs, so every tick reports every
standing finding again.

**`herdr-version-unsupported` has two roles.** It is a finding in a pass, which still exits 0, and it is also a closed
error code (a refusal, exit 3, section 9) that `pane launch`, `pane relaunch` and `profile apply` end with, and that
`pane import` ends with (Herdr's version is read first, and the import stops with no report; it is never a finding of
the import). Section 8 has the Herdr version.

**Panes outside the registry.** Each Herdr pane that no record names (a Claude Code pane, a plain shell) is reported as
`unregistered-herdr-pane` in every whole-fleet pass, and no Holler verb adopts it. That is expected and is not a
failure. A pass is whole-fleet only when it names neither a pane nor a profile, so a pass that names one does not report
it, and the finding names no command. `holler pane list` shows the same panes as `unmanaged` (section 4).

**`unmanaged-stalled`** is that pane seen another way: with `--unmanaged-stalled-after SECONDS`, a whole-fleet pass
reports each unmanaged pane whose screen has not changed for at least SECONDS, naming its pane id, label, idle time and
the threshold. There is no default threshold: without the flag nothing is reported. A pane whose idle time is unknown
is never stalled, and Herdr 0.8.2 reports no time of the last screen change, so **this kind cannot fire against real
Herdr yet** (ADR-0021 section 14).

`profile-drift` (a live pane that differs from its spec) is PROPOSED (#665): it is a code of the closed list and not yet a doctor kind.

## 6. Profiles

A profile is a named set of pane specs, kept in the registry beside the panes. A pane belongs to at most one profile
(`pane-in-other-profile`). A profile holds names and non-secret values, never a secret: an environment variable is a
NAME only, and a spec that carries a value is refused with `profile-secret-refused` (I7). A profile's **live panes** are
the records whose `profile` names it.

- `create` makes an empty profile, a detached copy (`--from PROFILE`), or a snapshot of every pane (`--from-current`,
  each pane becomes a member).
- `delete` is refused with `profile-has-live-panes` while panes belong to it; with `--keep-panes` each member is
  detached first and keeps running.
- `list` and `show` read only. `show` prints the specs, the live panes and, per pane, every field that differs from its
  spec.
- `apply` converges the live panes on the profile, as the reference below says.
- A spec-editing verb with `--profile P` (`pane launch`, `pane relaunch`, `pane close`) edits P's spec for the pane and the
  live pane in one transaction: a failed run puts P's specs back (I8).

### The `profile apply` reference

The contract is ADR-0021 section 8, "Apply as built (#664)", with its definition of "adopt", and section 9's apply row.
**To adopt** a pane is to record that a pane which already has a record, is healthy and matches its spec belongs to P,
and to touch nothing live. Apply adopts records, never processes: a Herdr pane, a tmux session or a server that no
record names is never adopted (`pane import` is what records existing panes).

Apply reads P and the registry, then observes every recorded spec pane. **Healthy** is a live observation, never the
stored health: the harness answers its health call, Herdr's snapshot lists the record's pane, and the TUI does not show
a session other than the session of record (a shown session that cannot be read does not count against it).

One step per spec pane, in workspace, row and column order; the first matching row applies:

| Row | Situation | Step |
|---|---|---|
| a | The pane's record belongs to another profile, and no `--take-over`. | `refuse`, outcome `refused` (`pane-in-other-profile`). |
| b | No record. | `create` through the launch transaction, outcome `created`. |
| c | Healthy and matches its spec. | `adopt`, outcome `adopted`: the record's profile becomes P (with `--take-over` from another profile, leave then join, two writes). |
| d | Healthy and differs from its spec. | `report`, outcome `reported`, with the pane's differences; nothing live is touched. |
| e | Not healthy, role orchestrator. | `refuse`, outcome `refused` (`orchestrator-not-relaunched`); nothing is written, even under `--take-over`. |
| f | Not healthy, role agent. | `relaunch`, outcome `relaunched`; under `--take-over` from another profile the leave and join happen first, then the relaunch. |

- **A step** is `{pane, action, outcome, code, diff, detail}`; an absent `code`, `diff` or `detail` prints `null`. The
  actions are `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name)
  and `observe` (a recorded pane whose health or snapshot call failed). The outcomes are `created`, `relaunched`,
  `adopted`, `reported`, `refused`, `failed`, `skipped` (a step after a failed one) and `planned` (a dry run's step that
  would act; row d's stays `reported`). `detail` is the step's own line, inside `data`.
- **Refusals do not stop the run.** A `refused` or `reported` step leaves the exit code alone. A refusal from the launch
  transaction is a `refused` step with that code; a `usage` answer of the launch for a step's spec is refused with the
  open code `spec-not-launchable`.
- **A failure stops it.** A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the
  run, and the later steps are `skipped`. A run that ends with a `failed` step prints **only the failure**: in JSON the
  failure envelope, whose `error.code` is the failed step's code and whose message is
  `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`; in text one `error:` line on
  stderr and nothing on stdout. So `skipped` appears only in that count. A dry run with a failed step prints no plan.
- A step refused or failed after its launch or relaunch had begun carries the command that reconciles.
- **`--dry-run`** prints the plan as the same table (for a run that does not fail), runs no probe, issues no split and
  writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [...]}`, every key always present.
- **Exit codes:** 0 for a run with no failed step, 1 for a failed step, 2 for `usage` (a plan with a create and no
  `--herdr-session`, checked before any step, in a dry run too), 3 for `profile-not-found`.
- `orchestrator-not-relaunched` and `spec-not-launchable` are open codes of apply: refusals, exit 3, not in the closed
  list of section 9.

## 7. Grid notation

Positions are 1-based and written row first. Text prints `r2c1` (row 2, column 1); JSON prints
`{"row":2,"col":1,"pos":"r2c1"}`, in that key order.

Input accepts `r<row>c<col>`, `c<col>r<row>` (labelled, either order) and a bare `<row>,<col>`, which is always row
first, never column first. Upper case and surrounding whitespace are accepted. Two refusals: `grid-ambiguous` for text
that cannot be read one way (interior whitespace, a missing half, a repeated label, unlabelled digits, anything
unreadable), and `grid-out-of-range` for a zero, a number above 65535, or a cell outside the workspace.

**Pane names such as `hj-c1r2` predate this standard and list the column first, so `hj-c1r2` sits at `r2c1`.** No pane is
renamed, and a name is never read as a position: a pane's position comes from Herdr's snapshot. The name itself is
refused as a position, because it holds no comma and a labelled cell must start with `r` or `c`.

Each line below is a form and what `GridPos::parse` does with it (the guide's test checks every line):

```text
ok r2c1
ok c1r2
ok 2,1
ok R2C1
refused grid-ambiguous hj-c1r2
refused grid-ambiguous r2
refused grid-ambiguous 21
refused grid-ambiguous r2 c1
refused grid-out-of-range r0c1
refused grid-out-of-range 0,1
```

## 8. Probe, argv, layout, Herdr version

**The health probe.** A pane spec can carry a health check, an argv run before a launch or relaunch acts, and strings its
output must contain. A check that fails refuses the run with `probe-failed` (exit 3) and changes nothing. For example,
`check ["curl","-s","http://127.0.0.1:8095/v1/models"]` with `expect ["qwen38"]`:

```bash
holler pane relaunch demo-alpha --check-arg curl --check-arg=-s --check-arg http://127.0.0.1:8095/v1/models --expect qwen38
```

**The argv rule.** A command and a health check are each an argv, given as repeated `--command-arg ARG` or one
`--command-json JSON` array, and `--check-arg ARG` or `--check-json JSON`; never both forms, and never a shell string.
A stored command that is a string is `command-not-argv`. A value that starts with `-` goes in the `=` form,
`--check-arg=-s`, so it is not read as a flag.

**Layout.** Herdr has no grid of its own: a workspace's rows and columns are the Herdr adapter's configuration, and a
position is reached by right and down splits. A cell outside the workspace is `grid-out-of-range`; a cell inside it that
no single split reaches is the open code `grid-unreachable`. No verb relocates a healthy pane: `relaunch` moves a pane only
with `--grid`, and `launch` refuses an occupied cell (`grid-occupied`).

**The Herdr version.** The Herdr adapter checks Herdr's API version when it connects and on every version call. An
unknown version is `herdr-version-unsupported`, whose message names the supported ones; no Holler verb upgrades Herdr.
`launch` and `relaunch` record the version they saw on the pane's record, and `holler pane doctor` prints Herdr's version
and each host's recorded one, and reports an unknown version as a finding (section 5).

## 9. JSON output reference

`--format=json` (or `--json`, the same under `pane` and `profile`; `--json --format=text` is a usage error) prints
exactly one envelope per result on stdout, and every diagnostic goes to stderr. `schema_version` is 1:

```json
{"schema_version":1,"ok":true,"data":{"panes":[]},"error":null}
```

```json
{"schema_version":1,"ok":false,"data":null,"error":{"code":"pane-not-found","message":"pane not found: demo-alpha"}}
```

`error` is `null` exactly when `ok` is true, and `data` is `null` exactly when it is false; the error has a `code` and a
one-line `message`, and nothing else. Each verb supplies only its `data`; its `--help` describes it.
`holler pane watch` is the one exception: in JSON mode it prints a stream, one envelope per line (NDJSON), resumable with
`--since`. Adding a field does not change `schema_version`; removing, renaming or retyping one does, and is a breaking
change.

**The exit codes**, the same in text and JSON mode:

| Exit | Meaning |
|---|---|
| 0 | ok |
| 1 | runtime failure: something went wrong while doing the work |
| 2 | usage: the request is malformed |
| 3 | refusal: the request was understood and declined, working as designed |

**Every closed error code**, with its class and exit code as `holler_pane::error::class_of` gives them:

| Code | Class (exit) | Meaning |
|---|---|---|
| `not-implemented` | failure (1) | The verb cannot do the work yet; every stub exits 1. |
| `usage` | usage (2) | A missing or conflicting argument, a bad name. |
| `grid-ambiguous` | refusal (3) | A position that cannot be read one way (section 7). |
| `grid-out-of-range` | refusal (3) | A row or column outside its bounds or the workspace (section 7). |
| `command-not-argv` | refusal (3) | A command given as a shell string, not an argv. |
| `env-name-invalid` | refusal (3) | A blank or whitespace environment variable name. |
| `generation-conflict` | failure (1) | Another writer changed the record first; running the verb again can succeed. |
| `probe-failed` | refusal (3) | The health check declined the launch or relaunch. |
| `profile-conflict` | failure (1) | The profile moved during the live change; the message names the command that reconciles. |
| `profile-not-found` | refusal (3) | The named profile does not exist. |
| `profile-exists` | refusal (3) | The name is taken. |
| `profile-has-live-panes` | refusal (3) | Delete is declined while panes belong to the profile. |
| `pane-not-in-profile` | refusal (3) | The pane is outside the `--profile` scope. |
| `pane-in-other-profile` | refusal (3) | A pane belongs to at most one profile. |
| `profile-secret-refused` | refusal (3) | A profile holds names, never values (I7). |
| `herdr-version-unsupported` | refusal (3) | Herdr's API version is unknown; also a doctor finding (section 5). |
| `timeout` | failure (1) | A bound ran out while doing the work. |
| `pane-not-found` | refusal (3) | The named pane has no record. |
| `session-not-found` | refusal (3) | The named harness session does not exist. |
| `store-corrupt` | failure (1) | Stored state cannot be read back; the store fails closed. |
| `unavailable` | failure (1) | The hub, Herdr or a harness cannot be reached, or the live state is not there after the act. |
| `profile-drift` | failure (1) | PROPOSED (#665): a doctor finding listed in the closed set; never an error today. |

**Open codes.** The list above is the closed set. A verb or an adapter owns further codes, which are not in that list and
are always a refusal, exit 3: among them `agent-unusable` and `agent-key-invalid` (section 2), `orchestrator-not-relaunched`
and `spec-not-launchable` of `profile apply` (section 6), `import-blocked` of `pane import` (section 3), and `pane-exists`,
`grid-occupied`, `port-in-use`, `grid-unreachable`, `orchestrator-pane`, `server-unhealthy`, `session-of-other-pane`,
`session-busy`, `session-holds-question`, `pane-parked` and `pane-unmanaged` (section 1). ADR-0021 section 9 lists each
verb's.

## 10. Migration and cutover

The cutover (issue [#654](https://github.com/Performant-Labs/holler/issues/654), operator-run) moves the fleet from the
old launcher's files to the registry. This is its procedure; where it differs from #654's body (SYNC is read from
`pane list` and `doctor`, not from the roster, and the first pane is watched through a full overnight, not a day), this
guide is the standing text.

Before step 1, confirm the binary: `holler --version` prints the version and then the commit the binary was built from,
such as `0.4.0 (bda1069)` (issue [#851](https://github.com/Performant-Labs/holler/issues/851)). It must name the commit
of `main` the cutover runs, without `-dirty`; a version number alone does not tell two builds apart.

One fact about the live fleet comes first; it is a fact, not a step:

- **Some Claude Code panes look like OpenCode panes to the import.** Their old files say `harness = "opencode"` and
  `mode = "attach"` in `sessions.toml`, and their `pane.env` holds an `anthropic/...` model. The import reads only those
  files and what it observes, and `harness-not-opencode` fires only for a pane whose `sessions.toml` names another
  harness, so the import would adopt such a pane as an OpenCode pane, with a record. No mechanism excludes a pane from
  the import yet: leaving such panes alone is issue [#835](https://github.com/Performant-Labs/holler/issues/835), not
  built.

The steps, in order:

1. **Have the launcher table, then run the dry run.** The import reads `STATE_ROOT/launcher.toml`, or the file
   `--launcher FILE` names: one TOML table per pane, named as the pane, with `port` (1 to 65535), `cwd` (an absolute
   path) and `command` (an array of strings, an argv; a string is the finding `command-not-argv`). No other key is
   accepted. No file the old launcher wrote has this shape, so the operator provides it:

   ```toml
   [demo-alpha]
   port = 8095
   cwd = "/srv/demo"
   command = ["opencode"]
   ```

   **Without it every pane is `source-missing` (a problem)**, so the dry run reports it for every pane and a real run is
   refused with `import-blocked`. If the table has another shape, the dry run says so: `source-unreadable` for every
   pane. Then run the dry run on the live fleet and read every finding:

   ```bash
   holler pane import --from /srv/fleet-state --herdr-session demo --orchestrator demo-mo --dry-run
   ```

   Name the orchestrator (MO) pane with `--orchestrator PANE`; it needs its own `pane.env`, which the operator writes
   before the cutover (the dry run reports a missing one as `source-missing`). Set `HOLLER_HERDR_SESSION` and pass the
   same name as `--herdr-session`: the flag stays required (section 2). **If the two differ, the import finds no Herdr
   pane and every pane is `pane-unlabelled`**: a dry run in which every pane is `pane-unlabelled` is a session mismatch
   until shown otherwise, and a real run would be refused with `import-blocked`.
2. **Fix drift with the old tools.** Files that disagree about a pane's session give the problem `sources-disagree` and
   no record for that pane, and a real run is refused (`import-blocked`) until they agree. Each other problem of the
   table in section 3 is fixed the same way, then the dry run is run again.
3. **Import:** the same command without `--dry-run`. A real run writes one record per pane and the profile `fleet` (from
   today's panes and their layout), or nothing at all. The layout is each pane's position as Herdr's snapshot shows it
   (the entry whose label is the pane's name), never a position read from a name. A session id read from a file becomes
   the session of record only when the files agree and the pane's healthy server lists it: the import reads the old
   files as the source of the records it creates, never as observed state (I6). A real run is refused with
   `import-blocked` (exit 3, nothing written) while any pane has a problem.
   **A second real run changes nothing and says why.** A pane that has a record is `already-imported` and is skipped
   before any other check; when every pane is `already-imported` the run has nothing to write and exits 0, without
   reaching the check of the profile `fleet`. `profile-exists` (exit 3, nothing written) ends a real run only when there
   is a pane to record while the profile `fleet` already exists, that is, when a new pane directory has appeared since
   the first import.
4. **Bring SHOWN and the session of record together.** Run `holler pane doctor` until it reports no
   `shown-driven-mismatch`, and `holler pane list` shows SYNC `ok` for every pane (section 4).
5. **Cut one pane over** to `holler pane launch PANE` and `holler pane reset PANE`, watch it **through a full overnight**,
   then move the rest. A launch records the pane's model and does not apply it yet (#829, section 2), and
   `reset --first` is not used on a parked or held pane (section 3).
6. **Seven days with no invisible session.** Every day `doctor` reports no `shown-driven-mismatch` and no stray session
   older than an hour. A pass keeps no memory, so read "older than an hour" by comparing the `session` of each
   `stray-session` finding between two passes an hour apart. Then the operator signs off.
7. **Record each step** in `docs/handoffs/<#654>-cutover/decisions.md`.

**The files the registry replaces.** The old launcher wrote, per pane, `sessions.toml`, `last-session`, `owned-sessions`
and `pane.env`; the launcher table of step 1 is the import's own input beside them. They are **replaced by registry
state** and are no longer read after the cutover. The table of what replaces each is
[ADR-0021 section 13](adr/ADR-0021.md#13-migration-from-the-launchers-files); this guide does not restate it. Retiring
the launcher's writers is pfleet's follow-up (#653, in another repository). The files are kept, read-only, for one
release, with one exception: `scripts/pane-restart` still writes `pane.env` when it is used as a rollback (section 11).

A pane whose files name another harness gets no record from the import (`harness-not-opencode`; a Claude Code pane whose
files name `opencode` is the case of the fact above). After the import, `holler pane list` shows each Herdr pane with no
record as `unmanaged` (section 4), and a whole-fleet `doctor` reports it as `unregistered-herdr-pane` in every pass,
which is expected and is not counted against the seven days (section 5).

## 11. Rollback

**`scripts/pane-restart` stays as the fallback until the 7-day sign-off.** For an OpenCode pane, its usage is:

```bash
scripts/pane-restart PANE --model PROVIDER/ID [--effort E] [--key-file PATH]
```

- **After the sign-off** only the script's OpenCode path is retired, in a later story, not this one. Its Claude Code path
  (issue #784) stays: a Claude Code pane is not in the registry, no verb restarts it, and `scripts/pane-restart` stays
  its way. Claude Code panes are not part of the 7-day criterion.
- **What the script does to a registry pane.** It takes that pane out of the new regime: it rewrites `pane.env` and
  starts a TUI on a new session that is no pane's session of record, and the record's model and session of record go
  stale. The next `doctor` reports what it observes about the pane, by the kinds of section 5, depending on where the
  new session landed: `shown-driven-mismatch` when the TUI shows a session on the recorded server that is not the
  session of record, `tui-foreign-session` when the TUI is on another server, and `stray-session` for the new session.
  `--fix` repairs only the mismatch, by selecting the *old* session of record, which undoes the script's switch: **do not
  run `--fix` on a pane that was restarted with the script.**
- **The way back** is `holler pane relaunch PANE --model PROVIDER/ID`: it restarts the server, keeps the session of record
  when the restarted server still lists it (and creates a new one only when it does not), and records the new model.
  Then `doctor` is clean for that pane again. **Until #829 lands, the model is recorded and not applied** (section 2):
  the relaunched server runs the model its OpenCode configuration selects, so the way back returns the pane to the
  registry's control, not to the model the script set. While a pane must run a model other than that one, the script
  is the only way to put it there.
- The same command is the way back from an `agent-unusable` refusal of `holler pane relaunch PANE --agent KEY`: that
  refusal comes after the old server was stopped, so the pane has no server and its record is unchanged until
  `holler pane relaunch PANE` runs again without `--agent` (section 2).
- Using the fallback is a rollback, and is recorded as one in `docs/handoffs/<#654>-cutover/decisions.md`.

## 12. Later

Deferred, so not documented here as existing; none of them is available:

- applying a pane's model, effort and environment to its harness server (#829);
- DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
- an import that leaves the panes Holler does not drive alone (#835);
- the shared subprocess runner of the adapters and the probe (#696);
- recreating the Herdr workspace from the stored profiles (#666);
- the PROPOSED profile verbs and the PROPOSED doctor finding `profile-drift` (#665).
