//! What one run of the `holler` binary produced, and how a test collects it. Moved here from
//! `main.rs` (#667, decision 3) so the two targets that include the scratch set
//! (`pane_integration` and `profile_apply_scenario`) share one definition: `scratch/mod.rs`
//! names it as `crate::{output_of, Out}`, and each target's `main.rs` re-exports it.

use std::process::Command;

use holler_pane_testkit::envelope::{check_envelope, Envelope};

/// What one run of the `holler` binary produced.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    /// Stdout as exactly one envelope that agrees with the exit code (#638's checker).
    pub fn envelope(&self) -> Envelope {
        check_envelope(&self.stdout, self.code)
            .unwrap_or_else(|fault| panic!("stdout is one envelope: {fault}: {self:?}"))
    }

    /// The error code of the envelope on stdout.
    pub fn error_code(&self) -> Option<String> {
        self.envelope().error.map(|e| e.code)
    }

    /// The `data` of a successful envelope on stdout.
    pub fn data(&self) -> serde_json::Value {
        let envelope = self.envelope();
        assert!(envelope.ok, "a successful envelope: {self:?}");
        envelope.data
    }
}

/// Run `command` to its end and collect what it printed.
pub fn output_of(mut command: Command) -> Out {
    let output = command.output().expect("run the holler binary");
    Out {
        code: output.status.code().expect("holler exited, not killed"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}
