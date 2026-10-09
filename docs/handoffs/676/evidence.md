# Evidence appendix: #676 (source facts in unchanged code that the diff and the tests rely on)

Written by F (Phase 6 of the script). Line numbers are as of the F diff (post-change files).

- **Fact:** `PaneCode::parse` finds a closed code by its exact wire string and returns `None` for any other text, so `class_of`'s `let Some(closed) = PaneCode::parse(code) else` sends every non-closed code to the open-or-malformed branch.
  **Source:** `crates/holler-pane/src/error.rs:126-128`
  **Verbatim excerpt:**
  > ```
  >     pub(crate) fn parse(text: &str) -> Option<PaneCode> {
  >         PaneCode::ALL.iter().copied().find(|c| c.as_str() == text)
  >     }
  > ```

- **Fact:** `is_valid_code` is the workspace's one code validator and refuses the empty string, so `class_of("")` takes the malformed (Failure) arm.
  **Source:** `crates/holler-pane/src/error.rs:163-166`
  **Verbatim excerpt:**
  > ```
  > pub const fn is_valid_code(code: &str) -> bool {
  >     let bytes = code.as_bytes();
  >     if bytes.is_empty() {
  >         return false;
  > ```

- **Fact:** a `PaneError::Refused`'s `code()` is its open code (the `Err` side of `classify`), which T's `an_open_code_is_a_refusal` passes to `class_of`.
  **Source:** `crates/holler-pane/src/error.rs:497-502`
  **Verbatim excerpt:**
  > ```
  >     pub fn code(&self) -> &str {
  >         match self.classify() {
  >             Ok(closed) => closed.as_str(),
  >             Err(open) => open.as_str(),
  >         }
  >     }
  > ```

- **Fact:** a malformed code read off the wire becomes `unavailable`, which is the precedent for `class_of` classing a malformed code as a Failure.
  **Source:** `crates/holler-pane/src/error.rs:599-611`
  **Verbatim excerpt:**
  > ```
  >     pub(crate) fn from_wire(code: String, message: String, detail: Option<String>) -> PaneError {
  >         if let Some(closed) = PaneCode::parse(&code) {
  >             return PaneError::from_closed(closed, message, detail);
  >         }
  >         match RefusalCode::parse(code) {
  >             Ok(code) => PaneError::Refused { code, message },
  >             Err(bad) => PaneError::Unavailable {
  >                 what: format!(
  >                     "the reply carried an error with an unusable code ({bad}): {}",
  >                     excerpt(&message)
  >                 ),
  >             },
  >         }
  > ```

- **Fact:** text mode and JSON mode both take an error's exit code from the one private `exit_code`, so changing its body changes both formats alike.
  **Source:** `crates/holler-cli/src/output.rs:283` and `crates/holler-cli/src/output.rs:292`
  **Verbatim excerpt:**
  > ```
  >             settle(written, exit_code(&error))
  > ```
  > ```
  >             let code = exit_code(&error);
  > ```

- **Fact:** `settle` only turns a failed write on a success (0) into 1. An error keeps its own code, so a refusal whose write fails still exits 3.
  **Source:** `crates/holler-cli/src/output.rs:328-334`
  **Verbatim excerpt:**
  > ```
  > fn settle(written: io::Result<()>, code: i32) -> i32 {
  >     if written.is_err() && code == 0 {
  >         1
  >     } else {
  >         code
  >     }
  > }
  > ```

- **Fact:** `emit_stream` returns the first non-zero code `emit` returns, so a refusal item ends a stream with 3 (T's `emit_stream_exits_3_on_a_refusal_item`).
  **Source:** `crates/holler-cli/src/output.rs:231-237`
  **Verbatim excerpt:**
  > ```
  >     for item in items {
  >         let code = emit(sink, format, item, &text);
  >         if code != 0 {
  >             return code;
  >         }
  >     }
  >     0
  > ```

- **Fact:** an `ErrorBody`'s code is always well-formed: `ErrorCode::new` accepts only what `is_valid_code` accepts. `class_of`'s malformed branch therefore cannot be reached from the CLI.
  **Source:** `crates/holler-cli/src/output.rs:104-110`
  **Verbatim excerpt:**
  > ```
  >     pub fn new(code: &str) -> Result<Self, InvalidCode> {
  >         if is_valid_code(code) {
  >             Ok(Self(code.to_owned()))
  >         } else {
  >             Err(InvalidCode(code.to_owned()))
  >         }
  >     }
  > ```

- **Fact:** the process exits with whatever the `pane` or `profile` verb returns, unchanged, so `main.rs` needs no edit for exit 3 to reach the shell.
  **Source:** `crates/holler-cli/src/main.rs:112` and `crates/holler-cli/src/main.rs:225-228`
  **Verbatim excerpt:**
  > ```
  >     std::process::exit(dispatch(&cli, choice));
  > ```
  > ```
  >         Command::Pane(cmd) => run_with_stdio(choice.format, |ctx| holler_cli::pane::run(cmd, ctx)),
  >         Command::Profile(cmd) => {
  >             run_with_stdio(choice.format, |ctx| holler_cli::profile::run(cmd, ctx))
  >         }
  > ```
