# Evidence — #642a (facts in unchanged code that the diff relies on)

## F (Phase 6, implement)

- **Fact:** `FakeHarness` answers a call to a killed or never-served port with `unavailable` and the text "the harness server on port N"; the adapter's refused-connection message (`Call::send_by`, `HttpError::Refused`) uses the same text.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:484-488`
  **Verbatim excerpt:**
  > fn unreachable_server(port: u16) -> PaneError {
  >     PaneError::Unavailable {
  >         what: format!("the harness server on port {port}"),
  >     }
  > }

- **Fact:** the test kit's `op` strings for the four methods that can time out in 642a are exactly the adapter's `OP_SERVE`, `OP_CREATE_SESSION`, `OP_LIST_SESSIONS` and `OP_ABORT` (`lib.rs`); the hermetic tests pin the equality through `HarnessOp::as_str`.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:47-51`
  **Verbatim excerpt:**
  > HarnessOp::Serve => "harness.serve",
  > HarnessOp::Health => "harness.health",
  > HarnessOp::CreateSession => "harness.create_session",
  > HarnessOp::ListSessions => "harness.list_sessions",
  > HarnessOp::Abort => "harness.abort",

- **Fact:** `not-implemented` is the documented answer of a method whose story has not landed, which is why `attach_tui`, `select_session` and `shown_session` return `PaneError::NotImplemented` in 642a.
  **Source:** `crates/holler-pane/src/error.rs:404-405`
  **Verbatim excerpt:**
  > /// `not-implemented`: the skeleton's answer for a verb or method whose story has
  > /// not landed. (#637 defines it; every stub returns it.)

- **Fact:** the conformance suite binds the adapter to answer `session-not-found` for an abort of an unknown id by checking `GET /session/:id` first, and to answer `unavailable` for an unanswering port before it looks at the session id (the order `abort` follows: the existence check is the first request).
  **Source:** `crates/holler-pane-testkit/src/conformance/harness.rs:12-16`
  **Verbatim excerpt:**
  > //! - `abort`, `attach_tui` and `select_session` of an id the server does not know are
  > //!   `session-not-found` (cases 8, 11 and 13). Raw OpenCode acknowledges an abort of an
  > //!   unknown id, so the adapter checks `GET /session/:id` first.
  > //! - A call to a port whose server does not answer is `unavailable`, checked before the
  > //!   session id (case 6).

- **Fact:** the port's contract is synchronous, bounded by I5 (default 10 s) or `Timeout`, and `Send + Sync`; `Timeouts::call` and the per-method `Call` deadline implement that bound.
  **Source:** `crates/holler-pane/src/ports.rs:173-175`
  **Verbatim excerpt:**
  > /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
  > /// thread) in async code. Every method returns within I5's bound (default 10 s) or
  > /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.

- **Fact:** an `unavailable` error renders as `unavailable: <what>`, so the adapter's `what` strings are written to read after that prefix.
  **Source:** `crates/holler-pane/src/error.rs:677`
  **Verbatim excerpt:**
  > PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),

- **Fact:** every pane-control error message is one line and never echoes a secret; the adapter's `one_line` replaces control characters in every external text it quotes, and no message carries an environment value.
  **Source:** `docs/adr/ADR-0021.md:329`
  **Verbatim excerpt:**
  > merged; a code whose condition disappears is retired, not recycled. Every message is one line and never echoes a secret.

- **Fact:** OpenCode's web app answers a route it does not serve with a `200` of its HTML page, so the adapter never treats a `200` alone as success (`json_of` and the shape checks).
  **Source:** `crates/holler-body/src/http_attach_driver.rs:40-42`
  **Verbatim excerpt:**
  > //!   genuinely fire-and-forget. `POST {endpoint}/api/session/{id}/prompt_async`
  > //!   is **not** a real route: it fell through to the SPA's catch-all and
  > //!   returned a `200` of the web UI's own `index.html`, not a driver success.

- **Fact:** a frozen OpenCode server still accepts the TCP connection and never answers, while a killed one refuses within about 6 ms; `http::request` therefore keeps `TimedOut` (connected, no answer) apart from `Refused`.
  **Source:** `docs/research/opencode-pane-spike.md:189-192`
  **Verbatim excerpt:**
  > - **Frozen (SIGSTOP):** a TCP connect still succeeds (the kernel accepts it), and `GET /global/health` gets **no
  >   answer**; the client's own 2 s limit is all that ends it (`000` after 2008 ms). Only an HTTP timeout reveals a
  >   wedge. After `SIGCONT` the server answers again within about 150 ms, and the TUI still obeys `select-session`.
  > - **Killed (SIGKILL):** the connection is refused within about 6 ms.

- **Fact:** a request sent while the server boots can be accepted and never answered, so `serve` sends only bounded health GETs (`boot_try` each) until the first healthy answer.
  **Source:** `docs/research/opencode-pane-spike.md:198-201`
  **Verbatim excerpt:**
  > then **1 or 2 GETs sent at about 570-680 ms were accepted and never answered in 30 s**, while GETs sent a few ms
  > later were answered at once (seen on every run). An earlier manual run had one such request hang for more than
  > 3 minutes. **Every request to OpenCode needs a client timeout, and none but a health poll may be sent before the
  > first healthy answer.**

- **Fact:** `httparse` is already a workspace dependency, so the crate's `httparse = { workspace = true }` adds no crate and changes no workspace manifest line.
  **Source:** `Cargo.toml:72`
  **Verbatim excerpt:**
  > httparse = "1.10.1"

- **Fact:** `httparse` 1.10.1 recognises only `HTTP/1.0` and `HTTP/1.1` in a response's version, so any other version (an `HTTP/2` head, an SSH banner) is a parse error, which `http::request` reports as `Garbled`. This is a dependency's source, outside the repository (the cargo registry checkout).
  **Source:** `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/httparse-1.10.1/src/lib.rs:758-762`
  **Verbatim excerpt:**
  > pub fn parse_version(bytes: &mut Bytes) -> Result<u8> {
  >     if let Some(eight) = bytes.peek_n::<[u8; 8]>(8) {
  >         // NOTE: should be const once MSRV >= 1.44
  >         let h10: u64 = u64::from_ne_bytes(*b"HTTP/1.0");
  >         let h11: u64 = u64::from_ne_bytes(*b"HTTP/1.1");

## T (Phase 4, author RED: rework after the outside diff gate's round 1)

- **Fact:** `httparse::parse_chunk_size` returns the index just past the size line's CRLF (the start of the chunk data) and the chunk size. So `http.rs`'s `let end = line + size;` is the end of the chunk data, and the two bytes after it are the chunk's CRLF (diff gate r1, NV-1). A chunk extension after `;` is skipped. `ac1_content_length_and_chunked_replies_read_to_the_same_bytes` (two chunks of `0x12`) and `ac1_a_chunked_reply_with_an_extension_and_a_trailer_reads_its_body` exercise it. This is a dependency's source, outside the repository (the cargo registry checkout).
  **Source:** `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/httparse-1.10.1/src/lib.rs:1251-1263`
  **Verbatim excerpt:**
  > /// Parse a buffer of bytes as a chunk size.
  > ///
  > /// The return value, if complete and successful, includes the index of the
  > /// buffer that parsing stopped at, and the size of the following chunk.
  > ///
  > /// # Example
  > ///
  > /// ```
  > /// let buf = b"4\r\nRust\r\n0\r\n\r\n";
  > /// assert_eq!(httparse::parse_chunk_size(buf),
  > ///            Ok(httparse::Status::Complete((3, 4))));
  > /// ```
  > pub fn parse_chunk_size(buf: &[u8])

## F (Phase 6, implement: round 2, after the outside diff gate's round 1)

- **Fact:** `std::os::unix::process::CommandExt::process_group` is a safe method (a plain `fn`, not an `unsafe fn`), stable since Rust 1.64.0, and a process group id of 0 makes the child's own pid its process group id. So `start` (`server.rs`) needs no `unsafe` for `process_group(0)`, and the group `stop` kills is the child's own (diff gate r1, NV-3 and NV-4). This is the Rust 1.98.1 standard library, outside the repository, read from the rendered source that rustup's `rust-docs` component installs (`~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/share/doc/rust/html/src/std/os/unix/process.rs.html`); the line numbers are the source's own.
  **Source:** `library/std/src/os/unix/process.rs:174-204` (Rust 1.98.1)
  **Verbatim excerpt:**
  > /// Sets the process group ID (PGID) of the child process. Equivalent to a
  > /// `setpgid` call in the child process, but may be more efficient.
  > ...
  > /// A process group ID of 0 will use the process ID as the PGID.
  > ...
  > #[stable(feature = "process_set_process_group", since = "1.64.0")]
  > fn process_group(&mut self, pgroup: i32) -> &mut process::Command;

- **Fact:** `Child::id` is the child's OS process id, so with `process_group(0)` it is also the child's process group id. `serve` returns it, and `stop` passes it to `kill_group` (diff gate r1, NV-4). The same Rust 1.98.1 standard library and rendered copy as above (`.../html/src/std/process.rs.html`).
  **Source:** `library/std/src/process.rs:2351-2370` (Rust 1.98.1)
  **Verbatim excerpt:**
  > /// Returns the OS-assigned process identifier associated with this child.
  > ...
  > #[stable(feature = "process_id", since = "1.3.0")]
  > ...
  > pub fn id(&self) -> u32 {
  >     self.handle.id()
  > }
