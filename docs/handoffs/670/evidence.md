# Evidence appendix: #670 (F, Phase 6)

Source facts in **unchanged** code that the diff and T's tests rely on. Each excerpt is copied
verbatim from the file at the line given.

- **Fact:** `PaneError::Usage` has the closed code `usage`, so `output::exit_code` (an error coded
  `usage` exits 2) and `output::usage_body` get their code from `holler-pane`, not from a literal in
  `holler-cli`.
  **Source:** `crates/holler-pane/src/error.rs:94-96`
  **Verbatim excerpt:**
  > match self {
  >     PaneCode::NotImplemented => "not-implemented",
  >     PaneCode::Usage => "usage",

- **Fact:** `PaneError::code()` is the stable code string of any error, and `ErrorCode::from(&PaneError)`
  is infallible because of it.
  **Source:** `crates/holler-pane/src/error.rs:402-409`
  **Verbatim excerpt:**
  > /// The kebab-case code of this error: one of [`ALL_CODES`], or the open code of a
  > /// [`Refused`](PaneError::Refused).
  > pub fn code(&self) -> &str {
  >     match self.classify() {
  >         Ok(closed) => closed.as_str(),
  >         Err(open) => open.as_str(),
  >     }
  > }

- **Fact:** `is_valid_code` is the one code validator in the workspace, and `ErrorCode::new` calls it
  instead of keeping a second grammar.
  **Source:** `crates/holler-pane/src/error.rs:156-158`
  **Verbatim excerpt:**
  > /// This is the only code validator in the workspace; the CLI's output module calls
  > /// it. It is a `const fn`, so a verb's code constant can be checked at build time.
  > pub const fn is_valid_code(code: &str) -> bool {

- **Fact:** `Ports` is `Copy`, which is why `VerbCtx` holds it by value and `Wiring::ports()` lends it.
  **Source:** `crates/holler-pane/src/ports.rs:224-227`
  **Verbatim excerpt:**
  > /// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
  > /// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
  > #[derive(Clone, Copy)]
  > pub struct Ports<'a> {

- **Fact:** `SpecFlags::validate()` returns the codes below because it calls these guards unchanged: a
  grid text that is not a cell is `grid-ambiguous`, a zero or an overflow is `grid-out-of-range`.
  **Source:** `crates/holler-pane/src/grid.rs:44-51`
  **Verbatim excerpt:**
  > pub fn parse(text: &str) -> Result<GridPos, PaneError> {
  >     let lower = text.trim_ascii().to_ascii_lowercase();
  >     let (row, col) = split_cells(&lower).ok_or_else(|| PaneError::GridAmbiguous {
  >         what: format!("{} ({FORMS})", excerpt(text)),
  >     })?;
  >     match (coordinate(row), coordinate(col)) {
  >         (Some(row), Some(col)) => Ok(GridPos { row, col }),
  >         _ => Err(PaneError::GridOutOfRange {

- **Fact:** `--command-json` and `--check-json` map to `usage` for text that is not JSON and to
  `command-not-argv` for JSON that is not an array of strings, through `Argv::from_json`.
  **Source:** `crates/holler-pane/src/argv.rs:48-57`
  **Verbatim excerpt:**
  > pub fn from_json(text: &str) -> Result<Self, PaneError> {
  >     serde_json::from_str::<Argv>(text).map_err(|e| {
  >         if e.is_data() {
  >             PaneError::CommandNotArgv
  >         } else {
  >             PaneError::Usage {
  >                 message: format!("not valid JSON: {e}"),
  >             }
  >         }
  >     })

- **Fact:** `--env` maps to `profile-secret-refused` for `NAME=value` and to `env-name-invalid` for an
  empty name or one with whitespace, through `EnvVarName::parse`, and neither refusal carries the text.
  **Source:** `crates/holler-pane/src/argv.rs:97-104`
  **Verbatim excerpt:**
  > pub fn parse(text: &str) -> Result<Self, PaneError> {
  >     if text.contains('=') {
  >         return Err(PaneError::ProfileSecretRefused);
  >     }
  >     if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
  >         return Err(PaneError::EnvNameInvalid);
  >     }
  >     Ok(Self(text.to_owned()))

- **Fact:** The startup banner is always one line on stderr, written by `main.rs` before dispatch, so a
  real `holler pane ... --format=json` run has a non-empty stderr (the in-process seam test is where
  "nothing on `err`" holds, and the process tests assert only the refusal line and stdout).
  **Source:** `crates/holler-proto/src/log.rs:329-338`
  **Verbatim excerpt:**
  > // Always text, regardless of `format`: the banner is the one line a human
  > // reads to confirm the settings took effect, and it precedes blocking I/O.
  > let line = format!(
  >     "{} INFO {} -- logging_started level={} format={}",
  >     timestamp(),
  >     Component::Cli.column(),
  >     config.debug.as_str(),
  >     config.format.as_str()
  > );
  > eprintln!("{line}");

- **Fact:** An ADR 0003 row is parsed by `docs_cli_test` after this normalisation, so a row can keep
  `[optional]` groups and a trailing annotation (cut at two spaces or ` (`) and still parse as the bare
  verb; this is why the `pane` and `profile` rows list no positionals.
  **Source:** `crates/holler-cli/tests/docs_cli_test.rs:132-144`
  **Verbatim excerpt:**
  > fn normalise(raw: &str) -> Vec<String> {
  >     let mut s = raw.to_string();
  >     // Cut a trailing annotation: two+ spaces, " (", or " — ".
  >     for sep in ["  ", " (", " — ", " -- "] {
  >         if let Some(i) = s.find(sep) {
  >             s.truncate(i);
  >         }
  >     }
  >     // Drop optional groups [ ... ], innermost first so nesting such as
  >     // `[--advertise HOST[:PORT]]` collapses cleanly.
  >     while let Some(close) = s.find(']') {
  >         let Some(open) = s[..close].rfind('[') else { break };
  >         s.replace_range(open..=close, "");

## Rework 1 (F, after S pass 2, item 1)

Source facts in **unchanged** code that the `resolve_tail` fix relies on. The clap behaviour behind
it (`num_args` bounds the values of one occurrence of a positional, and a flag between positionals
starts another) is the crate's, not this repo's, so it is not quoted: it is shown by running the
built binary before and after the fix (handoff-F.md, "Proof").

- **Fact:** The tail of `say` is bounded only by `num_args = 0..=2`, which is the whole of clap's
  limit on its positionals (`interrupt` has the same attribute at `cli.rs:536`, `answer` has
  `1..=2` at `cli.rs:587`), so nothing in the clap tree itself stops a third value that arrives
  in a later occurrence of `rest`.
  **Source:** `crates/holler-cli/src/cli.rs:499-500`
  **Verbatim excerpt:**
  > #[arg(value_names = ["SESSION", "TEXT"], num_args = 0..=2)]
  > pub rest: Vec<String>,

- **Fact:** Each prompt verb calls `route(x.resolve(), ...)` as its first statement and returns the
  `Stop`'s message and exit code at once, so a `Usage` from `resolve_tail` ends the run with exit 2
  before the `--pane`/`--profile` refusal, the `--timeout` check, `--parts-file` or any hub. (`say`
  is quoted; `interrupt_cmd.rs:39-42` and `answer_cmd.rs:39-42` are the same four lines with their
  own names.)
  **Source:** `crates/holler-cli/src/say_cmd.rs:116-119`
  **Verbatim excerpt:**
  > let Routed { session, arg } = match route(say.resolve(), &say.profile) {
  >     Ok(routed) => routed,
  >     Err(stop) => return err(stop.message, stop.exit_code),
  > };
