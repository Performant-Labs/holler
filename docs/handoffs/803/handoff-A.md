# Handoff-A: #803 - The setup wizard builds panes through `holler pane` and `holler profile apply`  (up-front plan review)

**Date:** 2026-10-12
**Branch:** `issue-803-implementation` at `7819016`, merged with `origin/main` at `6eff9cc`
**Brief reviewed:** `docs/handoffs/803-brief.md` (sha256 `7ee5ef5d23f9f523...`)   **Reuse map:** none from O (an automated run, with no `survey.md`). The brief's D1 and "Blast radius" name the objects, and my own map is at the end.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with 0 `block` and 12 `warn` findings.

**The plan extends the right objects.**

- The wizard stays a skill.
- All pane work goes through the CLI verbs that already exist: `pane launch --spec-only`, `profile apply`, `pane close --profile`, `profile delete`, and `pane get`, `list` and `doctor`. Every contract the brief cites matches the code. I checked:
  - apply's exits 0, 1 and 3, and `data.steps`;
  - `--herdr-session` on launch, "Required unless --spec-only";
  - the JSON keys of `get`, `list` and `doctor`;
  - `profile delete`, which is refused while members remain;
  - ADR-0021 section 5, under which verbs run CLI-side and are configured from the CLI process's environment.

  The last point is what makes D8's wrapper the right choke point.
- `herdr.sh` is extended (one door, one scrub), not paralleled.
- The one new script, `lib/panes.sh`, is justified in writing: no existing script parses `[[pane]]` or `layout`. It depends only downward, on `instance.sh`'s printed contract.
- Nothing changes in Rust production code, the wire, the protocol or a golden file. ADR-0021 is amended in the same change, and ADR 0014 rightly stays as it is.

**Why no block.** Each finding has a fix that stays inside D8's or D1's own stated intent, or follows a pattern already in the repo, and none needs an operator decision. That is the bar #831's plan check used. The four that matter most:

- **W1. The one door has two holes.** The `server-start` allow-list keeps `HERDR_*`, and D8 shares it as it is. That lets `HERDR_SOCKET_PATH` and `HERDR_PANE_ID` reach `holler` and, through the tmux server, every harness. And with `WIZARD_TMUX_SOCKET` missing, a non-default instance falls back to tmux's default server.
- **W2. The profile name P is never defined.**
- **W3. The documented sequence is not rerunnable.** Its first line, `profile create`, refuses `profile-exists`.
- **W4. Case 3's guard is attributed to a refusal that never comes.** Apply never answers `pane-exists`, and it adopts or relaunches an unprofiled record whose name matches a wizard pane.

T should read W1 to W8 and W12 before authoring. F should read W1 to W4, W6 and W9 to W12.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | D8: `holler` "under that same allow-list (one list, shared with `server-start`, not copied)"; Acceptance 9 | cross-cutting (isolation between instances); pattern consistency | **Two holes in the one door.**<br>**(a) `HERDR_*` passes.** The `server-start` list keeps `HERDR_*` (`herdr.sh:240`), and `herdr.rs:706,720` pins that for the server. Shared as it is, the list passes `HERDR_SOCKET_PATH`, `HERDR_PANE_ID` and `HERDR_SESSION` to `holler`. That matters in two ways:<ul><li>`HERDR_SOCKET_PATH` is the adapter's second socket source (`env.rs:170-171`), and D8 sets no socket for a non-default instance. So it would send that instance's calls to whatever Herdr server the agent's terminal belongs to.</li><li>The first call starts the tmux server with the CLI's own environment (host `lib.rs:236-245`), and harnesses inherit that server's environment (`ProcessEnv::Inherit`, opencode `lib.rs:199-203`). So the agent's pane identity reaches every pane.</li></ul>The repo's own list for a `holler` verb run removes exactly these variables: `SCRUBBED`, `tests/pane_integration/main.rs:104-115`, and the spike's wrapper, `scripts/spikes/herdr-lib.sh:55`.<br>**(b) A lost literal fails open.** If `WIZARD_TMUX_SOCKET` is missing, `HOLLER_TMUX_SOCKET` is unset, and the host uses tmux's default server (`env.rs:89-94`; ADR-0021:362-363). That is the operator's live server, where two instances' `r1c1` meet, which is the very case D8 sets the socket for. `herdr.sh` already refuses the matching Herdr-session case (`herdr.sh:61-64`). | **F:**<ul><li>Put one base list in one function. `server-start` uses the base plus `HERDR_*`, which keeps its behavior. `holler` uses the base plus the `HOLLER_*` literals and no `HERDR_*`.</li><li>Refuse (exit 2, no call) a non-default instance whose `WIZARD_TMUX_SOCKET` is unset, empty or relative. Refuse the default instance when `WIZARD_HERDR_SOCKET` is unset. This is the same treatment as an unset `WIZARD_STATE_DIR`.</li><li>Optionally run `check_pane` before the call, which keeps E22's guard at the new door.</li><li>Record in `decisions.md` that this departs from D8's "that same allow-list". There is still one list, not a copy.</li><li>In the docs, do not say that a pane's provider credential comes from the shell rc. A harness starts as `new-window ... -- env -- argv`, with no shell (host `tmux.rs:79-91`), and the scrub drops exported variables. This is U7.</li></ul>**T:**<ul><li>Add `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_SESSION` and `TMUX` to Acceptance 9's sentinels, plus the two refusals.</li><li>The fake `holler` finds its root and the real `ps` from its own path. It must not rely on a test-only variable added to the production list. Today the fake `herdr` relies on `HERDR_FAKE_ROOT` (`proof_fakes/herdr:5`), which this fix removes from the `holler` verb's list.</li></ul> |
| 2 | warn | D1: `grid CONFIG` prints "the profile name", `plan CONFIG PROFILE`, `teardown CONFIG PROFILE`; D18 case 2: "the profile `P` exists" | naming; contract shape | **P is never defined.** No line says how the wizard names its profile. Yet `grid` must print it from the config alone, and D18 tells a rerun (case 2) from foreign panes (case 3) by whether P exists. If two runs choose differently, a rerun of the same config reads as case 3 and is refused, and its teardown closes nothing. | **F and T:**<ul><li>Make P one deterministic function of the config, computed only in `panes.sh`.</li><li>`grid` prints it, and Stage 1 carries it as the literal `<profile>`, so the agent never chooses it.</li><li>Recommended: the workspace label (`workspace`, default `holler`), which matches one-to-one what the wizard builds. `<prefix>` is the alternative, following the existing rule that the prefix names every file.</li><li>Record the choice in `decisions.md`. T pins it in `panes_803`.</li></ul> |
| 3 | warn | D9: "A rerun is the same sequence, because apply is idempotent"; D18 case 2; Acceptance 2 | pattern consistency (idempotence) | **The sequence is not rerunnable.**<ul><li>Its first line, `profile create P`, refuses `profile-exists` when P exists (`profile/create.rs:5-7`). The skill's Gate rule stops on any failed call, so every case-2 rerun stops at Stage 6's first line. The `spec` lines and the apply are idempotent.</li><li>A pane removed from the config keeps its spec in P, because `--spec-only` never removes a spec. Stage 7's dry-run diff catches this, but the text does not say what is then offered.</li></ul> | **F:**<ul><li>`panes.sh plan` leaves out the `profile-create` line when the skill says P exists, for example `plan CONFIG PROFILE --rerun`, set by D18 case 2. The agent still orders nothing.</li><li>For a spec in P that the config no longer names, the Stage 7 Gate offers `pane close NAME --profile P --spec-only`, a Holler verb.</li></ul>**T:** `proof_803` runs the documented sequence twice on one registry and pins that the second run converges: no create, and every pane adopted. |
| 4 | warn | D18 case 3: "Names that clash are `pane-exists`" | contract shape | **No Holler refusal guards case 3.** The wizard runs no live launch, and a `--spec-only` launch does not read the registry. Apply refuses only a record of *another* profile (`tx_apply.rs:25-34`, row a). A record with no profile whose name matches a wizard pane is:<ul><li>adopted, if it is healthy and matches (row c);</li><li>reported, if it differs (row d);</li><li>relaunched, if it is an unhealthy agent (row f).</li></ul>The relaunch touches a pane the wizard promised to leave alone. `pane-exists` comes only from a live `pane launch`. | **F:**<ul><li>The guard is the wizard's own pre-check. Stage 3 compares the config's names with `holler pane list`. For any name that a record outside P holds, it refuses and offers D18's choices.</li><li>The backstop is Stage 7's dry-run diff: any `adopt`, `reported` or `relaunch` step in a fresh build stops the run.</li><li>Do not cite `pane-exists` as Holler's answer. `grid-occupied` is a real create refusal.</li></ul>**T:**<ul><li>Pin both texts.</li><li>The fake's apply implements rows a to f as `tx_apply.rs` states them, with no invented `pane-exists`.</li></ul> |
| 5 | warn | Blast radius: `instance.rs` "not edited"; `skilltext_838.rs` "stay" | scope (pins the brief missed) | **Two more pinned tests must change.**<ul><li>`instance.rs:283-287` (`documented_defaults_match_the_skill_text`) requires `SKILL.md` to contain `tailscale serve --bg --https <serve_https_port> <hub_port>`. That contradicts Acceptance 10. It also requires the two `[[session]]` lines `endpoint = "http://127.0.0.1:4700x"`, which D4 and D5 retire.</li><li>`skilltext_838.rs:107` pins "after the bare top-level keys and before `[[orchestrator]]`".</li></ul> | **T:**<ul><li>In RED, retire the `tailscale` and `endpoint` pins in `instance.rs`. Keep `holler hub serve --listen 127.0.0.1:<hub_port>` and the `[instance]` comment pins (lines 297-303).</li><li>Re-pin 838's phrase to `[[pane]]`.</li><li>Record both in handoff-T-red. Both files stay well under 600 lines.</li></ul>**F:** never keep retired text in the skill to satisfy an old pin. |
| 6 | warn | D1 (c) and D11: `panes.sh` "applies `instance.sh`'s own rules to panes" | anti-duplication (accepted, because it is justified) | **The port rule exists twice.** `panes.sh` re-implements `instance.sh:165-189`: base + i, a pane's own port, the 65535 limit, the 47001 default range unless every pane sets its own port, and duplicate ports. The `[[session]]` branch it copies can no longer be reached by any config the wizard accepts. The brief justifies this copy in writing, so it is not a Phase 7 block. Reading `[instance]` or `workspace` a second time would be. | **F:**<ul><li>Take `name`, `prefix`, `state_dir`, `herdr_session`, `backend_port_base`, `hub_port` and `workspace` only from `instance.sh`'s printed lines (`instance.sh:206-208`).</li><li>Locate `instance.sh` the way `herdr.sh` locates `ledger.sh`: `WIZARD_LIB`, else the script's own directory (`herdr.sh:59,120`).</li><li>Copy only the value-syntax helpers (`trim`, `value_of`, `instance.sh:29-42`). The header says they accept exactly `instance.sh`'s subset, and that the port rule is `instance.sh`'s.</li><li>Add `panes.sh` to Stage 0's file checks (`SKILL.md:376,396`). Otherwise an install from before this story passes Stage 0 and fails at Stage 1.</li></ul>**T:** add a `panes_803` cross-check that the default range `panes.sh` refuses starts at the `backend_port_base` that `instance.sh` prints for a config with no `[instance]` table. Then the two `47001` literals cannot drift apart.<br>**O:** see follow-up 1 below. |
| 7 | warn | The new modules `herdr_803.rs` and `skilltext_803.rs`; Acceptance 12 and 21 | anti-duplication (test harness); choke point | **Three reuse points and one loose predicate.**<ul><li>(a) `herdr.rs`'s `Env` (lines 26-135: the fake `herdr`, `base`, `run`, `run_in_pane`, `recorded`) and its `text` are the harness `herdr_803` needs. A copy of them is a Phase 7 rejection.</li><li>(b) `skilltext.rs:377-408` already checks every documented verb against its script, over a fixed list of scripts.</li><li>(c) `code_lines` and `section` (`skilltext.rs:52-63,80-87`) are the code-fence parser and the section slicer. The small `read`, `flat` and `between` helpers are a per-module pattern and may repeat.</li><li>(d) Acceptance 12 keeps `scoped_to_instance` (`skilltext.rs:89-92`), which also accepts `HOLLER_STATE_DIR=`. So a `HOLLER_STATE_DIR=<state_dir> holler pane ...` line would pass. That line skips the scrub and the Herdr and tmux variables, which for a non-default instance means tmux's default server.</li></ul> | **T:**<ul><li>Make `Env` and `text` `pub(super)`, and add a fake `holler` that logs its argv and environment. This is the same move the brief makes for `proof.rs`'s `Host`.</li><li>Add `panes` to the script list and the verb list of `documented_verbs_and_flags_exist_in_the_scripts`.</li><li>Reuse `code_lines` and `section` through `pub(super)`.</li><li>For the `holler pane` and `holler profile` words, require `herdr.sh holler` on the line.</li><li>Run Acceptance 21's bash 3.2 scan over every `lib/*.sh`. All six pass today, and any later script is then covered.</li></ul> |
| 8 | warn | The fake `holler` (Blast radius; Acceptance 2, 16, 17 and 19) | anti-duplication (test double); contract shape | **The fake re-implements production rules with no cross-check.**<ul><li>Its planner rule would be a third copy of `plan.rs:1-30,168-190`, beside the adapter itself and the test kit's `SplitOnly`, which #853 cross-checks against the planner. The wizard runs no live launch, so in `proof_803` that rule tests only the fake.</li><li>The skill and the fake must both match the real JSON. Otherwise both pass against a fiction.</li></ul> | **T:**<ul><li>Keep the fake to what `proof_803` asserts.</li><li>If the planner refusal stays (Acceptance 2), it is one function whose header cites `plan.rs`, and no test name claims to check the adapter's order.</li><li>Use the real shapes:<ul><li>`data.pane.harness.port` and `data.pane.session_of_record` (`get.rs:52-56`, `pane.rs:269-278,426-429`);</li><li>`data.panes[]` with `name`, `pos`, `profile` and `sync`, and `data.unmanaged` (`list.rs:78-86`);</li><li>`data.findings` (`doctor.rs:5-7`);</li><li>`data.steps` and `outcome`, apply's exit codes and rows a to f (`apply.rs:1-28`, `tx_apply.rs:21-34`).</li></ul></li><li>`proof.rs` embeds the same file (lines 27-37), so `hub`, `body` and `roster` stay exactly as they are.</li></ul> |
| 9 | warn | D15 (teardown) | state and persistence | **Teardown leaves the instance's tmux server running.** `pane close` stops the pane's tagged processes and closes its Herdr pane, but never ends its tmux session (host `lib.rs:95-99`; `launch.rs:44`, "the tmux session stays"). No ledger row records the tmux server, so `stop-owned.sh teardown` never stops it.<ul><li>For a non-default instance, the server's socket is `<state_dir>/tmux.sock` (D8). So `teardown --purge-state` would unlink that socket under a live server.</li><li>For the default instance, sessions `r1c1` and the rest stay in the user's own tmux server.</li></ul> | **F:**<ul><li>The teardown text and the docs say what stays and how a person ends it.</li><li>`--purge-state` is not offered while that tmux server runs.</li><li>The wizard signals nothing outside its ledger (the ownership rule).</li></ul>**O:** see follow-up 2 below. |
| 10 | warn | "Rules for this story", Bounded: "the verbs carry their own 10 s per-call bound"; D9's Stage 7 | cross-cutting (bounds) | **Apply has no overall cap** (`tx_apply.rs:45-58`, and line 55). A run costs up to 10 s to observe. Each create then adds up to the 10 s launch budget (`tx_launch.rs:94`), plus one port bound and a rollback. So a 16-pane build can outrun an agent shell tool's default timeout, which is two minutes in Claude Code. A killed apply can leave a step that had acted without being reconciled. | **F:**<ul><li>Stage 7 states the expected duration, computed as that sum over the plan's creates.</li><li>The agent gives the command a timeout above it, or runs it in the background with its output under `<scratch>` and reads the output when it ends.</li><li>The tool's own timeout never ends an apply.</li></ul> |
| 11 | warn | Amendments (1): "its last sentence"; D17: "section 11's last paragraph"; D16: the `herdr.sh` header | ADRs; naming | **Two placement and naming details.**<ul><li>The quoted sentence ends the *first* paragraph of ADR-0021 section 11 (line 975). The section then continues with the `send_text` paragraph and "Deferred to #654" (978-987). So "last sentence" and "last paragraph" point to different places.</li><li>`herdr.sh`'s header calls the script "the setup wizard's one door to Herdr" (line 2).</li></ul> | **F:**<ul><li>Place the wizard sentences right after the quoted sentence.</li><li>In the header, add that the script is also the door through which the wizard runs Holler's pane and profile verbs.</li><li>Keep "#734" and "unverified" within the first 20 lines (`herdr.rs:639-651`).</li></ul> |
| 12 | warn | D8 and Acceptance 8 (`panes.sh env`); D9 (`--herdr-session`) | contract shape (the printed literals) | **Three details of the literals.**<ul><li>(a) A workspace label may contain spaces (`instance.sh:195`). An unquoted `WIZARD_HERDR_WORKSPACES=...` typed into `<herdr_env>` then splits into several words.</li><li>(b) The XDG rule for the default socket must be the one in `env.rs:191-198`, which ignores an empty or relative `XDG_CONFIG_HOME`.</li><li>(c) `panes.sh` (for `--herdr-session`) and `herdr.sh` (for `HOLLER_HERDR_SESSION`) each map the empty session to `default`. That is one literal kept in two places, and E32 refuses a mismatch.</li></ul> | **F:** print shell-quoted literals, and follow that XDG rule.<br>**T:**<ul><li>Acceptance 8 gets a label with a space, checked by evaluating the printed line in bash, and a case with an empty `XDG_CONFIG_HOME`.</li><li>One `proof_803` step runs the plan's apply line through `herdr.sh holler` for the default instance. That pins that the two `default`s agree.</li></ul> |

## Reuse & Analogous-Feature map (A-derived: the baseline for the Phase 7 gate)

**Relevant code mapped**

- **`agent-skills/herdr-setup/lib/herdr.sh`:**
  - the guards on the instance and the session, 49-74;
  - `check_pane`, 149-164;
  - `run`, 209-229, with its pane guard at 224-227;
  - `server-start`'s allow-list, 235-253;
  - `label`, 261-278;
  - the header, 1-41.
- **`lib/instance.sh`:**
  - the value parser, 28-61;
  - the messages for `[instance]`'s order and for misplaced keys, 80-99;
  - the port resolution, 165-189;
  - `workspace`, 191-198;
  - the printed contract, 206-209.
- **`lib/ledger.sh`, `stop-owned.sh`, `collide.sh` and `inventory.sh`:** unchanged. `ledger.sh` already admits roles `hub`, `herdr` and `body` at stages 4 to 9 (lines 11 and 122-124).
- **`SKILL.md`:**
  - Stage 0's file check, 376 and 396;
  - `<herdr_env>`, 725-729;
  - the two rules for a non-default instance, 701-707.
- **The CLI contracts consumed:**
  - `pane/launch.rs`, 60-75;
  - `pane/args.rs`, `SpecFlags`;
  - `profile/apply.rs`, 1-28 and 41-58;
  - `tx_apply.rs`, 1-60;
  - `profile/create.rs`, `profile/delete.rs` and `pane/close.rs`;
  - `pane/get.rs`, 52-56, and `pane/list.rs`, 78-86;
  - `pane/doctor.rs`, 5-7;
  - `pane/wiring/env.rs`;
  - ADR-0021 section 5: verbs run CLI-side and are configured from the CLI process's environment.
- **The tests:**
  - `tests/wizard/main.rs`: one module per story;
  - `herdr.rs`: `Env` (26-135) and the scrub tests (698-724, 797-823);
  - `proof.rs`: `Host` (43-173) and `FAKES` (27-37);
  - `skilltext.rs`: `code_lines` (52-63), `section` (80-87), `scoped_to_instance` (89-92), the scoping test (144-173) and the verb test (377-408);
  - `instance.rs`, 277-313, and `skilltext_838.rs`, 93-110;
  - `proof_fakes/holler`, `herdr` and `common.sh`;
  - `tests/pane_integration/main.rs`, 104-115 (`SCRUBBED`).

**Closest analogous features**

- **For D8:** `herdr.sh server-start`'s scrubbed environment (#771).
- **For `panes.sh check` and `env`:** `instance.sh`, a config validator that prints the literals the skill quotes.
- **For `plan` and `teardown`:** `stop-owned.sh restart`, a script that prints a command for the agent to run.

**Objects this change touches**

- `herdr.sh` (its verbs) and `panes.sh` (new);
- `SKILL.md`, `docs/setup-wizard.md`, section 6 of `docs/pane-control.md`, and ADR-0021 (section 11, section 15 item 16 and "Decisions taken");
- the README and the CHANGELOG;
- `proof_fakes/holler`;
- the wizard test modules the brief names, plus `instance.rs` and `skilltext_838.rs` (W5).

**Extend or new**

- **EXTEND `herdr.sh`.**
  - The `holler` verb reuses the scrub through one function: a base list, to which `server-start` adds `HERDR_*` (W1).
  - `run` becomes an allow-list.
  - `label` goes.
- **NEW `lib/panes.sh`**, justified by D1 (c), and because no script parses `[[pane]]` or `layout`.
  - It reads `instance.sh`'s output and never re-parses `[instance]` or `workspace`.
  - Its copy of the port rule is the one accepted duplicate (W6).
- **EXTEND the test pieces:**
  - `proof_fakes/holler`;
  - `herdr.rs`'s `Env` and `proof.rs`'s `Host`, both through `pub(super)`;
  - `skilltext.rs`'s verb and scoping tests (W7).

**What Phase 7 checks first**

- a second copy of the allow-list in `herdr.sh`;
- a copy of `Env`, `Host`, `code_lines` or `section` in a new module;
- a second reader of `[instance]` or `workspace` in `panes.sh`;
- a fake `holler` whose JSON or apply rows differ from the code;
- any `HOLLER_STATE_DIR=... holler pane|profile` line in a code block.

## Notes for O

Not required, since the verdict is PASS. These are follow-ups to consider; none blocks this story.

1. **Retire `instance.sh`'s `[[session]]` port branch** once the wizard refuses `[[session]]` (W6). Until then it is a second, unreachable copy of the rule.
2. **A Holler-owned way to end a pane's tmux session**, or a ledger row for the instance's tmux server (W9).
3. **The Herdr adapter's default socket for the default session.** For a session named `default`, the adapter's default is the named-session path (`env.rs:170-189`), which is why D8 must set the socket by hand (U1). A rule for this in the adapter would remove the wizard's copy of the XDG rule. That is Rust, so it is not this story's work.
4. **Registry panes carry no Herdr label.** ADR-0021 section 5 says "launch writes none", so the old wizard's visible `r1c1` labels go away. Stage 10 or 11 should say how a person finds a pane: the POS column of `holler pane list`, or the display plugin's `$pos`.

## Patterns referenced

- `agent-skills/herdr-setup/lib/herdr.sh` and `agent-skills/herdr-setup/lib/instance.sh`
- `crates/holler-cli/src/pane/wiring/env.rs`, and ADR-0021 section 5
- `crates/holler-cli/tests/pane_integration/main.rs` (`SCRUBBED`)
- `crates/holler-pane/src/tx_apply.rs` and `crates/holler-cli/src/profile/apply.rs`
- `crates/holler-cli/tests/wizard/herdr.rs`, `proof.rs` and `skilltext.rs`
