# [feat(wizard): the setup wizard builds panes through holler pane and holler profile apply (retire direct Herdr control and attach mode)](https://github.com/Performant-Labs/holler/issues/803)

Part of epic #633. This is the complete statement of story #803, written as one text on 2026-10-11 (21:11 MDT) against `main` at `d6ae539` (#851 merged) and against the design of story #831 (the Claude Code harness; branch `issue-831-implementation`, at `d137a1d`: F is done and T-green passed at 21:06 MDT; no PR yet, not merged). It is a **preparation**: nothing was launched, and the story does not start until #831 has merged (the operator's decision of 2026-10-11, 20:39 MDT, E3). It supersedes the text of issue #803 where they differ (the differences are listed in "Where this brief is the decision"). The story is **a rewrite of the setup wizard skill and its helper scripts, with a new helper script, the docs and the tests**; Holler has no UI (`uiSurface: false`), and no Rust production code changes.

**What waits for #831.** Everything that names a Claude Code pane, the flag `--harness`, the port policy `none`, the Claude login rule and the Claude-specific findings rests on #831's contract as committed on its branch (R1 to R4, R12). Its S phase, its PR and its merge remain, and the text moved once already while this brief was written (the ADR's first commit named the command constant `CLAUDE_COMMAND`; the branch now has `CLAUDE_CODE_COMMAND`), so it can move again. Each such line is marked **REFRESH AFTER MERGE of #831**; "Facts that wait for #831" lists them all with the excerpt to re-cut. The build run re-cuts those excerpts from merged `main` and patches the marked lines before T writes a test. Every other line is checked against `main` at `d6ae539`. An excerpt marked `R` is another story's text; it shows what that text was when this brief was written.

**Marks.** *(issue)* is a decision of the issue text (E1, E2). *(operator)* is quoted from the issue's last comment (E3) or the epic (decision 3, not excerpted; E62). *(decided by the preparer, to be confirmed at the plan check)* is the preparer's judgment under the operator's standing rule to decide and report: the plan check (A) re-reads it against the code and either keeps it or sends it back. "Questions only the operator can answer" lists the five the preparer did not decide. Cite an entry as E<n> or R<n>; the Evidence section is verbatim and was cut by the commands shown under each heading.

## Where this brief is the decision

### The facts that force the design

- **F1. The wizard is an agent skill, and what is code is small.** `agent-skills/herdr-setup/SKILL.md` (1853 lines) is read and followed by an agent; six bash helpers sit in its `lib/`; `docs/setup-wizard.md` is its reasoning; the tests are the `wizard_scripts` target, one module per story: text assertions over the skill and docs, and runs of the helpers against fake `herdr`, `holler`, `ps` and the like first on `PATH` (E49, E51 to E56, E59). So "code under test" is the helper scripts; the rest is skill text, tested by assertions that fail without the text.
- **F2. Today's wizard drives Herdr by hand and builds a remote, attach-mode fleet.** Stage 8 splits panes with `herdr.sh run pane split`, labels them with `herdr.sh label`, and types each orchestrator's command into its pane with `pane run` and `send-keys` (E14 to E17, E22, E23). Stages 4, 5 and 9 start `opencode` over `ssh` on a *remote* host with `--hostname 0.0.0.0`, create a session by `POST /session`, and type `opencode attach` into a pane (E9 to E11, E18); Stage 7 joins and runs one body per remote host (E13); Stage 6 advertises the hub and runs `tailscale serve` for those bodies (E12).
- **F3. Every `holler pane` and `holler profile` verb needs the hub, and Herdr, tmux and OpenCode are configured from the environment.** With no hub a verb answers `unavailable`; `--herdr-session` stays a required flag of every verb that reaches Herdr, and must equal `HOLLER_HERDR_SESSION` (E32). So nothing of the registry can be read before the hub is up, and the hub (today Stage 6) must come before the Herdr server's workspace and the profile.
- **F4. A workspace's size is configuration, not discovery.** The adapter knows a workspace's rows and columns only from `HOLLER_HERDR_WORKSPACES=label=RxC`, and a workspace not listed is `unavailable` (E43, E44). The adapter makes the workspace itself (`workspace.create`; its root pane is r1c1, E39), so the wizard issues no `workspace create`, `workspace rename` or `pane split`.
- **F5. A grid of two or more rows and two or more columns cannot be created by one `profile apply`.** The adapter adds a new row only below a row of one pane ("place rows before columns", E39, E40), one pane per call; `profile apply` creates in `(workspace, row, column)` order (E41, E42). Row 1 would hold several panes before row 2's first cell is asked for, and that cell is `grid-unreachable`. This is read from the code, not run; story #667's brief reached the same reading independently (R7). The wizard's layout is column-first (E5), and a column-first order *is* reachable: column 1 top to bottom, then each next column.
- **F6. Registry panes are local to the hub.** The registry is the hub's; epic #633's decision 3 puts Herdr's server on the hub's machine (not excerpted: its sentence names the operator's machines), and the Herdr adapter talks to a local socket (E61); the harness server runs inside the pane's tmux session on that machine (E45; ADR-0021 section 11, E37). A pane cannot sit on a remote host, so the wizard's remote sessions have no place among registry panes.
- **F7. The spec's `command` is a preamble, not the pane's program.** The launch runs the spec's `command` in the pane's tmux session first, then starts the harness server from the argv the harness adapter builds (E45). What a registry pane runs is decided by its harness, so an orchestrator's free-form `cmd` (E6) has no field to go in.
- **F8. A loopback `ws://` is allowed.** `holler body join --server ws://127.0.0.1:<hub_port>` needs no `tailscale serve` and no `hub_host` (E46). With no remote host the wizard's existing "no sessions" path already starts the hub with no `--advertise` and no serve entry (E12, E8).
- **F9. An old wizard-built layout cannot be adopted.** `holler pane import` reads the *old launcher's* per-pane files and a launcher table that no wizard file provides (E63, E65); `holler profile create NAME --from-current` snapshots only panes that already have a record (E64). A layout built by the current wizard has neither: its panes are plain Herdr panes running `opencode attach`, with no record, no tmux session and no launcher files. The operator's rule that the wizard adopts what exists through `pane import` or `--from-current` therefore has nothing to act on there (D18 says what the wizard does).
- **F10. A pane name is a session name.** Lower-case letters and digits, `-` allowed between them, 1 to 32 characters (E47). The wizard's default names (`r1c1`) and the examples (`alpha`, `o1`) are valid; a name such as `Alpha` or `my_pane` is not.
- **F11. The pinned tests are keyed to stage headings and to retired phrases** (`## Stage 4 `, `herdr.sh label <pane-id> <label>`, `workspace create`, `--stage 8`, the `tailscale` and `ssh` rows, E49, E50, E51, E52, E53). Stages 0 to 3, 10 and 11 keep their numbers and headings, and the count "12 stages (Stage 0 through Stage 11)" is pinned in `skilltext_w5_rename.rs` and `docs/setup-wizard.md`; so this story re-uses the slots 4 to 9 and does not add or remove a stage.
- **F12. The wizard's rename scan fails the brief itself.** `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` scans every tracked file, `docs/handoffs/` included, for the old spelling of the skill's directory and command (E50; #848 is open). Issue #803's own line 5 uses that spelling, so E1 starts at line 7. No file this story writes may contain it.

### The decisions

**D1. The wizard stays a skill; the new code is one helper, `lib/panes.sh`, and the rest is text.** *(decided by the preparer, to be confirmed at the plan check)* `panes.sh` is a pure function of a config file and the environment, bash 3.2 compatible like its siblings (no associative arrays, no `mapfile`, POSIX awk). Its verbs: `check CONFIG` (validate the pane config, name every problem by key, exit 1), `grid CONFIG` (the cells), `env CONFIG` (the environment literals), `plan CONFIG PROFILE` (the ordered `holler` arguments of D9), `teardown CONFIG PROFILE` (D15). It calls `instance.sh` for the `[instance]` table and does not parse it again. Why a script: the cell arithmetic, the column-first order and the quoting are exactly what an agent gets wrong at 16 panes, and a script can be tested against a fixture without Herdr (the repo's pattern, E49, E54, E55). Rejected: **(a)** a Rust `holler wizard` verb (an unasked product surface, Rust production code, and the issue calls the wizard a skill); **(b)** leaving the arithmetic to the agent in prose (untestable, and the one error that matters, D9, is silent until the third row); **(c)** extending `instance.sh` to read `[[pane]]` (its 567-line test file is built on `[[session]]`, E24, and its output lines are a contract the skill quotes; `panes.sh` reads the base port from `instance.sh`'s own output instead).

**D2. The twelve stages stay; slots 4 to 9 change.** *(decided by the preparer, to be confirmed at the plan check)* Stages 0, 1, 2, 3, 10 and 11 keep their numbers, headings and names (F11). The middle becomes, in this order, which the hub-first rule of F3 forces:

| Stage | Today (E7) | After |
|---|---|---|
| 4 | Start the remote agent backends | **Bring up the hub** (Stage 6's text, with no `--advertise` and no `tailscale serve`) |
| 5 | Create a first session per backend | **Start the Herdr server** (Stage 8's server text; ledger `--role herdr --stage 5`) |
| 6 | Bring up the Holler hub | **Create the profile** (`profile create`, then one `--spec-only` launch per pane) |
| 7 | Migrate the config, join and run one body per remote host | **Build the panes** (`profile apply`, after column 1 is launched, D9) |
| 8 | Build the Herdr workspace (M+N panes) | **Join and run the body** (one body, on this machine, D13) |
| 9 | Attach each pane to its real remote session | **Check the registry** (`pane doctor`, `pane list`, `profile show`, D14) |

The ledger's `--stage` range stays 4 to 9 (`ledger.sh` is not edited): the hub records `--stage 4`, Herdr `--stage 5`, the body `--stage 8`; `stop-owned.sh teardown` stops in reverse of that order (E77), so the body stops first and the hub last, after the panes were closed through Holler (D15). Rejected: **(a)** keep every number and only replace Stage 8's commands (the hub is Stage 6, after the profile would be needed, F3; and the server start is inside Stage 8); **(b)** delete Stages 4, 5 and 9 and renumber to nine stages (the "12 stages" pin, and about a dozen tests keyed to `## Stage 10`, `## Stage 11`); **(c)** keep Stages 4, 5, 9 as "skipped" stubs (a stage that never runs is a lie in a table the user reads).

**D3. Stage 3 shows the plan from the config; the registry's dry run comes where the hub exists.** *(decided by the preparer, to be confirmed at the plan check; deviates from the issue's "with `--dry-run` shown at Stage 3")* Stage 3 is a hard stop before anything changes (E82), and `holler profile apply NAME --dry-run` needs the hub and a profile that is already in the registry (E32, E33): at Stage 3 on a fresh machine there is neither, and creating a profile is a registry write. So Stage 3 always prints the plan derived from the config by `panes.sh grid` (a table: cell, name, harness, role, directory, port; the workspace, its `RxC`, the profile name; "the model, effort and environment are recorded, not applied, until #829", E71; the pane count) and, **only on a rerun whose hub the ledger shows live and whose profile exists**, also `holler profile apply NAME --dry-run --format=json` read-only, with its steps. Stage 7 then runs the same dry run once more *before the first live change* and stops, asking, if its steps differ from the table Stage 3 showed. Rejected: **(a)** start the hub before assent (breaks "nothing changes before assent", E82); **(b)** show no dry run before Stage 7 (the user would assent to a table the registry has not checked, on every rerun); **(c)** create the profile in Stage 3 to be able to dry-run it (a state change before assent).

**D4. The config: `[[pane]]` replaces `[[orchestrator]]` and `[[session]]`, and `layout` keeps its meaning with one new rule.** *(decided by the preparer, to be confirmed at the plan check)* Each grid cell is one `[[pane]]` with `name` (F10), `dir` (absolute or `~/`), `harness` (`opencode` or `claude`, default `opencode`; **REFRESH AFTER MERGE of #831**: `claude` is accepted only once the harness exists, R2), `role` (`agent` or `orchestrator`, default `agent`), and optional `agent` (an OpenCode agent key, `--agent`), `backend_port`, `model`, `effort`, `ctx_soft`, `ctx_hard`. The top level takes the same `model` (`provider/id`), `effort`, `ctx_soft` and `ctx_hard` as defaults for every pane, because `holler pane launch` needs all four for a new spec (E69: "every position, directory, model, effort, ceiling and port-policy flag is needed"), so the config either has them at the top or on every pane; `panes.sh check` refuses a pane that ends with none. `layout` stays "outer = columns, inner = top to bottom" (E5), now over `[[pane]]` names only, and **must be a staircase: no column is taller than the column to its left** (the first column is the tallest). That is exactly the set the adapter can build, column 1 first (F5), and it admits every rectangular default grid (`r1c1`, `r2c1`, ...). It does *not* admit today's documented example `[["o1"], ["alpha", "beta"]]` (o1 alone, two stacked beside it): there `r2c1` would be empty, and the adapter will not place `r2c2` without it (E40); the same panes as `[["alpha", "beta"], ["o1"]]` are legal. `panes.sh check` refuses a taller column naming the key `layout` and the column. At least one pane must have role `orchestrator`; several are allowed (the MO pattern, E2). The Herdr workspace's rows are the first column's height and its columns are the number of columns (`RxC`, F4). The top-level `workspace` key stays optional; **with none the label is `holler`** (the label is now required by `HOLLER_HERDR_WORKSPACES` and the pane's `--workspace`, E43, so "Herdr labels the workspace itself" can no longer hold). The label must contain no `=` or `,` (E43); the existing `workspace` validation (1 to 40 characters, no such character) already guarantees it. `hub_host` is no longer used (F8): it is accepted, ignored, and `check` prints one warning line saying so. Rejected: **(a)** keep two tables (`[[orchestrator]]` with `cmd`, `[[session]]` with `endpoint` and `remote_host`): their fields have no meaning for a registry pane (F6, F7), and two tables for one thing is how the "flat namespace" collision rule came to exist; **(b)** a new row-first key replacing `layout` (every existing rectangular config would need a rewrite for no gain, and a silent transpose is worse than a refusal); **(c)** accept ragged layouts and let the adapter refuse at the third pane (the user would learn it after two servers were already running); **(d)** a profile file for `profile import` (#665 is PROPOSED and unmerged, R10; `pane launch --profile P --spec-only` exists today, E32).

**D5. An existing config is refused with a conversion, never rewritten silently.** *(decided by the preparer, to be confirmed at the plan check)* `panes.sh check` exits 1 for a file with `[[orchestrator]]`, `[[session]]`, or the keys `endpoint`, `mode`, `session_id`, `remote_host`, `remote_tailnet_host` or `cmd`, naming each key and saying the wizard no longer builds attach-mode or remote sessions. Stage 1 then offers the conversion **as text shown to the user** (the confirm step of #842 Part A applies, R8): each `[[orchestrator]]` becomes a `[[pane]]` with `role = "orchestrator"` and `harness` taken from the first word of its `cmd` (`opencode` or `claude`; any other command cannot be a registry pane: Stage 1 says so and asks what to do); each `[[session]]` is dropped only on the user's yes (it is a remote backend, F6). The old file is never overwritten: the converted one is written next to it. Rejected: **(a)** convert silently (loses remote sessions without a word); **(b)** keep reading the old tables as a legacy path (the whole of Stages 4, 5 and 9 would have to stay, and the acceptance forbids direct Herdr control in the skill).

**D6. Remote sessions leave the wizard.** *(decided by the preparer under the issue's "retire the attach mode" (E1) and epic decision 3 (E61 shows the adapter's local socket); listed in the operator's questions because it removes a capability)* The wizard's remote half is attach mode: a backend the wizard starts over `ssh` on another machine, a TUI that attaches to it, a body per remote host (E9, E13, E18). Registry panes are local (F6), and the issue retires the attach mode. So the skill loses: the manual-relay mode for a missing `ssh`; the per-host loops of Stages 2, 3 and 7; the remote inventory and ledger fetch; the remote `$HOME/.holler` comparison; token labels folding in a remote host; `tailscale serve`, `--advertise` and `serve_https_port` use (the `[instance]` key stays validated by `instance.sh`, which is not edited, and is unused); and the Stage 0 rows for `tailscale` and `ssh`. The helper scripts keep their remote-capable forms (`inventory.sh`, `collide.sh`, `stop-owned.sh`, `ledger.sh` are not edited); the skill just stops sending them to another host. A user who needs a remote, headless OpenCode session is told in Stage 1 that the wizard does not build one, and that ADR 0014's attach mode and `holler body` still exist for it (ADR-0021 section 11 says so, E37). Rejected: **(a)** keep remote sessions as a body-only path with no pane (a second wizard inside the first, with the old Stages 4, 5, 7 intact and nothing the issue's acceptance can test); **(b)** keep both in one config (two stage maps in one skill).

**D7. Hub, then Herdr, then the registry, then the body.** *(decided by the preparer, to be confirmed at the plan check)* D2's order, with: Stage 4 is the existing "no remote host" hub start (E12 minus `--advertise` and `tailscale serve`; Stage 3's `collide.sh` call leaves `WIZARD_SERVE_HTTPS_PORT` empty, as its N=0 rule says, E8). Stage 5 is `herdr.sh server-start` and the ledger record (E79), then a `holler pane doctor` run whose first line prints Herdr's version (E72): it is the one read that proves the adapter reaches *this* Herdr session (D8), and `herdr-version-unsupported` is a Stage 5 gate. Stage 6 is `profile create` and the `--spec-only` launches. Stage 7 is the build. Stage 8 is the body. Rejected: the issue's implied order (workspace before hub): a registry verb with no hub is `unavailable` (E32).

**D8. Every Holler call runs through `herdr.sh holler`, in a scrubbed environment, with the instance's literals defined once.** *(decided by the preparer, to be confirmed at the plan check)* Two facts decide it. The adapters are configured from the environment (E32, E70): `HOLLER_STATE_DIR`, `HOLLER_HERDR_SESSION`, `HOLLER_HERDR_WORKSPACES=<label>=<R>x<C>`, `HOLLER_HERDR_SOCKET`, `HOLLER_TMUX_SOCKET`. And a registry pane's processes run in a tmux server, which takes the environment of the first `holler pane` call that starts it (tmux's own rule; **UNVERIFIED here**), so a call made from the driving agent's shell would carry the agent's variables into every pane: the leak that #771 fixed for the Herdr server, whose `server-start` runs `env -i` with an allow-list (E21). So `herdr.sh` gains one verb, `holler <args...>`, which runs `holler` under that same allow-list (one list, shared with `server-start`, not copied) with the instance's `HOLLER_*` literals set from the `WIZARD_*` values the skill already writes inline: `WIZARD_STATE_DIR` becomes `HOLLER_STATE_DIR`, `WIZARD_HERDR_SESSION` becomes `HOLLER_HERDR_SESSION` (the literal `default` for the default instance, which `herdr session list` calls `default`), and three new ones, `WIZARD_HERDR_WORKSPACES`, `WIZARD_HERDR_SOCKET` and `WIZARD_TMUX_SOCKET`. It refuses to run with `WIZARD_STATE_DIR` unset, as `herdr.sh` refuses an unset instance name today (a lost value must never reach the default hub). `<herdr_env>` (Stage 1) gains those three, printed by `panes.sh env CONFIG` so the agent computes none: the extent is the first column's height by the number of columns; the socket is set for the default instance only, `$XDG_CONFIG_HOME/herdr/herdr.sock` (`~/.config` when unset), the spike's path of the default session (E48), because the adapter's own default is the named-session path `.../herdr/sessions/<name>/herdr.sock` (E70) (**UNVERIFIED against a real Herdr**); the tmux socket is `<state_dir>/tmux.sock` for a non-default instance only, so two instances' `r1c1` cannot meet on tmux's default server, and the default instance keeps tmux's default server, as the live fleet's wiring does. Every pane call in the skill is `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane ...` or `... holler profile ...`; the scoping test (E51) accepts it (a line needs `herdr.sh` or `HOLLER_STATE_DIR=`) and gains `holler pane` and `holler profile` in its word list, so a bare call fails. What else a pane needs (a provider credential) comes from the user's shell rc or the OpenCode configuration, as it does for the Herdr server today, not from the wizard. The orchestrator's `HOLLER_STATE_DIR` is no longer typed into its pane by an `export` (E17): it reaches the pane through the tmux server's environment (also UNVERIFIED), and Stage 10's fresh-session briefing check (E19) is the proof, and stays. Rejected: **(a)** bare `holler pane` calls from the agent's shell (the leak above); **(b)** a second script that copies the allow-list (two lists drift); **(c)** `--env HOLLER_STATE_DIR` in the spec (an env NAME is recorded, not applied, until #829, E71); **(d)** reading the socket from `herdr session list` (its format is itself unverified, E21); **(e)** one tmux server for every instance.

**D9. Panes are built in column-first order: all specs, then column 1 live, then `profile apply`.** *(decided by the preparer, to be confirmed at the plan check; Questions 2)* From F5: Stage 6 writes every pane's spec with `holler pane launch NAME --profile P --spec-only <all flags>` in layout order (no Herdr call; `--herdr-session` is not needed, E69); Stage 7 then (1) prints `holler profile apply P --dry-run` (D3), (2) launches each **first-column** pane that has no record, top to bottom, with `holler pane launch NAME --profile P --herdr-session S` (the profile's spec fills every other flag), and (3) runs `holler profile apply P --herdr-session S`, which now finds column 1 present, adopts it, and creates the remaining cells in row order, each a `right` split of a row that is exactly one pane wide. `panes.sh plan` prints exactly this sequence, tagged `profile-create`, `spec`, `dry-run`, `launch`, `apply`; the agent never orders anything. A rerun launches only the column-1 panes that `holler pane list --format=json` does not show. Apply is still the issue's mechanism (idempotent, adopts healthy panes, never kills one); the extra launches exist only because apply's creation order cannot build the grid by itself. If a follow-up changes apply's order or the planner (the operator files it, R7's question), the `launch` tag and Phase A disappear and this decision is REFRESHED. Rejected: **(a)** `profile apply` alone (F5: it stops at the first cell of row 2 for any grid with two rows and two columns, each later cell a step refused with `grid-unreachable`, which by the guide's rule leaves the exit code 0 (E34), so the wizard would have to notice it); **(b)** `pane launch` for every pane in column order and apply only to verify (the issue wants apply to create, and a half-built rerun would then have no convergent path); **(c)** change `tx_apply` or the planner here (Rust production code, outside this story and #667's question).

**D10. An orchestrator is a pane with `role = "orchestrator"`; its `cmd` is retired.** *(decided by the preparer, to be confirmed at the plan check; deviates from the issue's "orchestrators become profile specs (role orchestrator, `command` argv, `dir`)"; Questions 3)* F7: the spec `command` runs *before* the harness server in the pane's tmux session and does not replace it, and the harness is `opencode` or `claude`; so "any CLI agent, a plain shell script" (E6) cannot be a registry pane, and the wizard sets no spec `command`. Stage 1 asks the harness for each orchestrator, refusing any other `cmd` with the reason. `profile apply` never relaunches an orchestrator (`orchestrator-not-relaunched`, E34): Stage 7 and 9 report that refusal and ask, never work around it. The `AGENTS.md` briefing question stays in Stage 3 (and #842 changes it, R8); the *write* moves to the start of Stage 7, before the first launch, because an agent reads the file when it starts and no command is typed into a pane any more. Rejected: keep `cmd` as the spec `command` (it would run as a preamble and then the harness would start beside it, a different thing from what the user typed).

**D11. OpenCode ports are fixed per pane and come from `panes.sh`.** *(decided by the preparer, to be confirmed at the plan check)* An OpenCode pane's `--port-policy` is `fixed:PORT` (the one policy, E32), PORT its own `backend_port`, else `backend_port_base + i` over the OpenCode panes in layout order (`i` from 0; Claude panes take none). `panes.sh` reads the base from `instance.sh`'s `instance:` line, and applies `instance.sh`'s own rules to panes, since that script's rule counts `[[session]]` tables and there are none now: a non-default instance whose computed port falls in the first instance's range (47001 up) is refused unless every OpenCode pane sets its own `backend_port`; two panes on one port, or a port above 65535, are refused, each naming the key. Stage 3's `collide.sh` is the authority on a port in use (`WIZARD_BACKEND_PORTS` = these ports, the local call only, E60); `holler pane launch` itself refuses `port-in-use` (E32). **REFRESH AFTER MERGE of #831**: the Claude pane's policy is the word `none` and the flag is `--harness claude` (R3, item 12; R12 shows both as built on the branch).

**D12. Model, effort and the context ceilings are recorded, not applied; Stage 1 asks once.** *(decided by the preparer, to be confirmed at the plan check)* The launch needs them (D4) and records them; the server runs the model its OpenCode configuration selects, until #829 (E71). Stage 1 proposes, as one confirm-or-override, the `model` read from the local `~/.config/opencode/opencode.jsonc` (Stage 2's per-host model resolution, E66, moves to this machine and stays read-only), `effort = "medium"`, and the ceilings `800000` and `900000` that the guide's own launch example uses (E69); none of those values is a default of Holler, so the question is asked, never assumed. Stage 3's plan says in one line that the model is recorded and the server runs what OpenCode selects. Rejected: leave the model out (the launch refuses without it); apply it through `HOLLER_OPENCODE_SERVE_ARGS` (not the wizard's job, and #829 owns it). **REFRESH AFTER MERGE of #831**: for a Claude pane `model` and `effort` become `--model` and `--effort` of the process (R3, item 11), so the wizard may state that they are applied for Claude panes and recorded for OpenCode panes.

**D13. One local body drives the OpenCode panes; a Claude Code pane has none yet.** *(decided by the preparer, to be confirmed at the plan check)* A body is how the hub drives a session (R4, item 18; R6). Stage 8 derives one body config from the registry: for each OpenCode pane, a `[[session]]` row with `name` = the pane's name, `harness = "opencode"`, `mode = "attach"`, `endpoint = "http://127.0.0.1:<port>"`, `session_id` = the pane's session of record, both read from `holler pane get NAME --format=json` (`data.pane.harness.port` and `data.pane.session_of_record`, E78), never from the config; it drops what the body denies, exactly as the derived config does today (`[instance]`, `backend_port`, `workspace`, and now every `[[pane]]` key; E68). The config is written to `<body_config>` = `<state_dir>/herdr/setup/<prefix>-sessions.toml`, the wizard's own directory (the rule that every wizard file lives there), replacing `~/sessions.toml` and the remote path of E68. It mints one token labelled `<prefix>-local` (the label rule of E13 minus the remote host), joins with `--server ws://127.0.0.1:<hub_port>` (F8), runs one `holler body run`, and records it `--role body --stage 8`. #839's design assumes exactly this ("one attach-mode body session per registry pane whose endpoint is that pane's server", R6), and its brief names the one semantic dependency between the two stories (its clash table's row for this story, line 444 of `docs/handoffs/839-brief.md` at `bf9d1c2`, not excerpted because that line carries the old spelling F12 forbids): a body's attach `session_id` is configuration for #839, while the issue's item 3 takes the per-session `session_id` out of the wizard's config. It stays configuration where the body reads it: the master config has no `session_id`, and the *derived body config* has one, read from the registry. No body is started without one. **REFRESH AFTER MERGE of #831**: a Claude pane gets no row (no body driver until #850, R9, R4); the wizard says so in Stage 3, and Stage 10's `holler say` round trip covers OpenCode panes only. Rejected: write the `session_id` from the config (the config no longer has one, E1 item 3); one body per pane (many processes, one roster label each, for no gain).

**D14. Stage 9 reads the registry; it types and reads no pane.** *(decided by the preparer, to be confirmed at the plan check)* `holler pane doctor --format=json` (read `data.findings`, E73), `holler pane list --format=json` (every pane present, at its cell, `profile` = P, SYNC `ok`), and `holler profile show P` (no differing field). The gate is "no finding except `unregistered-herdr-pane`": that kind is expected for the Herdr server's own first workspace pane and for restored panes, and Stage 11 says how to close them. Any other finding, including `shown-driven-mismatch`, is a Gate: show it, name the command it names, ask. The old proof "your exact prompt text and its reply both appear in `pane read`" (E19) is replaced by SYNC `ok` (SHOWN equals the session of record) plus the `holler say` round trip. DRIVEN prints `-` until #839 (E80): the text says that, so `-` is not read as a failure. **REFRESH AFTER MERGE of #839** (expected to merge before this story, see "Clashes"): DRIVEN is then the session a joined body advertises, SYNC and `shown-driven-mismatch` compare it, and the sentence changes to say which. Rejected: keep `pane read` as a second proof (no Holler verb reads a pane's screen, and the issue forbids a direct Herdr pane command).

**D15. Teardown goes through Holler, then through the ledger.** *(decided by the preparer, to be confirmed at the plan check)* "Tearing the instance down" (E20) becomes: with the hub still live, `holler pane close NAME --profile P` for each pane (reverse layout order; `panes.sh teardown CONFIG PROFILE` prints the lines), then `holler profile delete P`, then `stop-owned.sh teardown <state_dir>` for the body, the Herdr server and the hub. A pane's close stops its harness server, closes its Herdr pane and removes its record (E81). If the hub is gone (a stale ledger row) the panes cannot be closed through Holler: the skill says so and asks, and does not fall back to a Herdr command. Stage 3's warning about the restored workspace stays (the server holds it and teardown stops that server). Rejected: stop the server and let the panes die (the registry would keep records of panes that no longer exist, for `doctor` to find).

**D16. `herdr.sh` keeps the server and session guards, gains `holler`, and loses every pane verb.** *(issue Work 4; the allow-list and the new verb are decided by the preparer, to be confirmed at the plan check)* It keeps `server-start` (the scrubbed environment, E21), `check-session`, `check-pane`, `session-delete`, `log-path`, and `run`, which is restricted to an allow-list of read-only words: `status`, `session list`, `workspace list`. Any other `run` word (every `pane ...`, `workspace create`, `workspace rename`, `workspace focus`, `server stop`, `session attach`) exits 2 with a message that names `holler pane` and runs no Herdr command. The `label` verb is removed (a mistaken `label` is the script's ordinary usage error). The `check_pane` guard on `pane split|run|send-keys` (E22) goes with those words; `check-pane` stays as a verb for Stage 2. The new verb `holler` is D8's. The header's verb list and the "Stage 8 and 9" comments are updated. `workspace focus` is dropped: Stage 10's closing instructions tell the user to attach and name the workspace, the same step a person takes anyway. Rejected: delete `run` (Stage 5 needs `status`, Stage 11 needs `workspace list`); leave `run` open (the acceptance, E1, scans both the skill and `herdr.sh` for a direct pane command).

**D17. The contract text changes first and is small.** *(decided by the preparer, to be confirmed at the plan check)* ADR 0014 is **not edited**: its decision stands (a body can attach to a session it does not own), and the bodies the wizard now runs are attach-mode bodies of exactly that kind (D13). What is retired is the *wizard's* use of attach mode, which no ADR describes, so the record is ADR-0021 section 11's last paragraph and `docs/setup-wizard.md` (see "Amendments"). The epic's contract block is a GitHub issue body the pipeline does not edit (R1's precedent); no contract word of #633 changes. Rejected: amend ADR 0014 (it would claim a decision that is not changing and clash with #839's edits to the same ADR, whose brief amends point 5 and the Consequences, R6).

**D18. The wizard never rebuilds or adopts a layout it did not create through Holler.** *(decided by the preparer, to be confirmed at the plan check; Questions 4)* Stage 2 reads, read-only: the ledger, `herdr.sh run session list`, and, when the instance's hub is live, `holler pane list --format=json` (which also lists unmanaged Herdr panes in its second table). Stage 3's plan then names which of four cases holds (and the standing rule of item 5), and the user's yes covers only that case:
1. **Fresh**: no live hub row and no registry panes. Build (D9).
2. **A rerun of this wizard**: a live `hub` row, and the profile `P` exists. Show `profile apply P --dry-run`; apply is idempotent (adopt healthy, relaunch unhealthy agents, report differences, never kill a healthy pane; the orchestrator is never relaunched, E34).
3. **The registry already has panes outside `P`** (an import, or panes launched by hand). The wizard touches none of them. Names that clash are `pane-exists`, occupied cells `grid-occupied`; the plan lists them and the choices are another workspace, another instance, or stop. `profile create P --from-current` is offered **only if the user asks** to put un-profiled records under the wizard's profile, and `--take-over` only if the user asks (guide section 3, E64). The live fleet's hub is *foreign* to the wizard's ledger, so `collide.sh` already refuses a hub port it listens on (E60); this case is for an instance whose hub the wizard started.
4. **A layout the previous wizard built** (a ledger with `backend` rows, or Herdr panes named by the config that `pane list` shows as unmanaged): F9 says nothing can adopt it. The wizard does not touch it and offers: a second instance beside it (a new `[instance]` name, state directory, ports and Herdr session, which Stage 1 already knows how to ask), or, with the user's own yes, the ledger teardown the skill has always had (which stops the old processes and the server that holds the panes), or stop. Never a conversion in place.
5. **Restored panes.** A restarted Herdr server restores its saved session, and its panes have no record: they are unmanaged and no Holler verb adopts them (E76), so today's "reuse branch" (E75: reuse the restored structure, re-attach the dead viewer panes) is **retired**. They are never reused, split or closed; the wizard builds its workspace beside them with the user's yes (the adapter creates it, F4), and refuses a `workspace` label that a restored workspace already uses (the adapter finds a workspace by its label, so a restored workspace of that label would be taken for ours, and its occupied cells refused by the launch as `grid-occupied`). Stage 3 lists them as today; `holler pane list` shows them in its second table.
Rejected: **(a)** run `pane import` against a wizard-built layout (no launcher files, F9); **(b)** write launcher files for it (it would recreate the files the epic retires); **(c)** rebuild over it ("never rebuild a live fleet", and the panes may hold live conversations).

**D19. #666 is a soft preference only.** *(operator, E3)* The wizard is built against the merged stories and #831's harness, and where it would have copied the recreated workspace's layout it uses the current ADR-0021 row-first positions (`r<row>c<col>`, E1: the wizard's names already agree). Nothing waits for #666, and no code or text here depends on it.

**D20. Rigor is `in-session`.** *(issue, E1)* The issue says `rigor: in-session`; the task's default for non-trivial work is `second-opinion`, and #649's D20 recorded `second-opinion` as the operator's choice for the epic's remaining stories (E74). The issue's explicit line wins here; the operator may raise it at the plan check (nothing in this brief changes). `uiSurface: false`, so D and U never run.

**D21. The Claude login is never touched.** *(operator, epic decision 9, E62; the mechanism is #831's, **REFRESH AFTER MERGE of #831**, R3 item 10)* The skill never runs `claude auth`, `claude login` or any command that reads, prints or copies a login, and never names a credential variable's value. Whether the login is usable comes only from `holler pane doctor`'s finding `claude-login-missing` (a problem, asked once per pass when a Claude pane is in scope); the skill's answer is "log in once, by hand, in a plain terminal outside the wizard, then re-run Stage 9". A spec `env` that names a login variable is refused by the engine (`claude-login-env-refused`); the wizard passes no `--env` at all.

**D22. The selector UX is not in this story.** *(decided by the preparer; Questions 5)* The issue puts "the pane count and the selector UX" in "epic #769, change 10" (E2). No such change exists in the epic's table, its stories, or any issue or doc in the repository (searched: #769 body and comments, #803, #842, #848, `docs/`), so nothing says what it builds. This story gives Stage 1 the plain questions the config needs (a directory and a harness per pane, the orchestrator, the three defaults of D12) and keeps the existing pane-count rule and the `r<row>c<col>` names (pinned by #776 and #786); it allows one read-only aid, listing `~/Projects` with `ls -1`, as a hint and pins nothing about it. The orchestrator's default cell stays `r1c1` as pinned; placing the MO at the last cell is a Stage 1 default that belongs to the selector story.

### Where this brief differs from the issue text

Issue #803 is the source of truth where this brief is silent (E1, E2, E3). The brief is the decision where they differ:

1. **The stage order.** The issue rewrites Stage 8 only; the hub must come first (F3), so Stages 4 to 9 are re-used in a new order (D2, D7).
2. **The dry run at Stage 3.** Shown there only when the hub and profile already exist; always at Stage 7 before the first change (D3).
3. **Orchestrators as specs with a `command`.** No spec `command`; the orchestrator is a role on an OpenCode or Claude Code pane (D10).
4. **"Create the profile and apply it".** Applied, but only after the first column is launched, because apply alone cannot build a multi-row, multi-column grid (D9, F5).
5. **"The per-session `endpoint` and `session_id` fields leave the config".** The whole `[[session]]` table and every remote key leave, and the body's rows get `endpoint` and `session_id` from the registry (D4, D5, D6, D13). The issue does not mention Stage 7; the body config cannot stay as it is once no stage creates a session.
6. **`herdr.sh`.** The restored-saved-session handling is *retired*, not kept: a restored pane has no record and nothing adopts it (D18, item 5). `run` is an allow-list, and a new verb, `holler`, keeps the agent's environment out of the panes (D8, D16).
7. **Blockers.** #644, #664 and #642 have merged (11:45, 12:14 and 11:56 MDT on 2026-10-11); the story waits for #831 by the operator's decision (E3); #666 is a soft preference only (D19).
8. **Layouts.** The acceptance's "same grid and names" holds for every layout the adapter can build (a staircase, D4); a layout with a taller column to the right is refused with the reason, not built.

## Scope

In order:

1. **The contract and docs first** ("Amendments" below): ADR-0021 section 11 and "Decisions taken", `docs/setup-wizard.md` (including the design note the issue's Work 1 asks for, as its section "How the wizard builds panes"), the pointer and the creation-order sentence in `docs/pane-control.md`, the README and `docs/README.md` wording, the CHANGELOG.
2. **`lib/panes.sh`** and its tests (RED first), then the changes to `lib/herdr.sh`.
3. **`SKILL.md`**: the frontmatter description and introduction; the Connections section (four connections become two: hub and body over loopback, and the body and its pane's harness over loopback; Herdr is reached by Holler's adapter, not by the wizard); the config section; the checklist table; Stage 0 rows; Stage 1; Stage 2; Stage 3; Stages 4 to 9 as D2; Stage 10; Stage 11; "If a stage fails"; Related.
4. **The tests**: new `panes_803.rs`, `herdr_803.rs`, `skilltext_803.rs`, `proof_803.rs`; the existing pins that move ("Blast radius").
5. **Not in this story:** any Rust production code; the selector UX (D22); remote sessions (D6); applying a pane's model (#829); `say --pane` and DRIVEN (#839, #850); recreating the live workspace (#666, operator-run); the cutover (#654); a profile file (`profile import`, #665); a new verb that adopts an old wizard layout (F9); closing the Herdr server's own first workspace for the user.

## Amendments, first (exact edits)

**ADR-0021, `docs/adr/ADR-0021.md`.** (1) Section 11 ("How attach mode (ADR 0014) relates", E37), its last sentence, "Attach mode with an operator-written `endpoint` and `session_id` stays for sessions Holler does not own (a session with no pane record).", keeps its words and gains: "The setup wizard (#803) builds registry panes only: it starts no backend, creates no session and attaches no TUI, and runs one body on the hub's machine whose attach rows take their `endpoint` and `session_id` from the registry. It no longer builds a session with no pane record." (2) "Decisions taken": one new item, at the next free number when the PR is made (#831 appends 22 to 25 and #839 and #835 append theirs; this one is renumbered at rebase), "**The setup wizard builds registry panes through `holler pane` and `holler profile apply` (operator, 2026-10-11, #803):** remote attach-mode sessions are not built by it; the first column is launched before `profile apply` because the adapter adds rows before columns." (3) **REFRESH AFTER MERGE of #831:** section 15 item 16 ("The wizard", R4) ends "lands with #803, not here": after this story it reads "As built (#803): the wizard passes `--harness claude` and `--port-policy none` for a Claude pane and builds the mixed grid through `profile apply`"; the "Not built here" list of section 15 and the "Deferred to named stories" row, if #831 left one for #803, are updated to say it is done. ADR 0014 is not edited (D17).

**`docs/setup-wizard.md`** (583 lines). By section: (a) the H1 and first paragraph (E25) become the wizard's purpose under Holler's pane control (the title no longer says "attach-mode sessions"); (b) "Shape" (lines 13 to 40): the diagram becomes one machine (Herdr workspace of registry panes, the hub, one body); (c) "The config file" (E5, E6): `[[pane]]`, the staircase rule for `layout`, the defaults, the retired keys and their message, `hub_host` ignored; the "master file is not the body's config" paragraph keeps its drops and says the body rows now come from the registry; the manual-relay paragraph is deleted (D6); (d) "The viewing mechanism" (E26) becomes "How the wizard builds panes": the design note (the profile, spec-only launches, column 1 first, apply; the `herdr-mirror` warning stays), with the hazard stated as the code has it (E39, E40, E42) and pointers to `docs/pane-control.md` sections 3, 6, 7 and 8, not copies; (e) the "Label every token" section keeps the label rule with `<prefix>-local` and drops the remote-host history; (f) the gotcha bullets: the restored-Herdr bullet and the "Stage 8 labels and builds the grid" bullet (E27) are rewritten for Stages 5 and 7, the `AGENTS.md` bullet stays; (g) "Stopping and tearing down": the per-host and `tailscale serve` sentences go, the pane close / profile delete / ledger order (D15) comes in; (h) "Automated setup" (E28): the stage prose follows D2, and "a 12-stage wizard (Stage 0 through Stage 11)" stays word for word; (i) "Collision preflight": the remote inventory sentences go; (j) "Herdr session": the `workspace create`/`rename` paragraph and the `Stage 8 and Stage 9 run every Herdr command through herdr.sh` sentence become Stage 5 and the allow-list of D16, and `--stage 8` becomes `--stage 5`; (k) a new section "If you built a workspace with the previous wizard" (D5, D18); (l) "Basic Herdr commands" gains the note on the server's own first workspace; (m) "Related" adds ADR-0021 and `docs/pane-control.md`.

**`docs/pane-control.md`.** One pointer paragraph at the end of section 6 ("Profiles"): the setup wizard builds its panes with `profile create`, `pane launch --profile P --spec-only`, `pane launch --profile P` and `profile apply`, and where the rest is documented. In section 8 ("Layout", E35) one factual sentence: a cell is reached by one split per call, rows before columns, and `profile apply` creates in row-then-column order, so a profile whose first cells of later rows are missing next to a wider row 1 is created by launching the first column first (read from the code, E40, E42; the real-adapter run is #667). **REFRESH AFTER MERGE of #831:** section 1's "boundary" paragraph (E31), section 10's closing paragraphs and section 11's Claude Code bullet are rewritten by #831 (its branch changes 138 lines of this file, by `git diff --stat` against `main`); re-cut E31, E36 and E65 and place the pointer after #831's text. The test `docs_pane_control_test.rs` pins the guide to the code's tables and must stay green.

**`README.md` and `docs/README.md`.** README lines 154 and 177 (E29) and the agent prompt's scenario menu (E30): "remote attach-mode sessions" become panes; scenario B is removed and the cost line says what the wizard builds. `docs/README.md` line 39 (the `setup-wizard.md` index entry): "watching multiple attach-mode sessions ... plain `opencode attach` panes" becomes the new purpose. The README's install command and rename note are not touched (pinned, E50).

**`CHANGELOG.md`.** Under `## [Unreleased]`: an `### Enhancements` item linking #803 (the wizard builds panes through `holler pane` and `holler profile apply`, a mixed grid of OpenCode and Claude Code panes, teardown through Holler), and a new `### Breaking Changes` item (the `docs/releasing.md` bucket order puts it after Enhancements): the wizard's config is `[[pane]]`; `[[orchestrator]]` and `[[session]]`, remote attach-mode sessions and manual-relay mode are gone; `layout` must be a staircase; link #803.

## Acceptance

Each item is a test or a checkable text assertion that fails without the change. T writes them RED first, in the new modules named; F makes them GREEN; the existing target stays green.

1. **Same grid and names (the issue's acceptance).** `panes_803`: for the first-run fixture of epic #769 (eight orchestrators in four columns of two, E67), `panes.sh grid` prints the names `r1c1 r2c1 r1c2 r2c2 r1c3 r2c3 r1c4 r2c4` at exactly those cells and the extent `2x4`; `panes.sh check` exits 0. For a 4 x 4 mixed fixture (OpenCode and, **REFRESH AFTER MERGE of #831**, Claude panes; the orchestrator at `r4c4`) the grid prints sixteen distinct cells.
2. **Column 1 first, then apply.** `panes_803`: `panes.sh plan` on a 2 x 2 fixture prints, in this order, `profile-create`, one `spec` per pane in layout order, `dry-run`, a `launch` for each first-column pane top to bottom and for no other, then `apply`; the `spec` lines carry `--spec-only`, no `--herdr-session`, `--workspace`, `--grid rRcC`, `--project` (absolute, `~/` expanded), `--model`, `--effort`, `--role`, `--ctx-soft`, `--ctx-hard`, `--port-policy fixed:PORT`; the `launch`/`apply` lines carry `--herdr-session`. A workspace name with a space is quoted so it is one argument. `proof_803`: the sequence run literally against the fake `holler` (whose planner enforces E40's rule) ends with every pane recorded at its cell in profile P, and the fake refuses a row-major build, which pins that the order matters (a test of the fake, not of the adapter).
3. **Mixed harness flags.** **REFRESH AFTER MERGE of #831.** `plan` on the mixed fixture gives a Claude pane `--harness claude --port-policy none`, no `--agent`, and gives an OpenCode pane no `--harness`; ports are assigned over OpenCode panes only.
4. **A taller column is refused.** `panes_803`: `[["o1"], ["alpha","beta"]]` exits 1 naming `layout` and the column; `[["alpha","beta"], ["o1"]]` passes.
5. **Retired keys are refused, each named.** `panes_803`: one fixture per `[[orchestrator]]`, `[[session]]`, `endpoint`, `mode`, `session_id`, `remote_host`, `remote_tailnet_host`, `cmd`: exit 1, a message naming the key and "attach"; `hub_host` alone: exit 0 with one warning line.
6. **The other validity rules.** `panes_803`: a pane name outside the grammar (F10), a duplicate name, a name missing from `layout` or extra in it, a relative `dir`, a `harness` or `role` outside its set, no orchestrator, a pane with no resolvable `model`, `effort`, `ctx_soft` or `ctx_hard`, `ctx_soft` above `ctx_hard`, a workspace label with `=` or `,`: each exit 1 naming its key.
7. **Ports.** `panes_803`: OpenCode panes get `backend_port_base + i` or their own `backend_port`; a non-default instance in 47001 up without its own ports, a duplicate, and a port above 65535 are refused naming the key.
8. **The environment literals.** `panes_803`: `panes.sh env` prints `WIZARD_HERDR_WORKSPACES=<label>=<R>x<C>` (rows are the first column's height), `WIZARD_HERDR_SOCKET` for the default instance only and `WIZARD_TMUX_SOCKET=<state_dir>/tmux.sock` for a non-default one only; the default workspace label is `holler`.
9. **No direct Herdr control in `herdr.sh`, and Holler runs scrubbed.** `herdr_803`: `herdr.sh label ...` is a usage error; `herdr.sh run pane split|run|send-keys|read|get|list|rename`, `run workspace create|rename|focus` and `run server stop` exit 2, name `holler pane`, and the fake `herdr` records no call; `run status`, `run session list` and `run workspace list` still reach it with `--session` first; the header lists no `label`. `herdr.sh holler pane list` runs the fake `holler` with `HOLLER_STATE_DIR`, `HOLLER_HERDR_SESSION` (`default` for the default instance), `HOLLER_HERDR_WORKSPACES` and, when set, the socket variables, and with none of the sentinel variables the test exports (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, a made-up secret name); it exits 2 and runs nothing with `WIZARD_STATE_DIR` unset. The old `label_*` tests and the `run pane ...` cases of `herdr.rs` are removed or rewritten to this.
10. **No direct Herdr control in the skill, the docs or the README.** `skilltext_803`: `SKILL.md`, `docs/setup-wizard.md` and the README's wizard section contain none of `pane split`, `pane run`, `send-keys`, `pane rename`, `pane read`, `workspace create`, `workspace rename`, `herdr.sh label`, `opencode attach`, `--hostname 0.0.0.0`, `POST /session`, `tailscale serve --bg`, and no `ssh` command in any code block; `herdr.sh run` appears only with `status`, `session list` or `workspace list`.
11. **The stage order and names.** `skilltext_803`: the headings of Stages 0 to 11 exist once each, in order, with the names of D2; the byte offsets show hub (4) before Herdr server (5) before profile (6) before panes (7) before body (8) before check (9); "12 stages" and `Stage N/11` pins stay green.
12. **Scoping.** `skilltext.rs` (edited): the word list gains `holler pane` and `holler profile`, and the whole skill passes: every such line in a code block goes through `herdr.sh holler` with `<herdr_env>`; a bare `holler pane ...` line fails.
13. **The dry run.** `skilltext_803`: Stage 3 states the config-derived table and, only on a rerun with a live hub and an existing profile, `profile apply ... --dry-run --format=json`; Stage 7 runs the dry run before the first launch and stops on a difference from Stage 3's table (D3).
14. **The migration cases.** `skilltext_803`: Stage 3 names the four cases of D18, says the wizard touches no pane outside its profile, offers `--from-current` and `--take-over` only on request, says an old layout is not adopted and offers a second instance or the user's own ledger teardown, and no code block contains `pane import`.
15. **The body.** `skilltext_803`: Stage 8 has one local body, its rows' `endpoint` and `session_id` read from `holler pane get ... --format=json`, the join `--server ws://127.0.0.1:<hub_port>`, the label `<prefix>-local`, `--role body --stage 8`; the skill has no `--advertise`, no `tailscale serve` command and no `remote_host`; `serve_https_port` remains only as the `[instance]` key that `instance.sh` still validates (the #838 text proposes it), marked unused. **REFRESH AFTER MERGE of #831:** it states that a Claude pane has no body row.
16. **Teardown through Holler.** `skilltext_803` and `proof_803`: the text orders `pane close NAME --profile P` for each pane, `profile delete P`, then `stop-owned.sh teardown`, and says what to do when the hub is gone; the scenario runs `panes.sh teardown` against the fake and ends with an empty registry and an empty profile list.
17. **The registry check.** `skilltext_803`: Stage 9 uses `pane doctor --format=json`, `pane list --format=json` and `profile show`, gates on any finding except `unregistered-herdr-pane`, and says DRIVEN prints `-` until #839 (**REFRESH AFTER MERGE of #839**); `proof_803`: the fake's doctor returns no finding after the documented sequence.
18. **The login.** `skilltext_803`: the skill never runs `claude auth` or `claude login`, never passes `--env`, and names `claude-login-missing` with the by-hand remedy. **REFRESH AFTER MERGE of #831.**
19. **The two-instance proof stays green and gains pane verbs.** `proof.rs` is unchanged except that `Host` and its helpers become `pub(super)`; `proof_803` builds instance B's panes through the fake `holler` with B's state directory on every call (the fake logs each call's state directory, E54, and the test fails on one without it), leaves A's snapshot unchanged, and creates no `$HOME/.holler`.
20. **The amendments.** `skilltext_803`: ADR-0021 section 11 carries the wizard sentence; "Decisions taken" has the new item; `docs/setup-wizard.md` has the section "How the wizard builds panes", the migration section and the unchanged "a 12-stage wizard (Stage 0 through Stage 11)"; the README's scenario menu has no scenario B and `docs/README.md` no "attach-mode"; the CHANGELOG has the Enhancements and Breaking Changes items.
21. **Hygiene.** `panes.sh` and `herdr.sh` contain none of `declare -A`, `mapfile`, `readarray`, `${...,,}`, `${...^^}`, `|&` (bash 3.2). The rename scan (E50) passes over the whole tree, this brief included. `docs_pane_control_test` passes. The whole `wizard_scripts` target, `cargo clippy --workspace --all-targets -- -D warnings` and `bash scripts/lint.sh` pass; no `.rs` file reaches 900 lines (E58). The CI test command (`test.unit.command`, E57) is green.
22. **Stages 0 and 2 are local.** `skilltext_803`: the Stage 0 table has `tmux` and `opencode` rows (**REFRESH AFTER MERGE of #831**: and `claude`), the `holler` row also runs `holler pane --help`, and there is no `tailscale` or `ssh` row; Stage 2 checks each pane's `dir` and the harness binary, reads the local OpenCode model (E66's source, now this machine's), runs `check-pane`, `check-session` and the local inventory, and contains no `ssh` or `tailscale` command.
23. **Stage 1.** `skilltext_803`: it asks each pane's directory and harness, the orchestrator, and `model`, `effort` and the ceilings once with D12's proposals; runs `panes.sh check` and `instance.sh` in its Verify; offers D5's conversion as text shown to the user; says remote sessions are not built and where attach mode still lives; keeps the Herdr-session question and the workspace-name question in the order #838 pinned.

**Operator-run, not part of the PR's gate** (none of it can run without a real Herdr; "What cannot be verified" lists why): the repeat of the new-user run on a clean workstation (#778's shape), now building a mixed grid, then `holler pane doctor`, `holler pane list` and a teardown.

## Blast radius

Line counts are `wc -l` at `d6ae539`. `scripts/lint.sh` fails a `*.rs` file under `crates/` at 900 lines and warns at 600 (E58); it does not count `SKILL.md`, shell scripts or docs.

**Edited or created, outside the tests**

| File | Lines now | What changes |
|---|---|---|
| `agent-skills/herdr-setup/SKILL.md` | 1853 | rewritten where Scope item 3 says; not counted by lint |
| `agent-skills/herdr-setup/lib/herdr.sh` | 284 | `label` removed; `run` an allow-list; `holler` added; header updated (E21 to E23) |
| `agent-skills/herdr-setup/lib/panes.sh` | new | `check`, `grid`, `env`, `plan`, `teardown` (D1) |
| `lib/instance.sh`, `collide.sh`, `inventory.sh`, `ledger.sh`, `stop-owned.sh` | 211, 281, 181, 177, 241 | **not edited** (their test files stay as they are) |
| `docs/setup-wizard.md` | 583 | the sections of "Amendments" |
| `docs/setup-wizard-two-instances.md` | 77 | one sentence: the proof now also builds panes through the fake `holler` |
| `README.md`, `docs/README.md` | 499, 39 | wording at E29, E30 and the index line |
| `docs/pane-control.md` | 798 | a pointer paragraph in section 6 and one sentence in section 8; **REFRESH AFTER MERGE of #831** (#831 rewrites parts of this file) |
| `docs/adr/ADR-0021.md` | 1121 | section 11's last sentence and one "Decisions taken" item |
| `CHANGELOG.md` | 991 | an Enhancements item and a Breaking Changes item |
| `crates/holler-cli/tests/wizard/proof_fakes/holler` | 70 | `pane`, `profile` verbs with a registry under `HOLLER_STATE_DIR` and the planner rule of E40 |

**Tests** (all under `crates/holler-cli/tests/wizard/`, the one `wizard_scripts` target, E49, E59)

| File | Lines now | What changes |
|---|---|---|
| `panes_803.rs`, `herdr_803.rs`, `skilltext_803.rs`, `proof_803.rs` | new | the acceptance of "Acceptance"; each stays well under 600 lines; four `mod` lines in `main.rs` (27) |
| `proof.rs` | 825 | `Host` (E56) and its helpers become `pub(super)`; nothing else, the file must not grow |
| `herdr.rs` | 874 | the `label_*` tests (E53), the `run pane ...` cases and `the_skill_runs_stages_8_and_9_through_the_wrapper...` go or change; the file shrinks |
| `skilltext.rs` | 656 | the scoping word list (E51); the pins in the next table |
| `preflight.rs` (**890**), `ownership.rs`, `start.rs`, `instance.rs` | 890, 673, 645, 567 | **not edited**: the scripts they drive are unchanged, and `preflight.rs` is ten lines from the gate |

**The pinned phrases that move.** The preparer's reading, from the test names and the greps of E49 to E53; the exact list is what the RED run of the old target shows against the new text, and T records it in its handoff. "Retired" means the subject no longer exists; "moves" means the same fact in a new stage or form.

| Test module | Retired | Moves or is rewritten |
|---|---|---|
| `skilltext.rs` | `stage_two_fetches_each_remote_ledger_and_passes_it_to_collide`, `the_join_url_and_advertise_carry_the_serve_port`, `the_remote_teardown_form_names_its_third_argument`, the remote half of `stage_two_runs_the_herdr_checks_and_fetches_each_remote_home_state_dir_rule` and of `teardown_runs_on_every_host_and_turns_off_only_its_own_serve_entry` | `the_logs_rule_covers_stages_four_to_nine_and_stage_eight_sets_the_log_dir` (the `WIZARD_LOG_DIR` pin moves to Stage 5), `the_herdr_server_is_recorded_in_full_from_the_pid_server_start_prints` (`--stage 5`), `the_derived_body_config_names_what_to_drop_and_what_to_set` and `the_body_config_path_is_defined_once` (the body is Stage 8, local), `the_check_port_sentence_comes_before_the_stage_six_start_block` (the hub is Stage 4), `the_body_reuse_rule_compares_the_expanded_path`, `the_token_label_folds_in_the_prefix_and_the_stop_path_for_herdr_is_named` (`<prefix>-local`), `stages_three_and_four_use_the_ports_instance_sh_printed` (ports now from `panes.sh`), `holler_and_herdr_commands_in_code_blocks_are_scoped_to_the_instance` (word list grows) |
| `skilltext_772a.rs` (zero sessions) | `stages_three_four_five_seven_and_nine_say_skipped_with_no_sessions`, `stage_three_plan_says_which_stages_are_skipped`, `stage_one_accepts_zero_sessions_when_an_orchestrator_exists`, `stage_one_gate_does_not_stop_on_an_empty_session_list`, `the_docs_and_readme_say_a_local_only_config_is_valid` (there is no session-less run: every pane is a pane) | `stage_six_skips_tailscale_serve_with_no_remote_host_but_starts_the_hub` (always now, Stage 4), `stage_ten_verifies_the_hub_and_the_orchestrator_panes_not_sessions`, `the_checklist_shows_skipped_stages_with_a_distinct_symbol` (the legend stays) |
| `skilltext_772b.rs` (Stage 8) | `stage8_labels_every_pane_with_its_slot_name` (E52), `stage8_builds_the_grid_row_first` | `stage8_reads_the_status_line_of_herdr_0_8` (Stage 5), `stage8_handles_a_restored_saved_session` and `stage3_plan_names_a_stopped_saved_session` (D18, item 5), `the_docs_herdr_section_matches`, `only_the_local_collide_line_sets_the_herdr_host_flag` (the only collide line) |
| `skilltext_w3_stage8.rs`, `skilltext_w4_stage8.rs` | `stage8_restored_panes_are_a_reuse_branch_on_request`, `stage8_says_to_check_column_widths_after_the_build`, `base_pane_is_the_new_workspaces_first_pane_and_the_count_is_scoped` | `stage8_build_in_it_is_a_new_workspace_with_a_way_to_find_its_pane`, `stage3_build_in_it_means_a_new_workspace`, `stage8_ask_is_covered_by_the_plan_yes` (to Stage 7), `stage3_plan_names_the_new_workspace_and_lists_restored_panes`, `briefing_is_not_offered_without_sessions_and_is_name_free` (the write moves to Stage 7; **also #842**), `docs_reuse_restored_panes_only_on_request`, `docs_carry_the_same_rules` |
| `skilltext_w5_workspace.rs` | `stage8_shows_the_create_label_call_and_the_base_pane_rename` (E52's neighbour) | `the_derived_body_config_drops_workspace`, `the_docs_list_workspace_among_the_keys_the_body_config_drops` (Stage 8), `stage3_plan_names_the_workspace` (stays) |
| `skilltext_w4_stage2.rs` | all eight tests (the skipped remote checks do not exist) | none |
| `skilltext_w3_stage1.rs`, `skilltext_w3_stage3.rs` | `there_are_two_collide_lines_one_local_and_one_remote`, `the_docs_say_the_remote_call_has_no_hub_only_variables`, `hub_host_is_optional_for_zero_sessions_and_required_from_one` | `scenarios_a_and_b_and_the_readme_propose_r1c1` (scenario B goes), `stage_six_leaves_advertise_out_without_a_hub_host`, `a_local_only_run_leaves_the_serve_port_empty_on_the_local_call` (always) |
| `skilltext_w6_preflight.rs` | the `tailscale` and `ssh` rows and commands (lines 100, 101, 125, 126, 173 to 175) | `stage0_prints_one_vertical_status_table_with_every_row`, `stage0_runs_the_machine_level_commands` (new rows: `tmux`, `opencode`, **REFRESH AFTER MERGE of #831**: `claude`; the `holler` row also runs `holler pane --help`) |
| `skilltext_w5_basics.rs`, `skilltext_838.rs`, `skilltext_772c.rs`, `skilltext_w5_stage0.rs`, `skilltext_w5_rename.rs` | none | stay (`skilltext_838`'s Stage 3 boundary `## Stage 4 ` still exists; the rename scan, E50, is why this brief avoids the old spelling) |
| `herdr.rs` | `label_renames_the_pane_with_the_session_flag_first`, `label_without_a_session_runs_a_bare_rename`, `label_refuses_empty_and_malformed_labels_without_a_call`, `label_from_a_pane_of_another_session_is_refused_without_a_rename`, `label_from_a_pane_of_the_own_session_is_accepted`, `a_label_with_a_non_ascii_letter_is_refused_under_a_utf8_locale` (E53) | `every_stage_command_carries_the_session_and_no_bare_server_stop_is_issued`, `a_pane_of_a_different_session_stops_before_any_split` (the guard words change), `the_skill_runs_stages_8_and_9_through_the_wrapper_and_forbids_a_bare_server_stop` (Stages 5 and 7) |
| `proof.rs` | none | `stage4_backend` and the scenario's `build` step are scripts-level and stay valid as a test of the scripts; the skill no longer drives them, so `proof_803.rs` is the scenario "as the skill tells" |

## Depends on

Each verified with `gh issue view` on 2026-10-11 (21:1x MDT). Times are MDT.

| Story | State | Why it matters here |
|---|---|---|
| #644 `holler pane launch` and `relaunch` | closed 11:45, merged | the verb Stage 6 and 7 run |
| #664 `holler profile apply` | closed 12:14, merged | Stage 7 |
| #642 the OpenCode adapter | closed 11:56, merged | the harness behind every OpenCode pane |
| #649 the real wiring and the scratch scenario | closed 17:35, merged (`2d217dd`) | the environment variables of D8 (E70, E43) |
| #652 the operator guide | closed 18:13, merged (`0e62b51`) | `docs/pane-control.md`, which the wizard docs point to and which `docs_pane_control_test` pins |
| #802 `ServeSpec`, #832 unmanaged panes, #851 `holler --version` with the commit | closed, merged (#802: `7284001`; #832: `652841b` and `bda1069`; #851: `d6ae539`) | the server start through the host adapter; `pane list`'s second table (D14, D18); Stage 0 can name the commit |
| **#831 the Claude Code harness** | **open**; branch `issue-831-implementation` at `d137a1d` (A, T-red, F and T-green passed; S, the PR and the merge remain) | **the gate** (E3): this story starts once #831 has merged; it is needed for the Claude panes of a mixed grid (E2) |
| #666 recreate the workspace from profiles | open, operator-run | **soft preference only** (E3, D19); nothing here depends on it |
| #839 DRIVEN follows the session of record (absorbs #828, closed 15:56) | open; branch `issue-839-implementation` at `bf9d1c2` (its brief, amended after its plan check) | not needed to build, and expected to merge before this story (its own order, its brief's line 444); D13 and D14 carry REFRESH rows |
| #850 a body driver for a Claude Code pane | open | not needed; a Claude pane has no body row until it lands (D13) |
| #829 apply the pane's model, effort and environment | open | not needed; D12 says "recorded, not applied" |
| #835, #648, #647 part 2, #667, #665, #842, #848 | open | not needed; see "Clashes" |

## Facts that wait for other stories

A line marked **REFRESH AFTER MERGE of #NNN** states the *expected* contract of a story that has not merged. The build run re-cuts the named excerpt from merged `main` and patches the line before T writes a test.

| Story | What in this brief rests on it | Excerpt to re-cut | What to check |
|---|---|---|---|
| **#831** (required) | `harness = "claude"` in `[[pane]]` and in `panes.sh`'s accepted set (D4); `--harness claude` and `--port-policy none` in `plan` (D9, D11); model and effort applied for Claude (D12); no body row for a Claude pane (D13); the login rule and the finding `claude-login-missing` (D21); Acceptance 1 (the 4 x 4 mixed fixture), 3, 15 (last sentence), 18; the `claude` Stage 0 row; ADR-0021 section 15 item 16 (the "wizard" item); the `docs/pane-control.md` boundary paragraph and the "Later" list | R1 to R4 and R12 from merged ADR-0021 section 15, `docs/handoffs/831-brief.md` and `crates/holler-cli/src/pane/args.rs` (the `--harness` flag), plus `tx_launch::port_for`; E31, E36, E65 from merged `docs/pane-control.md` | the flag spellings and the policy word; whether `agents --json`-based `process-in-use` refuses a Claude pane already running in a tmux session of the same name (a user's hand-set-up pane in the wizard's session); the names `CLAUDE_CODE_COMMAND` and `is_claude_code_command` do not appear in the wizard; the finding names the doctor prints for a Claude pane (`claude-login-missing`, `pane-waiting`) |
| #839 (expected to merge first) | D14's "DRIVEN prints `-` until #839" and Acceptance 17; D13's body rows (each pane's `session_id` is read from the registry, R6) | R6 from the merged brief and its clash table's row for this story; merged `docs/pane-control.md` sections 4 and 10 | DRIVEN is then compared; `shown-driven-mismatch` can fire for a body that is behind; Stage 9's gate already stops on it; a pane no body drives refuses `reset --first` (`pane-not-driven`), which this story never runs |
| #850 | D13: a Claude pane gets a body row | R9 | the row's shape for a Claude pane |
| #842 (if it merges first) | D10: where the `AGENTS.md` briefing is written, Stage 1 step 4's confirm text, the "Look at panes" bullet that names `herdr pane list` and `herdr pane read` | R8, R11 | the bullet must say `holler pane list`; Part C (an even grid by ratios) is obsolete once this story merges, because the adapter already splits by ratio (E39) |
| a follow-up to #667's creation-order question (not filed) | D9: if apply creates column 1 first, or the planner accepts the row-major order, the `launch` tag and Phase A go | R7; E40, E42 | the plan loses its `launch` lines; Acceptance 2 loses them |
| #647 part 2 | D14: new doctor kinds (`unattached`, `agent-cannot-dispatch`) stop Stage 9's gate by the "any finding but one" rule | merged `docs/pane-control.md` section 5 (E72) | an orchestrator pane running as the server's default agent may raise `agent-cannot-dispatch`; the remedy is the pane's `agent` key (D4) |
| #665 | D4(d): `profile import FILE` could replace the spec-only loop | R10 | none unless the operator confirms #665 |

## Clashes with stories in flight

State read-only on 2026-10-11, 21:1x MDT: the branches and worktrees below, not touched; they move quickly (#831 passed T-green and #839's brief was amended while this one was written), so re-read them at launch. "Shared" means the same file and, where it matters, the same hunk.

| Story (worktree) | Shared files and hunks | Effect on this story |
|---|---|---|
| **#831** (`0831-claude-harness`, `d137a1d`) | `docs/adr/ADR-0021.md` (its new section 15, edits to sections 1 and 2, "Deferred", "Decisions taken" 22 to 25); `docs/pane-control.md` (138 lines changed: sections 1, 2, 3, 5, 6 and 10 to 12); `CHANGELOG.md`; `docs/README.md`; `cli-surface.txt`. Its brief item 5 hands the wizard to this story and says item 16 of section 15 will be edited by it (R1, R4) | **Required first.** Our ADR edit is section 11 and one "Decisions taken" item (next free number); the pane-control pointer goes after #831's text (REFRESH rows). No wizard file is touched by #831 |
| #835 (`0835-import-leave-alone`) | `docs/adr/ADR-0021.md` sections 2, 3, 9, 13; `docs/pane-control.md` (sections 1 and 3, `pane import`; section 10's "Claude Code panes look like OpenCode panes" fact; section 12); `CHANGELOG.md`; `cli-surface.txt` | none in code. This story never calls `pane import` (D18); the paragraph of section 10 that #835 rewrites is not ours. Mechanical hunks in CHANGELOG and the ADR's "Decisions taken" |
| #839 (`0839-driven`, `bf9d1c2`) | `docs/adr/ADR-0021.md` sections 8, 11 (points 1 to 3 of the section whose last sentence we edit), "Deferred", "Decisions taken"; ADRs 0014, 0005, 0003; `docs/pane-control.md` (`reset`, `park`, the DRIVEN column, section 10 steps 4 to 6, section 12); `CHANGELOG.md` | adjacent, not overlapping, hunks in ADR section 11 (we edit its last sentence; it edits its numbered points); renumber "Decisions taken". Its own clash table has a row for this story (line 444 of its brief at `bf9d1c2`): one semantic dependency (a body's attach `session_id` is configuration), which D13 keeps; it orders this story last. It assumes "one attach-mode body row per registry pane" (R6): this story builds exactly that. No wizard file |
| #648 (`0648b-roster-shown-driven`) | `CHANGELOG.md`, `docs/adr/ADR-0021.md`, `docs/pane-control.md` (section 4's roster paragraphs, sections 10 and 11), `crates/holler-cli/src/cli.rs` | docs hunks only; Stage 10 could later read the new roster columns |
| #647 part 2 (`0647-agent-cannot-dispatch`) | brief only so far: `docs/pane-control.md` section 5 (finding rows), the ADR, the finding table | none now; D14's "any finding" rule covers its kinds (REFRESH row) |
| #667 (`0667-apply-scenario`, `e52797a`) | `CHANGELOG.md`, `docs/pane-control.md` (drops its bullet from the "Later" list); its decision 10 and question 1 are D9's hazard (R7) | our section-8 sentence about creation order sits near its edit: re-read it so the two do not both state it. If its follow-up lands first, D9 shrinks |
| #665 (`0665-profile-io`) | `docs/adr/ADR-0021.md`, `cli-surface.txt`, `profile_verbs/import.rs` | none |
| **#842** (open issue; `0842-confirm-show-config` at `7284001`, no commits) | `SKILL.md` (Stage 1 step 4 and Verify, Stage 3's briefing paragraph and its N=0 sentence, Stage 8's briefing block and build), `docs/setup-wizard.md`, a new `skilltext_<id>.rs` and its `mod` line, the older `skilltext*.rs` it re-pins | **the one real clash.** Part C rewrites the Stage 8 build this story deletes; Part B writes the briefing block this story moves to Stage 7 and adds a "Look at panes" bullet that names `herdr pane list` and `herdr pane read`, which D10 and acceptance 10 forbid (R8, R11). Part A is independent |
| #848 (open issue) | `skilltext_w5_rename.rs` | none; it removes F12's trap, and until it merges nothing may quote the old spelling. **#839's brief carries it today** (line 444 of `docs/handoffs/839-brief.md` at `bf9d1c2` names the skill's old directory), so that brief fails the scan unless #848 merges first or the line is edited; worth telling its owner |
| #655, #828, #832-review-fixes | handoffs and ADR hunks only | none |

**Recommended merge order.** The other briefs' order, with this story after them: #667, #835, #648 and **#831** (the operator's gate, E3), then **#842 reduced to Parts A and B** (Part C closed as obsolete by this story; its "Look at panes" bullet saying `holler pane list`), then #839, #647 part 2 and #665, then **#803**. #803 can *start* as soon as #831 has merged (the operator's rule) and be built beside the others; it *merges* last, because (1) it touches no file the others edit except the shared ends of `docs/pane-control.md`, `docs/adr/ADR-0021.md` and `CHANGELOG.md`, so rebasing it onto all of them is one mechanical pass, while rebasing #839 (15 `Ports` literals, by its brief) onto it would gain nothing; (2) #839's own brief orders it so and names no file clash; (3) with #839 merged first, D13 and D14 are written once against a DRIVEN that is wired, not twice. #842 goes before it because both rewrite Stage 1, 3 and 8 of the same `SKILL.md` and `docs/setup-wizard.md`, and its pinned text is then re-homed once. If #842 cannot go first, the build run takes Part B's briefing text into Stage 7 itself and the operator closes Part C. If the operator wants #803 earlier for the release, nothing in this brief blocks it: only the REFRESH rows of #839 and #647 part 2 change.

## Pipeline

`rigor: in-session` (D20), `uiSurface: false`. Phase order `O -> A -> T(author/RED) -> F -> T(verify/GREEN) -> A(anti-duplication) -> S`; O is this brief. Launch from the existing worktree `.claude/worktrees/0803-wizard-panes` of the primary checkout (branch `issue-803-implementation`, created at `d6ae539`; **rebase it onto `origin/main` when the run starts, after #831 has merged, re-cut the REFRESH excerpts then, and rebase again before the PR**), with the clone's `.env` copied in and `.aftersight/releases` linked, as `CLAUDE.md` says. The workflow call is `scriptPath: '.aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs'`, `args: { argsVersion: 2, repoPath: <the worktree>, issueNumber: 803, briefPath: 'docs/handoffs/803-brief.md', uiSurface: false, rigor: 'in-session', stages: <the phases array of review-models.sh resolve --project <worktree> --json, as it is>, models: { F: 'claude-opus-5-5' }, roles: <the content of .claude/agents/*.md> }`; `verifyCommand` is left out, and phase F's command is `test.unit.command` (E57). The test and production path globs of `.aftersight/pipeline.config.json` do not list `agent-skills/**` or `docs/**` (E57); the wizard stories before this one (#838, #810 and the rest of epic #769) ran under the same globs with skill text and scripts, so no change is expected, and if the runner rejects an edit of those paths the fix is a config line the operator approves. The PR is opened by the script; the run's agent checks CI and the gates, adds the AI disclosure to the PR body with `gh pr edit` if the script's body lacks it, and merges (squash, branch deleted). The PR says `Closes #803`. Nothing here is launched by the preparer.

## Rules for this story

- **Public repository.** No personal hostname, tailnet name, IP, account name or token anywhere: every example host, state directory and workspace is a placeholder (`hub.example.ts.net` is not even needed now; `~/Projects/demo` is the example directory).
- **The old spelling of the skill's directory and command appears in no file** (F12, E50).
- **No live fleet, no real Herdr session, no running pane, no real Claude or OpenCode session.** Every test uses the fakes under `PATH`; the opt-in live run is the operator's.
- **A subscription login is never read, copied, logged or named by value; env var NAMES only** (D21).
- **Format only the files you touch.** Never run `cargo fmt --all` over the workspace. No Rust production file changes here; the new test files are formatted by `rustfmt <file>`.
- **Conventional Commit subjects**, the `Co-Authored-By` trailer the hooks add, no direct push to `main`; the PR goes through the pipeline. Run `bash scripts/setup-hooks.sh` once in a fresh clone.
- **Amend first.** The ADR and docs edits of "Amendments" are the first commit after this brief; the tests are RED before any script or skill text changes.
- **bash 3.2** for every script (no associative arrays, no `mapfile`, POSIX awk); the Linux and macOS scripts are one set.
- **Bounded.** `profile apply`, `pane doctor` and every other Holler call in the skill has a stated timeout expectation (the verbs carry their own 10 s per-call bound); a call that fails is a Gate with a question, never a retry loop and never a fallback to a Herdr command.

## AI assistance

This brief was prepared by Claude (Sonnet 5.5, Claude Code) at the operator's direction on 2026-10-11, from issue #803 and its comments, the merged code and docs at `d6ae539`, and, read-only, the briefs and branches of the stories named above. The maintainer who launches the run is accountable for it, per `CONTRIBUTING.md`. Nothing was launched, merged, closed or edited on GitHub in preparing it.

## Questions only the operator can answer

1. **Do remote attach-mode sessions leave the wizard for good (D6)?** The brief infers it from "retire the attach mode" (E1) and epic decision 3, "Herdr's server runs on the same machine as the hub" (E61 shows the adapter's local socket): a registry pane cannot be remote, so a remote OpenCode session would be a body with no pane. If the operator wants that path kept as a headless, pane-less option, it is a second design (Stages 4, 5 and 7 as they are, per remote host) and a different story.
2. **Should the creation-order follow-up be filed (D9, F5)?** `profile apply` cannot build a grid of two or more rows and columns by itself (read from the code, not run; #667's brief asks the same). The wizard works around it with Phase A. The fix is either apply's order (column 1 first) or the planner; filing it is the operator's. When it merges, D9 shrinks.
3. **Are orchestrators that are not an OpenCode or Claude Code harness (a shell script, another CLI) acceptable losses (D10)?** The issue says an orchestrator becomes a spec with a `command`; the spec's `command` is only a preamble (E45), so such an orchestrator cannot be a registry pane.
4. **For a layout the previous wizard built, is "leave it alone, offer a second instance, or the user's own teardown" right (D18, case 4)?** The operator's rule is to adopt what exists through `pane import` or `profile create --from-current`; neither can act on a wizard-built layout (F9). Adopting it would need a new verb, outside this story.
5. **Is the pane-count and folder-selector UX ("epic #769, change 10", E2) a story to be filed (D22)?** It is not in the epic's table or in any issue or doc, so this brief cannot say what it builds; #803 keeps the existing pane-count rule, asks a directory and a harness per pane in plain words, and defaults the orchestrator to `r1c1` as pinned.

## What cannot be verified without a real Herdr, tmux or Claude Code

None of the following can be checked by a test in this repository, and none was run. Each is stated in the text it affects as UNVERIFIED, and the first live run (the operator's repeat of #778) is what settles it.

- **U1.** The default session's socket path and the session name `default` (D8; E48 says the default session's socket is `$XDG_CONFIG_HOME/herdr/herdr.sock`, a spike result, not run here).
- **U2.** That the tmux server inherits the environment of the first `holler pane` call (D8), so the scrub keeps the agent's variables out and `HOLLER_STATE_DIR` reaches the orchestrator pane.
- **U3.** That launching the first column and then running `profile apply` produces the grid in a real Herdr. The planner is read, not run (E39, E40); #667's scenario covers a 2 x 1 workspace only (R7).
- **U4.** What a fresh headless Herdr server holds before the first launch (the old text assumes one pane, E16, E15), and what the adapter's new workspace does beside it. The wizard reports that pane, never closes it, and Stage 11 says how.
- **U5.** `herdr session list`'s output format (the text says the session name comes first; the header calls it unverified, E21).
- **U6.** A Claude Code pane end to end, including `process-in-use` for a pane the user set up by hand in the same tmux session name, and the login (R3). No test starts a Claude Code, and the login is never read.
- **U7.** A provider credential reaching the OpenCode server inside tmux after the scrub (D8): it must be in the OpenCode configuration or the tmux server's environment.
- **U8.** The 10-second budget of a launch for a real `opencode serve` on a cold start (the guide's `timeout`); a failure there is a Gate with a question, not a retry.

## Evidence (verbatim)

Cut on 2026-10-11 21:11 MDT from `main` at `d6ae539`, in the worktree `0803-wizard-panes` with no change but this file. Every excerpt below was produced by the command shown under its heading, which pipes the source through `sed -n 'A,Bp'`; the line numbers on the left are added by `awk` after the cut. Nothing is retyped. An `E<n>` excerpt is a file on `main` (or issue #803, #769 or #633 as they stand). An `R<n>` excerpt is another story's text, cut from a named commit or from a GitHub issue; it shows what that text was when this brief was written and is re-cut after that story merges (see "Facts that wait for other stories"). The commits `d137a1d` (#831), `bf9d1c2` (#839) and `e52797a` (#667) are commits of those stories' branches (local, unmerged); the branches keep moving, so their line numbers will too. The numbers follow the order of this section, not the order of the text above it.

**E1: `issue #803 body, lines 7-29`** — Issue #803 body, Problem and Work: what the wizard does today and what it must do (the issue's own line 5, which quotes the skill directory by its old name, is left out on purpose: the wizard's rename scan, E50, fails on that spelling in any tracked file).

Command: `gh issue view 803 -R Performant-Labs/holler --json body -q .body | sed -n '7,29p' | awk -v s=7 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
7	- **Stage 8** drives Herdr directly through `lib/herdr.sh`: `pane split`, `pane run` (types the orchestrator command into the pane), `send-keys`, `pane rename` (the `label` verb) and `workspace create`. #633 decision 1: only Holler commands change panes or touch Herdr; nobody types into a pane.
8	- **Stages 4, 5 and 9** are attach mode (ADR-0014): start `opencode serve`, create a session, attach a TUI to it. #633 replaces this: Holler creates the session of record and the TUI attaches to it.
9	- **Pane labels** set through Herdr (the wizard's default `r<row>c<col>` names) are a record of pane identity the pane registry does not hold; the epic's "one registry" rule would call it drift.
10	- **The wizard ledger** (`<state_dir>/herdr/setup/wizard-ledger.toml`) tracks processes the wizard started, not panes or sessions, so it does not overlap, except its `herdr` server row, which stays valid while the wizard starts the Herdr server.
11	
12	What already agrees: the wizard's default pane names use the same row-first `r<row>c<col>` as #633's `GridPos`; Herdr and the hub sit on the same machine (decision 3).
13	
14	## Work (blocked until the verbs exist: #644 `holler pane launch`/`relaunch`, #664 `holler profile apply`, #642 the OpenCode adapter)
15	
16	1. A design note first, before any change: how the wizard maps to the verbs. Stage 8's workspace becomes a profile (named, stored in the hub registry, panes at `GridPos` r1c1...), applied with `holler profile apply NAME [--dry-run]`; orchestrators become profile specs (role orchestrator, `command` argv, `dir`), launched by Holler, not typed into a pane.
17	2. Rewrite Stage 8 to create the profile from the wizard config and apply it, with `--dry-run` shown at Stage 3 as the plan; the wizard no longer calls `pane split`, `pane run`, `send-keys` or `pane rename`.
18	3. Replace Stages 4, 5 and 9 with the session-of-record flow of `holler pane launch`; the per-session `endpoint` and `session_id` fields leave the config.
19	4. Decide what stays in `herdr.sh`: starting and stopping the Herdr server the wizard created (ledger rows), `check-pane`, `check-session`, `session-delete`, and the restored-saved-session handling; the pane verbs and `label` go.
20	5. Update `docs/setup-wizard.md`, the README and the tests (`skilltext*.rs`, `herdr.rs`, `proof.rs`); keep the Linux and macOS scripts bash 3.2 compatible.
21	
22	## Acceptance
23	A fresh run of the wizard on a clean workstation builds the same workspace (same grid and names) with no direct Herdr pane command in the skill or in `herdr.sh`; the pane registry holds every pane the wizard created; `holler pane doctor` reports no SHOWN/DRIVEN mismatch; teardown removes the profile and the panes through Holler.
24	
25	## Not now
26	Nothing changes in the wizard before #644, #664 and #642 land: the wizard's current direct-control stages are what the operator's repeat run (#778 of epic #769) tests today.
27	
28	## Pipeline
29	`rigor: in-session`. Blocked by #644, #664 and #642 (and wants #666 closed so the recreated workspace is the reference).
~~~~

**E2: `issue #803 body, lines 33-34`** — Issue #803 body, "Mixed grids (amended 2026-10-11)": the typical grid, every pane registered, Claude Code panes need #831.

Command: `gh issue view 803 -R Performant-Labs/holler --json body -q .body | sed -n '33,34p' | awk -v s=33 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
33	## Mixed grids (amended 2026-10-11)
34	The wizard's typical grid is 4 x 4 with the MO at r4c4 and a mix of OpenCode and Claude Code panes, each in its own `~/Projects` folder; the wizard lists the folders and lets the user pick the agent per pane, and every pane must register with the Holler server. The Claude Code panes need the harness story #831 (epic #633, decision 9); until it lands the wizard can register OpenCode panes only. The pane count and the selector UX are the wizard's own (epic #769, change 10).
~~~~

**E3: `issue #803 comment 0, lines 1-1`** — Issue #803, the operator's comment of 2026-10-11 (20:39 MDT): ships with the rest of #633, #666 is a soft preference, blocked by #831 only for Claude Code panes.

Command: `gh issue view 803 -R Performant-Labs/holler --json comments -q '.comments[0].body' | sed -n '1,1p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	Operator decision, 2026-10-11: this story ships **in the same release as the rest of #633**, which is cut once every code story of the epic has merged, and the cutover (#654) comes after that release. So "wants #666 closed so the recreated workspace is the reference" is a **soft preference, not a blocker**: #666 runs after the cutover, which runs after the release, so waiting for it would wait on itself. Build against the merged stories and #831's Claude Code harness (mixed grids); where the wizard would have copied the recreated workspace's layout, use the current ADR-0021 row-first layout instead. It is blocked by #831 only for the Claude Code panes of a mixed grid.
~~~~

**E4: `agent-skills/herdr-setup/SKILL.md:66-80`** — SKILL.md: the Herdr server is a ledger process; what is still OpenCode-specific (Stages 4 and 9, attach mode).

Command: `sed -n '66,80p' agent-skills/herdr-setup/SKILL.md | awk -v s=66 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
66	The Herdr server the wizard started for the instance is a ledger process like any other: it
67	stops through `stop-owned.sh` (a `herdr` row), never through `herdr.sh run server stop`.
68	
69	A ledger left at the old path (`<state_dir>/wizard-ledger.toml`, before the wizard moved its files to
70	`<state_dir>/herdr/setup/`) is not read: its processes look foreign. The operator may move the file by hand.
71	
72	**What's genuinely generalized here vs. what's still OpenCode-specific:** every orchestrator
73	(command, working directory, and there can be more than one) is fully config-driven and can be
74	any CLI agent. The session side already carries a per-entry `harness` field (per
75	`docs/adr/ADR-0005.md`'s config shape) — but **Stage 4 (starting a backend) and Stage 9
76	(attaching a live view) only have real, implemented logic for `harness = "opencode"` today.** A
77	config entry with a different harness value is real config Holler itself may already support
78	for spawn-mode sessions, but this wizard's own backend-start and live-view mechanics haven't
79	been built for anything but OpenCode's attach-mode HTTP server yet — see the explicit gate on
80	this in Stages 4 and 9, don't silently assume another harness works the same way.
~~~~

**E5: `agent-skills/herdr-setup/SKILL.md:146-165`** — SKILL.md, the config: `layout` is columns of names (outer = columns, inner = top to bottom); `hub_host`; `workspace` (1 to 40 characters).

Command: `sed -n '146,165p' agent-skills/herdr-setup/SKILL.md | awk -v s=146 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
146	# LAYOUT — one field, a nested array: outer = columns, left to right; inner = each column's
147	# panes, top to bottom. This is the ENTIRE workspace layout — every `[[orchestrator]]` name and
148	# every `[[session]]` name is a real slot in this grid, nothing assumed. Every `[[orchestrator]]`
149	# name must appear in `layout` exactly once; every `[[session]]` name must appear exactly once;
150	# `[[orchestrator]]` and `[[session]]` names must not collide with each other (one flat
151	# namespace) — Stage 1 validates all of that. This example — o1 alone in its own column,
152	# alpha/beta stacked in a second column — is today's usual shape, but it's not assumed.
153	#
154	# MUST come before every table header below (`[[orchestrator]]`, `[[session]]`) — a bare
155	# top-level key placed after any table header is parsed as belonging to THAT table, not the
156	# file's top level. This bit the wizard's own real config once; don't repeat it.
157	layout = [["o1"], ["alpha", "beta"]]
158	
159	# THIS machine's own tailnet FQDN — the hub advertises it so remote bodies can dial back.
160	# (Bare top-level key for the same "must come before any table header" reason as layout.)
161	hub_host = "hub.example.ts.net"
162	
163	# OPTIONAL — the name of the Herdr workspace: 1 to 40 characters, a letter or digit first, then
164	# letters, digits, spaces, `.`, `_` or `-`. Bare top-level key, before every table header.
165	workspace = "herald-relay"
~~~~

**E6: `agent-skills/herdr-setup/SKILL.md:183-224`** — SKILL.md, the config: `[[orchestrator]]` (name, dir, cmd) and `[[session]]` (harness, mode, endpoint, remote_host, session_id).

Command: `sed -n '183,224p' agent-skills/herdr-setup/SKILL.md | awk -v s=183 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
183	# Orchestrators — one or more. Each is real, named config: `name` is how `layout` below refers
184	# to it, `dir` is its working directory, `cmd` is commonly "claude" but can be any CLI agent (an
185	# opencode TUI, a plain shell script, whatever briefing that orchestrator needs). Each drives
186	# whichever sessions it's briefed to via Holler's own CLI (the say, roster and interrupt verbs, run
187	# against this instance's hub; see Stage 8's briefing) — the wizard itself doesn't assign sessions to
188	# orchestrators; that's a matter of what each orchestrator is told to do, not layout.
189	[[orchestrator]]
190	name = "o1"
191	dir = "~/Projects/holler"
192	cmd = "claude"
193	
194	# `harness` is real, per-session config (per docs/adr/ADR-0005.md) — "opencode" is what this
195	# wizard's Stage 4/Stage 9 actually know how to start and live-view today; a different value is
196	# valid Holler config but this wizard will flag it as unimplemented rather than guess a
197	# mechanism for it (see Stages 4 and 9's Gates).
198	#
199	# `remote_host`/`remote_tailnet_host` are PER SESSION, not a single global value — attach mode
200	# requires a session's body and backend to be co-located (ADR 0005), but that co-location can
201	# happen on a DIFFERENT host per session/body-group. `remote_host` is the SSH alias/hostname
202	# (matching your `~/.ssh/config` or a plain resolvable name); `remote_tailnet_host` is that same
203	# machine's tailnet FQDN, used to build the `wss://`/`http://` URLs the hub and `opencode
204	# attach` actually dial — not always identical to `remote_host` (an SSH alias and a MagicDNS
205	# name are different namespaces, even when they point at the same box). Stage 7 groups sessions
206	# by `remote_host` and runs one Holler body per distinct host, not one body for everything.
207	
208	[[session]]
209	name = "alpha"
210	harness = "opencode"
211	mode = "attach"
212	endpoint = "http://127.0.0.1:47001"
213	remote_host = "remote-a"
214	remote_tailnet_host = "remote-a.example.ts.net"
215	# session_id filled in by Stage 5 — leave absent or blank in the template
216	
217	[[session]]
218	name = "beta"
219	harness = "opencode"
220	mode = "attach"
221	endpoint = "http://127.0.0.1:47002"
222	remote_host = "remote-a"
223	remote_tailnet_host = "remote-a.example.ts.net"
224	```
~~~~

**E7: `agent-skills/herdr-setup/SKILL.md:337-352`** — SKILL.md: the stage checklist table (the names Stage 4 to 9 have today; 12 stages, 0 to 11).

Command: `sed -n '337,352p' agent-skills/herdr-setup/SKILL.md | awk -v s=337 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
337	| Stage | Name                                                     | Status |
338	|-------|----------------------------------------------------------|--------|
339	| 0     | Preflight                                                | ✅     |
340	| 1     | Load the session config                                  | ✅     |
341	| 2     | Config checks                                            | ✅     |
342	| 3     | Present the plan, get assent                             | 🔄     |
343	| 4     | Start remote agent backends (N=2: alpha, beta — harness: opencode) | ⬜ |
344	| 5     | Capture session IDs                                      | ⬜     |
345	| 6     | Bring up the hub                                         | ⬜     |
346	| 7     | Join + run the body                                      | ⬜     |
347	| 8     | Build the Herdr workspace (M+N panes)                    | ⬜     |
348	| 9     | Attach panes to sessions                                 | ⬜     |
349	| 10    | Verify end-to-end                                        | ⬜     |
350	| 11    | Basic Herdr commands                                     | ⬜     |
351	```
352	
~~~~

**E8: `agent-skills/herdr-setup/SKILL.md:942-966`** — SKILL.md Stage 3: the plan lists orchestrators and sessions grouped by `remote_host`.

Command: `sed -n '942,966p' agent-skills/herdr-setup/SKILL.md | awk -v s=942 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
942	**Do:** using what Stages 1 and 2 actually found (not a generic template), write out a concrete
943	plan and show it before touching anything:
944	
945	- **Orchestrators (from the config):** the real M, each one's name/dir/cmd — e.g. "2
946	  orchestrators: o1 (`claude`, `~/Projects/holler`), o2 (`claude`,
947	  `~/Projects/other-project`)." Not assumed to be exactly one, not assumed to be Claude.
948	- **With no sessions (N=0),** say so: "Stages 4, 5, 7 and 9 are skipped (no sessions)", reported as skipped, never as failed, and with
949	  no remote host the `tailscale serve` line of Stage 6 is skipped too (no remote host dials the
950	  hub), so leave `WIZARD_SERVE_HTTPS_PORT` empty on the local call (a port never used must not
951	  refuse the plan); the hub is still started and every orchestrator pane is still built.
952	- **Sessions (from the config), grouped by their own `remote_host`** — the real N, names,
953	  ports (the ones `instance.sh` printed in Stage 1, never recomputed), and **harnesses**,
954	  organized per host, not as one flat list — e.g. "2 hosts: remote-a
955	  (alpha opencode 127.0.0.1:47001, beta opencode 127.0.0.1:47002), remote-b (gamma opencode
956	  127.0.0.1:47001)." This grouping is what Stage 7 will actually build (one body per host) —
957	  get it right here, not there. If any session's harness isn't `opencode`, say so explicitly
958	  here and flag that Stages 4/9 don't have implemented logic for it yet — don't silently plan
959	  as if every entry were OpenCode.
960	- **The collision preflight, per host — a refusal, not a warning.** Before anything else in the
961	  plan is shown as final, run `collide.sh <host-label> <inventory file>` for each host, with the
962	  instance's contract values set inline on that command: `WIZARD_INSTANCE_NAME`,
963	  `WIZARD_STATE_DIR` (that host's resolved one), `WIZARD_BACKEND_PORTS` (the ports `instance.sh`
964	  printed for the sessions that run on that host) and `WIZARD_LEDGER` (that host's ledger: the
965	  fetched `<scratch>/ledger-<host>.toml` for a remote host; `<state_dir>/herdr/setup/wizard-ledger.toml`
966	  for this machine). The local call also carries the four hub-only variables; a remote host's call
~~~~

**E9: `agent-skills/herdr-setup/SKILL.md:1077-1090`** — SKILL.md Stage 4: remote OpenCode backends, "only implemented for harness opencode".

Command: `sed -n '1077,1090p' agent-skills/herdr-setup/SKILL.md | awk -v s=1077 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1077	## Stage 4 — Start the remote agent backends (one per config entry)
1078	
1079	**Ask:** nothing further — Stage 3 already covered which processes are reused (a live ledger row) vs. started fresh; do
1080	what the assented-to plan said, for every `[[session]]` entry the plan called "start fresh."
1081	
1082	**With no sessions, this stage does nothing: report it as skipped (⏭️), never as failed.**
1083	
1084	**Only implemented for `harness = "opencode"` today.** If a config entry's harness is anything
1085	else, this is a real gate — stop and tell the user this wizard doesn't have a start mechanism
1086	for that harness yet, rather than guessing one. Don't skip the entry silently and don't
1087	improvise a plausible-looking command for an unfamiliar harness.
1088	
1089	**In manual-relay mode** (per Stage 0's `ssh` row and Stage 2), every `ssh <remote_host> "…"` below is a command to hand
1090	the user, not one you run — print it verbatim, wait for them to paste back the real output, then
~~~~

**E10: `agent-skills/herdr-setup/SKILL.md:1153-1160`** — SKILL.md Stage 4: the backend start command, over ssh, `--hostname 0.0.0.0`.

Command: `sed -n '1153,1160p' agent-skills/herdr-setup/SKILL.md | awk -v s=1153 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1153	**Do**, for every `opencode`-harness entry the plan marked as needing a fresh start (its own
1154	`remote_host` — read per-entry from the config, not a single value shared by all — and its
1155	`<port>`), printing the new pid:
1156	```bash
1157	ssh <that entry's remote_host> "mkdir -p <logs_dir> && cd ~ && { nohup opencode --port <port> --hostname 0.0.0.0 --model <provider>/<model> > <logs_dir>/<prefix>-opencode-<name>.log 2>&1 & echo \$!; }"
1158	```
1159	repeated once per entry (note each printed pid), then:
1160	```bash
~~~~

**E11: `agent-skills/herdr-setup/SKILL.md:1189-1213`** — SKILL.md Stage 5: a first session created by `POST /session` on each backend.

Command: `sed -n '1189,1213p' agent-skills/herdr-setup/SKILL.md | awk -v s=1189 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1189	## Stage 5 — Create a real first session on each backend, capture its session_id
1190	
1191	**Ask:** nothing — this is fully mechanical, looped over every `opencode`-harness config entry
1192	(the same harness restriction as Stage 4 applies here). With no sessions, there is nothing to create:
1193	this stage is skipped (⏭️), never as failed. In manual-relay mode, the same rule from
1194	Stage 4 applies: hand the user the exact `ssh` command, capture the id they paste back.
1195	
1196	`GET /session` is a **global, per-user list, not scoped by port**. Don't trust `session[0]` as
1197	"the one just created" — create it with a real POST and read the id straight back from that
1198	response, for every entry:
1199	
1200	```bash
1201	ssh <that session's remote_host> "curl -s -X POST http://127.0.0.1:<port>/session -H 'Content-Type: application/json' \
1202	  -d '{\"directory\":\"'\$HOME'\",\"model\":{\"id\":\"<model>\",\"providerID\":\"<provider>\"}}'" \
1203	  | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])'
1204	```
1205	The command prints the id; write each one down as a literal next to its session name (a shell
1206	variable would not survive to the next command).
1207	
1208	**Verify:** every printed id is non-empty and starts with `ses_`. Write each one back into
1209	your in-memory/working copy of the config's `session_id` field (you'll persist the completed
1210	file in Stage 7) — every later stage that references a session needs the exact id, not a guess.
1211	
1212	**Gate:** an empty id means that entry's POST failed (wrong provider/model name, the model endpoint down
1213	for that host) — check the raw response body before moving on to the next entry.
~~~~

**E12: `agent-skills/herdr-setup/SKILL.md:1227-1244`** — SKILL.md Stage 6: the hub start, with `--advertise` and `tailscale serve`.

Command: `sed -n '1227,1244p' agent-skills/herdr-setup/SKILL.md | awk -v s=1227 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1227	**Do**, only if the plan called for a fresh hub (skip entirely if the ledger has a live `hub` row,
1228	which means the hub is reused) — bringing up the hub itself is host-agnostic, done exactly once
1229	regardless of how many remote hosts sessions are split across (token minting is per-host and
1230	happens in Stage 7). `mkdir -p <logs_dir>` first, then start it, printing the program's own pid:
1231	```bash
1232	{ HOLLER_STATE_DIR=<state_dir> nohup holler hub serve --listen 127.0.0.1:<hub_port> --advertise <hub_host>:<serve_https_port> > <logs_dir>/<prefix>-hub.log 2>&1 & echo $!; }
1233	tailscale serve --bg --https <serve_https_port> <hub_port>
1234	```
1235	Note the printed pid as a literal (it is not kept anywhere else). With no `hub_host` (N=0 and none in the config), leave `--advertise` out: `holler hub serve` then persists no advertise address and the hub advertises nothing remote, which is right when no body will dial it. `--advertise` carries the
1236	**serve port**: bodies dial `wss://<hub_host>:<serve_https_port>`, and a hub advertised without
1237	it would send them to port 443, which for a second instance is the first instance's serve
1238	endpoint. `<hub_port>` and `<serve_https_port>` are the instance's (defaults 41807 and 443); the
1239	hub's state — token store, pepper, identity key, control socket — is the instance's own
1240	`<state_dir>`, so a second instance beside this one shares none of it. Check the ledger first
1241	(see Stage 4, "Reuse on a rerun"): a `live` `hub` row means the hub is reused, do not start
1242	another. If the plan's `tailscale serve` is already in place for this exact port pair (and the
1243	ledger has a live `hub` row), leave it alone.
1244	
~~~~

**E13: `agent-skills/herdr-setup/SKILL.md:1373-1395`** — SKILL.md Stage 7: the body is joined and run on each remote host, from a derived config.

Command: `sed -n '1373,1395p' agent-skills/herdr-setup/SKILL.md | awk -v s=1373 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1373	3. **Join and run the body on this host**, using *this host's own* derived body config and *its
1374	   own* resolved `<state_dir>` (`mkdir -p <state_dir>` and `mkdir -p <logs_dir>` there first):
1375	   ```bash
1376	   scp <scratch>/<prefix>-sessions-<remote_host>.toml <remote_host>:<body_config>
1377	   ssh <remote_host> "HOLLER_STATE_DIR=<state_dir> holler body join --server wss://<hub_host>:<serve_https_port> --token <token_id>:<secret> --hub-key <hub_key>"
1378	   ssh <remote_host> "mkdir -p <logs_dir> && { HOLLER_STATE_DIR=<state_dir> nohup holler body run --config <body_config> --debug quiet > <logs_dir>/<prefix>-holler-body.log 2>&1 & echo \$!; }"
1379	   ```
1380	   The join URL carries the **serve port** (`<serve_https_port>`): `wss://<hub_host>` alone means
1381	   port 443, which for a second instance is the first instance's serve endpoint. `body join` and
1382	   `body run` get the **same** `HOLLER_STATE_DIR` on this host (its resolved `<state_dir>`): the
1383	   token's pairing is stored under it, and a body started with another directory finds no
1384	   pairing. With an absent `[instance]` table `<body_config>` is `~/sessions.toml` and the
1385	   state directory is the default one, as before. Note the pid the last command prints.
1386	
1387	   After the body shows up in `HOLLER_STATE_DIR=<state_dir> holler roster` (Verify, below),
1388	   **record** that pid on this host:
1389	   `record --pid <pid> --role body --stage 7 --session <the first session on this host>`
1390	   (remote form, Stage 4), and confirm with `list` that the recorded `cmd` starts with `holler`.
1391	   Reuse rule: a `live` `body` row for this host whose recorded `cmd` names the expanded path of
1392	   `<body_config>` (the recorded command line holds the path with `~` expanded, e.g. the host's
1393	   `$HOME/sessions.toml`, never the literal `~/sessions.toml`) means the body is reused, skip `body run`; a restarted wizard that finds the row `stale` runs it
1394	   again and records the new pid.
1395	
~~~~

**E14: `agent-skills/herdr-setup/SKILL.md:1412-1431`** — SKILL.md Stage 8: "Every Herdr command in this stage and in Stage 9 goes through herdr.sh".

Command: `sed -n '1412,1431p' agent-skills/herdr-setup/SKILL.md | awk -v s=1412 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1412	## Stage 8 — Build the Herdr workspace (M+N panes)
1413	
1414	**Ask:** nothing further — Stage 3's plan already said whether the Herdr server/workspace gets
1415	reused, extended, or built fresh. When the wizard creates a new workspace (below), Stage 3's plan
1416	named it and listed the restored panes beside it, and the "Proceed with this plan?" yes covers
1417	creating it; a "no" means stop, or reuse the restored panes only if the user asks.
1418	
1419	**Every Herdr command in this stage and in Stage 9 goes through `herdr.sh`** (in `$WIZARD_LIB`),
1420	written in full as `<herdr_env> bash $WIZARD_LIB/herdr.sh run <args>`, where `<herdr_env>` is the
1421	literal Herdr settings from Stage 1 (`WIZARD_INSTANCE_NAME`, `WIZARD_HERDR_SESSION`,
1422	`WIZARD_INSTANCE_PREFIX`, `WIZARD_STATE_DIR` and `WIZARD_LOG_DIR=<logs_dir>`, plus `HERDR_BIN` when
1423	`herdr` is off `PATH`). They are set inline on **every** command, never exported: a lost export
1424	would send a bare `herdr` command to the default workspace. `herdr.sh run <args>` runs
1425	`herdr --session <herdr_session> <args>`: the session flag is on **every** command, `server stop`
1426	and `session attach` included. **A bare `herdr server stop` is never issued, by this skill or by
1427	the wrapper**: which server it stops is not established, and it could stop another instance's.
1428	The wrapper refuses to run when `WIZARD_INSTANCE_NAME` is unset, and when the instance is not the
1429	default one and sets no `herdr_session`. In prose below, `herdr.sh run pane list` stands for that
1430	whole command.
1431	
~~~~

**E15: `agent-skills/herdr-setup/SKILL.md:1523-1563`** — SKILL.md Stage 8: the row-first build with `pane split`, and the labels through `herdr.sh label`.

Command: `sed -n '1523,1563p' agent-skills/herdr-setup/SKILL.md | awk -v s=1523 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1523	**Do (build the layout, row first)**, using Stage 1's resolved `layout` (the nested array, every
1524	orchestrator name included as a real slot alongside every session name — **not** an assumed
1525	single fixed orchestrator pane, and **not** raw `[[session]]`/`[[orchestrator]]` file order,
1526	which says nothing about columns). Flatten `layout` column by column, top to bottom within each
1527	column, into an ordered list of slots — each slot is either an orchestrator name or a session
1528	name. **The first slot in that flattened list is the pre-existing base pane** — it needs no
1529	split, it already exists; every other slot needs exactly one split to create its pane. Build
1530	the grid row first:
1531	- Create the first row by splitting right from the last pane of the row, one pane per column.
1532	- Then split each first-row pane down, once per further slot in its column, from the
1533	  most-recently-created pane in that column.
1534	
1535	The column-by-column order nests unevenly: Herdr nests every split inside the target pane's own
1536	cell, so a column split down before the next column is split right leaves the later columns
1537	spanning only part of the height.
1538	
1539	```bash
1540	# layout = [["o1"], ["alpha", "beta"]]  (this skill's example: o1 alone in column 1 — the
1541	# pre-existing base pane, no split — alpha/beta stacked in column 2)
1542	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <base-pane> --direction right   # -> alpha's pane (column 2, row 1), e.g. <p3>
1543	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <p3> --direction down              # -> beta's pane (column 2, row 2), e.g. <p4>
1544	
1545	# layout = [["o1"], ["alpha", "beta"], ["o2"], ["gamma", "delta"]]  (2 orchestrators, 4 sessions)
1546	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <base-pane> --direction right   # -> alpha's pane (col 2), e.g. <p3>
1547	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <p3> --direction right          # -> o2's pane (col 3), e.g. <p5>
1548	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <p5> --direction right          # -> gamma's pane (col 4), e.g. <p6>
1549	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <p3> --direction down              # -> beta's pane (col 2, row 2), e.g. <p4>
1550	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane split --pane <p6> --direction down              # -> delta's pane (col 4, row 2), e.g. <p7>
1551	```
1552	Keep a running map of pane id → slot name as you go (`<base-pane>` -> whatever slot 1 actually
1553	is, `<p3>` -> slot 2, and so on) — this is what both the orchestrator-launch step below and
1554	Stage 9's attach step key off, instead of assuming which pane is which.
1555	
1556	**Do (label every pane, right after the splits):** give each pane, orchestrator or session, its
1557	slot name as its label, through `herdr.sh label <pane-id> <label>` (not under `run`; the label
1558	matches `^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$`), once per pane and with the usual `<herdr_env>` in
1559	front, e.g. `<base-pane>` gets `o1` and `<p3>` gets `alpha`:
1560	```bash
1561	<herdr_env> bash $WIZARD_LIB/herdr.sh label <base-pane> o1
1562	<herdr_env> bash $WIZARD_LIB/herdr.sh label <p3> alpha
1563	```
~~~~

**E16: `agent-skills/herdr-setup/SKILL.md:1565-1575`** — SKILL.md Stage 8: Verify and Gate (pane count, labels, "unverified until #734 and #778").

Command: `sed -n '1565,1575p' agent-skills/herdr-setup/SKILL.md | awk -v s=1565 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1565	**Verify:** `herdr.sh run pane list` (count panes in that workspace only, when the wizard created one) shows exactly (M+N−1) new panes beyond the pre-existing base pane
1566	(M+N total in the workspace, M of which map to `[[orchestrator]]` entries and N of which map to
1567	real sessions), in the expected layout, and every pane's label equals its slot name. Then read
1568	`herdr.sh run pane list` again and tell the user if the widths look uneven: chained right-splits
1569	may leave columns at about 1/2, 1/4, 1/8, 1/8 of the width unless Herdr rebalances. This is
1570	unverified until the operator story #734 and the new-user run #778 check it with the real Herdr.
1571	
1572	**Gate:** if `herdr.sh run pane split` errors, or the pane count (in that workspace only) doesn't match M+N, stop and confirm
1573	the target workspace/pane with the user before retrying — don't chain more splits on a layout
1574	you haven't confirmed, and don't silently build fewer/more panes than the config calls for.
1575	
~~~~

**E17: `agent-skills/herdr-setup/SKILL.md:1628-1645`** — SKILL.md Stage 8: the orchestrator is launched by typing its command into the pane (`pane run`, `send-keys`).

Command: `sed -n '1628,1645p' agent-skills/herdr-setup/SKILL.md | awk -v s=1628 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1628	If it's genuinely idle, launch
1629	using that orchestrator's own `cmd` from its own `dir` (both read from the config file back in
1630	Stage 1 — that's where this is decided per orchestrator, never guessed here, and never assumed
1631	to be `claude` even though that's today's common case):
1632	```bash
1633	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane run <orch-pane> "cd <that orchestrator's dir> && export HOLLER_STATE_DIR=<state_dir> && <that orchestrator's cmd>"
1634	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane send-keys <orch-pane> enter    # `pane run` types the command; it does NOT press Enter
1635	```
1636	The `export` in the pane makes a bare `holler` the orchestrator types reach this instance's hub
1637	too (the briefing says the same in words). This is a **fresh session with equivalent working context** (the right directory, so any
1638	project `CLAUDE.md`/MCP config in scope) — **not** a resume of whichever session is driving
1639	this wizard right now. Don't claim it's "the same session" to the user; say plainly it's a new
1640	one rooted in the same project. Repeat this whole sub-step for every orchestrator — one launch
1641	per `[[orchestrator]]` entry, each in its own mapped pane, each with its own `dir`/`cmd`.
1642	
1643	**Verify**, for every orchestrator's pane:
1644	```bash
1645	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane read <orch-pane>
~~~~

**E18: `agent-skills/herdr-setup/SKILL.md:1654-1688`** — SKILL.md Stage 9: `opencode attach` typed into each pane, verified by `pane read`.

Command: `sed -n '1654,1688p' agent-skills/herdr-setup/SKILL.md | awk -v s=1654 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1654	## Stage 9 — Attach each pane to its real remote session
1655	
1656	Stage 9's Herdr commands use `<herdr_env> bash $WIZARD_LIB/herdr.sh run ...` exactly as Stage 8
1657	does (same inline settings, same session flag; `herdr.sh run session attach` for viewing). The
1658	instance's own Herdr server is never stopped through the wrapper: it is a ledger process and stops
1659	through `stop-owned.sh` (ownership rule), never `herdr.sh run server stop` and never a bare
1660	`herdr server stop`.
1661	
1662	**Ask:** nothing — you have the N session panes from Stage 8 (mapped to session names via
1663	`layout`, not raw file order) and the N session ids from Stage 5.
1664	
1665	**With no sessions, this stage does nothing: report it as skipped (⏭️), never as failed.**
1666	
1667	**Only implemented for `harness = "opencode"` today.** If a session's harness isn't `opencode`,
1668	this is a real gate — stop and tell the user this wizard doesn't have a live-view mechanism for
1669	that harness yet (its Holler-side session may still be real and working; it just has no known
1670	"watch it live in a pane" command here). Don't improvise an attach-like command for an
1671	unfamiliar harness.
1672	
1673	**Do**, for every (pane, `opencode`-harness session) pair in order — plain local panes running
1674	`opencode attach` against *that session's own* `remote_tailnet_host` (not a single shared
1675	value — a session on a different host uses a different URL here; `opencode attach --help`
1676	confirms the positional URL argument is an example, not a hostname restriction):
1677	```bash
1678	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane run <pane> "opencode attach http://<that session's remote_tailnet_host>:<port> -s <session_id>"
1679	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane send-keys <pane> enter    # `pane run` types the command; it does NOT press Enter
1680	```
1681	
1682	**Verify**, for every pane — use `herdr.sh run pane read <pane>` (not `herdr.sh run pane get`, which only
1683	returns metadata/title, not actual scrollback content) and confirm the title is a distinctive,
1684	real conversation identifier, not a generic shell prompt.
1685	
1686	**Gate:** any pane still showing a generic title/prompt means the attach didn't land — check
1687	you actually sent `enter` (the single most common miss) before assuming something deeper is
1688	wrong.
~~~~

**E19: `agent-skills/herdr-setup/SKILL.md:1698-1715`** — SKILL.md Stage 10: the round trip through `holler say`, and the `pane read` proof.

Command: `sed -n '1698,1715p' agent-skills/herdr-setup/SKILL.md | awk -v s=1698 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1698	**Do**, for every session in the config:
1699	(`<label>` is the Stage 7 token label.)
1700	```bash
1701	HOLLER_STATE_DIR=<state_dir> holler say <label>/<name> "reply with just: <name> ready"
1702	```
1703	
1704	**Verify:** the reply appears in the CLI output here for every session, regardless of harness.
1705	For `opencode`-harness sessions, additionally confirm it appears in the matching pane's real
1706	content: run `<herdr_env> bash $WIZARD_LIB/herdr.sh run pane read <pane>` and confirm your exact
1707	prompt text and its reply both appear, with a "Last finished HH:MM:SS" timestamp matching when you ran `say`. That match is
1708	what proves the pane is a genuine live view of the session Holler is driving, not a
1709	disconnected second conversation. Repeat for every session — a partial pass (2 of 3 verified)
1710	is a real gap for the untested one, not good enough. A non-`opencode` session that isn't Stage
1711	9-attached to any pane is verified on the CLI half alone — say so plainly rather than treating
1712	it the same as a fully pane-verified one. Also confirm every orchestrator pane from Stage 8 is
1713	still showing a live prompt, not a crashed/exited shell.
1714	
1715	**If Stage 3's AGENTS.md briefing was installed, prove it with a genuinely new session — a
~~~~

**E20: `agent-skills/herdr-setup/SKILL.md:1802-1822`** — SKILL.md "If a stage fails": teardown through `stop-owned.sh` on every host.

Command: `sed -n '1802,1822p' agent-skills/herdr-setup/SKILL.md | awk -v s=1802 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1802	- **Tearing the instance down:** first, the Herdr server the wizard started also holds any restored
1803	  workspace of the user (with any auto-resumed agent), and teardown stops that server: list those
1804	  panes (`pane list`) and ask before teardown. For the default instance, offer to leave the server
1805	  running and stop only the hub (`stop-owned.sh stop <state_dir> <hub pid>` instead of `teardown`). Otherwise run `bash $WIZARD_LIB/stop-owned.sh teardown <state_dir>` on
1806	  **every host the run touched** (this machine, and each distinct `remote_host` through the
1807	  remote form at the ownership rule, with that host's own `<state_dir>`). It stops only ledger
1808	  processes, in reverse start order (this includes the instance's Herdr server, so the named
1809	  Herdr session stops with it; what Herdr keeps of that session's saved layout is Herdr's own,
1810	  report it and do not delete it by hand), removes only that host's ledger, and prints what it
1811	  left (stale entries, foreign processes, other files, other instances). Pass `--purge-state`
1812	  only when the user asked to remove the instance's whole state directory; it is refused for the
1813	  default state directory. The instance's `tailscale serve` entry is not a process of the
1814	  wizard: it is the instance's own serve entry, so teardown leaves it and says so, and to continue
1815	  (a rebuild reuses it, Stage 3 reports it as the instance's own port pair) nothing more is needed;
1816	  to remove it, with the user's yes, turn off **only that entry**
1817	  with `tailscale serve --https=<serve_https_port> off`. Once the Herdr server has stopped, the
1818	  named session stays listed as `stopped`: with the user's yes, delete it with
1819	  `<herdr_env> bash $WIZARD_LIB/herdr.sh session-delete` (own session, stopped only), never with a
1820	  bare `herdr session delete`. Never `tailscale serve reset`: it wipes
1821	  every serve entry, the first instance's included.
1822	- **A join fails with `no such token` for one host:** that host's own token mint and its join
~~~~

**E21: `agent-skills/herdr-setup/lib/herdr.sh:8-48`** — herdr.sh: the header (what is UNVERIFIED until #734) and the verbs (`run`, `server-start`, `label`, `check-session`, `check-pane`, `session-delete`, `log-path`).

Command: `sed -n '8,48p' agent-skills/herdr-setup/lib/herdr.sh | awk -v s=8 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
8	#
9	# UNVERIFIED until operator story #734 checks it with the real binary: what `--session` does
10	# (which server and socket it selects), what a pane sees in HERDR_PANE_ID / HERDR_SESSION, and
11	# the `session list` status words used by session-delete. Do not trust any of it blind.
12	#
13	# bash 3.2 compatible (no associative arrays, no GNU-only flags): runs on Linux and macOS.
14	#
15	# Configuration (environment, set from the instance table by the skill):
16	#   WIZARD_INSTANCE_NAME    the instance `name`; REQUIRED. An unset or empty value is refused
17	#                           (exit 2): the literal "default" is the default instance
18	#   HERDR_BIN               the herdr binary when it is not on PATH (default: herdr)
19	#   WIZARD_LIB              where ledger.sh lives (default: this script's own directory)
20	#   WIZARD_HERDR_SESSION    the instance `herdr_session` (empty = no named session)
21	#   WIZARD_INSTANCE_PREFIX  the instance `prefix` (default: the instance name)
22	#   WIZARD_LEDGER           the ledger file (default: ${WIZARD_STATE_DIR}/herdr/setup/wizard-ledger.toml)
23	#   WIZARD_STATE_DIR        the instance state directory (default: $HOME/.holler)
24	#   WIZARD_LOG_DIR          where the server log goes (default: <state dir>/logs, created)
25	#
26	# Verbs:
27	#   run <herdr args...>   run `herdr --session <name> <args...>`
28	#   server-start          start the headless server in the background, logging to log-path,
29	#                         in a scrubbed environment (PATH HOME USER LOGNAME SHELL TERM LANG LC_*
30	#                         TMPDIR XDG_* HERDR_* plus the session set a pane needs: SSH_AUTH_SOCK
31	#                         DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS COLORTERM TZ EDITOR
32	#                         VISUAL LANGUAGE, the proxy variables in both cases and SSL_CERT_FILE
33	#                         SSL_CERT_DIR NODE_EXTRA_CA_CERTS; nothing else); prints the server's pid alone on the first
34	#                         stdout line
35	#   label <pane> <label>  rename a pane (`herdr pane rename`); label ^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$
36	#   check-session         refuse when the named session exists and the ledger did not create it
37	#   check-pane            refuse when this process runs in a pane of a different session
38	#   session-delete [name] delete this instance's own STOPPED session (`herdr session delete`);
39	#                         refuses a running or listed-live-in-the-ledger session, any other
40	#                         name, the default session and a session that is not listed
41	#   log-path              print the server log path for this instance
42	set -u
43	
44	die() {
45	  echo "herdr.sh: $*" >&2
46	  exit 1
47	}
48	
~~~~

**E22: `agent-skills/herdr-setup/lib/herdr.sh:209-229`** — herdr.sh: `run` passes any Herdr command through, and guards `pane split`, `pane run`, `pane send-keys` with `check_pane`.

Command: `sed -n '209,229p' agent-skills/herdr-setup/lib/herdr.sh | awk -v s=209 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
209	  run)
210	    [ $# -gt 0 ] || die "run needs a Herdr command"
211	    for a in "$@"; do
212	      case "$a" in
213	        --session | --session=*) die "do not pass --session; herdr.sh adds it" ;;
214	      esac
215	    done
216	    # Without a session name, commands that act on one server are refused outright.
217	    if [ -z "$session" ]; then
218	      case "$1 ${2:-}" in
219	        "server stop" | "session attach")
220	          die "'herdr $1 $2' without --session is never issued; configure herdr_session"
221	          ;;
222	      esac
223	    fi
224	    # Nothing that changes a layout runs from a pane of another session.
225	    case "$1 ${2:-}" in
226	      "pane split" | "pane run" | "pane send-keys") check_pane ;;
227	    esac
228	    herdr_cmd "$@"
229	    ;;
~~~~

**E23: `agent-skills/herdr-setup/lib/herdr.sh:261-278`** — herdr.sh: the `label` verb (`herdr pane rename`).

Command: `sed -n '261,278p' agent-skills/herdr-setup/lib/herdr.sh | awk -v s=261 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
261	  label)
262	    [ $# -eq 2 ] || die "usage: herdr.sh label <pane-id> <label>"
263	    [ -n "$1" ] || die "label needs a pane id"
264	    [ -n "$2" ] || die "label must not be empty"
265	    # Like the other pane verbs: never rename a pane from a pane of another session.
266	    check_pane
267	    # The character classes are spelled out: a bracket range follows collation in a UTF-8 locale
268	    # on bash 3.2, so a non-ASCII letter could pass a range such as A-Za-z.
269	    case "$2" in
270	      [ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789]*) ;;
271	      *) die "label '$2' is not valid (^[A-Za-z0-9][A-Za-z0-9._-]{0,31}\$)" ;;
272	    esac
273	    case "$2" in
274	      *[!ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-]*) die "label '$2' is not valid (^[A-Za-z0-9][A-Za-z0-9._-]{0,31}\$)" ;;
275	    esac
276	    [ "${#2}" -le 32 ] || die "label '$2' is longer than 32 characters"
277	    herdr_cmd pane rename "$1" "$2"
278	    ;;
~~~~

**E24: `agent-skills/herdr-setup/lib/instance.sh:160-185`** — instance.sh: each `[[session]]` backend port is its own `backend_port`, else base plus index.

Command: `sed -n '160,185p' agent-skills/herdr-setup/lib/instance.sh | awk -v s=160 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
160	    if (nondefault) {
161	        if (hub_port == D_hub_port) err("[instance] hub_port must not be " D_hub_port " (the default) for a non-default instance: another instance may own it")
162	        if (serve == D_serve) err("[instance] serve_https_port must not be " D_serve " (the default) for a non-default instance: another instance may own it")
163	    }
164	
165	    # Resolve each session backend port: its own backend_port, else base + index. With no
166	    # [instance] table the endpoint port is used, exactly as before this table existed.
167	    for (i = 1; i <= nsess; i++) {
168	        p = ""
169	        if (sport[i] != "") {
170	            if (substr(sport[i], 1, 2) == "n:" && substr(sport[i], 3) ~ /^[0-9]+$/ && length(sport[i]) <= 11 &&
171	                substr(sport[i], 3) + 0 >= 1 && substr(sport[i], 3) + 0 <= 65535)
172	                p = substr(sport[i], 3) + 0
173	            else err("session " sname[i] ": backend_port must be an integer from 1 to 65535")
174	        } else if (!has_instance && match(send[i], /:[0-9]+$/)) {
175	            p = substr(send[i], RSTART + 1) + 0
176	        } else {
177	            p = base + i - 1
178	            if (p > 65535) err("session " sname[i] ": backend port " p " is above 65535 (backend_port_base + index)")
179	        }
180	        rport[i] = p
181	    }
182	    allown = 1
183	    for (i = 1; i <= nsess; i++) if (sport[i] == "") allown = 0
184	    if (nondefault && !allown) for (i = 1; i <= nsess; i++)
185	        if (rport[i] != "" && rport[i] >= D_base && rport[i] < D_base + nsess)
~~~~

**E25: `docs/setup-wizard.md:1-12`** — docs/setup-wizard.md: the title and the first paragraph ("attach-mode sessions").

Command: `sed -n '1,12p' docs/setup-wizard.md | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	# Watching multiple attach-mode sessions in one Herdr workspace
2	
3	A pattern for driving several Holler-connected OpenCode sessions from one local terminal
4	workspace: one local orchestrator pane, plus one live view pane per remote session, all
5	watchable and interactable in one place — without giving up the fail-closed guarantees attach
6	mode already has ([ADR 0005](adr/ADR-0005.md)).
7	
8	This is not a Holler feature — Holler's CLI (`hub`, `body`, `say`, `roster`, …) is unaware
9	Herdr exists. It is a way of arranging terminal panes around a real Holler hub/body pair, using
10	[Herdr](https://herdr.dev), a terminal workspace manager. If you don't use Herdr, the hub/body
11	setup itself is unaffected; this page only concerns the *viewing* layer on top of it.
12	
~~~~

**E26: `docs/setup-wizard.md:136-154`** — docs/setup-wizard.md: "The viewing mechanism: plain panes, not herdr-mirror" (`opencode attach` panes, `herdr pane read`).

Command: `sed -n '136,154p' docs/setup-wizard.md | awk -v s=136 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
136	## The viewing mechanism: plain panes, not `herdr-mirror`
137	
138	**Use plain local Herdr panes running `opencode attach <url> -s <session_id>`, not
139	`herdr-mirror`.** `opencode attach` accepts any URL, not just `localhost` — bind the remote
140	`opencode` process to `--hostname 0.0.0.0` (safe on a tailnet-only host; loopback access for
141	the co-located Holler body keeps working too) and point `attach` at
142	`http://<remote-host>.<tailnet>.ts.net:<port>` over the tailnet. The result is an ordinary
143	local pane as far as Herdr is concerned — closing it, moving it, or mixing it with other local
144	panes carries no special risk, because there is no cross-machine mirror relationship to break.
145	
146	`herdr-mirror` (a plugin for live-streaming a remote *workspace*, not just a session) is a
147	different, heavier mechanism with real documented failure modes — including the mirror
148	daemon's own reconciliation killing the real remote processes it was only supposed to be
149	viewing, not just their local viewer panes.
150	The tradeoff for the simpler `opencode attach` approach: you lose Herdr's own agent-status
151	tracking (`idle`/`working`/`blocked`) for the remote session, since Herdr only classifies panes
152	it manages directly — a plain `opencode attach` pane is just a terminal running a program from
153	Herdr's point of view, invisible to `herdr agent list` even while fully live and working. Use
154	`herdr pane list` / `herdr pane read` to see it instead.
~~~~

**E27: `docs/setup-wizard.md:253-262`** — docs/setup-wizard.md: the bullet "Stage 8 labels and builds the grid".

Command: `sed -n '253,262p' docs/setup-wizard.md | awk -v s=253 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
253	- **Stage 8 labels and builds the grid.** Every pane gets its slot name as a label
254	  (`herdr.sh label <pane-id> <label>`, right after the splits) and Verify checks that each label
255	  equals its slot. The grid is built row first (split right along the first row, then each
256	  first-row pane down), because the column-by-column order nests unevenly. Its status check reads
257	  Herdr 0.8.x's `status: not running` / `status: running` line under `server:` and still accepts
258	  the older `server: not running`.
259	- **A resumed orchestrator can't prove the `AGENTS.md` briefing works.** It may already know about
260	  Holler from its own past turns. To test the briefing, exit that session, launch a fresh
261	  `claude` (no `--resume`), and ask it something with no Holler context, like "Send a hello to
262	  alpha" — it should find the roster, resolve the namespaced session name, and get a real reply.
~~~~

**E28: `docs/setup-wizard.md:465-475`** — docs/setup-wizard.md: "Automated setup", the stages in prose (remote OpenCode backends, session IDs, one body per remote host, `opencode attach` panes).

Command: `sed -n '465,475p' docs/setup-wizard.md | awk -v s=465 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
465	machine that doesn't have it). Stage
466	0 comes first and needs no config: it is read-only and machine-level, prints one status table of what
467	this machine has (`herdr`, `holler`, `tailscale`, `ssh`, the skill's `lib/` scripts, the Herdr
468	server and saved sessions, a hub running here, an existing ledger and config), then prints the
469	stage checklist as a vertical table. If `herdr` is missing it asks, as its own yes/no, whether to
470	install it (via [herdr.dev's install script](https://herdr.dev/install.sh) or `brew install herdr`) and
471	re-checks. It ends with a second, separate yes/no — "All systems go. Proceed?" — before anything past it runs; a yes to installing Herdr is never treated
472	as a yes to also building a workspace in the same run, and a no ends the wizard cleanly. Only once that second question gets a
473	yes does Stage 1 onward run: load the config described above, the config checks of Stage 2 (including per-host SSH
474	reachability, with a manual-relay fallback when no local `ssh` client exists), present the plan
475	and get explicit assent before touching anything, start the remote OpenCode backends, capture
~~~~

**E29: `README.md:150-157`** — README.md: the pointer to the wizard ("one local orchestrator plus remote attach-mode sessions").

Command: `sed -n '150,157p' README.md | awk -v s=150 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
150	**Attaching to a session you already have running** (no `sessions.toml` authored by hand at
151	all): see [Attach convenience](#attach-convenience) below — `holler body attach init` writes the
152	`[[session]]` row for you from a live OpenCode endpoint.
153	
154	For a single local orchestrator plus one or more remote attach-mode sessions in one terminal
155	workspace, see [Set up a Herdr workspace with an agent](#set-up-a-herdr-workspace-with-an-agent).
156	
157	## The most practical way to run Holler
~~~~

**E30: `README.md:238-244`** — README.md: the agent prompt's scenario menu (`cmd = "claude"`, remote attach-mode sessions).

Command: `sed -n '238,244p' README.md | awk -v s=238 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
238	2. Offer me a short choice instead of an open-ended question: (A) just a local orchestrator,
239	   nothing remote yet — propose `name = "r1c1"`, `dir` = my current directory, `cmd = "claude"`
240	   as one confirm-or-override, and compute `layout` for me; (B) that plus one or more remote
241	   attach-mode sessions — same orchestrator default, then ask only for what's genuinely
242	   unknowable per session (how many, each one's name, its host, its OpenCode port); (C) let me
243	   write or paste my own `sessions.toml` myself; (D) something more complex — a real interview,
244	   but still using Tailscale for discovery per step 1 wherever it applies.
~~~~

**E31: `docs/pane-control.md:22-35`** — docs/pane-control.md section 1: "The boundary" (a registry pane is an OpenCode pane; a Claude Code pane is outside the rule).

Command: `sed -n '22,35p' docs/pane-control.md | awk -v s=22 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
22	observed, never inferred from files (I6).
23	
24	**The boundary.** The rule is about registry panes. A registry pane is an OpenCode pane with a record: the record's
25	harness kind is a closed set with one member, `opencode`. A pane that does not run OpenCode (in the fleet, a pane that
26	runs Claude Code) has no record and can have none, so it is outside the rule: no `holler pane` verb restarts it, and
27	for a Claude Code pane `scripts/pane-restart` is still the way to restart it, now and after the cutover's sign-off.
28	The import (`pane import`) creates no record for a pane whose files name another harness (the finding `harness-not-opencode`,
29	section 10). A Herdr pane that no record names is **unmanaged**, a state that is observed and never stored:
30	`holler pane list` over the whole registry shows it in a table of its own (section 4), and `holler pane get PANE` shows it
31	when PANE is its label or its Herdr pane id. A verb that acts on a pane (`relaunch`, `switch`, `reset`, `park`, `unpark`,
32	`close`) refuses it with `pane-unmanaged` (exit 3), and a name that is neither a record nor an unmanaged pane is
33	`pane-not-found`. A whole-fleet `doctor` pass reports it as `unregistered-herdr-pane` (section 5). Sections 3, 4, 5, 10 and
34	11 repeat the consequence where it matters.
35	
~~~~

**E32: `docs/pane-control.md:77-103`** — docs/pane-control.md section 2: the environment variables, and `--herdr-session` stays required.

Command: `sed -n '77,103p' docs/pane-control.md | awk -v s=77 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
77	**What a verb needs.** Every `pane` and `profile` verb needs the hub: the registry lives there. With no hub running a verb
78	answers `unavailable` (exit 1) and its message names the missing hub; it touches no adapter. The hub is the only thing a
79	verb needs before it starts: Herdr, tmux and OpenCode are reached when a verb first uses them. So the read verbs
80	(`holler pane list`, `holler pane get PANE`, `holler pane watch`, `holler profile list`, `holler profile show NAME`) work
81	with no Herdr running. `pane list` over the whole registry and `pane get` of a name with no record also read Herdr, for
82	the unmanaged panes (section 4); without Herdr, `list` still lists every record and says on one `warning:` line that the
83	unmanaged panes are not listed.
84	
85	The Herdr session, the tmux socket and the OpenCode binary the adapters use come from environment variables. The one to
86	know is `HOLLER_HERDR_SESSION`, the Herdr session the panes live in (Herdr's own `HERDR_SESSION` is not read); with it
87	unset, each Herdr call answers `usage` naming it. A variable that is missing or malformed never stops a verb from
88	starting: the port it configures answers the error at its first use. The full list, with each variable's default, is the
89	module doc of `crates/holler-cli/src/pane/wiring.rs`, and this guide does not copy it.
90	
91	**`--herdr-session NAME` stays a required flag** of every verb that reaches Herdr: `launch` (unless `--spec-only`),
92	`pane import`, and `profile apply` when its plan has a create. `relaunch` has no such flag and acts in the Herdr session
93	its record names, which must be the adapter's session too. The flag is required even when `HOLLER_HERDR_SESSION` is set, and is not defaulted from it: the
94	variable configures the adapter, and the flag names the session the verb acts on. **The two must name the same
95	session**, and a mismatch is not answered the same way by every verb:
96	
97	- `launch` answers a session other than the adapter's with `unavailable` (exit 1), naming both sessions, and writes
98	  no record. In `profile apply` the same answer is a create step that fails, which stops the run (exit 1, section 6).
99	- `pane import` looks for each pane's label only in the session the flag names, and the adapter's snapshot holds only
100	  its own session, so with a mismatch it finds no Herdr pane at all: **every pane it would record is `pane-unlabelled`**
101	  (a problem). A dry run exits 0 listing them, and a real run is refused with `import-blocked`. Read a dry run in which
102	  every pane is `pane-unlabelled` as a session mismatch first (section 10, step 1).
103	
~~~~

**E33: `docs/pane-control.md:359-377`** — docs/pane-control.md section 3: `holler profile apply` and its steps.

Command: `sed -n '359,377p' docs/pane-control.md | awk -v s=359 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
359	### `holler profile apply NAME`
360	
361	Converge the live panes on a profile: create, adopt or relaunch, and never kill a healthy pane. Its flags are
362	`--dry-run` (print the plan, change nothing), `--take-over` (move a pane that belongs to another profile) and
363	`--herdr-session NAME` (needed when the plan creates a pane). The full reference is in section 6, which ADR-0021
364	section 8 ("Apply as built") and its section 9 apply row state as the contract.
365	
366	```bash
367	holler profile apply fleet --dry-run
368	holler profile apply fleet --herdr-session demo
369	```
370	
371	Each step is `{pane, action, outcome, code, diff, detail}`. The actions are `create`, `adopt`, `report`, `relaunch`,
372	`refuse` and `observe`; the outcomes are `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`, `skipped`
373	(after a failed step) and `planned` (a dry run's step that would act). A `refused` or `reported` step does not stop the
374	run or change the exit code; a `failed` step stops the run and exits 1. Its own open codes are
375	`orchestrator-not-relaunched` and `spec-not-launchable`. It exits 2 (`usage`) when the plan has a create and no
376	`--herdr-session`, 3 for `profile-not-found`, 1 for a failed step, and 0 otherwise.
377	
~~~~

**E34: `docs/pane-control.md:516-558`** — docs/pane-control.md section 6: the `profile apply` reference (the step table, refusals, `--dry-run`, exit codes).

Command: `sed -n '516,558p' docs/pane-control.md | awk -v s=516 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
516	### The `profile apply` reference
517	
518	The contract is ADR-0021 section 8, "Apply as built (#664)", with its definition of "adopt", and section 9's apply row.
519	**To adopt** a pane is to record that a pane which already has a record, is healthy and matches its spec belongs to P,
520	and to touch nothing live. Apply adopts records, never processes: a Herdr pane, a tmux session or a server that no
521	record names is never adopted (`pane import` is what records existing panes).
522	
523	Apply reads P and the registry, then observes every recorded spec pane. **Healthy** is a live observation, never the
524	stored health: the harness answers its health call, Herdr's snapshot lists the record's pane, and the TUI does not show
525	a session other than the session of record (a shown session that cannot be read does not count against it).
526	
527	One step per spec pane, in workspace, row and column order; the first matching row applies:
528	
529	| Row | Situation | Step |
530	|---|---|---|
531	| a | The pane's record belongs to another profile, and no `--take-over`. | `refuse`, outcome `refused` (`pane-in-other-profile`). |
532	| b | No record. | `create` through the launch transaction, outcome `created`. |
533	| c | Healthy and matches its spec. | `adopt`, outcome `adopted`: the record's profile becomes P (with `--take-over` from another profile, leave then join, two writes). |
534	| d | Healthy and differs from its spec. | `report`, outcome `reported`, with the pane's differences; nothing live is touched. |
535	| e | Not healthy, role orchestrator. | `refuse`, outcome `refused` (`orchestrator-not-relaunched`); nothing is written, even under `--take-over`. |
536	| f | Not healthy, role agent. | `relaunch`, outcome `relaunched`; under `--take-over` from another profile the leave and join happen first, then the relaunch. |
537	
538	- **A step** is `{pane, action, outcome, code, diff, detail}`; an absent `code`, `diff` or `detail` prints `null`. The
539	  actions are `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name)
540	  and `observe` (a recorded pane whose health or snapshot call failed). The outcomes are `created`, `relaunched`,
541	  `adopted`, `reported`, `refused`, `failed`, `skipped` (a step after a failed one) and `planned` (a dry run's step that
542	  would act; row d's stays `reported`). `detail` is the step's own line, inside `data`.
543	- **Refusals do not stop the run.** A `refused` or `reported` step leaves the exit code alone. A refusal from the launch
544	  transaction is a `refused` step with that code; a `usage` answer of the launch for a step's spec is refused with the
545	  open code `spec-not-launchable`.
546	- **A failure stops it.** A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the
547	  run, and the later steps are `skipped`. A run that ends with a `failed` step prints **only the failure**: in JSON the
548	  failure envelope, whose `error.code` is the failed step's code and whose message is
549	  `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`; in text one `error:` line on
550	  stderr and nothing on stdout. So `skipped` appears only in that count. A dry run with a failed step prints no plan.
551	- A step refused or failed after its launch or relaunch had begun carries the command that reconciles.
552	- **`--dry-run`** prints the plan as the same table (for a run that does not fail), runs no probe, issues no split and
553	  writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [...]}`, every key always present.
554	- **Exit codes:** 0 for a run with no failed step, 1 for a failed step, 2 for `usage` (a plan with a create and no
555	  `--herdr-session`, checked before any step, in a dry run too), 3 for `profile-not-found`.
556	- `orchestrator-not-relaunched` and `spec-not-launchable` are open codes of apply: refusals, exit 3, not in the closed
557	  list of section 9.
558	
~~~~

**E35: `docs/pane-control.md:588-612`** — docs/pane-control.md section 8: the health probe, the argv rule, "Herdr has no grid of its own", the Herdr version.

Command: `sed -n '588,612p' docs/pane-control.md | awk -v s=588 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
588	## 8. Probe, argv, layout, Herdr version
589	
590	**The health probe.** A pane spec can carry a health check, an argv run before a launch or relaunch acts, and strings its
591	output must contain. A check that fails refuses the run with `probe-failed` (exit 3) and changes nothing. For example,
592	`check ["curl","-s","http://127.0.0.1:8095/v1/models"]` with `expect ["qwen38"]`:
593	
594	```bash
595	holler pane relaunch demo-alpha --check-arg curl --check-arg=-s --check-arg http://127.0.0.1:8095/v1/models --expect qwen38
596	```
597	
598	**The argv rule.** A command and a health check are each an argv, given as repeated `--command-arg ARG` or one
599	`--command-json JSON` array, and `--check-arg ARG` or `--check-json JSON`; never both forms, and never a shell string.
600	A stored command that is a string is `command-not-argv`. A value that starts with `-` goes in the `=` form,
601	`--check-arg=-s`, so it is not read as a flag.
602	
603	**Layout.** Herdr has no grid of its own: a workspace's rows and columns are the Herdr adapter's configuration, and a
604	position is reached by right and down splits. A cell outside the workspace is `grid-out-of-range`; a cell inside it that
605	no single split reaches is the open code `grid-unreachable`. No verb relocates a healthy pane: `relaunch` moves a pane only
606	with `--grid`, and `launch` refuses an occupied cell (`grid-occupied`).
607	
608	**The Herdr version.** The Herdr adapter checks Herdr's API version when it connects and on every version call. An
609	unknown version is `herdr-version-unsupported`, whose message names the supported ones; no Holler verb upgrades Herdr.
610	`launch` and `relaunch` record the version they saw on the pane's record, and `holler pane doctor` prints Herdr's version
611	and each host's recorded one, and reports an unknown version as a finding (section 5).
612	
~~~~

**E36: `docs/pane-control.md:787-798`** — docs/pane-control.md section 12: "Later" (#666 and the others).

Command: `sed -n '787,798p' docs/pane-control.md | awk -v s=787 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
787	## 12. Later
788	
789	Deferred, so not documented here as existing; none of them is available:
790	
791	- the roster extension: the pane columns on `holler roster` (#648);
792	- applying a pane's model, effort and environment to its harness server (#829);
793	- DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
794	- an import that leaves the panes Holler does not drive alone (#835);
795	- the shared subprocess runner of the adapters and the probe (#696);
796	- the real-adapter scenario for `profile apply` (#667);
797	- recreating the Herdr workspace from the stored profiles (#666);
798	- the PROPOSED profile verbs and the PROPOSED doctor finding `profile-drift` (#665).
~~~~

**E37: `docs/adr/ADR-0021.md:866-885`** — ADR-0021 section 11: how attach mode (ADR 0014) relates; "stays for sessions Holler does not own".

Command: `sed -n '866,885p' docs/adr/ADR-0021.md | awk -v s=866 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
866	### 11. How attach mode (ADR 0014) relates
867	
868	[ADR 0014](ADR-0014.md) stays in force, unchanged. What changes is **who chooses the session**. For a pane in the registry,
869	Holler creates the session of record: the pane verbs start the harness server (the argv `HarnessPort::serve_command`
870	builds, which `HostPort::run` starts in the pane's tmux session, #802), create the session over its API (`create_session`)
871	and attach the TUI to it (`attach_tui`). The body that drives that session is still not its
872	parent, so ADR 0014's teardown rule (point 3) and its rule against typing into a PTY (point 4, which I4 restates for pane
873	verbs) apply as they are. The difference is that the session id comes from `session_of_record`, which a verb writes after it
874	observes the session. It is no longer pasted by an operator or guessed by a repair. That answers ADR 0014's open "automatic
875	discovery of attachable sessions (e.g. from a Herdr pane)" for registered panes: there is no discovery; the registry names
876	the session. Attach mode with an operator-written `endpoint` and `session_id` stays for sessions Holler does not own (a
877	session with no pane record).
878	
879	`HerdrPort::send_text` and `send_keys` exist in the provisional port. Under I4 no verb may use them to change which
880	session a pane shows or what it is doing; #640 uses them only in its tests, which type into panes they created in a scratch
881	Herdr session, #646 names the permitted uses, and #645's tests fail on any keystroke.
882	
883	**Deferred to #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each registered pane,
884	and when it moves after a switch or reset. The wiring (#649) wires the ports and leaves DRIVEN to the cutover.
885	Until the cutover wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the
~~~~

**E38: `docs/adr/ADR-0014.md:9-16`** — ADR 0014: attach mode, points 1 to 4 (the body is not the parent; nothing types into a PTY).

Command: `sed -n '9,16p' docs/adr/ADR-0014.md | awk -v s=9 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
9	## Decision
10	
11	A body config row may **attach** to a harness session that is already running under something else — a terminal-multiplexer pane, an operator's own `opencode serve` — instead of **spawning** and owning it. The two legacy repos wrote this as a matched pair of ADRs (one per side of the old split); this repo folds them into one, because in the single-binary architecture there is only one process making the choice.
12	
13	1. **Spawn remains the default.** A config row with `mode = "spawn"` or no `mode` at all behaves exactly as [ADR 0012](ADR-0012.md) describes: the body execs `command` as an ACP v2 subprocess it fully owns, and `detach` or process exit tears that child down. Nothing about spawn changes.
14	2. **Attach is a first-class second transport, not a fallback.** A config row with `mode = "attach"` names an `endpoint` and a `session_id`; the body drives that session over its existing HTTP/ACP control surface and is **not its parent**. `session/new` is never called in attach mode — if `session_id` is missing, or the endpoint 404s on a lookup for it, the body fails closed with a precise error rather than creating a consolation session.
15	3. **Teardown is mode-aware, and attach's teardown does the least, not the most.** `detach`, a dropped hub connection, or this process exiting must never kill or replace an attached session — only the local control channel (the body's own connection into the endpoint) closes. This is the entire point of the mode: two things cannot both parent one process, so whichever one the operator is already watching (a Herdr pane, a terminal) must be the side that owns it, and Holler's body must be able to leave without taking it down.
16	4. **Holler still does not become a terminal multiplexer.** Attach never types into a PTY and never streams pane bytes on the Holler wire (`herdr pane send-keys` or equivalent stuffing is explicitly out of scope). Prompt and interrupt go through the same ACP/HTTP control surface an attached endpoint already exposes — the same surface a spawn-mode body uses as its interrupt fallback — never through the terminal the human is watching.
~~~~

**E39: `crates/holler-adapter-herdr/src/plan.rs:1-26`** — plan.rs: the split model (an empty workspace starts at r1c1; rows come first; "A cell that no single split reaches is grid-unreachable").

Command: `sed -n '1,26p' crates/holler-adapter-herdr/src/plan.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The split planner (epic #633; spike section 7): the `right` and `down` splits that
2	//! take a workspace to one holding every cell asked for, or the reason Herdr cannot.
3	//!
4	//! Herdr places no pane at a cell. A pane is made by splitting one, and a split divides
5	//! only the split pane's own cell (the nesting rule). So the plan is built the way the
6	//! spike built its 2x4 grid (`scripts/spikes/herdr-grid.sh`), one new cell per step and
7	//! never a pane that was not asked for:
8	//!
9	//! 1. An empty workspace starts at `r1c1`, the root pane of `workspace.create`.
10	//! 2. New rows come first, top to bottom. Row `r` is a `down` split of `r(r-1)c1`, the
11	//!    first pane of the last row, and that row must hold one pane: a `down` split of a
12	//!    pane in a wider row nests inside its cell, which is not a grid.
13	//! 3. Then each row's new columns, left to right. `rRcC` is a `right` split of
14	//!    `rRc(C-1)`, which must be the last pane of its row.
15	//!
16	//! A split's `ratio` is the share of its cell that the split pane keeps (Herdr's
17	//! `first` child): `1/(rows - r + 2)` when it makes row `r`, and `1/(cols - c + 2)` when
18	//! it makes column `c`, where `rows` and `cols` are the extent's. A whole extent built
19	//! this way has equal rows and equal columns (the spike's recipe). Ratios never change a
20	//! position (`crate::layout`).
21	//!
22	//! The checks, in order:
23	//!
24	//! - A cell outside the extent is `grid-out-of-range`, before anything else.
25	//! - A cell that exists is answered with no step, even in a workspace that is not a grid.
26	//!   A cell asked for twice is planned once.
~~~~

**E40: `crates/holler-adapter-herdr/src/plan.rs:168-190`** — plan.rs: a new row only below a row of one pane ("place rows before columns").

Command: `sed -n '168,190p' crates/holler-adapter-herdr/src/plan.rs | awk -v s=168 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
168	    /// A cell of a row that does not exist yet. Only the first cell of the next row is
169	    /// one split away: a `down` split of the first pane of the last row, which must hold
170	    /// one pane.
171	    fn below(&self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
172	        let last = self.rows();
173	        let above = GridPos {
174	            row: cell.row.saturating_sub(1),
175	            col: 1,
176	        };
177	        if above.row != last {
178	            return Err(gap(cell, above));
179	        }
180	        let width = self.width(last).unwrap_or(0);
181	        if width != 1 {
182	            return Err(refuse(
183	                cell,
184	                &format!(
185	                    "a row can be added only below a row of one pane, and row {last} has {} \
186	                     (a down split there nests inside one cell, which is not a grid); place \
187	                     rows before columns",
188	                    count(usize::from(width), "pane")
189	                ),
190	            ));
~~~~

**E41: `crates/holler-pane/src/tx_apply.rs:21-28`** — tx_apply.rs: apply's steps run "in (workspace, row, col) order".

Command: `sed -n '21,28p' crates/holler-pane/src/tx_apply.rs | awk -v s=21 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
21	//! - **The steps**, one per spec pane in `(workspace, row, col)` order; the first row that
22	//!   matches applies. (a) A record of another profile, without `--take-over`: `refused`
23	//!   (`pane-in-other-profile`). (b) No record: `create`, by `tx_launch::launch`. (c) Healthy and
24	//!   matching its spec: `adopt`, which records the pane as a member of P and touches nothing
25	//!   live. (d) Healthy and differing: `reported`, with its row of `diff_profile`. (e) Not
26	//!   healthy, the orchestrator: `refused` ([`ORCHESTRATOR_NOT_RELAUNCHED`]). (f) Not healthy, an
27	//!   agent: `relaunch` in place, by `tx_launch::relaunch`. Under `--take-over`, a record of
28	//!   another profile leaves it and then joins P, two writes, before anything else rows c, d
~~~~

**E42: `crates/holler-pane/src/tx_apply.rs:329-345`** — tx_apply.rs: the plan is sorted by cell, row first.

Command: `sed -n '329,345p' crates/holler-pane/src/tx_apply.rs | awk -v s=329 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
329	/// Every spec's row (§3), in `(workspace, row, col)` order: P's order is not the steps' (C6).
330	/// The comparison is `diff_profile`'s, whose rows come one per spec in P's order, so they pair
331	/// with the specs; a sort by cell is stable, so specs of one cell keep P's order.
332	fn plan(profile: &Profile, records: &[Pane], observed: &Observed, take_over: bool) -> Vec<Planned> {
333	    let mut chosen: Vec<(&ProfileSpec, Planned)> = profile
334	        .panes
335	        .iter()
336	        .zip(diff_profile(profile, records))
337	        .map(|(spec, diff)| {
338	            let row = choose(spec, diff, records, observed, &profile.name, take_over);
339	            let pane = spec.pane.clone();
340	            (spec, Planned { pane, row })
341	        })
342	        .collect();
343	    chosen.sort_by(|(a, _), (b, _)| cell_key(a).cmp(&cell_key(b)));
344	    chosen.into_iter().map(|(_, planned)| planned).collect()
345	}
~~~~

**E43: `crates/holler-cli/src/pane/wiring/env.rs:158-174`** — env.rs: `HOLLER_HERDR_WORKSPACES` is `label=RxC`, comma-separated.

Command: `sed -n '158,174p' crates/holler-cli/src/pane/wiring/env.rs | awk -v s=158 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
158	/// `HOLLER_HERDR_WORKSPACES`: items `label=RxC` (rows by columns) joined by `,`, the empty value
159	/// being no workspace. `usage` naming the variable for the first item that is empty, has no `=`,
160	/// more than one `=` or no `x`, has an empty label or a count that is not a whole number from 1 to
161	/// 65535, or repeats a label.
162	fn workspaces(list: &str) -> Result<BTreeMap<String, Extent>, PaneError> {
163	    let mut workspaces = BTreeMap::new();
164	    if list.is_empty() {
165	        return Ok(workspaces);
166	    }
167	    for item in list.split(',') {
168	        let (label, extent) = workspace(item)?;
169	        if workspaces.insert(label.to_owned(), extent).is_some() {
170	            return Err(bad_workspace(item, "its label is given twice"));
171	        }
172	    }
173	    Ok(workspaces)
174	}
~~~~

**E44: `crates/holler-adapter-herdr/src/adapter.rs:178-199`** — adapter.rs: a workspace whose rows and columns are not configured is `unavailable`.

Command: `sed -n '178,199p' crates/holler-adapter-herdr/src/adapter.rs | awk -v s=178 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
178	    /// The configured extent of `spec`'s workspace, once `spec` is in the session
179	    /// served. Neither check sends a request.
180	    fn extent_of(&self, spec: &HerdrSpec) -> Result<Extent, PaneError> {
181	        if spec.session != self.config.session {
182	            return Err(PaneError::Unavailable {
183	                what: format!(
184	                    "Herdr session {:?}: this adapter serves only the session {:?}",
185	                    spec.session, self.config.session
186	                ),
187	            });
188	        }
189	        self.config
190	            .workspaces
191	            .get(&spec.workspace)
192	            .copied()
193	            .ok_or_else(|| PaneError::Unavailable {
194	                what: format!(
195	                    "Herdr workspace {:?} of session {:?} is not configured, so its rows and \
196	                     columns are not known",
197	                    spec.workspace, spec.session
198	                ),
199	            })
~~~~

**E45: `crates/holler-pane/src/tx_launch.rs:638-663`** — tx_launch.rs: the spec's `command` is a preamble run in the tmux session before the server starts.

Command: `sed -n '638,663p' crates/holler-pane/src/tx_launch.rs | awk -v s=638 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
638	/// A1 to O2 (a relaunch's B3 to O2), each after a budget check, and the budget check before R.
639	/// The spec's command and the server run in the spec's directory, whatever directory the tmux
640	/// session was made in; the server's step is `server::start`.
641	fn bring_up(
642	    ports: Ports<'_>,
643	    plan: &Plan<'_>,
644	    seen: &Seen,
645	    budget: &Budget,
646	    boot_poll: Duration,
647	    made: &mut Made,
648	) -> Result<Live, PaneError> {
649	    budget.check()?;
650	    let herdr = ports.herdr.ensure_pane(&plan.target)?;
651	    if !seen.listed.iter().any(|p| same_pane(p, &herdr)) {
652	        made.pane = Some(herdr.pane_id.clone());
653	    }
654	    budget.check()?;
655	    let cwd = &plan.spec.host.cwd;
656	    ports.host.ensure_session(plan.name, cwd)?;
657	    made.session = true;
658	    if let Some(command) = &plan.spec.command {
659	        budget.check()?;
660	        ports.host.run(plan.name, cwd, command)?;
661	    }
662	    server::start(ports, plan, seen, budget, boot_poll)?;
663	    budget.check()?;
~~~~

**E46: `crates/holler-body/src/server_address.rs:156-169`** — server_address.rs: a plaintext `ws://` to a loopback host is permitted.

Command: `sed -n '156,169p' crates/holler-body/src/server_address.rs | awk -v s=156 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
156	/// The one fail-closed transport rule: a plaintext `ws://` address that is not
157	/// loopback is refused. Returns `Some(reason)` (for a CLI exit 3) when the
158	/// address must not be connected, `None` when connecting is permitted
159	/// (a `wss` address, or a `ws` address to a loopback host).
160	pub fn loopback_only_check(addr: &ServerAddress) -> Option<&'static str> {
161	    if addr.scheme == "ws" && !addr.is_loopback() {
162	        Some(
163	            "a plaintext ws:// to a non-loopback host is refused; \
164	             off-loopback pairing is wss:// (ADR 0002)",
165	        )
166	    } else {
167	        None
168	    }
169	}
~~~~

**E47: `crates/holler-proto/src/vocab.rs:8-14`** — vocab.rs: the name grammar of a pane (lower-case letters, digits, `-` between word characters; 1 to 32 characters).

Command: `sed -n '8,14p' crates/holler-proto/src/vocab.rs | awk -v s=8 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
8	//! - [`SessionName`] — a session **within** a single body. 1–32 chars, **no
9	//!   `/`**.
10	//! - [`RoutableName`] — the cross-hub router: `<label>/<session>`, or a bare
11	//!   name. Each `/`-separated segment is 1–32 chars.
12	//!
13	//! Segment grammar (ADR 0005): lowercase ASCII letters and digits only; `-`
14	//! is allowed **between** word chars, but a segment must not start or end
~~~~

**E48: `docs/research/herdr-api-spike.md:104-110`** — Herdr spike: the default session's socket and a named session's socket differ.

Command: `sed -n '104,110p' docs/research/herdr-api-spike.md | awk -v s=104 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
104	**Wire** (VERIFIED, `herdr-ops.sh`):
105	
106	- A Unix stream socket, owner-only (`srw-------`). Default session: `$XDG_CONFIG_HOME/herdr/herdr.sock` (on Linux
107	  `~/.config/herdr/herdr.sock`); a named session: `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`.
108	  `herdr session list` prints each session's socket path. Inside a Herdr pane, `HERDR_SOCKET_PATH` names it.
109	- One JSON request per line, and **one request per connection**: a second request on the same connection is
110	  refused (`ConnectionResetError` / broken pipe). `events.subscribe` is the exception: the connection stays open
~~~~

**E49: `crates/holler-cli/tests/wizard/main.rs:1-27`** — The `wizard_scripts` module list (one module per story).

Command: `sed -n '1,27p' crates/holler-cli/tests/wizard/main.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! Tests of the setup wizard's helper scripts (`agent-skills/herdr-setup/lib/`), epic #726.
2	//!
3	//! One module per story, each in its own file and owned by that story alone:
4	//! `instance` (#727), `preflight` (#728), `start` (#729), `herdr` (#730), `ownership` (#731).
5	//! Run one with `cargo test -p holler-cli --test wizard_scripts <module>::`.
6	
7	mod herdr;
8	mod instance;
9	mod ownership;
10	mod preflight;
11	mod proof;
12	mod skilltext;
13	mod skilltext_772b;
14	mod skilltext_772a;
15	mod skilltext_772c;
16	mod skilltext_w3_stage1;
17	mod skilltext_w3_stage8;
18	mod skilltext_w4_stage2;
19	mod skilltext_838;
20	mod skilltext_w3_stage3;
21	mod skilltext_w4_stage8;
22	mod skilltext_w5_basics;
23	mod skilltext_w5_workspace;
24	mod skilltext_w6_preflight;
25	mod skilltext_w5_stage0;
26	mod skilltext_w5_rename;
27	mod start;
~~~~

**E50: `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:38-88`** — skilltext_w5_rename.rs: the rename scan over every tracked file, `docs/handoffs/` included (#848 is open).

Command: `sed -n '38,88p' crates/holler-cli/tests/wizard/skilltext_w5_rename.rs | awk -v s=38 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
38	/// rename note (it contains `~/.claude/skills/<old name>`).
39	fn scan_hits(needle: &str) -> Vec<String> {
40	    let listing = Command::new("git")
41	        .args(["ls-files", "-z"])
42	        .current_dir(root())
43	        .output()
44	        .unwrap();
45	    assert!(listing.status.success());
46	    let mut hits = Vec::new();
47	    for rel in String::from_utf8(listing.stdout).unwrap().split('\0') {
48	        if rel.is_empty() || rel.ends_with("skilltext_w5_rename.rs") {
49	            continue;
50	        }
51	        let Ok(text) = std::fs::read_to_string(root().join(rel)) else {
52	            continue; // deleted in the working tree, or not UTF-8
53	        };
54	        for (i, line) in text.lines().enumerate() {
55	            if line.contains(needle) {
56	                hits.push(format!("{rel}:{}: {line}", i + 1));
57	            }
58	        }
59	    }
60	    hits
61	}
62	
63	#[test]
64	fn no_old_command_or_directory_outside_the_rename_note() {
65	    let old = format!("{}-{}", "setup", "wizard");
66	    let note_marker = format!("~/.claude/{}", old_skill_path());
67	    for needle in [format!("agent-skills/{old}"), format!("/{old}")] {
68	        let bad: Vec<String> = scan_hits(&needle)
69	            .into_iter()
70	            .filter(|h| {
71	                // `docs/<old>.md` is the doc and keeps its name.
72	                let stripped = h.replace(&format!("docs/{old}"), "").replace(&format!("../{old}"), "");
73	                stripped.contains(&needle) && !h.contains(&note_marker)
74	            })
75	            .filter(|h| !h.contains(&format!(".config/{old}")))
76	            .collect();
77	        assert!(bad.is_empty(), "old name `{needle}` remains:\n{}", bad.join("\n"));
78	    }
79	}
80	
81	#[test]
82	fn rename_note_is_in_readme_and_docs() {
83	    let marker = format!("~/.claude/{}", old_skill_path());
84	    for rel in ["README.md", "docs/setup-wizard.md"] {
85	        let text = flat(&read(rel));
86	        assert!(text.contains(&marker), "{rel} lacks the rename note");
87	        assert!(text.contains("by hand"), "{rel} note must say to remove it by hand");
88	    }
~~~~

**E51: `crates/holler-cli/tests/wizard/skilltext.rs:144-168`** — skilltext.rs: every `holler` or `herdr` command in a code block is scoped to the instance; the word list has no `holler pane` or `holler profile`.

Command: `sed -n '144,168p' crates/holler-cli/tests/wizard/skilltext.rs | awk -v s=144 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
144	fn holler_and_herdr_commands_in_code_blocks_are_scoped_to_the_instance() {
145	    let words = [
146	        "holler say",
147	        "holler roster",
148	        "holler interrupt",
149	        "holler hub status",
150	        "holler hub serve",
151	        "holler body join",
152	        "holler body run",
153	        "herdr pane",
154	        "herdr workspace",
155	        "herdr status",
156	        "herdr server",
157	        "herdr session",
158	    ];
159	    for (name, text) in all_texts() {
160	        // The AGENTS.md briefing is deliberately unscoped: it must not carry the state directory
161	        // (the orchestrator's pane environment sets it), see #761.
162	        let text = text.replace(&briefing(), "");
163	        for line in code_lines(&text) {
164	            let hit = words.iter().any(|w| line.contains(w));
165	            let marked = line.contains("default instance only");
166	            assert!(
167	                !hit || scoped_to_instance(&line) || marked,
168	                "{name}: unscoped: {line}"
~~~~

**E52: `crates/holler-cli/tests/wizard/skilltext_772b.rs:32-70`** — skilltext_772b.rs: Stage 8's pinned text (labels, restored sessions, `workspace create`).

Command: `sed -n '32,70p' crates/holler-cli/tests/wizard/skilltext_772b.rs | awk -v s=32 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
32	fn stage8() -> String {
33	    section(&skill(), "## Stage 8 ")
34	}
35	
36	fn stage3() -> String {
37	    section(&skill(), "## Stage 3 ")
38	}
39	
40	#[test]
41	fn stage8_labels_every_pane_with_its_slot_name() {
42	    let s = flat(&stage8());
43	    assert!(
44	        s.contains("herdr.sh label <pane-id> <label>"),
45	        "the label call is missing"
46	    );
47	    assert!(s.contains("right after the splits"), "when to label");
48	    assert!(s.contains("slot name"), "what the label is");
49	    assert!(s.contains("every pane's label equals its slot"), "the Verify");
50	}
51	
52	#[test]
53	fn stage8_reads_the_status_line_of_herdr_0_8() {
54	    let s = flat(&stage8());
55	    assert!(s.contains("status: not running"));
56	    assert!(s.contains("status: running"));
57	    assert!(s.contains("under `server:`"));
58	    assert!(s.contains("`server: not running`"), "old text stays accepted");
59	    assert!(s.contains("stays accepted"));
60	}
61	
62	#[test]
63	fn stage8_handles_a_restored_saved_session() {
64	    let s = flat(&stage8());
65	    assert!(s.contains("pane list"));
66	    assert!(s.contains("the wizard did not create"));
67	    assert!(s.contains("never split or close them"));
68	    assert!(s.contains("workspace create"), "the new workspace beside them");
69	    assert!(s.contains("with the user's yes"));
70	    assert!(s.contains("only if the user asks"), "reuse is on request");
~~~~

**E53: `crates/holler-cli/tests/wizard/herdr.rs:727-760`** — herdr.rs: the `label` tests (a rename with the session flag first).

Command: `sed -n '727,760p' crates/holler-cli/tests/wizard/herdr.rs | awk -v s=727 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
727	fn label_renames_the_pane_with_the_session_flag_first() {
728	    let env = Env::new(&[]);
729	    let out = env.run("second", "second", &["label", "w1:p3", "r1c2"]);
730	    assert!(out.status.success(), "{}", text(&out));
731	    assert_eq!(
732	        env.recorded(),
733	        vec!["--session second pane rename w1:p3 r1c2".to_owned()]
734	    );
735	}
736	
737	#[test]
738	fn label_without_a_session_runs_a_bare_rename() {
739	    let env = Env::new(&[]);
740	    let out = env.run("default", "", &["label", "w1:p3", "a.b_c-1"]);
741	    assert!(out.status.success(), "{}", text(&out));
742	    assert_eq!(env.recorded(), vec!["pane rename w1:p3 a.b_c-1".to_owned()]);
743	}
744	
745	#[test]
746	fn label_refuses_empty_and_malformed_labels_without_a_call() {
747	    let env = Env::new(&[]);
748	    let long = "a".repeat(33);
749	    for bad in [
750	        "",
751	        "-x",
752	        ".x",
753	        "_x",
754	        "a b",
755	        "a/b",
756	        "a;b",
757	        "é",
758	        long.as_str(),
759	    ] {
760	        let out = env.run("second", "second", &["label", "w1:p3", bad]);
~~~~

**E54: `crates/holler-cli/tests/wizard/proof_fakes/holler:1-10`** — proof_fakes/holler: the fake `holler` knows `hub`, `body` and `roster` only.

Command: `sed -n '1,10p' crates/holler-cli/tests/wizard/proof_fakes/holler | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	#!/bin/bash
2	# Fake holler: hub serve|status|token mint, body join|run, roster. Everything a hub keeps (its
3	# info, identity key, token store, roster) lives under HOLLER_STATE_DIR; when it is unset the
4	# default is $HOME/.holler, which the scenario asserts is never created. Every call is logged
5	# with the state directory it carried, so the scenario can check that none went without one.
6	. "$FAKE_ROOT/common.sh"
7	echo "state=${HOLLER_STATE_DIR:-} args=$*" >>"$FAKE_ROOT/calls"
8	sd="${HOLLER_STATE_DIR:-$HOME/.holler}"
9	
10	need_hub() {
~~~~

**E55: `crates/holler-cli/tests/wizard/proof_fakes/holler:58-70`** — proof_fakes/holler: any other verb is "unsupported" (exit 2).

Command: `sed -n '58,70p' crates/holler-cli/tests/wizard/proof_fakes/holler | awk -v s=58 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
58	
59	case "$1 ${2:-}" in
60	  "hub serve") hub_serve "$@" ;;
61	  "hub status") need_hub; echo "listening: $HUB_LISTEN pid $HUB_PID" ;;
62	  "hub token")
63	    need_hub
64	    label="$(flag_value --label "$@")"
65	    echo "tok-$label:secret-$label" | tee -a "$sd/tokens" ;;
66	  "body join") body_join "$@" ;;
67	  "body run") body_run "$@" ;;
68	  "roster ") roster ;;
69	  *) echo "fake holler: unsupported: $*" >&2; exit 2 ;;
70	esac
~~~~

**E56: `crates/holler-cli/tests/wizard/proof.rs:40-52`** — proof.rs: `Host`, the scenario's private host (not `pub`).

Command: `sed -n '40,52p' crates/holler-cli/tests/wizard/proof.rs | awk -v s=40 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
40	// The host: one temp directory, the fakes, and the processes this test started.
41	// ---------------------------------------------------------------------------------------------
42	
43	struct Host {
44	    root: tempfile::TempDir,
45	    real_ps: PathBuf,
46	    ports: RefCell<Vec<u16>>,
47	    owned: RefCell<Vec<(u32, String)>>,
48	}
49	
50	impl Host {
51	    fn new() -> Host {
52	        let root = tempfile::tempdir().unwrap();
~~~~

**E57: `.aftersight/pipeline.config.json:1-20`** — pipeline.config.json: the test command and the test and production path globs (`agent-skills/**` is in neither).

Command: `sed -n '1,20p' .aftersight/pipeline.config.json | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	{
2	  "configShapeVersion": 1,
3	  "project": "holler",
4	  "description": "Rust workspace: the Holler hub, body and CLI, and its pane adapters (tokio, wss loopback fleet).",
5	  "test": {
6	    "unit": { "command": ["cargo", "test", "--workspace", "--", "--skip", "roster_stays_accurate_under_concurrent_body_load"], "timeoutMs": 900000 },
7	    "e2e": null,
8	    "pluginSuite": null,
9	    "pluginTypecheck": null
10	  },
11	  "paths": {
12	    "test": ["crates/*/tests/**", "plugins/*/tests/**"],
13	    "production": ["crates/*/src/**", "crates/*/Cargo.toml", "plugins/*/src/**", "plugins/*/Cargo.toml", "plugins/*/herdr-plugin.toml", "Cargo.toml", "Cargo.lock", "scripts/**"]
14	  },
15	  "toolchain": [
16	    {
17	      "name": "rust",
18	      "command": ["rustc", "--version"],
19	      "versionRegex": "rustc (\\d+\\.\\d+\\.\\d+)",
20	      "hint": "install Rust stable with rustup (rust-toolchain.toml pins the channel)"
~~~~

**E58: `scripts/lint.sh:43-52`** — lint.sh: the file-size gate counts `*.rs` under `crates/` only; 900 fails.

Command: `sed -n '43,52p' scripts/lint.sh | awk -v s=43 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
43	# 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
44	#    file cross 1k unnoticed).
45	while read -r n f; do
46	  if [ "$n" -ge 900 ]; then
47	    echo "lint: $f is $n lines — decompose before merging"
48	    fail=1
49	  elif [ "$n" -ge 600 ]; then
50	    echo "warn: $f is $n lines"
51	  fi
52	done < <(find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | grep -v ' total$')
~~~~

**E59: `crates/holler-cli/Cargo.toml:535-539`** — Cargo.toml: the `wizard_scripts` test target.

Command: `sed -n '535,539p' crates/holler-cli/Cargo.toml | awk -v s=535 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
535	# The setup wizard's helper-script tests (epic #726): ONE target, one module per story,
536	# so the stories add files and never edit this manifest again.
537	[[test]]
538	name = "wizard_scripts"
539	path = "tests/wizard/main.rs"
~~~~

**E60: `agent-skills/herdr-setup/SKILL.md:977-990`** — SKILL.md Stage 3: `collide.sh` refuses (REFUSED) a hub port that a hub the ledger did not record already listens on, a Herdr session of the same name, a taken port.

Command: `sed -n '977,990p' agent-skills/herdr-setup/SKILL.md | awk -v s=977 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
977	  no hub and no Herdr, so they belong to the local call only.
978	  Its output is the inventory printed beside what the run will create: everything that runs and
979	  was not created by this instance is listed as "present, not touched" (so is this instance's own
980	  `tailscale serve` entry, when the ledger has a live `hub` row). It exits 1 and prints a
981	  `REFUSED:` line — naming the colliding item and the config key to change — when the plan
982	  needs a port that is in use, a hub port a hub already listens on, a state directory another
983	  process uses, or a Herdr session of the same name. (It does not check bodies per session: a
984	  body's session is not on its command line; Stage 7's ledger rule covers bodies.) A planned
985	  Herdr session counts only when `inventory.sh` ran with `WIZARD_INVENTORY_HERDR=1` (Stage 2). On any
986	  refusal, show the refusals, **do not ask "Proceed with this plan?"**, and stop until the config
987	  is changed and Stage 1 onward is re-run. Never start, stop or kill the colliding thing to make
988	  room: it was not created by this run. A stop from Stage 2's `check-pane` or `check-session`, or
989	  from the remote `$HOME/.holler` comparison, is a refusal of the same kind: show it, do not ask
990	  "Proceed with this plan?". What to offer for each case:
~~~~

**E61: `crates/holler-adapter-herdr/src/lib.rs:1-4`** — The Herdr adapter implements `HerdrPort` over Herdr's local socket (epic #633 decision 3, that Herdr's server runs on the hub's machine, is not excerpted: its sentence names the operator's machines).

Command: `sed -n '1,4p' crates/holler-adapter-herdr/src/lib.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `holler_adapter_herdr` — the Herdr adapter: it implements `holler_pane::HerdrPort`
2	//! over Herdr's local socket, and is the only place that converts a `GridPos` to
3	//! Herdr's own order and base (epic #633, decision 7; ADR-0021 section 10).
4	//!
~~~~

**E62: `issue #633 body, lines 20-20`** — Epic #633 body, decision 9: the wizard builds mixed grids and every pane must register with the Holler server.

Command: `gh issue view 633 -R Performant-Labs/holler --json body -q .body | sed -n '20,20p' | awk -v s=20 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
20	9. **A pane can run a Claude Code CLI session as well as an OpenCode server, and Holler launches, registers, drives and observes both** (operator, 2026-10-11). Panes run Claude Code CLI sessions today, set up by hand for a subscription login, and Holler cannot see or drive them (the epic's open scoping question of 2026-10-11). The setup wizard builds mixed grids (OpenCode and Claude Code panes, an orchestrator at the last cell) and every pane must register with the Holler server. This is a contract amendment (amend-first rule): the spike #830 decides the mechanism, #831 carries the contract change and the adapter, #832 makes any pane Holler cannot drive visible as `unmanaged` meanwhile. A subscription login is never read, stored or printed (I7: env var names only). (amended 2026-10-11, claude)
~~~~

**E63: `docs/pane-control.md:274-281`** — docs/pane-control.md section 3: `holler pane import` adopts the old launcher's panes from `STATE_ROOT`.

Command: `sed -n '274,281p' docs/pane-control.md | awk -v s=274 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
274	### `holler pane import --from STATE_ROOT --herdr-session NAME`
275	
276	Adopt the panes the old launcher runs into the registry, as the profile `fleet` (section 10). Its full synopsis is
277	`holler pane import --from STATE_ROOT --herdr-session NAME [--launcher FILE] [--orchestrator PANE] [--dry-run]`:
278	`--launcher FILE` names the launcher table (default `STATE_ROOT/launcher.toml`), `--orchestrator PANE` (repeatable)
279	names the orchestrator's pane, since a role is never read from a name, and `--dry-run` reports and writes nothing.
280	`--herdr-session` is declared optional and is required at run time (`usage` without it). The import adopts what is
281	running: it never starts, stops, types into or moves anything, and deletes nothing from the source files.
~~~~

**E64: `docs/pane-control.md:319-329`** — docs/pane-control.md section 3: `holler profile create NAME [--from-current]`.

Command: `sed -n '319,329p' docs/pane-control.md | awk -v s=319 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
319	### `holler profile create NAME`
320	
321	Create a profile: empty, a detached copy of another with `--from PROFILE` (no pane joins), or with `--from-current` a
322	snapshot of every pane in the registry, each of which becomes a member. A pane that belongs to another profile refuses
323	the whole create (`pane-in-other-profile`).
324	
325	```bash
326	holler profile create fleet --from-current
327	```
328	
329	Ends with `profile-exists` (3), or `profile-not-found` (3) for `--from`.
~~~~

**E65: `docs/pane-control.md:697-710`** — docs/pane-control.md section 10: the import needs the launcher table `STATE_ROOT/launcher.toml` that no wizard file provides.

Command: `sed -n '697,710p' docs/pane-control.md | awk -v s=697 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
697	1. **Have the launcher table, then run the dry run.** The import reads `STATE_ROOT/launcher.toml`, or the file
698	   `--launcher FILE` names: one TOML table per pane, named as the pane, with `port` (1 to 65535), `cwd` (an absolute
699	   path) and `command` (an array of strings, an argv; a string is the finding `command-not-argv`). No other key is
700	   accepted. No file the old launcher wrote has this shape, so the operator provides it:
701	
702	   ```toml
703	   [demo-alpha]
704	   port = 8095
705	   cwd = "/srv/demo"
706	   command = ["opencode"]
707	   ```
708	
709	   **Without it every pane is `source-missing` (a problem)**, so the dry run reports it for every pane and a real run is
710	   refused with `import-blocked`. If the table has another shape, the dry run says so: `source-unreadable` for every
~~~~

**E66: `agent-skills/herdr-setup/SKILL.md:882-889`** — SKILL.md Stage 2: the OpenCode `<provider>/<model>` is read from each remote host's `~/.config/opencode/opencode.jsonc`.

Command: `sed -n '882,889p' agent-skills/herdr-setup/SKILL.md | awk -v s=882 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
882	**Resolving `<provider>/<model>` (do this once per distinct `remote_host`, here in Stage 2).**
883	Stages 4 and 5 need the OpenCode model a session should use, written `<provider>/<model>`
884	(e.g. the provider key from the config's `provider` block plus one of that provider's model
885	ids). Read it from the host's `~/.config/opencode/opencode.jsonc` — the top-level `model` key
886	if present, otherwise the single provider/model pair the `provider` block defines. If the config
887	doesn't name a default or defines several candidates, **ask the user once** which
888	`<provider>/<model>` to use for that host, and reuse the answer for every session on it. Never
889	invent one or copy a value from this document.
~~~~

**E67: `issue #769 body, lines 3-3`** — Epic #769 body: the operator's first run as a new user: scenario A, eight orchestrator panes in four columns of two.

Command: `gh issue view 769 -R Performant-Labs/holler --json body -q .body | sed -n '3,3p' | awk -v s=3 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
3	The operator ran the wizard as a brand-new user on a clean workstation: no Holler state, no config, scenario A of Stage 1 (local orchestrators only, no remote sessions), eight orchestrator panes in four columns of two. The run finished (hub up, Herdr workspace of eight panes, every pane running its orchestrator), but the skill and its scripts needed a human at seven points.
~~~~

**E68: `agent-skills/herdr-setup/SKILL.md:1297-1310`** — SKILL.md Stage 7: the derived body config (what it drops, what it sets, `<body_config>` on the remote host).

Command: `sed -n '1297,1310p' agent-skills/herdr-setup/SKILL.md | awk -v s=1297 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1297	Third, **for each distinct `remote_host`** (looping, not just doing this once):
1298	1. **Derive** the body config for this host, into `<scratch>/<prefix>-sessions-<remote_host>.toml`
1299	   (`<scratch>` from Stage 1). The derived body config holds `[[session]]` tables for *only this
1300	   host's* sessions (with their real `session_id`s), and — because the body denies unknown keys —
1301	   it **drops** the `[instance]` table and every session's `backend_port`, and **sets** each
1302	   session's `endpoint` to `http://127.0.0.1:<port>`, `<port>` being what `instance.sh` printed
1303	   for that session in Stage 1 (with no `[instance]` table that is the endpoint the session
1304	   already has). `remote_host`/`remote_tailnet_host`, `hub_host`, `[[orchestrator]]`, `layout` and
1305	   `ext` tables are ignored by the body, so they may stay or be dropped; dropping them keeps the
1306	   copy small. Re-read the copy before sending it: no `[instance]`, no `backend_port`, every
1307	   `endpoint` ending in its own `<port>`. Use *that* per-host copy for `scp`/`--config`, never a
1308	   copy containing another host's sessions. `<body_config>`, the path it has **on the remote
1309	   host**, is defined once, here: `~/sessions.toml` when the config has no `[instance]` table,
1310	   otherwise `<state_dir>/<prefix>-sessions.toml` (that host's resolved `<state_dir>`), so a
~~~~

**E69: `docs/pane-control.md:166-184`** — docs/pane-control.md section 3: `holler pane launch` (the flags, what a launch needs without `--profile`, `--spec-only`, `--herdr-session`).

Command: `sed -n '166,184p' docs/pane-control.md | awk -v s=166 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
166	### `holler pane launch PANE`
167	
168	Create a pane: its Herdr pane, its tmux session, its harness server and its session of record, then its record
169	(section 2). Its flags are one per field of a pane spec: `--workspace`, `--grid`, `--project`, `--model`, `--effort`,
170	`--role`, `--agent`, `--env` (a variable's name, never its value), `--ctx-soft`, `--ctx-hard`, `--port-policy`, the command
171	and the health check (section 8). Without `--profile` every position, directory, model, effort, ceiling and port-policy
172	flag is needed; with `--profile P` the profile's spec for the pane fills the ones not given. The one port policy is
173	`fixed:PORT`. `--herdr-session NAME` is required unless `--spec-only`.
174	
175	```bash
176	holler pane launch demo-alpha --herdr-session demo --workspace demo --grid r1c1 --project /srv/demo --model anthropic/claude-sonnet-5-5 --effort medium --ctx-soft 800000 --ctx-hard 900000 --port-policy fixed:8095
177	```
178	
179	The checks come first and change nothing when one fails: a name that has a record (`pane-exists`), a failing health
180	check (`probe-failed`), an unsupported Herdr (`herdr-version-unsupported`), an occupied cell (`grid-occupied`), a port a
181	server already answers on (`port-in-use`). Then `agent-key-invalid` and `agent-unusable` (section 2), the grid codes
182	(section 7) and the codes of `--profile` (`profile-not-found`, `pane-in-other-profile`, `profile-conflict`). Once the
183	live change has started, a failure's message ends with the command that reconciles.
184	
~~~~

**E70: `crates/holler-cli/src/pane/wiring/env.rs:49-66`** — env.rs: the Herdr adapter's configuration: the session, the socket (`HOLLER_HERDR_SOCKET`, else `HERDR_SOCKET_PATH`, else the session's default), the workspaces.

Command: `sed -n '49,66p' crates/holler-cli/src/pane/wiring/env.rs | awk -v s=49 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
49	/// The Herdr adapter's configuration (D5): the session `HOLLER_HERDR_SESSION` names, its socket,
50	/// the workspaces `HOLLER_HERDR_WORKSPACES` lists, and the adapter's own 10 s bound (D9).
51	///
52	/// The session is `HOLLER_HERDR_SESSION` only: Herdr's `HERDR_SESSION` is no fallback, since no
53	/// source shows that Herdr sets it in its panes. The socket is `HOLLER_HERDR_SOCKET`, else
54	/// `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock`, with
55	/// `$HOME/.config` for an `XDG_CONFIG_HOME` that is unset, empty or relative (the XDG rule).
56	///
57	/// `usage`, naming the variable: no session (or an empty one), a socket variable that is not an
58	/// absolute path, no absolute directory to find the session's socket under, and a malformed
59	/// workspace list: an item that is empty, has no `=`, more than one `=` or no `x`, has an empty
60	/// label or a row or column count that is not a whole number from 1 to 65535, or repeats a label.
61	/// The empty list is no workspace.
62	pub fn herdr_config(env: Lookup<'_>) -> Result<HerdrConfig, PaneError> {
63	    let session = env(HERDR_SESSION)
64	        .filter(|session| !session.is_empty())
65	        .ok_or_else(|| {
66	            usage(format!(
~~~~

**E71: `docs/pane-control.md:45-53`** — docs/pane-control.md section 2: "The model is recorded, not applied yet" (#829).

Command: `sed -n '45,53p' docs/pane-control.md | awk -v s=45 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
45	**The model is recorded, not applied yet.** A pane's model, effort, environment variable names and context ceilings are
46	written to its record (and to its profile's spec), but no port carries them to the harness server yet: the server a
47	launch or relaunch starts receives the agent only (below), and the model it uses is whatever its OpenCode
48	configuration selects. Applying them is issue [#829](https://github.com/Performant-Labs/holler/issues/829), not built. Until it lands, `--model` and `--effort` change
49	what the record says, not what the server runs (section 11 says what that means for a restart on another model).
50	
51	**The agent.** `--agent KEY` on `launch` and `relaunch` sets the OpenCode agent the server starts with as its default
52	agent. A key that is empty or holds anything but ASCII letters, digits, `-` and `_` is refused with `agent-key-invalid`
53	before anything is done, and the key is never echoed. A key the running server cannot use is refused with the OpenCode
~~~~

**E72: `docs/pane-control.md:437-460`** — docs/pane-control.md section 5: what a doctor pass prints (Herdr's version first) and the finding kinds, `unregistered-herdr-pane` among them.

Command: `sed -n '437,460p' docs/pane-control.md | awk -v s=437 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
437	## 5. doctor
438	
439	`holler pane doctor` runs one reconcile pass over the panes in scope (one pane, a profile's panes, or every pane) and
440	reports what differs between the records and what Herdr, tmux and the harness show. A pass first prints Herdr's version
441	and each host's recorded Herdr API version, then one line per finding, `<pane> <pos> <kind>: <message>`, followed by what
442	`--fix` did or could do in brackets and the command to run, then a summary. JSON mode carries the same as `data`.
443	
444	The finding kinds:
445	
446	| Kind | What it means | The command it names |
447	|---|---|---|
448	| `herdr-version-unsupported` | Herdr reports an API version the adapter does not know; no Holler verb upgrades Herdr. | none |
449	| `observe-failed` | A port call of the pass failed, so nothing was concluded from it; the message names the call. | `holler pane doctor` again |
450	| `unregistered-herdr-pane` | A Herdr pane that no record names. Whole-fleet passes only. | none |
451	| `herdr-pane-missing` | Herdr has no pane where the record says the pane is. | `holler pane relaunch PANE` |
452	| `tmux-session-missing` | The pane's tmux session does not exist. | `holler pane relaunch PANE` |
453	| `server-wedged` | The pane's harness server accepts a connection and never answers. | `holler pane relaunch PANE` |
454	| `server-down` | The pane's harness server fails its health check, and is not wedged. | `holler pane relaunch PANE` |
455	| `no-session-of-record` | The record names no session of record; doctor never picks one. | `holler pane reset PANE` |
456	| `session-of-record-missing` | The pane's server answered, and it does not have the session of record. | `holler pane reset PANE` |
457	| `tui-foreign-session` | The TUI shows a session its pane's own server does not have. | `holler pane relaunch PANE` |
458	| `shown-driven-mismatch` | The TUI shows another session than the session of record, or its home screen. | `holler pane doctor PANE --fix` |
459	| `stray-session` | A session on a pane's server that is the session of record of no pane. | none |
460	| `unmanaged-stalled` | An unmanaged pane whose screen has not changed for the threshold or longer. Whole-fleet passes with `--unmanaged-stalled-after SECONDS` only. | none |
~~~~

**E73: `docs/pane-control.md:473-476`** — docs/pane-control.md section 5: doctor's exit codes; "A script reads `data.findings`".

Command: `sed -n '473,476p' docs/pane-control.md | awk -v s=473 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
473	A script reads `data.findings`. The verb fails only when it cannot run the pass: `usage` (2) for a bad pane or profile
474	name, a refusal (3) for a pane or profile that does not exist or a pane outside the profile, a failure (1) for a store
475	that cannot be read.
476	
~~~~

**E74: `docs/handoffs/649-brief.md:58-58`** — The #649 brief, D20: rigor `second-opinion` as the operator's choice for the epic's remaining stories.

Command: `sed -n '58,58p' docs/handoffs/649-brief.md | awk -v s=58 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
58	D20. **Rigor** is `second-opinion` (the operator's choice for the epic's remaining stories).
~~~~

**E75: `agent-skills/herdr-setup/SKILL.md:1498-1521`** — SKILL.md Stage 8: a restarted Herdr server restores its saved session; the "reuse branch" re-attaches dead viewer panes only if the user asks.

Command: `sed -n '1498,1521p' agent-skills/herdr-setup/SKILL.md | awk -v s=1498 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1498	**But a Herdr that has been run before does not start blank — it restores its saved session,
1499	and that restore is more than pane structure.** Confirmed live 2026-09-23: after killing the
1500	server and starting a fresh one, `herdr.sh run pane list` came back with all 3 panes from the previous
1501	run already present, the orchestrator pane's Claude conversation **auto-resumed** (Herdr
1502	re-launched it via `claude --resume <id>` on its own), and the two session-viewer panes back as
1503	**dead shells** showing `opencode --session <old id>` → `Session not found` (their old
1504	`session_id`s no longer exist on the freshly started backends). So check `herdr.sh run pane list`
1505	before splitting anything. Those panes are the user's, not the wizard's, so they are reused only
1506	on request. **Reuse branch:** if the user asked to reuse them and the count equals M+N, reuse the
1507	restored structure — don't split again (that produces M+N+… panes and a wrong layout) — skip
1508	re-launching an orchestrator pane that resumed idle and healthy, and re-attach only the dead
1509	viewer panes in Stage 9 using this run's *new* session ids. If the user asked to reuse them and
1510	the count doesn't match, that's the "config changed" case in "If a stage fails" below. If the user
1511	did not ask, build a new workspace as above.
1512	
1513	If reusing an existing server (per Stage 3's plan; that means a workspace in it, never a split
1514	of its existing panes), check whether a workspace matching this
1515	run's target shape already exists (`herdr.sh run workspace list` / `herdr.sh run pane list`) before splitting
1516	more — a persisted session can survive a server restart with its pane *structure* intact even
1517	though the processes inside are gone (confirmed live 2026-09-22); reusing that structure is
1518	allowed only if the user asked for it, and then verify pane **count matches (M+N)** for *this* config, not an old run's count. If it
1519	doesn't match (the config has grown/shrunk since the layout was built), this should already
1520	have been flagged in Stage 3's plan — don't silently under- or over-provision panes relative to
1521	the current config.
~~~~

**E76: `docs/pane-control.md:484-489`** — docs/pane-control.md section 5: panes outside the registry are `unregistered-herdr-pane` in every whole-fleet pass, and "no Holler verb adopts it".

Command: `sed -n '484,489p' docs/pane-control.md | awk -v s=484 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
484	the import). Section 8 has the Herdr version.
485	
486	**Panes outside the registry.** Each Herdr pane that no record names (a Claude Code pane, a plain shell) is reported as
487	`unregistered-herdr-pane` in every whole-fleet pass, and no Holler verb adopts it. That is expected and is not a
488	failure. A pass is whole-fleet only when it names neither a pane nor a profile, so a pass that names one does not report
489	it, and the finding names no command. `holler pane list` shows the same panes as `unmanaged` (section 4).
~~~~

**E77: `agent-skills/herdr-setup/lib/stop-owned.sh:72-82`** — stop-owned.sh: teardown stops ledger pids in reverse of start order, "by stage, then pid".

Command: `sed -n '72,82p' agent-skills/herdr-setup/lib/stop-owned.sh | awk -v s=72 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
72	
73	# Recorded pids in start order: by stage, then pid (both numeric).
74	ledger_pids() {
75	  ledger_list "$1" | awk -F'\t' '$1 ~ /^[0-9]+$/ && $4 ~ /^[0-9]+$/ { print $4 " " $1 }' |
76	    sort -n -k1,1 -k2,2 | awk '{ print $2 }'
77	}
78	
79	reverse_lines() {
80	  awk '{ l[NR] = $0 } END { for (i = NR; i >= 1; i--) print l[i] }'
81	}
82	
~~~~

**E78: `crates/holler-cli/src/pane/get.rs:50-59`** — get.rs: `holler pane get NAME --format=json` prints `{"pane": RECORD, "profile", "spec", "sync"}`.

Command: `sed -n '50,59p' crates/holler-cli/src/pane/get.rs | awk -v s=50 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
50	/// escaped. Times are UTC. The command, the probe's argv and strings, and the spec are
51	/// printed as JSON.
52	///
53	/// `--format=json` prints one envelope whose data is `{"pane": RECORD, "profile": NAME or
54	/// null, "spec": SPEC or null, "sync": "ok", "mismatch" or "unobserved"}`, the four keys
55	/// always present. RECORD is the pane record as the registry holds it, with its model, env
56	/// names, context ceilings (`"context": {"soft": N, "hard": N}`), command and probe; an
57	/// unset optional field is left out. SPEC is the profile's spec for the pane, with its own
58	/// context ceilings.
59	#[derive(Args, Debug)]
~~~~

**E79: `agent-skills/herdr-setup/SKILL.md:1432-1470`** — SKILL.md Stage 8: the session check, the server check, `server-start` and the ledger record of the `herdr` row.

Command: `sed -n '1432,1470p' agent-skills/herdr-setup/SKILL.md | awk -v s=1432 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1432	**Do (session check, before anything is built):**
1433	```bash
1434	<herdr_env> bash $WIZARD_LIB/herdr.sh check-pane      # stops if this run is inside a pane of a DIFFERENT Herdr session
1435	<herdr_env> bash $WIZARD_LIB/herdr.sh check-session   # stops if the named session exists and the ledger did not create it
1436	```
1437	These ran in Stage 2 and passed or were refused in Stage 3; run them again here, since time has
1438	passed. If `check-session` stops, tell the operator the session's name and ask (Stage 3 lists what
1439	to offer, including `session-delete` for a stopped session); never split panes of a session this
1440	wizard did not create (it counts as created only when the ledger has a **live** `herdr` row for
1441	it). If `check-pane` stops, do not split anything. **For the default instance, a running unnamed
1442	Herdr server with no live `herdr` row is foreign** (Stage 3's question "build in it, or stop"):
1443	only with the user's yes to building in it does this stage use that server, and the wizard never
1444	stops it; without that yes, stop here and ask. "Build in it" means a new workspace in that server
1445	(`herdr.sh run workspace create`): never split a pane the wizard did not create. Read the new
1446	workspace's first pane id from `herdr.sh run pane list`, filtered by the new workspace id; that
1447	pane is `<base-pane>` below.
1448	
1449	**Do (server check, first, only if the plan called for a fresh server):**
1450	```bash
1451	<herdr_env> bash $WIZARD_LIB/herdr.sh run status
1452	```
1453	Herdr 0.8.x prints the line `status: not running` or `status: running` under `server:`; the
1454	older `server: not running` text stays accepted. If it is not running, start one headlessly (`herdr` alone needs a real TTY and will fail
1455	with "cannot attach without a usable terminal" when run non-interactively — use the headless
1456	form; its log is `<logs_dir>/<prefix>-herdr-server.log`, see `herdr.sh log-path`). The default
1457	instance starts an unnamed server; a named session starts a server for that session. The **first
1458	line** of `server-start`'s output is the server's pid:
1459	```bash
1460	<herdr_env> bash $WIZARD_LIB/herdr.sh server-start
1461	sleep 3
1462	<herdr_env> bash $WIZARD_LIB/herdr.sh run status   # should now show `status: running` under `server:`
1463	<herdr_env> bash $WIZARD_LIB/herdr.sh run pane list
1464	```
1465	**Restored saved session:** starting the server restores the previous saved session of that name
1466	(its workspaces and panes, and an agent in a pane may resume). When `pane list` shows panes the
1467	wizard did not create, never split or close them: report them to the user and, with the user's
1468	yes, build a **new workspace** beside them with `herdr.sh run workspace create` (then split and
1469	label only that workspace's panes), or reuse them only if the user asks (see the reuse branch
1470	below).
~~~~

**E80: `docs/pane-control.md:420-427`** — docs/pane-control.md section 4: SHOWN against DRIVEN (DRIVEN prints `-` until it is wired).

Command: `sed -n '420,427p' docs/pane-control.md | awk -v s=420 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
420	**SHOWN against DRIVEN.** The session of record is the session the hub drives (I2), so SYNC compares SHOWN with it. It
421	is `MISMATCH` when the TUI showed another session or its home screen, and `-` when reconcile has never observed the pane
422	or the pane has no session of record. DRIVEN is printed as the record holds it and is not compared: nothing records it
423	yet (the hub wiring, #649, leaves it as stored; making it follow each pane's session of record is
424	[#839](https://github.com/Performant-Labs/holler/issues/839), not built), so it prints `-`. `pane doctor` uses the same rule for `shown-driven-mismatch` (section 5).
425	
426	A `holler pane watch` line prints the same cells as `key=value`, the column name in lower case, after the pane's name:
427	
~~~~

**E81: `docs/pane-control.md:250-253`** — docs/pane-control.md section 3: `holler pane close` stops the pane's processes, closes its Herdr pane and removes its record.

Command: `sed -n '250,253p' docs/pane-control.md | awk -v s=250 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
250	### `holler pane close`
251	
252	Close a pane: stop its processes (its harness server among them), close its Herdr pane and remove its record. With
253	`--profile P` it also removes P's spec for the pane; `--spec-only` removes the spec and touches nothing live.
~~~~

**E82: `agent-skills/herdr-setup/SKILL.md:325-332`** — SKILL.md: "Stage 3 is a hard stop": nothing that changes real state may happen before the user has said to proceed.

Command: `sed -n '325,332p' agent-skills/herdr-setup/SKILL.md | awk -v s=325 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
325	**Stage 3 is a hard stop, not just a gate like the others: nothing that changes real state
326	(starting a process, killing one, joining, splitting a pane) may happen before the user has
327	explicitly said to proceed.** Stage 0 (apart from installing Herdr or the skill, each on its own yes) and Stages 1 and 2 are read-only except the scratch directory and the config file you write from the user's answers (loading a file,
328	checking status; the only things they write are `<scratch>` and that config file, and installing the skill needs its own
329	yes, see Stage 0) so they're safe to run ahead of that; everything from Stage 4 onward is not.
330	
331	Keep a running status checklist as you go, as a **vertical table** (one row per stage, columns
332	Stage, Name, Status), not an inline list and not a dense bracket-number line (a human reading it
~~~~

**R1: `docs/handoffs/831-brief.md at d137a1d, lines 5-13`** — REFRESH AFTER MERGE of #831. #831's brief (branch issue-831-implementation, d137a1d): the goal and Work 5, "The wizard (story #803) uses it for the Claude panes of a mixed grid".

Command: `git show d137a1d:docs/handoffs/831-brief.md | sed -n '5,13p' | awk -v s=5 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
5	## Goal
6	`holler pane launch` and `relaunch` accept a harness choice of OpenCode or Claude Code (`--harness`). A Claude Code pane is a registry pane like any other: a record in the hub's pane registry with the session of record, a harness health, `SHOWN` and `DRIVEN`, `hold`, a `profile` membership, and the same verbs (`list`, `get`, `watch`, `switch`, `reset`, `park`, `say`, `interrupt`, `close`, `doctor`). A profile spec names the harness, so `holler profile apply` can build a grid that mixes OpenCode and Claude Code panes, with the orchestrator role on any of them.
7	
8	## Work (shaped by the spike's answers; the spike decides the mechanism)
9	1. The contract amendment, first: `HarnessKind` gains a variant (and `holler_proto::vocab::HARNESS_IDS` the id; a test pins both lists together), the `harness` field of `Pane` and `ProfileSpec`, ADR-0021 and the epic's contract block edited in one PR.
10	2. The `HarnessPort` impl for Claude Code in a new adapter crate or module, behind the same seam as the OpenCode adapter; the host adapter starts and stops the process by the recorded pid.
11	3. The test kit: a fake Claude Code harness and its conformance suite; the real-tool test is opt-in and `#[ignore]`.
12	4. `--harness` in the spec flags (declared once in `cli.rs`), `profile show` and `apply` compare it, doctor codes for a harness that cannot be reached.
13	5. The wizard (story #803) uses it for the Claude panes of a mixed grid.
~~~~

**R2: `docs/adr/ADR-0021.md at d137a1d, lines 1050-1079`** — REFRESH AFTER MERGE of #831. ADR-0021 section 15 as committed on #831's branch (d137a1d): items 1 and 2 (the kind; one command name) and item 3 (the endpoint).

Command: `git show d137a1d:docs/adr/ADR-0021.md | sed -n '1050,1079p' | awk -v s=1050 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1050	1. **The kind.** `HarnessKind` gains `Claude` (serde `claude`). `holler_proto::vocab::HARNESS_IDS` already lists `claude`, so
1051	   no id is added and the test that pins every `HarnessKind` serde name to that list covers the new variant. The id has two
1052	   uses that do not meet: a **pane** harness `claude` is the interactive CLI and is read only by the pane engine; a **body**
1053	   harness `claude` (ADR 0012) is the ACP bridge and is read only by `holler-body`. The doc comment of `HARNESS_IDS` says so.
1054	   `HarnessKind` gets two pure methods, the only place these facts live (a port method could be contradicted by a fake, and
1055	   one source of truth is the epic's rule): `shape()`, `Shape::ServerAndTui` for `opencode` and `Shape::SingleProcess` for
1056	   `claude`; and `abort_style()`, `AbortStyle::InPlace` for `opencode` and `AbortStyle::Relaunch` for `claude`.
1057	2. **One command name.** `pub const CLAUDE_CODE_COMMAND: &str = "claude"` and `pub fn is_claude_code_command(name)` in
1058	   `holler-pane/src/pane.rs`, beside `HarnessKind`, are the one place the name is written and the one rule that compares a
1059	   foreground command name with it (#835's import, a Claude launch's `process-in-use`, #832's unmanaged panes). **One signal:** the read-only `HostPort::foreground_commands(name) ->
1060	   Result<Vec<String>, PaneError>` that #835's amendment decides (required, no default, tmux's `#{pane_current_command}`, names
1061	   only). This story adds no second signal and no second constant. **Order:** whichever of #835 and #831 lands second adopts
1062	   the other's constant and method. If #835 has not merged when this story reaches its adoption step (item 17), this story
1063	   builds the method and the constant to #835's brief, exactly (signature, required, `FakeHost`, conformance case, the forwards
1064	   of the wiring's decorators), and #835 adopts them. **As built:** #835 had not merged when this story's F ran, so it copied
1065	   #835's committed code and tests for exactly those (its branch at `9bd101b` and `36b0de8`: the constant and the rule, the
1066	   required method, `TmuxHost`'s and `FakeHost`'s implementations, the host suite's case 10, the decorators' forwards), text
1067	   for text, so that whichever of the two lands second merges identical hunks. `HerdrPort::observe_pane`'s loss of its
1068	   default, also on #835's branch, is #835's and is not copied.
1069	3. **The endpoint.** A Claude Code process has no port. `holler_pane::ports` gains
1070	   `enum HarnessEndpoint { Port(u16), Process { pane: PaneName, pid: u32 } }`, and **every `HarnessPort` method keyed by
1071	   `port: u16` is keyed by `&HarnessEndpoint` instead** (`verify_serving`, `health`, `create_session`, `list_sessions`,
1072	   `abort`, `send_prompt`, `session_activity`; `attach_tui` and `shown_session` gain it too, `shown_session(pane, endpoint)`,
1073	   since a single process is read through its endpoint and not through a TUI). This changes the port rather than adding
1074	   default-armed twins, as #802 decided (decision 2 there): a call site that forgets the endpoint is a compile error, and
1075	   every implementation is in this workspace. `HarnessInfo.port` becomes `Option<u16>` (`#[serde(default,
1076	   skip_serializing_if = "Option::is_none")]`), so every stored `opencode` record reads and writes the same bytes. A record
1077	   is valid only when `opencode` has a `port`, and `claude` has no `port` and a `pid`; anything else does not load
1078	   (`deny_unknown_fields` and the fail-closed registry, section 7). `HarnessInfo::endpoint(&self, pane: &PaneName)` is the
1079	   one place a record becomes an endpoint.
~~~~

**R3: `docs/adr/ADR-0021.md at d137a1d, lines 1195-1238`** — REFRESH AFTER MERGE of #831. Section 15 items 10 to 13 (d137a1d): the login, spec fields, `--harness` and the port policy `none`, the doctor findings.

Command: `git show d137a1d:docs/adr/ADR-0021.md | sed -n '1195,1238p' | awk -v s=1195 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1195	10. **The login (I7).** A subscription login is never read, copied, stored or printed: only environment variable **names**
1196	    appear in any record, spec, argv, message or file. The process needs `HOME` and `PATH` and nothing else for the login. The
1197	    names that silently change it, `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_AUTH_TOKEN`,
1198	    `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX` and `CLAUDE_CODE_USE_FOUNDRY`, are (a) unset in the
1199	    launch argv with `env -u NAME` for each, and (b) refused in a spec's `env` at the plan as `claude-login-env-refused` (open,
1200	    exit 3; the message names the variable name, never a value). `login_status()` runs `claude auth status` and reads the
1201	    `authMethod` field only: `claude.ai` is `Ok`, anything else `Missing`. Nothing else of its output is kept or shown.
1202	    **As built:** the names are a fact of the kind, `HarnessKind::login_variables()` (`CLAUDE_LOGIN_VARIABLES` in `pane.rs`),
1203	    which the adapter's `env -u` list and the engine's plan both read, so the refusal `claude-login-env-refused` is the launch
1204	    engine's, raised in its plan before any port call, not the adapter's. The adapter's own `claude agents --json` and `claude
1205	    auth status` runs remove the same names from their environment, so the login they report is the one the panes use; an
1206	    answer with no `authMethod` is `unavailable`, quoting none of it, never `Ok`.
1207	11. **Spec fields and launch defaults in this story.** No new field of `Pane` or `ProfileSpec`. A Claude pane's `model` becomes
1208	    `--model model_id` and `--effort effort` (the provider is recorded and not passed). `--disallowed-tools AskUserQuestion` is
1209	    in every Claude launch (the one wait the hub cannot answer); a spec field that turns it off, a permission-mode field and an
1210	    allowed-tools list are a later story, so a pane launches in Claude Code's default permission mode and the hub never chooses
1211	    to bypass permission prompts. `--agent KEY` on a Claude pane is refused `agent-unsupported` (plan time): Claude's `--agent`
1212	    exists, and applying and checking it is a later story. `ServeSpec` stays the carrier (its doc: a new field of the spec for a
1213	    server goes there), and `process_command` takes it.
1214	12. **The flag and the policy.** `--harness opencode|claude` is declared once in `SpecFlags` (`cli.rs`), on `launch` and
1215	    `relaunch` and in the profile spec edits, as `harness.kind` of the spec. A new spec without it is `opencode`; an existing
1216	    spec keeps its own; a `relaunch` that names a different harness from the record is refused `harness-change-refused`
1217	    (open, exit 3: close the pane and launch it anew). `--port-policy` for `claude` is the word `none` (the policy grammar
1218	    gains it; `fixed_port_policy` of a claude record is `none`, so a spec round-trips), and any other policy, or `none` for
1219	    `opencode`, is `port-policy-unsupported` (open, exit 3, plan time). `profile show` and `profile apply` compare
1220	    `harness.kind` like any field (`profile_diff` already lists it) and never rewrite a live pane's harness. **As built (plan
1221	    review W13):** `--harness` is declared in `SpecFlags` (`holler-cli/src/pane/args.rs`, not `cli.rs`), typed by `validate`
1222	    (anything but `opencode` or `claude` is `usage`, and the text is never echoed), and absent unless given. The one policy
1223	    rule is `tx_launch::port_for(kind, policy)`: outside the grammar is `usage`, a policy the kind does not take is
1224	    `port-policy-unsupported`; `pane launch`'s check of its flags reports only the first. A record's snapshot names its policy
1225	    through `profile_snapshot::port_policy_of` (`none`, `NO_PORT_POLICY`, for a harness with no port). `profile apply` refuses
1226	    a recorded pane whose spec names another harness, `harness-change-refused`, before it judges the pane's health, and
1227	    `HarnessInfo` checks its fields against its kind through `#[serde(try_from = "RawHarnessInfo")]`, as `Profile` checks its
1228	    slug.
1229	13. **Doctor.** `FindingKind` gains `claude-login-missing` (a problem; `login_status` is `Missing`, reported once per run when
1230	    any recorded pane is `claude`, and its remedy names the login, never a command that reads one) and `pane-waiting` (a
1231	    warning; the entry is `waiting`, with the reason `waitingFor` gives). `server-down` is reused for "no `agents --json` entry
1232	    for the recorded pid" (the Claude process is the server). `shown-driven-mismatch` and the repair of a shown session that is
1233	    not the session of record are as section 8, with the repair a relaunch for a `SingleProcess` kind. **Not in this story:**
1234	    `pane-stalled` (a `busy` pane whose transcript has not grown). It needs a time that the hub observed, which the CLI-side
1235	    verbs do not keep and a registry field is forbidden (I6); the same missing source as #837, so it waits with it. **As
1236	    built:** `claude-login-missing` is about no pane, has no remedy (no Holler verb sets a login), and is asked once per pass
1237	    only when a pane in scope is a single process (a fleet with none never asks the Claude harness anything);
1238	    `pane-waiting` names `permission prompt` or `question (input needed)`, the activity's reason, and has no remedy.
~~~~

**R4: `docs/adr/ADR-0021.md at d137a1d, lines 1265-1311`** — REFRESH AFTER MERGE of #831. Section 15 items 16 to 18 and "Not built here" (d137a1d): the wizard, adopting existing Claude Code panes, and why `say` needs #839 and #850.

Command: `git show d137a1d:docs/adr/ADR-0021.md | sed -n '1265,1311p' | awk -v s=1265 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1265	16. **The wizard.** This story provides `--harness`, the spec's `harness.kind` and a mixed grid in `profile apply`. The wizard
1266	    (#803) is open and not built: its use of the flag for the Claude panes of a mixed grid lands with #803, not here.
1267	17. **Adopting the fleet's existing Claude Code panes (operator decision, 2026-10-11, recorded on #831).** Decided: **a re-run of
1268	    the import (`holler pane import --from STATE_ROOT --herdr-session NAME`) with `--adopt-claude-code`** adds them, with no
1269	    new verb. #835 makes the flag a refused argument (`usage`, naming #831) until this harness exists, and this story replaces
1270	    that one place. After it:
1271	    - The import classifies a pane by item 2's signal: a pane whose session has a live foreground command named
1272	      `CLAUDE_CODE_COMMAND` (`is_claude_code_command`) is a Claude Code pane; any other answer is OpenCode, as #835 decided.
1273	    - **The write step extends an existing profile.** Where the profile the import names (default `fleet`) exists, the import
1274	      still creates no profile; with `--adopt-claude-code` it adds the adopted panes' specs to it by one compare-and-swap, and
1275	      records their panes. A pane that already has a record is reported `already-registered` (info, skipped) and is never
1276	      touched, so a re-run is safe and idempotent. Without the flag, an existing profile is `profile-exists` as today.
1277	    - **The import acts on nothing live.** The record holds `kind: claude`, the pid and the session of record read from
1278	      `agents --json` for that pid (matched by pid, never by directory), and health `unknown`. A process Holler did not
1279	      start lacks the launch's `crossSessionInbound: accept` and its socket hook, so delivery to it would be held, and Holler
1280	      cannot see its argv. So an adopted pane's report ends with the next step, `holler pane relaunch NAME`, which restarts it
1281	      under Holler with `--resume <session of record>` and keeps its conversation; `doctor` reports `pane-waiting` if it is
1282	      left as it was and a message is held. This is the one adoption path; a separate verb was declined because relaunch is
1283	      already the verb that puts a pane under Holler.
1284	    - Order: this is the last item of the story. If #835 is not on `main` when it is reached, the story stops there and its PR
1285	      says "Part of #831", naming what remains.
1286	    - **As built: not yet.** #835 was not on `main` when this story reached the step, so the story stopped there: its PR says
1287	      "Part of #831", and the adoption is built, test first, once #835 has merged.
1288	18. **`holler say` and the acceptance step: it needs #839, and more (checked 2026-10-11).** The scratch scenario's step "deliver
1289	    a message to each through `holler say`" cannot be met by this story alone, for either harness:
1290	    - The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body that drives
1291	      its harness, addressed by the body's advertised name (ADR 0014 point 5; `send_prompt` and `pane_gate` in
1292	      `holler-hub/src/circuit/dispatch.rs`). `holler say --pane NAME` resolves the pane to its `session_of_record` and uses
1293	      that as the address (`resolve_pane_target`), which matches no body's name: the id is an OpenCode `ses_…` or a Claude
1294	      UUID, and the join from a body's `harness_session_id` to `Pane.session_of_record` is **#839** (DRIVEN, not built). So
1295	      `say --pane`, its holds and its pane gate reach no real pane of either harness until #839.
1296	    - A Claude Code pane additionally has **no body at all**: the hub constructs no adapter (decision 1 of section 5), and the
1297	      adapter this story builds is called by the pane engine, not by a body. A body driver for it is the follow-up story **#850**.
1298	    So this story proves delivery where it can, at `HarnessPort::send_prompt` (item 7) against `FakeClaude` and the opt-in
1299	    real-binary test, and through `reset --first`; the `say` step of the scenario is written down, marked blocked by #839 and
1300	    #850, and **#831 does not close on it**. Nothing here routes a prompt around the hub's gate: the Claude pane's `say` is not
1301	    given a CLI-side path, because that would be a second enforcement site for holds and parks. **As built (plan review W4 and
1302	    W12):** delivery is proven at `HarnessPort::send_prompt`, on `FakeClaude`, in the conformance suite's case 9 and in the
1303	    opt-in real test. `reset --first` reaches the relaunched process (the endpoint of the record the relaunch stored), but it is
1304	    the existing carve-out around the hub's prompt gate (section 8, the seam), which #828 is moving to the hub's prompt path: once
1305	    it does, a Claude pane's `reset --first` waits for #839 and #850 too.
1306	
1307	**Not built here, and where it goes.** The body driver for `say` (#850); the hub's join (#839); `pane-stalled` (with #837's
1308	time source); the `PreToolUse` hook gate; a permission-mode, allowed-tools and questions spec field; `--agent` for Claude;
1309	answering a permission prompt by hook; interrupt with replacement text; the wizard's use of the flag (#803); a Windows or
1310	macOS socket directory (the note's "Not tested").
1311	
~~~~

**R5: `issue #835 body, lines 8-12`** — REFRESH AFTER MERGE of #831 and #835. Issue #835 Work: `--adopt-claude-code` is refused (usage) until #831 has merged.

Command: `gh issue view 835 -R Performant-Labs/holler --json body -q .body | sed -n '8,12p' | awk -v s=8 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
8	2. **Switch: `--adopt-claude-code`.** With it, the import also adopts those panes. Until the Claude Code harness (#831) has merged, the switch is refused as `usage` (exit 2), naming #831, and nothing is written. After #831 it registers them with that harness.
9	3. **How the import tells the harness apart is NOT decided here.** The old files cannot tell, and `HostPort::ps` returns process ids only (no command line). The first step of the story is the contract amendment (the epic's amend-first rule) that adds the read-only signal, for example the pane's foreground command name, and the rule that uses it. A pane whose OpenCode server is merely down must still be adopted and flagged `server-down`, not skipped; so health on the recorded port is not the signal.
10	4. **Surface.** The flag is added to the verb's `cli-surface.txt` line and to ADR 0003's row for this verb. `--format=json` shows the new finding like every other finding.
11	
12	## Fixture
~~~~

**R6: `docs/handoffs/839-brief.md at bf9d1c2, lines 489-489`** — REFRESH AFTER MERGE of #839. #839's brief (branch issue-839-implementation, bf9d1c2), "A fleet fact the repository cannot show": the design assumes one attach-mode body session per registry pane, "the wizard's attach rows".

Command: `git show bf9d1c2:docs/handoffs/839-brief.md | sed -n '489,489p' | awk -v s=489 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
489	2. **A fleet fact the repository cannot show.** The design assumes the production fleet runs **one attach-mode body session per registry pane, whose endpoint is that pane's OpenCode server** (the wizard's attach rows, E88, do). If a body hosts several panes' sessions, or a pane is driven in spawn mode, `driver_of` and `session/rebind` still work per session, but the cutover (#654) must start one attach session per pane, and the guide says so. Please confirm the shape before the cutover.
~~~~

**R7: `docs/handoffs/667-brief.md at e52797a, lines 46-46`** — #667's brief (branch issue-667-implementation, e52797a), decision 10: apply creates in row-then-column order and the planner wants rows before columns (read, not run).

Command: `git show e52797a:docs/handoffs/667-brief.md | sed -n '46,46p' | awk -v s=46 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
46	10. **The topology is rows of one pane, and one thing the scenario does not pin is reported.** Every scratch workspace is 2 rows by 1 column, so a missing pane at r2c1 is one `down` split of r1c1 (E40, E41). Apply creates in `(workspace, row, column)` order (E33) and the adapter plans one cell at a time (E43); the planner refuses "a row added below a row of more than one pane" as `grid-unreachable` (E42). By reading those three excerpts, a profile whose r1c2 and r2c1 are both missing in a workspace that holds only r1c1 cannot be created by one apply, because r1c2 is placed first. *Read, not run.* The scenario does not use that shape, and it does not pin it either (a test of a defect pins the defect). It is reported to the operator in the handoff as a candidate follow-up, with the excerpts; filing it is the operator's. "Checked against Herdr's own snapshot" means: after a create, the snapshot holds exactly the earlier panes (same ids and cells) plus one pane at the spec's cell. The scenario cannot feed `plan_splits` a snapshot: its `GridMap` is built only from Herdr's layout tree (E44), and `plan_splits` has its own tests and #649's grid test. *Rejected:* importing `plan_splits` into the scenario.
~~~~

**R8: `issue #842 body, lines 22-26`** — Issue #842 (open, wizard text), Part B: the AGENTS.md briefing, which Stage 8 writes before it launches the orchestrator; its "Look at panes" bullet names `herdr pane list` and `herdr pane read`.

Command: `gh issue view 842 -R Performant-Labs/holler --json body -q .body | sed -n '22,26p' | awk -v s=22 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
22	## Part B: the `AGENTS.md` briefing is offered even with no sessions, and says more
23	
24	### Problem
25	Stage 3 does not offer the Holler self-status briefing when the config has no sessions (N=0): "this briefing is not offered at all (nothing to drive)". But the orchestrator it would brief is launched either way, and it is the only agent that will ever drive the sessions added later. Skipping the briefing at N=0 leaves nothing in place to prompt for it later: nobody revisits `AGENTS.md` when sessions are added, so the orchestrator starts without guidance exactly when it first needs it. The briefing itself also covers only the minimum (`roster`, `say`, `interrupt`) and leaves out facts the wizard already knows.
26	
~~~~

**R9: `issue #850 body, lines 1-6`** — REFRESH AFTER MERGE of #850. Issue #850: a Claude Code pane has no body, so `holler say --pane` cannot reach it.

Command: `gh issue view 850 -R Performant-Labs/holler --json body -q .body | sed -n '1,6p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	Part of epic #633 (decision 9). Follow-up to #831 (the Claude Code harness for panes); needs #839.
2	
3	## Why
4	The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body, which drives its harness (an ACP child, or an HTTP attach to an OpenCode server, ADR 0014). #831 launches, registers, observes and delivers to a Claude Code pane through the pane engine (`HarnessPort::send_prompt`, which writes to the session's inbox socket), but a Claude Code pane has **no body**, so `holler say --pane NAME` cannot reach it, the hub's holds and pane gate do not see a prompt to it, and an `interrupt` has no body to cancel through.
5	
6	This is also blocked by #839 (the hub's DRIVEN session follows each pane's session of record): until the hub joins a body's `harness_session_id` to `Pane.session_of_record`, `say --pane` reaches no real pane of either harness. #831 records this in ADR-0021 section 15.
~~~~

**R10: `issue #665 body, lines 3-6`** — Issue #665 (PROPOSED, open): `holler profile export` and `import` would read and write a profile as a file.

Command: `gh issue view 665 -R Performant-Labs/holler --json body -q .body | sed -n '3,6p' | awk -v s=3 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
3	## Scope
4	- `holler profile rename OLD NEW`: one hub-side transaction that changes the name and slug and every member's `Pane.profile`, with one change-log entry (handler `profile/rename`, reserved by #637).
5	- `holler profile export NAME [--out FILE]` and `holler profile import FILE [--name NAME]`: a profile as a file (specs only; no generation, no log). Names and non-secret values only (I7): import refuses a file with an env value (`profile-secret-refused`) and an existing name (`profile-exists`). (amended 2026-10-08, features) `export` writes `command` and `check` as argv arrays, and `import` refuses a shell string with `command-not-argv`. Positions in the file go through the one `GridPos` parser (#637), so `r2c1`, `c1r2` and `2,1` (row,col) all load as row 2, col 1; export writes `rRcC`. (amended 2026-10-08, grid)
6	- All three take `--format=text|json`.
~~~~

**R11: `issue #842 comment 0, lines 1-10`** — Issue #842, comment "Part C": the even grid by `--ratio` splits in Stage 8, the build this story retires (the Herdr adapter already splits by ratio, E39 and E40).

Command: `gh issue view 842 -R Performant-Labs/holler --json comments -q '.comments[0].body' | sed -n '1,10p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	## Scope addition: Part C, Stage 8 builds an even grid
2	
3	Found in the same live run, after the workspace was grown from 1 pane to 8 (2 rows × 4 columns). This belongs with Parts A and B; whoever owns the issue can fold it into the body.
4	
5	### Problem
6	Stage 8 builds the grid row-first with chained right-splits, each at Herdr's default ratio. Every split halves the pane it targets, so with 4 columns the widths come out as 1/2, 1/4, 1/8, 1/8. Measured live on Herdr `0.9.2-preview.2026-09-29` (`herdr pane layout`), 209 columns wide:
7	
8	| Column | 1 | 2 | 3 | 4 |
9	|---|---|---|---|---|
10	| Width | 105 | 52 | 26 | 26 |
~~~~

**R12: `crates/holler-cli/src/pane/args.rs at d137a1d, lines 70-79`** — REFRESH AFTER MERGE of #831. The spec flags as built on #831's branch (d137a1d): `--port-policy` is `fixed:PORT` for opencode and `none` for claude; `--harness opencode|claude`.

Command: `git show d137a1d:crates/holler-cli/src/pane/args.rs | sed -n '70,79p' | awk -v s=70 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
70	    /// The hard context ceiling a watchdog reads.
71	    #[arg(long, value_name = "N")]
72	    pub ctx_hard: Option<u32>,
73	    /// How the harness server's port is chosen: fixed:PORT for opencode, none for claude.
74	    #[arg(long, value_name = "POLICY")]
75	    pub port_policy: Option<String>,
76	    /// The harness the pane runs: opencode or claude. Without it, a new spec runs opencode and
77	    /// an existing spec keeps its own; a relaunch never changes a live pane's harness.
78	    #[arg(long, value_name = "HARNESS")]
79	    pub harness: Option<String>,
~~~~
