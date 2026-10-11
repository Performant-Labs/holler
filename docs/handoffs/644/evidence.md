# Evidence: #644 part 2, the OpenCode agent through `launch` and `relaunch`

Source facts in **unchanged** code that F's diff and T's tests rely on, cut from the tree at `9eb9978` plus F's
uncommitted change (line numbers are that tree's). Append-only; T adds its own entries at Phase 6.

## F (Phase 5/6 implement), 2026-10-11

- **Fact:** the act's one record write returns the stored record, and that is what `Acted.stored` holds.
  **Source:** `crates/holler-pane/src/tx_launch.rs:614-619`
  **Verbatim excerpt:**
  > ```rust
  >     let record = plan.record(seen, live, (options.now_ms)());
  >     let stored = ports.pane_store.cas_put(&record, expected)?;
  >     let unclosed = plan
  >         .old
  >         .and_then(|old| close_old(ports, plan, seen, old, &stored.herdr));
  >     Ok(Acted { stored, unclosed })
  > ```

- **Fact:** a successful live run's `Launched.pane` is that stored record, so `Report::new` reads `data.agent` from the record and never from the request's spec.
  **Source:** `crates/holler-pane/src/tx_launch.rs:589-592`
  **Verbatim excerpt:**
  > ```rust
  >         None => Ok(Launched {
  >             pane: Some(acted.stored),
  >             profile,
  >         }),
  > ```

- **Fact:** a `--spec-only` run's `Launched.pane` is `None`, so `data.agent` is `null` there even when `--agent` was given.
  **Source:** `crates/holler-pane/src/tx_launch.rs:468-471`
  **Verbatim excerpt:**
  > ```rust
  >     Ok(Launched {
  >         pane: None,
  >         profile,
  >     })
  > ```

- **Fact:** the record's `opencode_agent` is the in-flight spec's.
  **Source:** `crates/holler-pane/src/tx_launch.rs:403-404`
  **Verbatim excerpt:**
  > ```rust
  >             model: spec.model.clone(),
  >             opencode_agent: spec.opencode_agent.clone(),
  > ```

- **Fact:** `AgentKey` serializes as a plain string, so `Report.agent: Option<AgentKey>` serializes as the key or `null`.
  **Source:** `crates/holler-pane/src/argv.rs:134-136`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
  > #[serde(transparent)]
  > pub struct AgentKey(String);
  > ```

- **Fact:** a key is non-empty and holds only ASCII letters, digits, `-` and `_`: `-` and `none` are valid keys, and `(default)` cannot be one.
  **Source:** `crates/holler-pane/src/argv.rs:143-144`
  **Verbatim excerpt:**
  > ```rust
  >         let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
  >         if text.is_empty() || !text.chars().all(allowed) {
  > ```

- **Fact:** the line's empty value is `-`.
  **Source:** `crates/holler-cli/src/pane/list.rs:35`
  **Verbatim excerpt:**
  > ```rust
  > pub const NO_VALUE: &str = "-";
  > ```

- **Fact:** `text_value` prints a plain value unchanged and quotes the empty value `-` (so `, agent -` cannot be printed for a key `-`).
  **Source:** `crates/holler-cli/src/pane/list.rs:280-291`
  **Verbatim excerpt:**
  > ```rust
  > pub fn text_value(value: &str) -> String {
  >     let plain = !value.is_empty()
  >         && value != NO_VALUE
  >         && !value
  >             .chars()
  >             .any(|c| acts_on_terminal(c) || c.is_whitespace() || matches!(c, '"' | '\\' | '='));
  >     if plain {
  >         value.to_owned()
  >     } else {
  >         format!("{value:?}")
  >     }
  > }
  > ```

- **Fact:** a spec with no agent has no `opencode_agent` key in its JSON, so its JSON pointer reads nothing and `FieldValue::Key(None)` (`null`) is what `spec_field_values_serialize_like_the_spec` expects.
  **Source:** `crates/holler-pane/src/profile.rs:186-189`
  **Verbatim excerpt:**
  > ```rust
  >     /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
  >     /// server's default agent (#700). A name, never a secret (I7).
  >     #[serde(default, skip_serializing_if = "Option::is_none")]
  >     pub opencode_agent: Option<AgentKey>,
  > ```

- **Fact:** `FieldValue` serializes untagged, so `Key(Option<String>)` serializes as the string or `null`, like `Argv(None)`.
  **Source:** `crates/holler-pane/src/profile_diff.rs:164-166`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
  > #[serde(untagged)]
  > pub enum FieldValue {
  > ```

- **Fact:** the snapshot of a pane as a spec copies its agent, so `diff_spec` of a pane against its own snapshot stays empty with the 16th field.
  **Source:** `crates/holler-pane/src/profile_snapshot.rs:61-62`
  **Verbatim excerpt:**
  > ```rust
  >         model: pane.model.clone(),
  >         opencode_agent: pane.opencode_agent.clone(),
  > ```

- **Fact:** `profile show` prints one `FIELD: VALUE` line for every field of `SpecField::ALL`, through `FieldValue`'s `Display`, so it lists `opencode_agent` with no change of its own.
  **Source:** `crates/holler-cli/src/profile/show.rs:135-142`
  **Verbatim excerpt:**
  > ```rust
  > fn push_spec(lines: &mut Vec<String>, spec: &ProfileSpec) {
  >     lines.push(format!("spec {}", text(&spec.pane)));
  >     lines.extend(
  >         SpecField::ALL
  >             .into_iter()
  >             .map(|field| format!("  {}: {}", field.as_str(), field.value(spec))),
  >     );
  > }
  > ```

- **Fact:** `profile show` prints a difference as `FIELD: spec VALUE, live VALUE` through the same `Display`, so an agent difference prints `opencode_agent: spec KEY, live (default)`.
  **Source:** `crates/holler-cli/src/profile/show.rs:160-167`
  **Verbatim excerpt:**
  > ```rust
  > fn difference_line(difference: &FieldDiff) -> String {
  >     format!(
  >         "  {}: spec {}, live {}",
  >         difference.field.as_str(),
  >         difference.spec,
  >         difference.live
  >     )
  > }
  > ```

- **Fact:** `relaunch` prints through `launch.rs`'s `emit_outcome`, so the one `Report` (with `agent`) serves both verbs.
  **Source:** `crates/holler-cli/src/pane/relaunch.rs:53-58`
  **Verbatim excerpt:**
  > ```rust
  > pub fn run(args: &PaneRelaunch, ctx: &mut VerbCtx<'_>) -> i32 {
  >     match relaunch_request(args, ctx.ports) {
  >         Ok(request) => {
  >             let result = relaunch(ctx.ports, &request, &TxOptions::default());
  >             let name = &request.record.name;
  >             emit_outcome(ctx, Verb::Relaunch, name, request.profile.as_ref(), result)
  > ```

- **Fact:** a failure that reached the act keeps the harness's code and gets the reconcile step appended once (the message T's refusal table expects).
  **Source:** `crates/holler-cli/src/pane/launch.rs:450-457`
  **Verbatim excerpt:**
  > ```rust
  > fn failure_body(failure: TxFailure, profile: Option<&ProfileName>) -> ErrorBody {
  >     let mut body = ErrorBody::from(&failure.error);
  >     let step = reconcile_step(profile);
  >     if failure.acted && !body.message.contains(step.as_str()) {
  >         body.message = format!("{}; {step}", body.message);
  >     }
  >     body
  > }
  > ```

- **Fact:** the engine asks `verify_serving` after the health wait, in the act, and returns its refusal as the step's error (the verbs add no check of their own).
  **Source:** `crates/holler-pane/src/tx_launch/server.rs:103-107`
  **Verbatim excerpt:**
  > ```rust
  >     if !wait_healthy(ports.harness, plan.port, poll, budget.deadline())? {
  >         return Err(budget.expired());
  >     }
  >     budget.check()?;
  >     ports.harness.verify_serving(plan.port, &plan.serve)
  > ```

- **Fact:** an error from the act's bring-up is rolled back before any record is written.
  **Source:** `crates/holler-pane/src/tx_launch.rs:611-612`
  **Verbatim excerpt:**
  > ```rust
  >     let live = bring_up(ports, plan, seen, budget, options.boot_poll, &mut made)
  >         .map_err(|error| roll_back(ports, plan.name, &made, error))?;
  > ```

- **Fact:** an additive key in a verb's `data` leaves `schema_version` at 1.
  **Source:** `docs/adr/ADR-0021.md:693`
  **Verbatim excerpt:**
  > - `schema_version` is an integer and is 1 now. Adding a field to the envelope or to a verb's `data` does not change it.
