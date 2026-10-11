# Brief: #838 herdr-session-question

Repo: Performant-Labs/holler. Issue: #838 (part of epic #726). Rigor: in-session. UI surface: no. Kind: feature (skill text, docs and tests only).

## Problem
The Herdr session is reachable only through the optional `[instance]` table and the wizard never asks about it. Stage 1's scenario menu (A to D) covers orchestrators, sessions and layout, so a config built from it has no `[instance]` table and the wizard silently builds in the unnamed `default` Herdr server. A user with other Herdr sessions (seen live: a running `jupiter`, a stopped `default`) is never told they exist, and one who wants a named session must hand-write `[instance]` with the right `state_dir` and ports. Herdr has no session rename, so the choice has to be made when the workspace is first built. The GitHub issue (#838) is the source of truth; this brief restates it.

## Evidence (verbatim, as of `main` 652841b)

`agent-skills/herdr-setup/SKILL.md:420` (Stage 0 table row to extend):
```
| Herdr server state and saved sessions | info | running or stopped, and `herdr session list` |
```
Pinned by `crates/holler-cli/tests/wizard/skilltext_w6_preflight.rs:104` (it looks for the row prefix `| Herdr server state`); the row must keep that prefix and still say `herdr session list`.

`agent-skills/herdr-setup/SKILL.md:605-612` (Stage 1, where the new question goes, directly BEFORE this paragraph):
```
   **Then, once the orchestrators are settled, ask one more question: the workspace name.**
   Suggest **two dictionary words related to the task**, lower case, joined with a hyphen
   (Discord-style), for example `herald-relay`, `lantern-hub`, `quiet-tower`. ...
   the answer is written into the config as the top-level `workspace = "<name>"`, before every
   table header. ... The key is optional: with no
   `workspace` Herdr labels the workspace itself.
```
`crates/holler-cli/tests/wizard/skilltext_w5_workspace.rs:47-55` pins, inside Stage 1: `the workspace name`, `two dictionary words related to the task`, `joined with a hyphen`, `` `herald-relay` ``, `three alternatives on request`, `workspace = "<name>"`, `never a project's real name, never a person's name`. The new text may reword the opener of this paragraph ("Then, once the orchestrators are settled, ask one more question: the workspace name.") because it moves to the new question; the pinned phrases above must survive.

`agent-skills/herdr-setup/SKILL.md:614-619` (Stage 1 step 4, where the file is written and shown; the `[instance]` table is written here):
```
4. **Write the file from the answers** (on a first run create the directory first:
   `mkdir -p ~/.holler/herdr/setup`), **show it, and let the user confirm or edit before Stage 1
   re-validates it** ...
```
`agent-skills/herdr-setup/SKILL.md:627` (Stage 1 Verify, where the report states the loaded config): `**Verify:** state plainly **which of the three paths/sources** you actually loaded ...`

`agent-skills/herdr-setup/SKILL.md:164-181` (the config example, already documents the table; the position rule: after bare top-level keys, before `[[orchestrator]]`):
```
# [instance]
# name = "default"             # ^[a-z][a-z0-9-]{0,23}$
# hub_port = 41807             # 1024 to 65535
# serve_https_port = 443       # 1 to 65535 (the `tailscale serve` HTTPS port)
# state_dir = ""               # absolute or ~/ path; empty = Holler's default ($HOME/.holler)
# herdr_session = ""           # empty = no named Herdr session
# backend_port_base = 47001    # session i (0-based) listens on base + i unless it sets backend_port
```
`SKILL.md` "Two rules for a non-default instance" (Stage 1, after `instance.sh`): a non-default instance never uses the first instance's ports (`hub_port` 41807, `serve_https_port` 443, backend ports in 47001..47001+N) and "is driven from a terminal outside any Herdr pane: a driving agent that runs inside a Herdr pane cannot establish which session it is in, and Stage 2's `check-pane` stops it".

`agent-skills/herdr-setup/SKILL.md:975` (Stage 3 plan list, where the session line goes, next to the workspace line):
```
- **The workspace's name.** When the config has a `workspace`, the plan names it ("workspace
  `<workspace>`"); with none, say Herdr labels the workspace itself.
```
`docs/setup-wizard.md:264-` section "Running an instance beside another" (ends with the `[instance]` example, `name = "second"`, `hub_port = 41808`, `serve_https_port = 8443`, `state_dir = "~/.holler-second"`, `herdr_session = "second"`, `backend_port_base = 47101`). The new docs subsection goes at the end of this section, before `## Instance state and the ledger`.

`crates/holler-cli/tests/wizard/instance.rs:84-96` shows how `instance.sh` is driven by a test: `validate(&config(instance_table, TAIL))` writes a config to a temp dir and runs `agent-skills/herdr-setup/lib/instance.sh`; `a_full_non_default_instance_is_accepted_and_resolved` (line 114) asserts exit 0 and the `instance: name=second prefix=second hub_port=41808 serve_https_port=8443 state_dir=... herdr_session=second backend_port_base=47101` line. The new test can reuse the same approach (its own helper in the new file; do not edit `instance.rs`).

Test style to copy: `skilltext_w5_workspace.rs` (`flat()`, `between()`, `has_all()` over the whitespace-folded skill text); modules are registered one `mod` line each in `crates/holler-cli/tests/wizard/main.rs`.

## Acceptance criteria (each observable)
1. Stage 1 of `SKILL.md` has the Herdr-session question with both answers ("The default, unnamed session" recommended when the user has no reason to separate, writing no `[instance]` table, behaviour unchanged; "A new named session"), positioned after the orchestrator step (the scenario menu) and before the workspace-name question (asserted by the byte offsets of the three markers in the Stage 1 text), and says an existing config is not re-asked and that Stage 1's report states which session it targets.
2. The named-session branch states: the name pattern `^[a-z][a-z0-9-]{0,23}$`; a suggested name; the refusal of a name already in `herdr session list` (running or stopped), naming it, with the choice of another name or the default; the proposed `state_dir = "~/.holler-<name>"`, `hub_port = 41808`, `serve_https_port = 8443`, `backend_port_base = 47101`, `name` and `herdr_session` = the session name; that `[instance]` is written after the bare top-level keys and before `[[orchestrator]]`; that the file is shown and the user confirms or edits before `instance.sh` validates it. Building in an existing session is not offered.
3. It states, once, that a named session is driven from a terminal outside any Herdr pane and that `holler` calls in the user's own shells need `HOLLER_STATE_DIR=<state_dir>`.
4. Stage 3's plan names the Herdr session in both cases: "Herdr session: default (unnamed)" and "Herdr session: `<name>` (new, own state directory and ports)".
5. Stage 0's session row still starts `| Herdr server state` and still lists sessions with status: its Detail now says it lists every session from `herdr session list` with its status (running or stopped).
6. `docs/setup-wizard.md` documents the question (one subsection under "Running an instance beside another"): both answers, the proposed values, the refusal, the outside-a-pane rule and `HOLLER_STATE_DIR`.
7. A test feeds `instance.sh` a config with the proposed values for an example name (for example `work`: `name = "work"`, `herdr_session = "work"`, `state_dir = "~/.holler-work"`, `hub_port = 41808`, `serve_https_port = 8443`, `backend_port_base = 47101`) and asserts exit 0 and the resolved line.
8. `cargo test -p holler-cli --test wizard_scripts` (whole target), `cargo clippy --workspace --all-targets -- -D warnings` (as CI runs it; check `.github/workflows` for the exact flags) and `bash scripts/lint.sh` pass; the CI test command (`test.unit.command` in `.aftersight/pipeline.config.json`) is green.

## Files
Production (text only): `agent-skills/herdr-setup/SKILL.md` (Stage 0 row at line 420, Stage 1 new question before line 605 plus a sentence in Verify about the targeted session, Stage 3 plan list near line 975), `docs/setup-wizard.md`.
Tests: new `crates/holler-cli/tests/wizard/skilltext_838.rs` (+ one `mod skilltext_838;` line in `main.rs`). Edit an older `skilltext*.rs` assertion only where it pins wording this story changes (`skilltext_w5_workspace.rs` / `skilltext_w6_preflight.rs` are the likely ones; expect none).
Reuse map (extend, do not duplicate): the existing `[instance]` documentation, the "Two rules for a non-default instance" text and the `instance.sh` validator are referenced, not restated at length; no new script, no change to `instance.sh`, `herdr.sh`, `collide.sh`.

## Decisions already made (MO, from the issue)
- A named Herdr session stays a full second instance (`name`, `state_dir`, `herdr_session` and non-default ports, as `instance.sh` requires); only the question and the config assembly are new.
- The default answer is recommended when the user has no reason to separate; it writes no `[instance]` table.
- Port proposals are defaults only: Stage 3's `collide.sh` stays the authority and refuses a taken port. No new probing in Stage 1, which runs before the inventory.
- Existing config: the question is not re-asked; Stage 1's report states the session it targets (the config's `herdr_session`, or "default (unnamed)").
- Building in an existing session is not offered; choosing one by name gets the refusal plus the choice of another name or the default.

## Out of scope
Decoupling `herdr_session` from the rest of the instance; building in an existing running session; changes to `instance.sh`, `herdr.sh` or `collide.sh` validation; the Stage 8 rewrite of #803; any Rust production code; touching a live Herdr session (tests are text assertions plus `instance.sh` on a temp config).

## Test plan
RED first: the new `skilltext_838.rs` text assertions fail today (Stage 1 has no session question, Stage 3 names no session, the docs lack the subsection, the Stage 0 row lacks the status wording); the `instance.sh` acceptance test for the proposed values passes already (it documents that the branch's output is valid), so RED is the text assertions. Then F edits `SKILL.md` and `docs/setup-wizard.md` until GREEN, and the whole `wizard_scripts` target stays green.

## Risks
- Wording pinned by older `skilltext*.rs` tests (esp. the Stage 1 workspace paragraph and Stage 0 table): the whole target is run to catch it.
- The skill text is long and whitespace-wrapped: tests must use the folded `flat()` text.
- An agent might read the question as offering an existing session: the text must say plainly that it is refused, running or stopped.
