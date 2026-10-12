//! `--harness opencode|claude` among the spec flags (#831, ADR-0021 section 15 item 12; plan
//! review W13): declared once in `SpecFlags`, typed by `SpecFlags::validate` as the spec's
//! `harness.kind`, and absent unless given (a new spec then is `opencode`, an existing one keeps
//! its own).

use clap::Parser;
use holler_cli::pane::args::SpecFlags;
use holler_cli::pane::PaneCmd;
use holler_cli::{Cli, Command};
use holler_pane::pane::HarnessKind;

/// The spec flags of `holler pane <verb> demo-c1r1 <extra...>`.
fn flags(verb: &str, extra: &[&str]) -> SpecFlags {
    let mut argv = vec!["holler", "pane", verb, "demo-c1r1"];
    argv.extend_from_slice(extra);
    let cli = Cli::try_parse_from(&argv).unwrap_or_else(|e| panic!("{argv:?}: {e}"));
    match cli.command {
        Command::Pane(PaneCmd::Launch(launch)) => *launch.spec,
        Command::Pane(PaneCmd::Relaunch(relaunch)) => *relaunch.spec,
        other => panic!("not launch or relaunch: {other:?}"),
    }
}

#[test]
fn harness_is_typed_on_launch_and_on_relaunch() {
    for verb in ["launch", "relaunch"] {
        let claude = flags(verb, &["--harness", "claude"]).validate().unwrap();
        assert_eq!(claude.harness, Some(HarnessKind::Claude), "{verb}");
        let opencode = flags(verb, &["--harness", "opencode"]).validate().unwrap();
        assert_eq!(opencode.harness, Some(HarnessKind::Opencode), "{verb}");
        let absent = flags(verb, &[]).validate().unwrap();
        assert_eq!(absent.harness, None, "{verb}: no default at the flag");
    }
}

#[test]
fn an_unknown_harness_is_usage_and_never_echoed_raw() {
    for bad in ["codex", "Claude", "", "claude\u{1b}[2J"] {
        let refused = flags("launch", &["--harness", bad]).validate().unwrap_err();
        assert_eq!(refused.code(), "usage", "{bad:?}: {refused}");
        assert!(
            !refused.to_string().contains('\u{1b}'),
            "no control sequence: {refused}"
        );
    }
}
