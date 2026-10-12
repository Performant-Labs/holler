# Brief: #844 oc-tmux-utf8

Repo: Performant-Labs/holler. Issue: #844 (part of epic #633). Rigor: in-session. UI surface: no. Kind: bug.

## Problem
The OpenCode adapter reads a pane's TUI through one tmux call, `display-message -p` with a TAB-joined five-field format. When the
holler process has no UTF-8 locale (`LANG=C`, `LC_ALL=C`, or both unset), the tmux client prints every TAB as `_`, so
`parse_query` finds one field, not five, returns `Query::NoPane`, and `attach_tui`, `select_session` and `shown_session` all
report "no tmux session" for a pane that is fine. The #649 scenario hides it with a test-chosen `LANG=C.UTF-8`; production
inherits the operator's locale.

## Evidence (verbatim; the outside reviewer cannot read the repo)

Reproduced on this machine (tmux 3.7c, private socket, `LC_ALL=C`, `LANG` unset). Same call, with and without `-u`:
```
$ tmux -S $S display-message -p -t =demo: "#{session_name}<TAB>#{pane_dead}" | od -c
0000000   d   e   m   o   _   0  \n          <- no -u: TAB became "_"
$ tmux -S $S -u display-message -p -t =demo: "#{session_name}<TAB>#{pane_dead}" | od -c
0000000   d   e   m   o  \t   0  \n          <- with -u: TAB kept
```

The one place every adapter tmux process is built (`crates/holler-adapter-opencode/src/tui.rs:114-133`):
```rust
/// A `Command` for one tmux call: `<tmux_bin> [-S <path> | -L <name>] <args...>`, with
/// `TMUX` and `TMUX_PANE` removed from the child's environment. Every tmux process the
/// adapter starts is built here.
pub fn tmux_command(tmux: &TmuxConfig, args: &[String]) -> Command {
    let mut command = Command::new(&tmux.tmux_bin);
    match &tmux.socket {
        TmuxSocket::Default => {}
        TmuxSocket::Name(name) => {
            command.arg("-L").arg(name);
        }
        TmuxSocket::Path(path) => {
            command.arg("-S").arg(path);
        }
    }
    command
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE");
    command
}
```
Its single caller (`crates/holler-adapter-opencode/src/attach.rs:217`): `exec::capture(tui::tmux_command(self.tmux, args), self.op, until)`.
`tmux_command` is called nowhere else in the workspace (grep). The format that breaks, and the parser that fails closed on it
(`tui.rs:38-39`, `229-249`):
```rust
pub const QUERY_FORMAT: &str =
    "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}";
...
let fields: Vec<&str> = reply.splitn(5, '\t').collect();
let &[name, dead, status, start_command, title] = fields.as_slice() else {
    return Query::NoPane;
};
```
The existing pin of the argv shape (`crates/holler-adapter-opencode/tests/tui_test.rs:197-224`, test
`ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane`): it builds `tmux_command` for `Path`, `Name`, `Default`, and
asserts `args == flag ++ call` (so, today, no extra flag), and that `TMUX`/`TMUX_PANE` are removed from the child.

The fake tmux (`crates/holler-adapter-opencode/tests/fixtures/fake-tmux`) requires `$1` to be `-S` (`[ "$1" != "-S" ]` exits 64),
shifts `-S <path>`, and records every argument after that, one line per call. `attach_test.rs` asserts those recorded lines
exactly (`query()` at `attach_test.rs:134-136` is `["display-message","-p","-t","=demo-c1r1:",FORMAT]`; also `:203`, `:358`, `:409`).

The workaround to retire (`crates/holler-cli/tests/pane_integration/scratch/mod.rs:253-257`): a comment
"Chosen by the test, not inherited: without a UTF-8 locale the tmux client ..." and `("LANG", "C.UTF-8".to_owned())`.

The host adapter is not affected: its tmux formats are space-joined (`crates/holler-adapter-host/src/tmux.rs:26,29,35,39`:
`"#{pane_pid} #{window_id}"` etc.), and it runs no TAB format.

## Acceptance criteria
From the issue: "Make the adapter's tmux client force UTF-8 (`-u`) or parse the tab-escaped form, and pin it with a test that
runs with `LANG=C`." Made observable:
1. `tmux_command` puts `-u` on every tmux process the adapter starts, after the socket flag and before the call's arguments:
   `tmux [-S p | -L n] -u <args...>`. Pinned by `ac11c` (extended; see Test plan) for all three sockets.
2. With the process locale `LANG=C` and `LC_ALL` unset (and again with `LC_ALL=C`), against a real private tmux server, the
   adapter's `shown_session` for a pane whose title is `OC | ses_...` and whose start command is an `opencode attach` line
   returns that session id, not "no tmux session". The test fails (RED) before the change.
3. `cargo test -p holler-adapter-opencode` and `cargo test -p holler-cli --test pane_integration` pass; the #649 scenario no
   longer needs a test-chosen UTF-8 locale and its `LANG=C.UTF-8` line is removed (the scenario runs with the ambient `LANG`).
4. `grep -n '"-u"' crates/holler-adapter-opencode/src/tui.rs` shows the one place `-u` is added; no second tmux builder exists.
5. No behaviour change otherwise: every other `attach_test.rs` assertion on a tmux call still passes with only the `-u`
   position accounted for (see Files, Fake tmux).

## Files
Production (one): `crates/holler-adapter-opencode/src/tui.rs` (`tmux_command`: add `-u`; its doc comment and the module-doc
rule list, "One tmux server" / add a rule "UTF-8 output": why `-u`). Optionally one sentence in `docs/adr/ADR-0021.md` (the
`HarnessPort` as built (#642) list, around line 198-204) and a `CHANGELOG.md` bullet under `[Unreleased]` (Fixes).

Tests (T edits or adds):
- `crates/holler-adapter-opencode/tests/tui_test.rs`: update `ac11c` so `expected` is `flag ++ ["-u"] ++ call`.
- Fake tmux: no fixture change is needed, because `-u` goes after `-S <path>`, so `$1` is still `-S`. But the fixture records
  `-u` as the first recorded token of every call, so `attach_test.rs` expectations (`query()`, `:203`, `:358`, `:409` and the
  other recorded-call comparisons) must either start with `"-u"` or the fixture must skip a `-u` that follows the socket.
  Decided (below): the fixture skips it, so `attach_test.rs` keeps its assertions and the recorded calls stay "the call tmux was
  asked". `fake-tmux` keeps git mode 100755 and its POSIX-sh-and-`cat`-only rule (AC 33).
- New RED test: a real-tmux test run with `LANG=C` (see Test plan), in a new file, `crates/holler-adapter-opencode/tests/utf8_locale_test.rs`
  (not appended to `real_opencode_test.rs`, which is an opt-in real-OpenCode rig behind `OPT_IN`). Skips cleanly with an
  `eprintln!` when no `tmux` is on `PATH`, as `real_opencode/rig.rs:90` does for its opt-in.
- `crates/holler-cli/tests/pane_integration/scratch/mod.rs`: remove the `("LANG", "C.UTF-8")` entry and its comment (line 253-257).

Reuse map (extend, do not duplicate): `tmux_command` is the single builder; `support/fake_tmux.rs` (`Fake`, `fixture`) for the
hermetic side; the private-socket-in-`/tmp` pattern of `tests/real_opencode/rig.rs` (short path, `-f /dev/null`, never the default
server) for the real side.

## Decisions already made (MO)
1. `-u`, not tab-escape parsing. The issue offers both; `-u` is a client flag with no change to the query format, the parser or
   any reply, and is the fix the issue's reproduction names. Parsing `_` for TAB would be ambiguous with a title that holds `_`.
2. `-u` goes after the socket flag and before the call arguments (tmux accepts global flags in any order), so `$1` stays `-S` for
   the fake and the recorded-call comparisons only need the fixture's skip.
3. The fixture skips a leading `-u` when recording (one `[ "$1" = "-u" ] && shift` after `shift 2`), but the new real-tmux test is
   the pin that tmux really gets `-u`; `ac11c` pins the argv.
4. The locale of the child is not forced (`LC_ALL=C.UTF-8` would fail on a host without that locale); `-u` makes tmux assume UTF-8
   whatever the locale. No env var is added or removed.

## Out of scope
Anything in `holler-adapter-host` / `holler-adapter-claude` (space-joined formats; unaffected); a bounded subprocess runner
(#696); the `attach_tui` / pane-switch-after-relaunch behaviour (#845); test fakes (#846); flakes (#847, #848); locale handling
anywhere else in the CLI; a new setting or env variable; any live tmux or Herdr.

## Test plan
RED first (T authors, runs before F writes code):
1. `ac11c` extended to expect `-u`: fails today, args lack `-u`.
2. New real-tmux test (`utf8_locale_test.rs`): start a private tmux server (`-S <short /tmp path> -f /dev/null`) with a detached
   session named as a `PaneName` running `sleep`, set its pane title with `select-pane -T 'OC | ses_x'`, then run
   `tmux_command(&TmuxConfig { tmux_bin, socket: Path }, &query_args(&name))` with `.env("LANG","C").env_remove("LC_ALL")`
   (and again with `LC_ALL=C`), and assert `parse_query` returns `Query::Live { title: "OC | ses_x", .. }`, not `NoPane`. It needs
   no OpenCode. RED today: the reply has `_` for TAB, so `parse_query` returns `NoPane`.
3. GREEN after F adds `-u`; the fixture skip keeps `attach_test.rs` green unchanged. The #649 scenario with `LANG` removed is
   verified by `cargo test -p holler-cli --test pane_integration` (it is opt-in/skips without real binaries; T runs it if the
   rig's opt-in is available, otherwise states it was not run).
Run with `cargo test -p holler-adapter-opencode`; never `cargo fmt` workspace-wide (format only the touched files).

## Risks
- tmux versions without `-u`: it exists in tmux 1.x onward; no risk.
- `-u` applies to the client only; the server and panes' own locale are untouched.
- A test that mutates process env races other tests: the new test sets env on the child `Command` only, never `set_var`.
- tmux 3.7c prints `_` only for non-UTF-8 clients; a macOS system tmux may differ, so the new test asserts the behaviour (Live)
  not the bytes, and skips when tmux is absent.

## Clashes (sibling stories; files they are likely to touch)
- **#845** (pane switch after relaunch; OpenCode adapter / switch path): `crates/holler-adapter-opencode/src/attach.rs`,
  `src/tui.rs` (possible), `crates/holler-pane/src/tx_switch*` and `crates/holler-cli/tests/pane_verbs/switch.rs`. Our `tui.rs`
  edit is one function (`tmux_command`) plus docs; a merge should be mechanical. Real clash risk if #845 also edits
  `tmux_command` or the module doc; whichever lands second merges main.
- **#846** (`LinkedHost` test fakes in holler-cli tests): `crates/holler-cli/tests/pane_verbs/**` and `profile_verbs/**` rigs.
  We touch only `pane_integration/scratch/mod.rs`; no overlap expected.
- **#847** (`remote_admin_test` flake): unrelated crate (`holler-hub`/`holler-body` tests); none.
- **#848** (`skilltext_w5_rename` test): unrelated; none.
- **#696** (bounded subprocess runner shared by adapters): `crates/holler-adapter-opencode/src/exec.rs`, `crates/holler-adapter-host/src/exec.rs`,
  `attach.rs` (call site at line 217), and `Cargo.toml` of the adapters. We leave `exec.rs` and the call site alone; `tmux_command`
  keeps its signature, so #696 can consume it unchanged. Also shares `CHANGELOG.md` and `docs/adr/ADR-0021.md` with most siblings
  (different paragraphs).
