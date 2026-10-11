//! TRIAL (T's satisfiability check only, never committed).
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use holler_adapter_herdr::adapter::HerdrConfig;
use holler_adapter_herdr::plan::Extent;
use holler_adapter_host::TmuxSocket;
use holler_pane::PaneError;
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;
fn usage(m: String) -> PaneError { PaneError::Usage { message: m } }
pub fn herdr_config(env: Env<'_>) -> Result<HerdrConfig, PaneError> {
    let session = env("HOLLER_HERDR_SESSION").or_else(|| env("HERDR_SESSION")).ok_or_else(|| usage("no Herdr session: set HOLLER_HERDR_SESSION or HERDR_SESSION".into()))?;
    let socket = env("HOLLER_HERDR_SOCKET").or_else(|| env("HERDR_SOCKET_PATH")).map(PathBuf::from).unwrap_or_else(|| {
        let config = env("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env("HOME").unwrap_or_default()).join(".config"));
        config.join("herdr/sessions").join(&session).join("herdr.sock")
    });
    let mut config = HerdrConfig::new(session, socket);
    if let Some(text) = env("HOLLER_HERDR_WORKSPACES") {
        for item in text.split(',') {
            let bad = || usage(format!("HOLLER_HERDR_WORKSPACES: {item:?} is not label=RxC"));
            let (label, rc) = item.split_once('=').ok_or_else(bad)?;
            let (r, c) = rc.split_once('x').ok_or_else(bad)?;
            if label.is_empty() { return Err(bad()); }
            let rows = r.parse().map_err(|_| bad())?; let cols = c.parse().map_err(|_| bad())?;
            config = config.with_workspace(label, Extent { rows, cols });
        }
    }
    Ok(config)
}
pub fn tmux_socket(env: Env<'_>) -> Result<TmuxSocket, PaneError> {
    match env("HOLLER_TMUX_SOCKET") { None => Ok(TmuxSocket::Default), Some(p) if p.starts_with('/') => Ok(TmuxSocket::Path(p.into())), Some(p) => Err(usage(format!("HOLLER_TMUX_SOCKET {p:?} is not absolute"))) }
}
pub fn opencode_bin(env: Env<'_>) -> Result<PathBuf, PaneError> {
    if let Some(b) = env("HOLLER_OPENCODE_BIN") { return Ok(b.into()); }
    std::env::split_paths(&env("PATH").unwrap_or_default()).map(|d| d.join("opencode")).find(|c| std::fs::metadata(c).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
        .ok_or_else(|| PaneError::Unavailable { what: "no opencode on PATH: set HOLLER_OPENCODE_BIN".into() })
}
pub fn serve_args(env: Env<'_>) -> Result<Vec<String>, PaneError> {
    env("HOLLER_OPENCODE_SERVE_ARGS").map_or(Ok(Vec::new()), |t| serde_json::from_str(&t).map_err(|e| usage(format!("HOLLER_OPENCODE_SERVE_ARGS: {e}"))))
}
