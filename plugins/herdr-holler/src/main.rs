//! `herdr-holler`, the binary the plugin's manifest hooks run (issue #651):
//! `herdr-holler refresh` reads the hub's pane registry once and reports it to
//! Herdr (see the library). Display only: it never changes a pane.
//!
//! Exit 0 when the reports landed, including the `unknown` refresh of a hub
//! that cannot be read (that is the plugin doing its job; one stderr line says
//! why). Exit 1 when an endpoint cannot be resolved or a report did not land,
//! and 2 on a usage error.

use std::ffi::OsString;
use std::process::ExitCode;

use herdr_holler::{Endpoints, Reporter};

/// The one usage line.
const USAGE: &str = "usage: herdr-holler refresh";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [verb] if verb == "refresh" => refresh(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// One refresh, from the endpoints the plugin process's environment names.
fn refresh() -> ExitCode {
    let endpoints = match Endpoints::from_env() {
        Ok(endpoints) => endpoints,
        Err(e) => {
            eprintln!("herdr-holler: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut reporter = Reporter::new(endpoints);
    match reporter.refresh() {
        Ok(done) => {
            if let Some(cause) = reporter.hub_error() {
                eprintln!(
                    "herdr-holler: the hub could not be read ({cause}); {} pane(s) reported unknown",
                    done.pane_reports
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("herdr-holler: {e}");
            ExitCode::FAILURE
        }
    }
}
