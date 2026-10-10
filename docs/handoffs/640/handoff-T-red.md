# Handoff-T-red: Phase 4 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr

**Date:** 2026-10-09
**Branch:** issue-640-implementation (base `ce4b896`)
**Brief / wireframe reviewed:** `docs/handoffs/640-brief.md`; wireframe N/A (no UI surface)

## A precondition

Confirmed: A returned **PASS** on the plan (`docs/handoffs/640/handoff-A.md`, 0 block, 7 warn). Of the warns, only
finding 5 is T's, and it is applied: the four test transports of AC 16, 17, 28 and 29 share one interceptor,
`wire_herdr::Tap` (below), rather than four structs.

## What T added

Stubs, per Decision 17 (each stub body is marked `// stub (#640 part 2): F fills`):

- `src/transport.rs`: `MAX_REPLY_BYTES`, `Transport`, the `Arc<T>` forwarder (one line, written in full),
  `UnixSocketTransport::{new, socket}` in full; `exchange` returns `Err(PaneError::NotImplemented)`.
- `src/adapter.rs`: `DEFAULT_TIMEOUT`, `HerdrConfig` with `new`/`with_workspace` in full, `HerdrAdapter<T =
  UnixSocketTransport>` with `config()` in full; `connect`, `connect_with` and all seven `HerdrPort` methods return
  `Err(PaneError::NotImplemented)`. The field `transport` is never read by a stub, so it carries
  `#[allow(dead_code)] // #640 stub (#640 part 2): F fills, and removes this allow`. **F removes that allow.**
- `src/lib.rs`: `pub mod adapter; pub mod transport;`, and the module list in the crate docs now names all five
  modules (no re-exports). F may reword it.
- `Cargo.toml`: dev-dependency `tempfile = { workspace = true }` with a consumer comment (AC 41). `Cargo.lock` gains
  only the `"tempfile"` line under `holler-adapter-herdr`.

Test code (T's):

- `tests/wire_herdr/mod.rs` (650 lines): `WireHerdr`, the pinned API exactly (`new`, `set_protocol`,
  `create_workspace`, `add_tab`, `split`, `tree`, `requests`, `methods`, `answer`, `impl Transport`), plus `Tap` and
  `Tapped` (A finding 5): `Tap::new(Arc<WireHerdr>, hook)`, where the hook gets `(&WireHerdr, request line,
  deadline)` and returns `Tapped::Forward(line)` (the line, rewritten or not, goes to the fake) or
  `Tapped::Reply(line)` (the fake sees nothing).
- `tests/wire_herdr/serve.rs` (84 lines): `serve(Arc<WireHerdr>) -> Served`, `Served::path()`. The socket is bound
  before `serve` returns; `Drop` sets the stop flag, connects once to wake `accept`, joins the thread, and the
  `TempDir` removes the directory.
- `tests/transport_test.rs` (421), `tests/adapter_test.rs` (614), `tests/adapter_conformance_test.rs` (98). No file
  reaches 900; `adapter_test.rs` stays below the ~800 split point, so it was not split.

**The fake follows the merged parser on one point the brief's prose leaves out:** `session.snapshot`'s result nests its
lists under `snapshot` (`{"type":"session_snapshot","snapshot":{"version",...,"workspaces","tabs","panes",
"layouts":[],"agents":[]}}`), as part 1's schema-derived parser and fixtures do (`protocol.rs:388`,
`protocol_test.rs:449`). Decision 15 lists the fields but not that level.

**Before handing off, T checked the fake against part 1's own decoders** with a temporary test file (deleted and not
staged). `decode_reply` + `parse_pong`/`check_supported`/`parse_workspace_created`/`parse_pane_info`/`parse_snapshot`/
`SessionState::workspace`/`parse_layout_export`/`grid_of`/`expect_ok`/`parse_read` all accept its answers. A closed or
unknown pane gives `PaneNotFound` for close, split, send_text, send_keys and read. An unknown tab gives
`Unavailable`. Protocol 99 and `None` are refused. A garbled line and `server.stop` give `invalid_request`. `serve()`
answers three sequential connections. AC 13's two literal trees are what `split(p1, Down)` then `split(p1, Right)`
produce, and `Request::Split{Down}.to_line()` contains both `"method":"pane.split"` and `"direction":"down"`, the
substrings AC 16's rewrite relies on. All passed. So a GREEN failure in these tests should point at `src/`, not at
the oracle. AC 35's grep still prints nothing, because the check lived outside `wire_herdr/`.

## Tests authored

Each test is pinned by name in the brief. Tier: **integration** (crate `tests/`) throughout, because each one drives
the public API of the new modules. The transport needs a real socket, and the adapter tests need a fake Herdr
behind a public seam. No `#[cfg(test)]` unit test can see the stubs' behaviour any more cheaply.

**Transport** (`tests/transport_test.rs`; each test has its own `UnixListener` at `<tempdir>/h.sock`, with the server
written in the test):

| # | Test | Pins |
|---|---|---|
| 1 | `exchange_writes_the_request_line_and_returns_the_reply_line` | the raw reply line without `\n`, compared as text; the server read exactly `Request::Ping.to_line()` |
| 2 | `each_request_opens_its_own_connection` | two exchanges, two accepts, each one line and then EOF (`read_to_end` gives `Ok([])`, so the client closed) |
| 3 | `a_missing_or_non_socket_path_is_unavailable_naming_it` | `Unavailable` whose `what` contains `path.display()`, for a missing path and a regular file |
| 4 | `a_socket_path_too_long_for_the_os_is_unavailable` | a 200-byte path: `Unavailable`, no panic |
| 5 | `a_silent_server_is_timeout_by_the_deadline` | `Timeout { op: "herdr.ping" }` at now+300ms, within 300ms + 2s |
| 6 | `a_dripping_server_is_timeout_by_the_deadline` | one byte every 50ms: `Timeout` within 300ms + 2s (the deadline bounds the whole reply) |
| 7 | `a_server_that_closes_without_replying_is_unavailable` | an empty EOF: `Unavailable` |
| 8 | `a_reply_over_the_limit_is_unavailable` | `MAX_REPLY_BYTES + 1` bytes and no newline, with the connection **held open**, so only the cap can end the read: `Unavailable` whose `what` contains `MAX_REPLY_BYTES.to_string()` (`16777216`) |
| 9 | `a_reply_ended_by_eof_without_a_newline_is_returned` | EOF after a line with no `\n`: `Ok(that line)` |
| 10 | `a_non_utf8_reply_is_unavailable` | `ff fe \n`: `Unavailable` |
| 11 | `a_passed_deadline_is_timeout_without_connecting` | `exchange(.., Instant::now())` is `Timeout`, and a non-blocking `accept()` afterwards is `WouldBlock` |
| 12 | `no_transport_error_echoes_typed_text` | `SendText` of `typed-secret-text` against a silent server (`Timeout { op: "herdr.pane.send_text" }`), a closing server and a missing path (each `Unavailable`): no `Display` or `Debug` contains the secret |
| 12a | `one_wire_condition_gives_one_answer_at_the_deadline` | 20 pings at now+50ms against a silent peer: each is exactly `Timeout { op: "herdr.ping" }` within 50ms + 2s. Then at most 20 kept connections are drained (non-blocking `accept` until `WouldBlock`), and each reaches EOF, after at most the ping line, within 2s. No transport worker panicked |

How 12a is built: the listener accepts nothing during the 20 exchanges, so each connection waits in the backlog. The
client cannot tell that from a server that accepted and stays silent. That lets the test drain the listener
afterwards, as the brief describes. "No panic on any thread" is checked by a chained panic hook, installed once by
`socket()`. It records a panic on any thread that is not a test's thread, `main` or a `test-server` thread. Every
server thread in the file is named `test-server`. So a transport worker that panics, for example on a failed
`send` after the caller gave up, fails 12a. The hook then calls the default hook, so panic output is unchanged.

**Adapter over the wire fake** (`tests/adapter_test.rs`, in process through `Arc<WireHerdr>` or a `Tap`):

| # | Test | Pins |
|---|---|---|
| 13 | `r2c1_and_r1c2_land_in_their_cells_and_read_back` | the brief's bullets exactly: `w1:p1`@r1c1, `w1:p2`@r2c1 (tree 1), `w1:p3`@r1c2 (tree 2); snapshot `p1 r1c1, p3 r1c2, p2 r2c1` with `to_string()` `r1c1 r1c2 r2c1`; the two recorded `pane.split` params |
| 15 | `a_closed_panes_space_goes_to_its_sibling` | after `close(a)`, the tree is the leaf `b` and the snapshot lists `b` at r1c1 |
| 16 | `a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place` | the `Tap` rewrites `"direction":"down"` to `"right"` in `pane.split`. `ensure r2c1` is `Unavailable` naming `w1:p2` and `r1c2`; the snapshot still has `w1:p2`@r1c2; no `pane.close`. A guard asserts that the fake really got `right` |
| 17 | `a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found` | the `Tap` sends `Request::Close{target}.to_line()` to the fake before forwarding the split: `Unavailable` |
| 18 | `an_occupied_cell_sends_no_mutating_request` | the second `ensure r1c1` is the same pane and sends exactly `["session.snapshot", "layout.export"]` |
| 19 | `nesting_is_refused_before_any_split` | (a) a row of two: `ensure r2c1` is `Refused{code == plan::GRID_UNREACHABLE}`; (b) after a nested slot at r1c2: the snapshot lists only `w1:p1`, `ensure r1c1` is `w1:p1`, `ensure r2c1` is `grid-unreachable`. Neither sends `pane.split` or `workspace.create` |
| 20 | `a_missing_workspace_is_created_only_for_r1c1` | `ensure r2c1` on an empty Herdr is `grid-unreachable` after only `session.snapshot`; `ensure r1c1` sends exactly one `workspace.create` `{"label":"w","focus":false}` and returns `w1:p1`@r1c1 |
| 21 | `a_pane_in_another_tab_neither_lists_nor_places` | a tab-2 pane is not listed; `ensure r2c1` splits `w1:p1`, and every `layout.export` reads `w1:t1` |
| 22 | `snapshot_lists_every_workspace_by_label` | an unconfigured `other` is listed by label with its tree positions; two workspaces labelled `w` list `w1:p1` then `w2:p1`, both `workspace == "w"`@r1c1; `session` is the config's on each |
| 23 | `session_and_workspace_errors_send_nothing` | the wrong session is `Unavailable` with no request; an unconfigured workspace is `Unavailable` naming it, with no request; duplicate `w` labels make `ensure_pane` `Unavailable` |
| 24 | `keys_go_out_verbatim` | the params `{"pane_id":"w1:p1","keys":["enter","ctrl+c"]}` |
| 25 | `read_asks_for_recent_text_and_trims` | after 5 typed lines, `read(p,2)` is `"line4\nline5"` with params `{"pane_id","source":"recent","lines":2,"format":"text"}`; `read(p,0)` is `""` and sends `"lines":1` |
| 26 | `version_and_the_gate` | `version()` is `PROTOCOL_22_VERSION`. Protocol `Some(99)` and `None` give `HerdrVersionUnsupported` from `connect_with`, with a one-line message containing `UNSUPPORTED_VERSION` and `protocol::SUPPORTED_VERSIONS` (and `99` for 99). After a good connect, `set_protocol(Some(99))` makes `version()` refuse while `send_text` succeeds |
| 27 | `config_is_validated_before_any_request` | 8 configs, each giving `Usage` with a one-line message, and the fake records no request (see "Readings" below for the substrings) |
| 28 | `one_deadline_covers_every_exchange_of_a_call` | the `Tap` records each deadline. For `ensure r2c1` (after an unmeasured `r1c1`) and for `snapshot()`, there are at least 2 exchanges, every one carries the same `Instant` `d`, and `before + DEFAULT_TIMEOUT <= d <= after + DEFAULT_TIMEOUT`, with no tolerance |
| 29 | `a_garbled_reply_is_unavailable_everywhere` | the `Tap` answers `not json`: `connect_with` is `Unavailable`; after a clean connect, each of the 7 methods is `Unavailable` |
| 30 | `the_fake_numbers_panes_in_base_36` | in a 1x10 workspace, r1c1..r1c9 are `w1:p1`..`w1:p9`, and r1c10 is `w1:pA`@r1c10, returned verbatim |

AC 14 (the mutants) is a GREEN-phase activity by the brief's own text. It needs no test of its own: it reuses AC 13,
16 and 20.

**Conformance** (`tests/adapter_conformance_test.rs`):

| # | Test | Pins |
|---|---|---|
| 31 | `the_adapter_passes_the_suite_from_an_empty_herdr` | `run_herdr_conformance` with a fresh empty `WireHerdr` per case, through `connect_with`: `Ok(())` |
| 32 | `the_adapter_passes_the_suite_with_a_root_pane` | the same, after `fake.create_workspace("scratch")`: `Ok(())` |
| 33 | `the_adapter_passes_the_suite_over_a_real_socket` | `serve()` per case, `HerdrAdapter::connect` on `served.path()`, the `Served` as the guard, in the brief's construction order: `Ok(())` |
| 34 | `no_case_calls_a_method_off_the_allow_list` | it runs the three fixtures again and collects all 33 fakes. Each fake recorded at least one request, so the check is not vacuous, and every method is in `protocol::ALLOWED_METHODS` |

**Greps (AC 35-39)** pass at RED by construction, and still pass with the stubs in place. Each printed nothing:
```
grep -nE "grid_of|parse_|decode_reply|plan_splits|check_supported" crates/holler-adapter-herdr/tests/wire_herdr/*.rs
grep -n "json!" crates/holler-adapter-herdr/src/adapter.rs crates/holler-adapter-herdr/src/transport.rs
grep -nE "Mutex|RefCell|Cell<|OnceLock|static mut" crates/holler-adapter-herdr/src/adapter.rs
grep -rnE "TcpStream|TcpListener|47001|47002|tmux|std::env|env::var|\.config/herdr|HERDR_|unsafe" crates/holler-adapter-herdr/
grep -n "thread::sleep" crates/holler-adapter-herdr/src/*.rs
```
F must keep them empty. In particular, `Mutex` is barred from `adapter.rs`, not from `transport.rs`, and
`thread::sleep` is barred from `src/` (the drip server in `tests/` uses it on purpose).

### Readings the tests pin where the brief leaves room

- **AC 8, "names the limit":** the decimal value of `MAX_REPLY_BYTES` (`16777216`) must appear in `what`. "16 MiB"
  alone does not pass.
- **AC 27, labels:** the brief's `a`/`b` labels are spelled `ws-a`/`ws-b` (and `ws-rows`, `ws-cols`, `ws-both`),
  because a substring test on a single letter means nothing. The substrings per case:
  - empty session: `session`;
  - relative socket `relative-dir/h.sock`: `socket` and the path;
  - zero timeout: `timeout`;
  - 0 rows: the label and `rows`;
  - 0 cols: the label and `cols`;
  - 0 by 0: the label and `rows`, and **not** `cols` ("whichever is zero, `rows` when both are");
  - `ws-a` (0 rows) with `ws-b` (0 cols): `ws-a` and `rows`, and not `ws-b`;
  - empty session with a relative socket: `session`, and not the relative path.
- **AC 28:** besides the brief's assertions, each measured call must make at least 2 exchanges, so "every exchange
  carries `d`" cannot pass on one.
- **Transport `op`:** `herdr.<wire method>` (`herdr.ping`, `herdr.pane.send_text`), as Decision 3 says. A finding 1
  (port-method naming) was not adopted, and no adapter test pins `op`.

## RED confirmation

Command: `cargo test -p holler-adapter-herdr --no-fail-fast` (after `cargo fmt -p holler-adapter-herdr`; `cargo
clippy -p holler-adapter-herdr --all-targets -- -D warnings` is clean, `cargo fmt --check -p holler-adapter-herdr`
exits 0, `bash scripts/lint.sh` exits 0, `cargo machete` is clean).

Summary: the code compiles, the run finishes in under a second, and nothing hangs.

| Target | Result |
|---|---|
| `layout_test`, `plan_splits_test`, `protocol_test` (part 1) | 10 + 19 + 33 = **62 passed, 0 failed** |
| `transport_test` | **0 passed, 13 failed** |
| `adapter_test` | **0 passed, 17 failed** |
| `adapter_conformance_test` | **0 passed, 4 failed** |

All 34 new tests fail. Every failure is the stub's `NotImplemented` meeting an assertion about the missing behaviour.
None is a compile error, a missing target, a setup failure or a timeout. Verbatim failing lines:

```
transport_test:
exchange_writes_the_request_line_and_returns_the_reply_line   transport_test.rs:158  exchange: NotImplemented   (expect)
each_request_opens_its_own_connection                          transport_test.rs:187  exchange: NotImplemented   (expect)
a_missing_or_non_socket_path_is_unavailable_naming_it          transport_test.rs:133  expected Unavailable, got Err(NotImplemented)
a_socket_path_too_long_for_the_os_is_unavailable               transport_test.rs:133  expected Unavailable, got Err(NotImplemented)
a_silent_server_is_timeout_by_the_deadline                     transport_test.rs:231  left: Err(NotImplemented)  right: Err(Timeout { op: "herdr.ping" })
a_dripping_server_is_timeout_by_the_deadline                   transport_test.rs:256  left: Err(NotImplemented)  right: Err(Timeout { op: "herdr.ping" })
a_server_that_closes_without_replying_is_unavailable           transport_test.rs:133  expected Unavailable, got Err(NotImplemented)
a_reply_over_the_limit_is_unavailable                          transport_test.rs:133  expected Unavailable, got Err(NotImplemented)
a_reply_ended_by_eof_without_a_newline_is_returned             transport_test.rs:310  left: Err(NotImplemented)  right: Ok("{\"id\":\"holler:ping\",\"result\":{}}")
a_non_utf8_reply_is_unavailable                                transport_test.rs:133  expected Unavailable, got Err(NotImplemented)
a_passed_deadline_is_timeout_without_connecting                transport_test.rs:331  left: Err(NotImplemented)  right: Err(Timeout { op: "herdr.ping" })
no_transport_error_echoes_typed_text                           transport_test.rs:359  left: Err(NotImplemented)  right: Err(Timeout { op: "herdr.pane.send_text" })
one_wire_condition_gives_one_answer_at_the_deadline            transport_test.rs:380  round 0  left: Err(NotImplemented)  right: Err(Timeout { op: "herdr.ping" })

adapter_test (each at adapter_test.rs:51, the `connect` helper's expect, unless named):
r2c1_and_r1c2_land_in_their_cells_and_read_back, a_closed_panes_space_goes_to_its_sibling,
a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place,
a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found, an_occupied_cell_sends_no_mutating_request,
nesting_is_refused_before_any_split, a_missing_workspace_is_created_only_for_r1c1,
a_pane_in_another_tab_neither_lists_nor_places, snapshot_lists_every_workspace_by_label,
session_and_workspace_errors_send_nothing, keys_go_out_verbatim, read_asks_for_recent_text_and_trims,
version_and_the_gate, one_deadline_covers_every_exchange_of_a_call, the_fake_numbers_panes_in_base_36
                                                                → connect: NotImplemented
config_is_validated_before_any_request       adapter_test.rs:500  HerdrConfig { session: "", socket: "/unused/h.sock", workspaces: {}, timeout: 10s }: expected usage, got Some(NotImplemented)
a_garbled_reply_is_unavailable_everywhere    adapter_test.rs:576  Some(NotImplemented)   (expected Some(Unavailable))

adapter_conformance_test:
the_adapter_passes_the_suite_from_an_empty_herdr   adapter_conformance_test.rs:47  connect: NotImplemented
the_adapter_passes_the_suite_with_a_root_pane      adapter_conformance_test.rs:47  connect: NotImplemented
the_adapter_passes_the_suite_over_a_real_socket    adapter_conformance_test.rs:60  connect: NotImplemented
no_case_calls_a_method_off_the_allow_list          adapter_conformance_test.rs:47  connect: NotImplemented
```

The conformance tests fail on the `expect("connect")` inside `fresh`, the construction step the brief pins (AC 33
step 4). They do not reach `Err([...11 failures])`. That is the "expect on `connect_with` that meets
`NotImplemented`" branch of the brief's Test plan.

**Halves that pass at RED, because nothing runs** (each test still fails on its error-code or value half):

- AC 11: the `WouldBlock` half (nothing connects).
- AC 12: the "no `Display`/`Debug` contains the secret" half (`NotImplemented` quotes nothing).
- AC 18, 20, 23: the "only these methods / no request" halves. They are never reached at RED: the `connect` expect
  fails first.
- AC 19: "no `pane.split`/`workspace.create`".
- AC 27: "the fake records no request".
- AC 34's allow-list half.
- AC 16: "no `pane.close`".

## Ready for F

RED is valid. F may implement against these tests.

For F:

- Fill the stub bodies in `src/transport.rs` and `src/adapter.rs` only, and remove the `#[allow(dead_code)]` on
  `HerdrAdapter::transport`.
- Write no tests. The wire fake and every file under `tests/` are T's. If a test looks wrong, report it, do not edit
  it.
- Keep greps 35-39 empty.
- The brief's open warns are F's to weigh: A finding 2 (no third `excerpt`) and A finding 3 (`Instant + Duration`
  overflow on a huge `timeout`; no test pins it).
