# Evidence: #803 (behaviour outside the diff that the change relies on)

Phase 6 (F) entries. Each fact is quoted from unchanged source at the branch's base.

- **Fact:** `instance.sh` prints the resolved instance on one line, keys in a fixed order, and the workspace on its own line; `panes.sh` reads `[instance]` and `workspace` only from these lines.
  **Source:** `agent-skills/herdr-setup/lib/instance.sh:206-208`
  **Verbatim excerpt:**
  > print "instance: name=" name " prefix=" prefix " hub_port=" hub_port " serve_https_port=" serve \
  >     " state_dir=" state_dir " herdr_session=" herdr " backend_port_base=" base
  > if (has_ws) print "workspace: " ws

- **Fact:** with no `[instance]` table the backend port base is 47001, so `panes.sh` takes the first instance's range from `instance.sh /dev/null` instead of a second literal.
  **Source:** `agent-skills/herdr-setup/lib/instance.sh:63`
  **Verbatim excerpt:**
  > D_hub_port = 41807; D_serve = 443; D_base = 47001

- **Fact:** with `WIZARD_SERVE_HTTPS_PORT` empty, `collide.sh` never marks a serve entry as the instance's own ("this instance's port pair, no live hub"), which is why Stage 3 lists an old serve entry as present, not touched.
  **Source:** `agent-skills/herdr-setup/lib/collide.sh:48`, `agent-skills/herdr-setup/lib/collide.sh:155-156`, `agent-skills/herdr-setup/lib/collide.sh:185-187`
  **Verbatim excerpt:**
  > serve_port=${WIZARD_SERVE_HTTPS_PORT:-}
  >
  > own_serve_port=""
  > if [ -n "$serve_port" ] && ! foreign_hub_port; then
  >
  >       st="present, not touched"
  >       if [ "$own_serve_port" = "$a" ] && [ "$own_hub" = 0 ]; then
  >         st="this instance's port pair, no live hub"

- **Fact:** the Herdr adapter takes its socket from `HOLLER_HERDR_SOCKET`, else `HERDR_SOCKET_PATH`, else the named session's default path; `herdr.sh holler` therefore drops every `HERDR_*` and sets `HOLLER_HERDR_SOCKET` itself for the default instance.
  **Source:** `crates/holler-cli/src/pane/wiring/env.rs:170-176`
  **Verbatim excerpt:**
  > fn herdr_socket(env: Lookup<'_>, session: &str) -> Result<PathBuf, PaneError> {
  >     for variable in [HERDR_SOCKET, HERDR_SOCKET_PATH] {
  >         if let Some(value) = env(variable) {
  >             return absolute(variable, value);
  >         }
  >     }
  >     let config = config_home(env).ok_or_else(|| {

- **Fact:** the configuration directory is `$XDG_CONFIG_HOME` only when it is absolute, else `$HOME/.config`; `panes.sh env` follows the same rule for the default socket.
  **Source:** `crates/holler-cli/src/pane/wiring/env.rs:189-198`
  **Verbatim excerpt:**
  > /// `$XDG_CONFIG_HOME`, else `$HOME/.config`, each only when it is an absolute path: the XDG base
  > /// directory rule ignores a value that is empty or relative.
  > fn config_home(env: Lookup<'_>) -> Option<PathBuf> {
  >     let directory = |variable: &str| {
  >         env(variable)
  >             .map(PathBuf::from)
  >             .filter(|dir| dir.is_absolute())
  >     };
  >     directory("XDG_CONFIG_HOME").or_else(|| directory("HOME").map(|home| home.join(".config")))
  > }

- **Fact:** `HOLLER_HERDR_WORKSPACES` is `label=RxC` items joined by `,`, and the empty value is no workspace, so the door may pass an empty `WIZARD_HERDR_WORKSPACES` through.
  **Source:** `crates/holler-cli/src/pane/wiring/env.rs:200-206`
  **Verbatim excerpt:**
  > /// `HOLLER_HERDR_WORKSPACES`: items `label=RxC` (rows by columns) joined by `,`, the empty value
  > /// being no workspace. `usage` naming the variable for the first item that is empty, has no `=`,
  > /// more than one `=` or no `x`, has an empty label or a count that is not a whole number from 1 to
  > /// 65535, or repeats a label.
  > fn workspaces(list: &str) -> Result<BTreeMap<String, Extent>, PaneError> {
  >     let mut workspaces = BTreeMap::new();
  >     if list.is_empty() {

- **Fact:** a spec-only launch sets (upserts) the profile's spec and changes nothing live, so the rerun plan's `spec` lines rewrite the specs in place.
  **Source:** `crates/holler-cli/src/pane/launch.rs:51`
  **Verbatim excerpt:**
  > /// `--spec-only` sets P's spec and changes nothing live. The harness is opencode unless

- **Fact:** without `--harness` an existing spec keeps its harness, which is why the skill tells the user to close a pane whose harness changed before a rerun.
  **Source:** `crates/holler-cli/src/pane/args.rs:76-77`
  **Verbatim excerpt:**
  > /// The harness the pane runs: opencode or claude. Without it, a new spec runs opencode and
  > /// an existing spec keeps its own; a relaunch never changes a live pane's harness.

- **Fact:** the text-mode apply prints a summary line and one `PANE: ACTION, OUTCOME` line per step, and one `error:` line on exit 1 or 3; Stage 7's Gate reads exactly that.
  **Source:** `crates/holler-cli/src/profile/apply.rs:56-59`
  **Verbatim excerpt:**
  > /// Prints `profile NAME: N steps, applied` (or `planned (dry run; nothing changed)`), then one
  > /// line per step, `PANE: ACTION, OUTCOME`, with the code and the detail of a refused step, and
  > /// under a reported step one `FIELD: spec VALUE, live VALUE` line per difference. A run that
  > /// exits 1 or 3 prints one `error:` line instead.

- **Fact:** a whole-registry `pane list --format=json` carries the unmanaged Herdr panes with their workspace and label, which Stages 2, 3 and 5 read to judge restored panes and a previous wizard's layout.
  **Source:** `crates/holler-cli/src/pane/list.rs:82-86`
  **Verbatim excerpt:**
  > /// `"drained"` or `{"parked": {...}}`); an empty value is null. A whole-registry list adds
  > /// `"unmanaged"`, an array left out when empty, of `{"state": "unmanaged", "pane_id",
  > /// "session", "workspace", "pos", "label", "cwd", "last_screen_change" (milliseconds since the
  > /// Unix epoch), "idle_seconds"}` (`null` for what Herdr does not report), and
  > /// `"unmanaged_unavailable"` (the failed call's code) when Herdr could not be read.

- **Fact:** `pane get --format=json` prints `{"pane": RECORD, ...}`, the record holding the pane's harness port and session of record that Stage 8 reads.
  **Source:** `crates/holler-cli/src/pane/get.rs:53-58`
  **Verbatim excerpt:**
  > /// `--format=json` prints one envelope whose data is `{"pane": RECORD, "profile": NAME or
  > /// null, "spec": SPEC or null, "sync": "ok", "mismatch" or "unobserved"}`, the four keys
  > /// always present. RECORD is the pane record as the registry holds it, with its model, env
  > /// names, context ceilings (`"context": {"soft": N, "hard": N}`), command and probe; an
  > /// unset optional field is left out. SPEC is the profile's spec for the pane, with its own
  > /// context ceilings.

- **Fact:** `pane close --profile P` needs P to hold a spec for the pane and a live close needs a record, so a teardown close of a pane never built answers `pane-not-found`, which the skill says is not a failure.
  **Source:** `crates/holler-cli/src/pane/close.rs:5-8`
  **Verbatim excerpt:**
  > //! - **Plan.** `PANE` and `--profile` are typed (`usage`). With `--profile P`, P is read
  > //!   (`profile-not-found`) and must hold a spec for the pane (`pane-not-in-profile`): this is
  > //!   checked here, before any write, because `ProfileScope::edit_spec` removes a missing spec as
  > //!   a no-op that still writes P. A live close then reads the record (`pane-not-found`).

- **Fact:** `profile delete` is refused while any pane belongs to the profile, which is why the teardown closes every pane before deleting the profile.
  **Source:** `crates/holler-cli/src/profile/delete.rs:1-3`
  **Verbatim excerpt:**
  > //! `holler profile delete NAME [--keep-panes]` (story #662, epic #633): delete a profile, refused
  > //! while any pane belongs to it; with `--keep-panes`, each such pane is detached first and keeps
  > //! running.

- **Fact:** the docs doc-test extracts inline `holler ...` spans one line at a time and stops at a line with no closing backtick, which is why ADR-0021 decision 34's `holler profile apply` span, wrapped across two lines, is not parsed (the bare form would fail clap: `<NAME>` is required).
  **Source:** `crates/holler-cli/tests/docs_cli_test.rs:117-125`
  **Verbatim excerpt:**
  >         while let Some(start) = rest.find('`') {
  >             let after = &rest[start + 1..];
  >             let Some(end) = after.find('`') else { break };
  >             let span = &after[..end];
  >             if span.starts_with("holler ") {
  >                 out.push(Doc { file: file.clone(), line_no, raw: span.to_string() });
  >             }
  >             rest = &after[end + 1..];
  >         }

## Added by T (Phase 7): the facts `proof_fakes/holler` models from unchanged code

- **Fact:** `profile apply --format=json` prints an envelope whose `data` is `{"profile", "dry_run", "steps"}`, each step `{"pane", "action", "outcome", "code", "diff", "detail"}`; the fake's dry-run output has that shape, and Stage 7 reads `data.steps`.
  **Source:** `crates/holler-cli/src/profile/apply.rs:61-62`
  **Verbatim excerpt:**
  > /// `--format=json` prints one envelope whose data is `{"profile", "dry_run", "steps"}`, each step
  > /// `{"pane", "action", "outcome", "code", "diff", "detail"}`.

- **Fact:** a real apply with a refused step exits 3 and prints only the refusal (`apply refused <N> step(s): ...`), never the steps; `proof_803::a_pane_of_another_profile_is_a_refused_step_exit_3_and_an_empty_stdout` pins the fake to that.
  **Source:** `crates/holler-cli/src/profile/apply.rs:12-17`
  **Verbatim excerpt:**
  > //! - **3, a real run with a `refused` step** (#853, ADR-0021 decision 33): only the refusal,
  > //!   coded as the first refused step's error, with the message `apply refused <N> step(s):
  > //!   <pane> (<code>), ...; <M> other step(s) ran`, then `; <pane>: <detail>` for each refused
  > //!   step that had started its act. Refusals do not stop the run, so every step ran; the
  > //!   steps are not printed (an envelope carries no data with a failure), and a script that
  > //!   needs them runs `--dry-run` first.

- **Fact:** the planner refuses a new row below a row of more than one pane as `grid-unreachable`, which is why the config's `layout` must be a staircase and `proof_803::a_new_row_below_a_live_row_of_two_panes_is_a_refused_grid_unreachable_step` exists.
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:27-29`
  **Verbatim excerpt:**
  > //! - Any other cell is [`GRID_UNREACHABLE`] when the workspace is not a grid (it has an
  > //!   unplaced pane), or when no single split makes it: it needs a pane that is neither
  > //!   there nor asked for, or it is in a new row below a row of more than one pane.

- **Fact:** `--herdr-session` and `HOLLER_HERDR_SESSION` must name the same session; the fake reduces every mismatch to `usage` (exit 2), so `proof_803::the_default_instances_apply_agrees_with_its_environment_on_the_session_default` catches a plan and a door that disagree. The real verbs answer a mismatch differently per verb (the excerpt's next lines); the test pins agreement, not the real code.
  **Source:** `docs/pane-control.md:135-139`
  **Verbatim excerpt:**
  > **`--herdr-session NAME` stays a required flag** of every verb that reaches Herdr: `launch` (unless `--spec-only`),
  > `pane import`, and `profile apply` when its plan has a create. `relaunch` has no such flag and acts in the Herdr session
  > its record names, which must be the adapter's session too. The flag is required even when `HOLLER_HERDR_SESSION` is set, and is not defaulted from it: the
  > variable configures the adapter, and the flag names the session the verb acts on. **The two must name the same
  > session**, and a mismatch is not answered the same way by every verb:

## Added by F (Phase 6 rework, S's REWORK items 1 to 3)

- **Fact:** `instance.sh` reads every line that starts with `[` as a table header, and reads `workspace` only while no header has been seen, so a continuation line of a `layout` written over several lines ends its top level and a `workspace` after it is never read; `panes.sh` therefore refuses such a `layout` (item 2).
  **Source:** `agent-skills/herdr-setup/lib/instance.sh:69-71`, `agent-skills/herdr-setup/lib/instance.sh:87`
  **Verbatim excerpt:**
  > if (substr(line, 1, 1) == "[") {
  >     seen_table = 1
  >     if (line ~ /^\[instance\]/) {
  >
  > if (section == "" && key == "workspace") {

- **Fact:** a pane verb keeps each configuration error for its port's first use, and the Herdr port is built lazily, so a verb with no `HOLLER_HERDR_SESSION` acts through its other ports before the Herdr error surfaces (item 1).
  **Source:** `crates/holler-cli/src/pane/wiring.rs:148`, `crates/holler-cli/src/pane/wiring.rs:160`
  **Verbatim excerpt:**
  > /// The ports `env` configures, each configuration error kept for its port's first use. No
  >
  > Arc::new(LazyHerdr::new(move || connect_herdr(&herdr_config)));

- **Fact:** with `HOLLER_TMUX_SOCKET` unset, the host and the harness use tmux's default server (item 1).
  **Source:** `crates/holler-cli/src/pane/wiring/env.rs:86-94`
  **Verbatim excerpt:**
  > /// The tmux server of the host and the harness (D6): `HOLLER_TMUX_SOCKET` as a socket path, or
  > /// tmux's default socket when it is unset. A relative or empty value is `usage` naming the
  > /// variable; it never falls back to the default socket.
  > pub fn tmux_socket(env: Lookup<'_>) -> Result<TmuxSocket, PaneError> {
  >     match env(TMUX_SOCKET) {
  >         Some(value) => absolute(TMUX_SOCKET, value).map(TmuxSocket::Path),
  >         None => Ok(TmuxSocket::Default),
  >     }
  > }

- **Fact:** a live close stops the pane's owned processes on the host before it closes the Herdr pane, so a close whose Herdr configuration is missing has already stopped the processes of the tmux session of that name when it fails, and the record and the Herdr pane stay (item 1).
  **Source:** `crates/holler-cli/src/pane/close.rs:180-184`
  **Verbatim excerpt:**
  > reached = Some(Step::StopOwned);
  > ports.host.stop_owned(&name)?;
  > reached = Some(Step::Close);
  > ports.herdr.close(&record.herdr.pane_id)?;
  > reached = Some(Step::Delete);

- **Fact:** a reset (or switch) of the orchestrator's pane is refused as `orchestrator-pane` unless `--as-operator` is passed (item 3).
  **Source:** `crates/holler-pane/src/tx_switch.rs:345-355`
  **Verbatim excerpt:**
  > fn refuse_orchestrator(record: &Pane, as_operator: bool) -> Result<(), PaneError> {
  >     if record.role == PaneRole::Orchestrator && !as_operator {
  >         return Err(PaneError::Refused {
  >             code: ORCHESTRATOR_PANE,
  >             message: format!(
  >                 "{} is the orchestrator's pane; pass --as-operator to change it",
  >                 record.name
  >             ),
  >         });
  >     }
  >     Ok(())

- **Fact:** after a successful reset the previous session stays on the pane's server, unrecorded, and doctor reports it as a stray session; nothing deletes a session (item 3).
  **Source:** `crates/holler-cli/src/pane/reset.rs:13-15`, `docs/pane-control.md:268`
  **Verbatim excerpt:**
  > //! message that does not land fails the run after the record, which stands. After a
  > //! successful reset the previous session stays on the server too, as a stray: no port deletes
  > //! a session.
  >
  > once it is recorded. The previous session stays on the server, and `doctor` then reports it as a `stray-session`.

- **Fact:** a Claude Code pane is reset by a relaunch on a fresh session id, and has no server list, so `stray-session` never fires for it (item 3).
  **Source:** `docs/pane-control.md:275`, `docs/pane-control.md:615`
  **Verbatim excerpt:**
  > an answer, and with `orchestrator-pane` and `server-unhealthy` as `switch` is. A Claude Code pane is reset by a relaunch on
  >
  > has no server list, so `session-of-record-missing`, `tui-foreign-session` and `stray-session` never fire for it. It has
