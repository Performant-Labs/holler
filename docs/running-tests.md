# Running the tests

**Issue:** [#172](https://github.com/Performant-Labs/holler/issues/172)

How to run holler's tests, by hand: the plain `cargo test` commands, the test-runner's selection flags, the `Automation` grammar, and the Ruby/octokit gotchas. The *design* of the harness lives in [`testing.md`](testing.md); this is the *how*.

## The `cargo test` commands

The harness is an ordinary `cargo test` suite. The commands below are the ones a developer runs; they are also the ones CI runs (see the table at the end). All are run from the workspace root.

### The whole thing

```
cargo test --workspace
```

Builds and runs every test in every crate (unit + integration). This is CI's "full test suite" step and the command to run after a change.

### One crate's integration tests

```
cargo test -p holler-cli
```

Runs the `holler-cli` integration tests (the wire harness + canary). Add `-p holler-hub` or `-p holler-proto` for the other crates.

### One integration test file

```
cargo test -p holler-cli --test <file>
```

`<file>` is the stem of a file in `crates/holler-cli/tests/`. The files that exist today (one behavior per file):

| `--test <file>` | What it covers |
|---|---|
| `wire_selftest` | **the canary** — test-of-tests; run this *first* (it is CI's first gate) |
| `stub_acp_test` | the `stub-acp` agent's ACP v2 behaviour (chunks, permission/`blocked`, cancel, crash) |
| `support_selftest` | unit-tests of the shared `tests/support` harness itself |
| `hub_serve_test` | `holler hub serve` on the wire: join, presence, the real protocol v2 |
| `cli_surface_test` | every ADR-0003 CLI command parses (and the pending ones don't) |
| `docs_cli_test` | every `holler …` command the docs show parses against the real clap tree |
| `logging_test` | hub/body JSON structured-logging behaviour |
| `token_cli_test` | `hub token list` / `delete` CLI surface |
| `cli_invocation_test` | CLI invocation/arg-handling behaviour |

### One test (function)

```
cargo test -p holler-cli --test <file> <fn-name>
```

Cargo treats the trailing argument as a test-name filter, so this runs only tests whose name contains `<fn-name>`. For a **lib** (unit) test use `--lib` instead of `--test <file>`:

```
cargo test -p holler-proto --lib codec
```

### The `--`-passed libtest flags

Everything after a **bare `--`** is passed straight to the test binary (libtest), not to cargo. The ones you'll reach for:

```
cargo test -p holler-cli --test <file> -- --exact <fn>      # match the name exactly (not as a substring)
cargo test -p holler-cli --test <file> -- --nocapture       # stream stdout/stderr live instead of capturing it
```

`--exact` turns the filter from a *substring* match into a *whole-name* match (so `-- --exact say` runs only `say`, not `say_roundtrip`). Two of the integration files use **rstest** parameterized cases (`cli_invocation_test`, `codec_test`); each case's libtest name is `<fn>::case_N_<suffix>` (list them with `-- --list`), so to run a single case, target it by that exact name:

```
cargo test -p holler-proto --test codec_test -- --exact names_grammar_invalid::case_1_empty
```

### The interop / cross-host opt-in

"Interop" is a **tag**, not a test file: cases labelled `test-tag-interop` are the cross-host ones, and their `Automation` resolves to the same command **plus `-- --ignored`** (everything after a bare `--` is passed to the test binary; `--ignored` tells libtest to run only the `#[ignore]`d tests — the real cross-process, possibly cross-host cases). So on a normal run you *invert* them away, and to run *just* them you select the tag:

```
ruby scripts/test-run.rb exec --tag-invert interop     # a normal run: skip the cross-host cases
ruby scripts/test-run.rb exec --tag interop            # run only the cross-host (ignored) cases
```

The runner does this automatically from the tag — you do not hand-write `-- --ignored`. (To drive one such case directly from cargo, append `-- --ignored` to its resolved command, e.g. `cargo test -p holler-cli --test <file> -- --ignored`.)

### The standalone binary

The `stub-acp` agent is its own binary (declared as a `[[test]]` with `harness = false`), so it builds like a test but is a normal executable:

```
cargo build -p holler-cli --test stub-acp
```

The harness launches it by that path (`cargo_bin`); you do not run it by hand.

## The runner: `scripts/test-run.rb`

The runner selects test cases from the GitHub issue **catalog** (issue [#168](https://github.com/Performant-Labs/holler/issues/168)) and, for the `auto` ones, runs their `Automation` fields for real — which invokes the `cargo test` commands above. It writes results back into a *test-run issue* (a `test-run`-labelled issue with a results table). It has five subcommands:

| Command | What it does |
|---|---|
| `discover` | Pull the catalog (issues labelled `test-case` with a `\| Test ID \|` header) and print it, one case per entry. **Needs a token** — a bare `discover` always builds the GitHub client (unlike `exec --list`), so it reads the *live* catalog (96 entries today) and aborts with a token error if there is no `GITHUB_TOKEN`/`gh`. |
| `start` | Create a new `test-run` issue with one `⏳ pending` row per selected case. Flags: `--applies`, `--group`, `--cat`, `--tag`, `--tag-invert`, `--type`. |
| `run ISSUE` | Run every `auto` case in test-run issue `ISSUE` and write ✅/❌ + evidence back into that issue. Flag: `--dir DIR`. |
| `record ISSUE TEST_ID pass\|fail [note]` | Manually record a (usually `manual`) case's result in the issue's table. |
| `exec` | Select cases and run them **locally, now** — streaming real `cargo test` output — with **no GitHub write at all**. The flags and their semantics are below. |

### `exec`'s selection flags (conjunction)

A bare `exec` (no flags) prints usage and exits. Any combination of the flags below is a **conjunction** over the catalog: every given axis must match, applied in sequence to the survivors of the previous one (Playwright's `--grep ∧ --project` model). A positional `TEST_ID` still works and *adds* one more conjunct.

| Flag | Axis | Meaning |
|---|---|---|
| `--applies hub\|body\|both\|all` | applies | only cases whose `Applies to:` matches (`all` = no constraint) |
| `--group G` | group | one of the 11 `#168` groups. Accepts the **full name** (`concurrency`), the **exact label stem** (`invoc` — only `invocation` has a stem distinct from its name; the other ten are full-name == stem), or the **raw label** (`test-grp-invoc`). It is an *exact* lookup, not a prefix: an unrecognized value (e.g. `invoca`) aborts listing the valid groups |
| `--cat C` | category | one of the **closed** set `smoke \| regression \| acceptance \| unit` (`all` = no constraint) |
| `--tag S` (repeatable) | tags | an **open-ended** set of `test-tag-*` tags; a case matches if it carries **ANY** of them (OR-within) |
| `--tag-invert S` (repeatable) | tag-invert | exclude any case carrying these tags (e.g. `--tag-invert interop` skips the cross-host cases on a normal run) |
| `--list FILE` | list-keep | keep only the Test IDs listed in `FILE` (one per line; `#` comments and blanks skipped) |
| `--list-invert FILE` | list-drop | drop the Test IDs listed in `FILE` |
| `--list` (bare, no file) | **token-free preview** | a whole-catalog preview that needs **no token and no octokit**: it passes a `nil` client, so `discover` returns `[]`, and it resolves the selection against that empty catalog (prints the matched IDs, or `0 case(s) matched`, then exit 0). It deliberately does **not** fetch the live catalog — see the "Token-free preview" note below |
| `--last-failed N` | list-keep (derived) | read the ❌ rows of test-run issue `N` and run exactly those |
| `--dir DIR` | (not a selection axis) | the checkout to run in (default `.`) |

Examples:

```
ruby scripts/test-run.rb exec --applies body --group io
ruby scripts/test-run.rb exec --cat smoke --tag-invert interop
ruby scripts/test-run.rb exec hlr-1000            # one specific case
ruby scripts/test-run.rb exec --list               # token-free whole-catalog preview (no token, no octokit; prints the matched IDs, none when the catalog is empty)
ruby scripts/test-run.rb exec --last-failed 123    # re-run only the ❌ rows of run #123
```

`exec` exits with the same status the underlying `cargo test` exits with — exit 0 only if every selected case passed.

## Integration check against the live catalog (story #172)

The acceptance for this story is to point the runner at the **real** catalog — 96 filled `test-case` issues created by the separate #168 retrofit — not just the synthetic fixtures #170 was built against. Run on `main` 2026-09-08:

- `discover` pulls the live catalog and the runner **addresses it end-to-end**: `exec --applies all` → 96, `--applies both` → 86, `--applies hub` → 10, `--group protocol` → 33. Selection, the `GROUPS` aliasing, and `Automation` resolution all work against real issues.
- **One true gap:** `exec --applies body` → **0 matched**. The live catalog carries only `Applies to: both` (86) and `Applies to: hub` (10) — **no `body` case exists yet**. The runner correctly reports "no catalog cases matched the selection (applies: body)" and exits non-zero (the designed behavior for an empty selection); it is the *catalog* that has no body cases to select, not a runner bug. It is a catalog-coverage observation for the #168 retrofit track, and the point stands: the runner addresses the live cases faithfully.
- `exec --list` (bare) is the **token-free** smoke (no token, no octokit) and exits 0 against an empty catalog — it is *not* how you read the live 96 (that needs a client path, e.g. `discover` or `exec --last-failed N`).

### The two non-obvious `Automation` shapes are exercised, not just the bare `tests/` form

Shape distribution across the 96: **93 `crates/<crate>/tests/` cases + 4 `src/` cases + 1 `;`-joined multi-crate case** (which spans two `tests/` segments), and **zero** fall back to the whole-workspace fallback and **zero** fail to parse. The common `crates/<crate>/tests/<file>.rs (<fn>)` form is the uninteresting 93; the interesting part is that the runner *also* resolves the two rarer shapes correctly. Verified by running the runner's *real* `Automation` parser (`scripts/automation.rb`) against the live catalog and then executing the resolved commands:

- **The semicolon multi-crate case** (case `hlr-1203`, logging group, the only `;`-joined case in the catalog) is a single `Automation` field with two segments across two crates:
  ```
  crates/holler-cli/tests/logging_test.rs (debug_flag_beats_env); crates/holler-proto/tests/log_test.rs (resolve_flag_beats_env)
  ```
  `split_segments` yields **2** segments; `parse_segment` resolves each to its own crate — `cargo test -p holler-cli --test logging_test debug_flag_beats_env` and `cargo test -p holler-proto --test log_test resolve_flag_beats_env` — **both `fallback_used == false`** (neither fell through to the whole-workspace fallback). Executing both: **each passes** (`1 passed` per crate). The runner runs all segments and `exit 0`s only if *all* pass.
- **The four `src/` cases** (`hlr-1001`–`hlr-1004`, all in `crates/holler-cli/src/cli.rs`) each resolve to a **unit** test — `cargo test -p holler-cli --lib <fn>` — via the `src/` grammar branch (`lib_test == true`, no fallback). All four functions exist in `cli.rs` and all four **pass** under `--lib`.

So the runner addresses the live catalog across all three shape classes (bare `tests/`, `;`-joined multi-crate, and `src/` unit), not just the common one.

## The `Automation` grammar

Each catalog case's `Automation` field is one or more `;`-separated **segments**; the runner resolves each to a `cargo` command it will run. The grammar (exact, from issue [#168](https://github.com/Performant-Labs/holler/issues/168) §5) is a **pure function** in `scripts/automation.rb`, shared by `run` and `exec` so they cannot drift. `<crate>` is derived from the path form — a `tests/<file>.rs` with **no** crate prefix means the `holler-cli` crate:

| Automation segment | Resolves to |
|---|---|
| `tests/<file>.rs` | `cargo test -p holler-cli --test <file>` |
| `tests/<file>.rs (<fn>)` | `cargo test -p holler-cli --test <file> <fn>` |
| `crates/<crate>/tests/<file>.rs (<fn>)` | `cargo test -p <crate> --test <file> <fn>` |
| `crates/<crate>/src/<path>.rs (<fn>)` | `cargo test -p <crate> --lib <fn>` (a *unit* test) |

- **Multiple segments** (separated by `; `) all must pass.
- **`manual`** (a leading `manual` segment) marks a manual procedure — the runner never executes it; `run` records it as pending and `exec` skips it.
- **Fallback:** a segment matching none of the forms above runs the **whole workspace** (`cargo test`) as a conservative fallback and sets `fallback_used` — the case's issue body must say so.
- **`interop`:** a case labelled `test-tag-interop` runs with `-- --ignored` (a real cross-process, possibly cross-host run). On the single binary there is **no sibling build and no env var** — `cargo_bin` finds `holler`. `env` is always `{}`.

## Ruby / octokit gotchas

- **`octokit` is required.** `test-run.rb` needs the `octokit` gem (`gem install octokit --user-install`). The selection *core* (`test_selection.rb` + `automation.rb`) is deliberately **stdlib-only**, so `ruby scripts/test_selection_test.rb` and the grammar unit test run on a bare runner with no gem install.
- **Auth:** the script uses `GITHUB_TOKEN` if set, else `gh`'s stored token via `gh auth token` — so a machine already logged in with `gh` needs no separate credential setup. If neither is present, the GitHub-touching paths abort with "no GITHUB_TOKEN and `gh auth token` failed".
- **Token-free preview:** `require 'octokit'` lives *inside* the lazy `client` builder, not at the top of the file, on purpose — a top-level `require` would fire at load, before `main` knows which subcommand was requested, and die with a `LoadError` on a box without the gem. So a bare `exec --list` (a whole-catalog **preview**) passes a `nil` client: `discover` returns `[]` immediately, no token is read, and octokit is **never loaded**. This is what lets CI's `exec --list` smoke run on a runner that has *no* GitHub token secret and a flaky gem environment.

Consequence worth knowing: the bare `exec --list` resolves the selection against that **empty** catalog, so it prints `0 case(s) matched` (or nothing if the selection matches nothing) and exits 0 — it is a *smoke* that the selection machinery runs cleanly, **not** a way to dump the live catalog. To list the *real* 96 cases from the live catalog, use a path that actually builds a client (e.g. `exec --last-failed N`, or a flagged selection that fetches).
- **macOS ships Ruby 2.6.** CI runs `test_selection_test.rb` on the runners' preinstalled Ruby (2.6 on macOS) — the selection core is stdlib-only precisely so no gem install is needed there. If *octokit* won't install on your system Ruby (e.g. an old macOS system Ruby), use a Homebrew Ruby: `/opt/homebrew/opt/ruby/bin/ruby scripts/test-run.rb …`. CI runs the `exec --list` smoke on **ubuntu only** for exactly this reason (macOS 2.6 is below what the octokit path needs, and the full minitest selection test already runs on both OSes).

## What CI runs (and in what order)

From [`.github/workflows/ci.yml`](../.github/workflows/ci.yml), on each of **`ubuntu-latest` + `macos-latest`** (Windows deliberately off — see [`testing.md`](testing.md#windows-is-deferred-off-the-ci-matrix)):

| Step | Command | Why in this order |
|---|---|---|
| Lint | `bash scripts/lint.sh` | source-shape guards that need no compiler — the cheapest check, so first |
| **Canary** | `cargo test -p holler-cli --test wire_selftest` | **the first gate** — `--test wire_selftest` alone (not `cargo test`, which also builds the libs) so CI goes red as early as possible if the runner swallows a failure |
| Full suite | `cargo test --workspace` | every test, every crate |
| Selection test | `ruby scripts/test_selection_test.rb` | the runner's pure-function core (skipped until the file exists; runs on preinstalled Ruby) |
| Runner smoke | `ruby scripts/test-run.rb exec --list` (ubuntu only, **no** token) | the `#170` "tolerate an empty catalog" smoke: a bare `--list` is token-free, so it runs against an empty catalog, prints `0 case(s) matched` / nothing, and exits 0 — it guards against a crash, not against catalog contents |
| Clippy | `cargo clippy --all-targets -- -D warnings` | warnings are failures |
| Unused deps | `cargo machete` (ubuntu only) | a crate must declare only what it consumes |
| PR size | advisory (ubuntu only) | a PR past 800 added lines is *warned*, not failed |
