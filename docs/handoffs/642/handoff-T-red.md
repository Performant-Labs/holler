# Handoff-T-red: Phase 4 - #642a the OpenCode adapter, server side (`serve`, `health`, `create_session`, `list_sessions`, `abort`, `http::request`)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (on 80901fa, A's PASS commit)
**Brief / wireframe reviewed:** docs/handoffs/642-brief.md (this run is 642a only); docs/handoffs/642/handoff-A.md. Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned PASS on the plan (handoff-A.md at 80901fa, 0 blocks, 8 warns). The warns that touch the tests are
settled as follows:

- **W-5 (AC 24's port rule).** The hermetic stub binds `127.0.0.1:0`, and a refused port is bound and then dropped, as the
  brief's own stub design and `attach_cli_test/fake_server.rs` both do. No test names a fixed port. The 48100-48199 rule
  applies to 642b's real-OpenCode rig, and this run adds none.
- **W-6 (dev-dependencies).** Only `holler-pane-testkit` is declared, for the `HarnessOp` pin. `tempfile` is not declared.
  The `serve` tests make their own scratch directory under `std::env::temp_dir()` and remove it on drop.
- **W-7 (the stub's cousin).** `tests/support/stub.rs` is modelled on `holler-cli/tests/attach_cli_test/fake_server.rs`.
  It uses `std::net` and threads, `Connection: close`, and bind-then-drop, and its module doc names that file as its cousin.
  A-dup should check the stub against that file.

## Surface landed before the tests (the brief's Test plan: "the minimum public stubs")

The brief's API, with no behaviour. Every stub body is marked `// stub (#642a): F fills`.

- `crates/holler-adapter-opencode/Cargo.toml`
  - dependencies: `holler-pane`, `serde_json`
  - dev-dependency: `holler-pane-testkit`
  - Each carries a one-line comment naming its use. F adds `httparse` together with the code that uses it, because an
    unused dependency fails `cargo machete`.
- `src/lib.rs`
  - `Resolver`, `ProcessEnv`, `Timeouts` (its `Default` is all zeros), `OpenCodeConfig`, and `OpenCodeHarness` with
    `new`, holding a private `_config`.
  - The `HarnessPort` impl: every method answers `PaneError::NotImplemented`. That is final for `attach_tui`,
    `select_session` and `shown_session` in 642a.
  - Re-exports `TmuxConfig` and `TmuxSocket`.
- `src/http.rs`: `Reply`, `HttpError { Refused, TimedOut, Garbled(String) }`, and `request`, which returns
  `Ok(Reply { status: 0, body: [] })`. That reply fails every AC 1 case. A stub that answered `Garbled` would have passed
  one of them.
- `src/tui.rs`: `TmuxSocket { Default, Name, Path }` and `TmuxConfig { tmux_bin, socket }`. 642a has nothing more here.
- `Cargo.lock`: the crate's dependency list only (mechanical).
- **Derives the tests rely on, which F keeps:**
  - `Debug, Clone, PartialEq, Eq` on `Reply`, `HttpError`, `ProcessEnv`, `TmuxSocket` and `TmuxConfig`.
  - `Debug, Clone, Copy, PartialEq, Eq` on `Timeouts`.
  - `Clone` on `OpenCodeConfig`.
- F replaces the stub bodies and the `_config` field, writes the crate docs, and may add `src/exec.rs`.

## Tests authored

**Files**

- `crates/holler-adapter-opencode/tests/hermetic_test.rs` (647 lines)
- `crates/holler-adapter-opencode/tests/support/stub.rs` (243 lines; a `#[path]` module)

There is no `[[test]]` entry to add, because this crate does not set `autotests = false`.

**Tier.** Every test is an integration test of the crate's public API against a loopback stub. That is the cheapest tier
that exercises the real socket behaviour the ACs name (refused versus connected-but-silent, framing, the raw request line).
No test runs OpenCode, tmux, `kill` or `/proc`, so the file runs on the Linux and macOS CI runners (Risk 6).

**What `serve` runs.** `serve` only ever spawns `false`, or `sh` on a scratch `serve` script (`touch spawned; exit 42`). The
script is run as `sh serve --port P ...` in the pane's working directory.

**The stub (`support/stub.rs`).**

- Replies are canned per method and raw path. The default is OpenCode's JSON 404.
- Bodies can be framed by `Content-Length` or chunked.
- It can send a raw non-HTTP answer.
- It has a frozen mode: it accepts and reads the request, and holds the socket without answering.
- It records each raw request line and body as received (W-4).
- `closed_port()` binds a port and drops it.
- `HTML` is OpenCode's web-app catch-all page: 263 bytes, with newlines.

I sanity-checked the stub with a throwaway raw-`TcpStream` test, then deleted it. Framing, the request record, raw mode,
the frozen hold and the refused port all behaved as intended. So the RED below is not hiding a broken fixture.

| Test | Pins |
|---|---|
| `ac1_content_length_and_chunked_replies_read_to_the_same_bytes` | AC 1: a sized and a two-chunk reply give status 200 and the same bytes |
| `ac1_a_closed_port_is_refused_within_a_second` | AC 1: `Refused` within 1 s |
| `ac1_a_frozen_server_times_out_within_the_timeout` | AC 1: `TimedOut` within the timeout (400 ms) plus 300 ms |
| `ac1_a_reply_that_is_not_http_is_garbled` | AC 1: an `SSH-2.0-...` answer is `Garbled` |
| `ac2_health_is_true_for_the_healthy_json` | AC 2: `{"healthy":true,...}` gives `Ok(true)` |
| `ac2_health_is_false_for_unbound_frozen_html_unhealthy_and_500` | AC 2: `Ok(false)` for an unbound port, a frozen stub (within `health` plus 300 ms), a 200 of HTML, a 200 `{"healthy":false}`, and a 500 |
| `ac3_create_session_titles_the_session_with_its_id` | AC 3: the request lines are exactly `POST /session` then `PATCH /session/<id>`; the PATCH body's `title` equals the id; the id is returned |
| `ac3_a_failed_title_patch_deletes_the_session_and_is_unavailable` | AC 3: a PATCH 500 gives `unavailable`, and the stub received `DELETE /session/<id>` |
| `ac3_a_patch_reply_with_another_title_is_unavailable` | AC 3 (W-3): a PATCH 200 whose `title` is not the id gives `unavailable` |
| `ac4_list_sessions_returns_the_listed_ids_in_order` | AC 4 |
| `ac11e_child_sessions_are_left_out_of_the_list` | AC 11e: a string `parentID` is left out; a missing or `null` one is kept |
| `ac5_abort_of_an_unknown_id_is_session_not_found_and_sends_no_abort` | AC 5: a 404 existence check gives `SessionNotFound { what: id }`, and no `.../abort` is sent. The Behaviour rule that a 400 reads as a 404 is checked in the same loop |
| `ac5_abort_of_an_idle_known_session_is_ok` | AC 5: `Ok`, and `POST /session/<id>/abort` was sent |
| `ac5_abort_of_a_session_that_stays_busy_times_out` | AC 5: `timeout` with `op == HarnessOp::Abort.as_str()`, within `call` plus 300 ms |
| `ac6_server_calls_to_an_unbound_port_are_unavailable` | AC 6 (642a clause): `create_session`, `list_sessions` and `abort` give `unavailable`, naming the port |
| `ac7_the_call_bound_caps_the_request_timeout` | AC 7: on a frozen stub with `call: 1 s` and the default 5 s `request`, the result is `timeout` with `op == HarnessOp::CreateSession.as_str()` in under 1.5 s |
| `ac8_serve_refuses_a_port_that_already_answers_healthy` | AC 8: `unavailable` naming the port; nothing is spawned (no marker); the only requests are `GET /global/health` |
| `ac8_serve_of_a_missing_binary_names_it` | AC 8: `unavailable`, and the message holds the missing path |
| `ac8_serve_on_a_frozen_port_times_out_and_spawns_nothing` | AC 8: `timeout` with `op == HarnessOp::Serve.as_str()`, within `call` plus 300 ms; nothing is spawned |
| `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status` | AC 8 (W-4): `false` gives `unavailable` in under 3 s with `call` = 10 s. `sh serve` gives `unavailable` in under 3 s, with the marker present (it ran in the `workdir`) and "42" in the message after the port is removed |
| `ac11_the_adapter_is_send_sync_and_static` | AC 11: a compile-time pin. It passes by construction (see below) |
| `ac11_default_timeouts_are_10s_5s_2s_500ms_5s` | AC 11 |
| `ac11d_an_html_200_for_the_session_is_unavailable_and_sends_no_abort` | AC 11d: `unavailable`; the message is one line of at most 200 bytes and names `/session/` and 200; no abort is sent |
| `ac11d_a_session_reply_for_another_id_is_unavailable_and_sends_no_abort` | AC 11d: the same rules, for a mismatched `id` |
| `ac11d_an_html_200_for_the_abort_is_unavailable` | AC 11d: an HTML abort reply gives `unavailable`; the message names `/abort` and 200 |
| `ac11d_an_html_200_for_the_list_is_unavailable` | AC 11d: an HTML `GET /session` gives `unavailable`, with the same message rules |
| `ac11d_a_session_id_is_percent_encoded_in_the_request_line` | AC 11d: `abort(port, "ses x/?")` sends a first raw line of exactly `GET /session/ses%20x%2F%3F HTTP/1.1`, and is `session-not-found` |

**Not authored in 642a, by the brief's 642a AC list:**

- AC 9, 10, 11a, 11b, 11c, 11f, 12-19a, 25 and 26
- the `attach_tui` clauses of AC 6 and AC 11d
- `real_opencode_test.rs`

**Gates rather than tests (checked at GREEN):** AC 20 (the hermetic half), 21, 22, 23 and 24.

## RED confirmation

`cargo test -p holler-adapter-opencode` gives `test result: FAILED. 1 passed; 26 failed`. Every failure is an assertion
about the missing behaviour. None is a compile error, a missing target or a harness fault.

```
ac11_default_timeouts_are_10s_5s_2s_500ms_5s   left: Timeouts { call: 0ns, request: 0ns, health: 0ns, boot_try: 0ns, settle: 0ns }
                                               right: Timeouts { call: 10s, request: 5s, health: 2s, boot_try: 500ms, settle: 5s }
ac1_content_length_and_chunked_...             left: 0  right: 200
ac1_a_closed_port_is_refused_within_a_second   left: Ok(Reply { status: 0, body: [] })  right: Err(Refused)
ac1_a_frozen_server_times_out_...              left: Ok(Reply { status: 0, body: [] })  right: Err(TimedOut)
ac1_a_reply_that_is_not_http_is_garbled        expected Garbled, got Ok(Reply { status: 0, body: [] })
ac2_health_is_true_for_the_healthy_json        left: Err(NotImplemented)  right: Ok(true)
ac2_health_is_false_for_...                    left: Err(NotImplemented)  right: Ok(false)
ac3_create_session_titles_...                  left: Err(NotImplemented)  right: Ok("ses_0123456789abcdefABCDEFghij")
ac3_a_failed_title_patch_...                   a PATCH that answers 500: expected unavailable, got Err(NotImplemented)
ac3_a_patch_reply_with_another_title_...       a PATCH reply whose title is not the id: expected unavailable, got Err(NotImplemented)
ac4_list_sessions_returns_...                  left: Err(NotImplemented)  right: Ok(["ses_b", "ses_a"])
ac11e_child_sessions_are_left_out_...          left: Err(NotImplemented)  right: Ok(["ses_a", "ses_c"])
ac5_abort_of_an_unknown_id_...                 left: Err(NotImplemented)  right: Err(SessionNotFound { what: "ses_missing" })
ac5_abort_of_an_idle_known_session_is_ok       left: Err(NotImplemented)  right: Ok(())
ac5_abort_of_a_session_that_stays_busy_...     a session that stays busy: expected timeout, got Err(NotImplemented)
ac6_server_calls_to_an_unbound_port_...        create_session: expected unavailable, got Err(NotImplemented)
ac7_the_call_bound_caps_...                    create_session on a frozen server: expected timeout, got Err(NotImplemented)
ac8_serve_refuses_a_port_...                   serve on a healthy port: expected unavailable, got Err(NotImplemented)
ac8_serve_of_a_missing_binary_names_it         serve of a missing binary: expected unavailable, got Err(NotImplemented)
ac8_serve_on_a_frozen_port_...                 serve on a frozen port: expected timeout, got Err(NotImplemented)
ac8_serve_of_a_program_that_exits_at_once_...  serve of `false`: expected unavailable, got Err(NotImplemented)
ac11d_an_html_200_for_the_session_...          an HTML session reply: expected unavailable, got Err(NotImplemented)
ac11d_a_session_reply_for_another_id_...       a reply for another session: expected unavailable, got Err(NotImplemented)
ac11d_an_html_200_for_the_abort_...            an HTML abort reply: expected unavailable, got Err(NotImplemented)
ac11d_an_html_200_for_the_list_...             an HTML list reply: expected unavailable, got Err(NotImplemented)
ac11d_a_session_id_is_percent_encoded_...      left: Err(NotImplemented)  right: Err(SessionNotFound { what: "ses x/?" })
```

**The one passing test.** `ac11_the_adapter_is_send_sync_and_static` is a compile-time pin. The stub already has the
brief's field types (`Arc<dyn Fn + Send + Sync>`), so it holds by construction and cannot be RED at runtime. It guards
against F adding a non-`Send` field. I left it as a separate test rather than folding it into a failing one.

**The stubbed code builds cleanly:**

- `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings`: clean.
- `rustfmt --check --edition 2021` on every new `.rs` file: clean.
- `bash scripts/lint.sh`: exit 0, with a 600-line warning for `hermetic_test.rs` at 647. The brief's split point is 800.
- `cargo machete`: nothing.
- `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode`: nothing.

**Staged by explicit path:**

- `Cargo.lock`
- `crates/holler-adapter-opencode/{Cargo.toml,src/lib.rs,src/http.rs,src/tui.rs,tests/hermetic_test.rs,tests/support/stub.rs}`
- `docs/handoffs/642/{handoff-T-red.md,decisions.md}`

## Ready for F

RED is valid. F may implement against these tests. Notes for F:

- **Messages.** The `unavailable` messages the tests read must contain:
  - the port (AC 6, AC 8's in-use case);
  - the binary path (AC 8);
  - the exit status (AC 8, which checks for "42" with the port removed);
  - for reply-shape failures, the route and the status `200`, on one line of at most 200 bytes (AC 11d).
- **Error payloads.** `SessionNotFound.what` is the id, verbatim (not percent-encoded).
- **`create_session`.** The request sequence is pinned as exactly `POST /session` then `PATCH /session/<id>` (AC 3). Send no
  extra request on the success path.
- **The serve tests' `workdir`.** It is a scratch directory, and the `sh serve` case requires the server's working directory
  to be that `workdir`.
