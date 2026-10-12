# Evidence: #839, DRIVEN follows each pane's session of record

Source facts in **unchanged** code that F's change, or F's diagnosis of a failing test, relies on. Each
entry quotes the source verbatim at the line named. F wrote the entries below on 2026-10-12 (Phase 6);
T appends its own at Phase 7.

## What the implementation relies on

- **Fact:** `Roster::advertise` replaces a token's whole set of rows, and a row missing from the frame
  goes `gone` at once (why `control/rebind` fills one field of one row instead, A's warn 1).
  **Source:** `crates/holler-hub/src/roster.rs:350-356`
  **Verbatim excerpt:**
  > /// A `session/presence` from a body: replace (not merge) that token's set
  > /// of rows. A row in the frame is upserted (`connected`, `last_seen` =
  > /// now); a row the token previously held but which is absent from the frame
  > /// goes `gone` **immediately**. Returns the names **rejected** by the
  > /// collision policy (a connected different-token holder blocks them) — the
  > /// rest of the frame was applied.
  > pub fn advertise(&self, token_id: &str, p: &Presence) -> Vec<String> {

- **Fact:** the connection's own presence handler writes the roster before the cache, two awaits apart,
  so a whole-set advertise from a control-socket task could put a stale frame back (the race of A's
  warn 1).
  **Source:** `crates/holler-hub/src/circuit/dispatch.rs:198-202`
  **Verbatim excerpt:**
  > roster.advertise(&token_id, &p);
  > ...
  > registry.update_presence(client_id, p.sessions).await;

- **Fact:** `HttpAttachDriver::attach`'s one await before it spawns anything is the existence check, so
  the rebind's attach dropped at its deadline leaves no driver task running, and a late attach is never
  swapped in.
  **Source:** `crates/holler-body/src/http_attach_driver.rs:203-204, 233`
  **Verbatim excerpt:**
  > let client = reqwest::Client::new();
  > match check_exists(&client, &endpoint, &session_id).await {
  > ...
  > let connection_task = tokio::spawn(async move {

- **Fact:** the hold's fast path consumes nothing, so `control/say` with `check_only` (which runs
  `talk::admit`, and so `would_refuse`) spends no grant.
  **Source:** `crates/holler-hub/src/holds.rs:359-365`
  **Verbatim excerpt:**
  > /// What [`Holds::admit`] would answer right now, without consuming
  > /// anything: the fast path that keeps a refused prompt from leaving
  > /// side effects behind. Never the authority (a grant can be consumed
  > /// between this and `admit`).
  > pub fn would_refuse(&self, key: &str, grant: Option<&str>) -> Option<holler_proto::WireError> {
  >     decide(&mut self.state(), key, grant, false).err()
  > }

## What the diagnosis of the failing tests relies on

- **Fact:** a roster row is named `<label>/<session>` only once the token's label is bound; otherwise it
  is the bare session name. `driven_support::TestHub::body` advertises without binding a label, so its
  rows are `oc`, never `kiwi/oc`.
  **Source:** `crates/holler-hub/src/roster.rs:853-858`
  **Verbatim excerpt:**
  > fn qualified_name(info: &TokenInfo, s: &SessionAd) -> String {
  >     match info.label.as_deref() {
  >         Some(label) if !label.is_empty() => format!("{label}/{}", s.name),
  >         _ => s.name.clone(),
  >     }
  > }
  **Source:** `crates/holler-hub/tests/driven_support/mod.rs:212-219`
  **Verbatim excerpt:**
  > self.registry.update_presence(&client_id, ads.clone()).await;
  > self.roster.advertise(
  >     &token_id,
  >     &Presence {
  >         hostname: label.to_owned(),
  >         sessions: ads,
  >     },
  > );

- **Fact:** a hold's `since` is a timestamp with microseconds, so two holds set in two hubs differ, and
  `check_only_test`'s comparison of `data` across the two hubs it builds per case cannot hold for a hold.
  **Source:** `crates/holler-proto/src/log.rs:345-350`
  **Verbatim excerpt:**
  > pub fn timestamp() -> String {
  >     format_timestamp(OffsetDateTime::now_utc())
  > }
  > ...
  > /// with exactly six fraction digits (the microseconds within the second,
  **Source:** `crates/holler-hub/tests/check_only_test.rs:33-36, 179-180`
  **Verbatim excerpt:**
  > fn error_shape(reply: &Value) -> Value {
  >     let error = error_of(reply);
  >     json!({"code": error["code"], "data": error["data"]})
  > }
  > ...
  > for check_only in [false, true] {
  >     let hub = hub();

- **Fact:** the source scans cut a file at its first `#[cfg(test)]`; an out-of-line test module file has
  none, so `panes/gate/tests.rs` is scanned whole, and its two code literals trip
  `the_pane_codes_and_the_pair_rule_are_defined_once`.
  **Source:** `crates/holler-cli/tests/hold_single_path_test.rs:35`
  **Verbatim excerpt:**
  > let text = text.split("#[cfg(test)]").next().unwrap();
  **Source:** `crates/holler-hub/src/panes/gate/tests.rs:99, 110`
  **Verbatim excerpt:**
  > assert_frame(&error, "demo-c1r1", "pane-shown-driven-mismatch");
  > ...
  > assert_frame(&error, "demo-c1r1", "pane-unhealthy");

- **Fact:** the e2e rig hands `hub_args` to `Hub::start_with_args` as the whole serve-args list, so its
  hubs start without `--listen 127.0.0.1:0` (which `start_with_env` passes) and bind the default port.
  **Source:** `crates/holler-cli/tests/driven_e2e_test.rs:66`
  **Verbatim excerpt:**
  > let hub = Hub::start_with_args(&state, &[], hub_args);
  **Source:** `crates/holler-cli/tests/support/mod.rs:259-261`
  **Verbatim excerpt:**
  > pub fn start_with_env(state: &StateDir, envs: &[(&str, &str)]) -> Hub {
  >     Self::start_with_args(state, envs, &["--listen", "127.0.0.1:0"])
  > }

- **Fact:** an attach row is visible in `presence_doc` before its task runs (the brief keeps it so, and
  T's own guard test pins it), then hidden while the first attach runs; so `rebind_test`'s
  `until_attached`, which waits only for the row to be visible, can return before the first attach.
  **Source:** `crates/holler-body/src/session_manager/task.rs:168-172`
  **Verbatim excerpt:**
  > pub(super) async fn run(mut mailbox: mpsc::Receiver<SessionCommand>, mut inner: Inner) {
  >     if inner.config.mode == SessionMode::Attach {
  >         inner.presence.lock().unwrap_or_else(PoisonError::into_inner).omitted_pending_attach = true;
  >         attempt_initial_attach(&mut inner).await;
  >     }
  **Source:** `crates/holler-body/tests/rebind_test.rs:80-86`
  **Verbatim excerpt:**
  > async fn until_attached(sm: &SessionManager, session: &str) {
  >     let deadline = Instant::now() + WAIT;
  >     while advertised(sm, session).is_none() {
  >         assert!(Instant::now() < deadline, "{session} never attached");
  >         tokio::time::sleep(Duration::from_millis(10)).await;
  >     }
  > }

- **Fact:** a name segment is made only of lowercase letters, digits and inner hyphens, so the `SESSION` that two
  older CLI tests type is not a name. `route_target_by` now refuses it as `unknown session` before any hub is
  contacted (brief, "The CLI keeps the human form name-only (P3)"), so those tests no longer see
  `no live holler hub reachable`.
  **Source:** `crates/holler-proto/src/vocab.rs:209-216, 221-223`
  **Verbatim excerpt:**
  > if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
  >     return Err(NameError::BadCharacter);
  > }
  > for &c in seg {
  >     if !(is_word(c) || c == b'-') {
  >         return Err(NameError::BadCharacter);
  >     }
  > }
  > ...
  > fn is_word(b: u8) -> bool {
  >     matches!(b, b'0'..=b'9' | b'a'..=b'z')
  > }
  **Source:** `crates/holler-cli/tests/cli_invocation_test.rs:328-333`
  **Verbatim excerpt:**
  > .args(["say", "SESSION", "TEXT"])
  > .assert()
  > .failure()
  > .code(1)
  > .stderr(contains("not implemented").not())
  > .stderr(contains("no live holler hub reachable"));

- **Fact:** a test from #665, which #839 does not touch, asserts that the pane file's inode changed between a save
  and a save-back. A filesystem can give the freed inode to the next file, so the assertion depends on inode reuse.
  It failed once in F's full run. Run alone it fails every time, both on this branch and on the tree before #839.
  **Source:** `crates/holler-hub/tests/profile_rename_faults_test.rs:236, 250-254`
  **Verbatim excerpt:**
  > let inode_before = std::fs::metadata(&panes_file).unwrap().ino();
  > ...
  > assert_ne!(
  >     std::fs::metadata(&panes_file).unwrap().ino(),
  >     inode_before,
  >     "the pane document is saved before the profile document"
  > );

## What T's test repairs (T-green) rely on

Added by T in Phase 7, excerpts copied from source by T.

- **Fact:** the hub binds a token's client id and label in the roster at authentication, before any
  presence, so every row is `<label>/<session>` from the first frame. `driven_support::TestHub::body`
  now does the same two calls before `advertise` (T-1).
  **Source:** `crates/holler-hub/src/circuit.rs:306-307`
  **Verbatim excerpt:**
  > roster.set_token(&params.token_id, &client_id);
  > roster.set_label(&params.token_id, &record.label);

- **Fact:** an attach's existence check asks `GET {endpoint}/session/{id}` first, so a test can see the
  first attach start on the fake server's request log. `rebind_test::until_attached` now waits for
  that request (after the row is hidden, `task.rs:168-172` above) and only then for the row (T-5).
  **Source:** `crates/holler-body/src/http_attach_driver.rs:376, 386-391`
  **Verbatim excerpt:**
  > /// The fail-closed existence check — `GET {endpoint}/session/{id}` first,
  > ...
  > async fn check_exists(
  >     client: &reqwest::Client,
  >     endpoint: &str,
  >     session_id: &str,
  > ) -> Result<(), DriverError> {
  >     let base = endpoint.trim_end_matches('/');
  >     let v1_url = format!("{base}/session/{session_id}");
