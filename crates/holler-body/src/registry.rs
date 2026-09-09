//! The body's in-process session registry (issue #187): the live map from
//! [`SessionName`] to that session's config + current A2A session state,
//! built once at `body run` startup from the validated session config
//! ([`crate::config::parse`]'s output). [`SessionRegistry::presence_doc`]
//! builds the `session/presence` params every heartbeat sends — issue #182's
//! `Presence { sessions: Vec::new() }` placeholder is this story's to fill.
//!
//! No ACP driver handle lives on [`SessionEntry`] yet, even though the
//! issue's own spec names one (`SessionEntry{config, state, driver handle}`)
//! — no driver type exists until the ACP driver story (#188), and an
//! unused field would trip the workspace's `dead_code` deny (issue #149).
//! #188 adds it alongside its first caller/reader, the same "land a helper
//! with its first caller" rule `scripts/lint.sh` enforces for `#[allow]`.

use std::collections::BTreeMap;

use holler_proto::{Mode, Presence, SessionAd, SessionName, SessionState};

use crate::config::{SessionConfig, SessionMode};

/// One registered session: its (validated) config and current A2A session
/// state. Every session starts `idle` — no turn has run yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub config: SessionConfig,
    pub state: SessionState,
}

/// The body's session registry: every configured session, keyed by its
/// (unique, config-validated) name.
#[derive(Debug, Clone, Default)]
pub struct SessionRegistry {
    sessions: BTreeMap<SessionName, SessionEntry>,
}

impl SessionRegistry {
    /// Build a fresh registry from a validated session list
    /// ([`crate::config::parse`]'s `ParsedConfig::sessions`) — every session
    /// starts `idle`. Uniqueness of `name` is already guaranteed by
    /// `config::parse`'s duplicate-name check, so this never drops a row.
    pub fn from_sessions(sessions: Vec<SessionConfig>) -> Self {
        let sessions = sessions
            .into_iter()
            .map(|config| {
                let name = config.name.clone();
                (name, SessionEntry { config, state: SessionState::Idle })
            })
            .collect();
        Self { sessions }
    }

    /// The number of registered sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// `true` iff no sessions are registered. A config file may legally
    /// contain zero `[[session]]` rows; it is `body run`'s discovery step
    /// (a *missing* config file) that refuses outright, not an empty one.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Look up one session entry by name.
    pub fn get(&self, name: &SessionName) -> Option<&SessionEntry> {
        self.sessions.get(name)
    }

    /// Iterate every registered session, in name order (`BTreeMap` order).
    pub fn iter(&self) -> impl Iterator<Item = (&SessionName, &SessionEntry)> {
        self.sessions.iter()
    }

    /// Build this body's `session/presence` params (the notification issue
    /// #182 sends on every heartbeat and reconnect): one [`SessionAd`] per
    /// registered session, reflecting its current state. `hostname` is the
    /// body's own hostname/label (`Presence::hostname`).
    pub fn presence_doc(&self, hostname: String) -> Presence {
        let sessions = self
            .sessions
            .values()
            .map(|entry| SessionAd {
                name: entry.config.name.as_str().to_string(),
                harness: entry.config.harness.clone(),
                state: entry.state,
                mode: match entry.config.mode {
                    SessionMode::Spawn => Mode::Spawn,
                    SessionMode::Attach => Mode::Attach,
                },
                harness_session_id: entry.config.session_id.clone(),
                turn_started_at: None,
                last_update_at: None,
                pending: None,
                turn_id: None,
                last_turn: None,
            })
            .collect();
        Presence { hostname, sessions }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #187
mod tests {
    use super::*;
    use crate::config::{Interrupt, SessionMode};

    fn spawn(name: &str) -> SessionConfig {
        SessionConfig {
            name: SessionName::parse(name).unwrap(),
            harness: "opencode".to_string(),
            mode: SessionMode::Spawn,
            command: Some(vec!["opencode".to_string(), "acp".to_string()]),
            cwd: None,
            env: None,
            interrupt: Interrupt::Acp,
            endpoint: None,
            session_id: None,
        }
    }

    #[test]
    fn empty_registry_has_no_sessions() {
        let reg = SessionRegistry::from_sessions(Vec::new());
        assert!(reg.is_empty());
        assert_eq!(reg.presence_doc("h".to_string()).sessions.len(), 0);
    }

    #[test]
    fn registered_session_is_gettable() {
        let reg = SessionRegistry::from_sessions(vec![spawn("alpha")]);
        let name = SessionName::parse("alpha").unwrap();
        assert!(reg.get(&name).is_some());
        assert_eq!(reg.get(&name).unwrap().state, SessionState::Idle);
    }
}
