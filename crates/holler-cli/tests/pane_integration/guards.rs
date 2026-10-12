//! D10 and A's W-1: the scenario's five guards, each a pure function, tested in the default
//! run. They touch no process, socket, file or environment.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::scratch::scratch_herdr::{gate, Gate};
use crate::scratch::{
    binary_env, check_herdr_session_args, check_inside, check_pane_name, check_session,
    launch_args, tmux_server_env, Settings, PANE_PREFIX,
};

const BASE: &str = "/tmp/hlr649-abcdef";

fn settings() -> Settings {
    let base = PathBuf::from(BASE);
    Settings {
        state_dir: base.join("state"),
        session: "holler649-0123abcd".to_owned(),
        herdr_socket: base.join("h649.ab/home/.config/herdr/sessions/holler649-0123abcd/herdr.sock"),
        tmux_socket: base.join("hlr-tmux-ab/s"),
        opencode: PathBuf::from("/usr/local/bin/opencode"),
        oc_root: base.join("oc"),
        path: "/opt/tmux/bin:/usr/local/bin:/usr/bin:/bin".to_owned(),
        base,
    }
}

fn names(env: &[(String, String)]) -> BTreeSet<String> {
    env.iter().map(|(name, _)| name.clone()).collect()
}

fn value<'a>(env: &'a [(String, String)], name: &str) -> &'a str {
    env.iter()
        .find(|(n, _)| n == name)
        .map_or_else(|| panic!("{name} is set"), |(_, v)| v.as_str())
}

// --- guard 1: opt-in -----------------------------------------------------------------------

#[test]
fn the_gate_runs_only_on_exactly_1() {
    assert_eq!(gate(Some("1")), Gate::Run);
    for other in [None, Some(""), Some("0"), Some("true"), Some("yes"), Some(" 1"), Some("1 ")] {
        assert_eq!(gate(other), Gate::Skip, "{other:?}");
    }
}

// --- guard 2: unique names -----------------------------------------------------------------

#[test]
fn only_a_holler649_session_passes_and_default_is_refused_by_name() {
    assert_eq!(check_session("holler649-0123abcd"), Ok(()));
    for bad in [
        "default",
        "Default",
        "holler640-0123abcd",
        "holler649-0123ABCD",
        "holler649-0123abc",
        "holler649-0123abcde",
        "holler",
        "",
    ] {
        assert!(check_session(bad).is_err(), "{bad:?} is refused");
    }
}

#[test]
fn only_a_hlr649_pane_name_passes() {
    for good in ["hlr649-c1r1", "hlr649-p1", "hlr649-anchor"] {
        assert_eq!(check_pane_name(good), Ok(()), "{good}");
    }
    for bad in ["c1r1", "hj-c1r1", "demo-c1r1", "hlr649-", "hlr649", "HLR649-c1r1", "hlr649-a b"] {
        assert!(check_pane_name(bad).is_err(), "{bad:?} is refused");
    }
    assert!(PANE_PREFIX.starts_with("hlr649"));
}

// --- guard 3: one directory ----------------------------------------------------------------

#[test]
fn a_path_outside_the_scratch_directory_is_refused() {
    let base = Path::new(BASE);
    assert_eq!(check_inside(base, &base.join("hlr-tmux-ab/s")), Ok(()));
    for bad in [
        "/tmp/tmux-1000/default",
        "/home/u/.config/herdr/sessions/x/herdr.sock",
        "/tmp/hlr649-abcdef/../elsewhere/s",
        "/tmp/hlr649-abcdefX/s",
        "relative/s",
        "/tmp/hlr649-abcdef",
    ] {
        assert!(check_inside(base, Path::new(bad)).is_err(), "{bad:?} is refused");
    }
}

// --- guard 4: the binary's environment ----------------------------------------------------

#[test]
fn the_binary_env_is_built_from_nothing_inherited() {
    let env = binary_env(&settings()).unwrap();
    let expected: BTreeSet<String> = [
        "HOME",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
        "PATH",
        "HOLLER_STATE_DIR",
        "HOLLER_DEBUG",
        "HOLLER_LOG_FORMAT",
        "HOLLER_HERDR_SESSION",
        "HOLLER_HERDR_SOCKET",
        "HOLLER_HERDR_WORKSPACES",
        "HOLLER_TMUX_SOCKET",
        "HOLLER_OPENCODE_BIN",
        "HOLLER_OPENCODE_SERVE_ARGS",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(names(&env), expected, "exactly these, no HERDR_*, TMUX or TMUX_PANE");
    for name in ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"] {
        assert!(value(&env, name).starts_with(&format!("{BASE}/oc/")), "{name} is scratch");
    }
    assert_eq!(value(&env, "HOLLER_STATE_DIR"), format!("{BASE}/state"));
    assert!(
        names(&env).iter().all(|n| n != "LANG" && !n.starts_with("LC_")),
        "no locale at all (#844): the adapter's tmux client forces UTF-8 itself: {env:?}"
    );
    assert_eq!(value(&env, "HOLLER_HERDR_SESSION"), "holler649-0123abcd");
}

#[test]
fn the_binary_env_refuses_a_real_session_or_a_path_outside_the_directory() {
    let refused = |change: fn(&mut Settings)| {
        let mut s = settings();
        change(&mut s);
        binary_env(&s).expect_err("refused")
    };
    refused(|s| s.session = "default".to_owned());
    refused(|s| s.session = "hj".to_owned());
    refused(|s| s.tmux_socket = PathBuf::from("/tmp/tmux-1000/default"));
    refused(|s| s.herdr_socket = PathBuf::from("/home/u/.config/herdr/herdr.sock"));
    refused(|s| s.state_dir = PathBuf::from("/home/u/.local/state/holler"));
    refused(|s| s.oc_root = PathBuf::from("/home/u"));
}

// --- guard 4, for an argument vector (#667 A13, A's W-2) -----------------------------------

const SCRATCH: &str = "holler649-0123abcd";

#[test]
fn a_herdr_session_argument_passes_only_when_it_names_the_scratch_session() {
    let passes: [&[&str]; 4] = [
        &["profile", "apply", "hlr649-apply-p", "--herdr-session", SCRATCH],
        &["profile", "apply", "hlr649-apply-p", "--herdr-session=holler649-0123abcd"],
        &["profile", "apply", "hlr649-apply-p", "--dry-run"],
        &["pane", "launch", "hlr649-ap1", "--herdr-session", SCRATCH, "--grid", "r1c1"],
    ];
    for args in passes {
        assert_eq!(check_herdr_session_args(args, SCRATCH), Ok(()), "{args:?}");
    }
    let refused: [&[&str]; 9] = [
        &["profile", "apply", "p", "--herdr-session", "default"],
        &["profile", "apply", "p", "--herdr-session", "holler649-ffffffff"],
        &["profile", "apply", "p", "--herdr-session"],
        &["profile", "apply", "p", "--herdr-session", "--dry-run"],
        &["profile", "apply", "p", "--herdr-session=default"],
        &["profile", "apply", "p", "--herdr-session="],
        &["profile", "apply", "p", "--herdr-session=holler649-ffffffff"],
        &["profile", "apply", "p", "--herdr-session", SCRATCH, "--herdr-session", "default"],
        &["profile", "apply", "p", "--herdr-session=default", "--herdr-session", SCRATCH],
    ];
    for args in refused {
        assert!(check_herdr_session_args(args, SCRATCH).is_err(), "{args:?} is refused");
    }
}

// --- the launch argv (#667 A14, A's W-5) ---------------------------------------------------

/// `launch_args` yields exactly the argument vector `Scratch::launch` built inline before
/// #667 moved it (`bda1069`, `scratch/mod.rs` lines 370-396), extra flags last.
#[test]
fn launch_args_is_the_flag_list_launch_sent_before_the_move() {
    let project = format!("{BASE}/oc/proj");
    let built = launch_args(SCRATCH, &project, "hlr649-ap1", "hlr649", "r2c1", 48123, &["--agent", "plan"]);
    let before: Vec<String> = [
        "pane",
        "launch",
        "hlr649-ap1",
        "--herdr-session",
        SCRATCH,
        "--project",
        project.as_str(),
        "--workspace",
        "hlr649",
        "--grid",
        "r2c1",
        "--model",
        "deadend/none",
        "--effort",
        "medium",
        "--ctx-soft",
        "100000",
        "--ctx-hard",
        "150000",
        "--port-policy",
        "fixed:48123",
        "--agent",
        "plan",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(built, before);
}

// --- guard 5: OpenCode's environment (W-1) ------------------------------------------------

#[test]
fn the_tmux_server_env_is_exactly_the_rigs_isolated_pairs_inside_the_directory() {
    let s = settings();
    let env = tmux_server_env(&s.opencode, &s.oc_root);
    let expected: BTreeSet<String> = [
        "PATH",
        "HOME",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
        "TERM",
        "OPENCODE_DISABLE_AUTOUPDATE",
        "OPENCODE_DISABLE_MODELS_FETCH",
        "OPENCODE_DISABLE_CLAUDE_CODE",
        "OPENCODE_DISABLE_LSP_DOWNLOAD",
        "OPENCODE_DISABLE_SHARE",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(names(&env), expected, "nothing else, no credential");
    for name in ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"] {
        assert!(value(&env, name).starts_with(&format!("{BASE}/oc/")), "{name} is scratch");
    }
    assert_eq!(value(&env, "XDG_CONFIG_HOME"), format!("{BASE}/oc/config"), "the dead-end config's home");
}
