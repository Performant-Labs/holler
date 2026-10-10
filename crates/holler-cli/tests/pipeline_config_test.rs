//! Pins the committed Aftersight OpenCode pipeline files (issue #704).
//!
//! The pipeline is data, not code: nothing fails a build if the committed files
//! drift, so these tests are the regression check, pinning the same facts the
//! pre-flight reports (the pinned release, the verify command, the stage-model
//! table) where CI runs them. `#674` shares `.aftersight/pipeline-pin.json` and
//! `.aftersight/pipeline.config.json` with this pipeline; the pins below allow
//! its `claude` entry but never let this story's files clobber it.
//!
//! `HOLLER_PIPELINE_TEST_ROOT` overrides the directory the files are read from
//! (the repository root by default). It exists so a broken scratch copy in a
//! temp directory can be shown to fail these tests without ever editing a
//! tracked file.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)] // #704

use std::path::{Path, PathBuf};

/// The eight roles the OpenCode release ships (the shim's agent blocks).
const ROLES: &[&str] = &[
    "orchestrator",
    "tester",
    "feature-implementor",
    "architecture-reviewer",
    "spec-auditor",
    "final-reviewer",
    "designer",
    "playwright-ui-walkthrough",
];

/// Every pipeline file this story committed; each is pinned by a test below.
const FILES: &[&str] = &[
    ".aftersight/pipeline-pin.json",
    ".aftersight/pipeline.config.json",
    ".aftersight/.gitignore",
    "opencode.json",
    "AGENTS.md",
];

fn pipeline_root() -> PathBuf {
    std::env::var("HOLLER_PIPELINE_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(pipeline_root().join(rel))
        .unwrap_or_else(|e| panic!("committed pipeline file {rel}: {e}"))
}

fn json(rel: &str) -> serde_json::Value {
    serde_json::from_str(&read(rel))
        .unwrap_or_else(|e| panic!("committed pipeline file {rel} must be valid JSON: {e}"))
}

#[test]
fn pin_names_the_opencode_release_and_a_claude_entry_stays_claude_s() {
    let pin = json(".aftersight/pipeline-pin.json");
    let opencode = pin
        .get("opencode")
        .expect("an `opencode` entry: this repo runs the OpenCode pipeline");
    assert_eq!(
        opencode["tag"], "opencode-pipeline-v0.1.0",
        "the operator-pinned release (a bump is a deliberate pin change, not drift)"
    );
    // The pin file is shared with the Claude adoption (#674): its entry may
    // appear, but it must stay that pipeline's own shape — never repointed or
    // half-overwritten by an OpenCode-side change.
    if let Some(claude) = pin.get("claude") {
        assert_eq!(
            claude["pipeline"], "claude",
            "a claude entry in the shared pin must remain #674's, untouched"
        );
        let tag = claude["tag"]
            .as_str()
            .unwrap_or_else(|| panic!("claude entry tag must be a string: {claude}"));
        assert!(
            tag.starts_with("claude-pipeline-v"),
            "a claude entry must name a claude release tag, got {tag}"
        );
    }
}

#[test]
fn aftersight_gitignore_holds_the_release_and_prompts_lines() {
    let ignore = read(".aftersight/.gitignore");
    for line in ["releases/", "prompts/"] {
        assert!(
            ignore.lines().any(|l| l.trim() == line),
            "`.aftersight/.gitignore` must hold `{line}` so the per-machine release link and generated prompts never enter git: {ignore}"
        );
    }
}

#[test]
fn config_test_command_is_the_ci_cargo_argv() {
    let config = json(".aftersight/pipeline.config.json");
    let command = config
        .pointer("/test/unit/command")
        .expect("`test.unit.command` (both pipelines read it; phase F runs it)")
        .as_array()
        .expect("`test.unit.command` is an argv array, never a shell string");
    let argv: Vec<&str> = command
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("argv elements are strings: {command:?}"))
        })
        .collect();
    assert!(
        argv.iter().all(|part| !part.is_empty()),
        "argv elements are non-empty: {argv:?}"
    );
    assert_eq!(
        &argv[..2],
        &["cargo", "test"],
        "phase F runs the workspace suite, not npm"
    );
    assert!(
        argv.windows(2)
            .any(|w| w == ["--skip", "roster_stays_accurate_under_concurrent_body_load"]),
        "the command keeps CI's skip (ci.yml's workspace test step): {argv:?}"
    );
}

#[test]
fn config_implement_stage_is_claude_cli_opus_xhigh() {
    let config = json(".aftersight/pipeline.config.json");
    let stage = config
        .pointer("/stages/implement")
        .expect("`stages.implement`: the operator-decided stage-model table");
    assert_eq!(
        stage["executor"], "claude-cli",
        "F runs on the claude-cli executor (login mode)"
    );
    assert_eq!(stage["model"], "claude-opus-5-5");
    assert_eq!(stage["effort"], "xhigh");
}

#[test]
fn every_role_block_is_glm_and_the_plugin_lives_behind_the_release_link() {
    let opencode = json("opencode.json");
    for role in ROLES {
        let block = opencode
            .pointer(&format!("/agent/{role}"))
            .unwrap_or_else(|| panic!("an agent block for `{role}`"));
        assert_eq!(
            block["model"], "zai/glm-5.3",
            "stage-model table: every role on zai/glm-5.3 ({role})"
        );
    }
    let plugin = opencode["plugin"]
        .as_array()
        .expect("the `plugin` entry")
        .first()
        .expect("one plugin path")
        .as_str()
        .expect("the plugin path is a string");
    assert!(
        Path::new(plugin.trim_start_matches("./"))
            .starts_with(".aftersight/releases/opencode"),
        "the plugin resolves through the per-machine `.aftersight/releases/opencode` link, never a copied release: {plugin}"
    );
}

#[test]
fn no_committed_pipeline_file_carries_a_home_path() {
    for rel in FILES {
        let text = read(rel);
        assert!(
            !text.contains("/home/") && !text.contains("/Users/"),
            "a committed pipeline file must be machine-independent (no home paths): {rel}"
        );
    }
}
