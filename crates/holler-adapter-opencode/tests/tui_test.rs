#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The pure tmux side of the OpenCode adapter (#642b, AC 9, 10, 11a-11c, 11f, 27): the
//! builders' exact argument vectors, the escapes, and the parsers of a TUI's title, its
//! start command and the one query. Nothing here starts a process, so it runs in CI on
//! Linux and macOS.

use std::ffi::OsStr;
use std::path::PathBuf;

use holler_adapter_opencode::tui::{
    attach_port, escape_arg, escape_dir, exact_target, parse_query, parse_title, query_args,
    remain_on_exit_args, respawn_args, tmux_command, tui_argv, Query, TitleShows, QUERY_FORMAT,
};
use holler_adapter_opencode::{ProcessEnv, TmuxConfig, TmuxSocket};
use holler_pane::PaneName;

/// The five fields of Decision 23, in order, joined by TAB.
const FORMAT: &str =
    "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}";

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&p| p.to_owned()).collect()
}

fn isolated(pairs: &[(&str, &str)]) -> ProcessEnv {
    ProcessEnv::Isolated(
        pairs
            .iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
    )
}

/// The `-t` values of an argument vector.
fn targets(args: &[String]) -> Vec<&str> {
    args.windows(2)
        .filter(|pair| pair[0] == "-t")
        .map(|pair| pair[1].as_str())
        .collect()
}

// ---- AC 9: parse_title ----

#[test]
fn ac9_parse_title_reads_a_whole_id_or_home_and_never_guesses() {
    assert_eq!(
        parse_title("OC | ses_0123456789abcdefABCDEFghij"),
        TitleShows::Session("ses_0123456789abcdefABCDEFghij".to_owned())
    );
    assert_eq!(parse_title("OpenCode"), TitleShows::Home);
    // A cut id, a default or a person's title, tmux's host name and nothing at all.
    for title in [
        "OC | ses_ede8…",
        "OC | New session - 2026-10-09T00:00:00Z",
        "OC | my notes",
        "somehost",
        "",
    ] {
        assert_eq!(
            parse_title(title),
            TitleShows::Unrecognised,
            "the title {title:?} names no whole session id"
        );
    }
}

// ---- AC 10: attach_port ----

#[test]
fn ac10_attach_port_reads_a_loopback_attach_line_and_nothing_else() {
    for line in [
        "env -u OPENCODE_DISABLE_TERMINAL_TITLE /usr/local/bin/opencode attach \
         http://127.0.0.1:48123 --dir /p --session ses_1",
        "env -i A=b /usr/local/bin/opencode attach http://127.0.0.1:48123 --dir /p --session ses_1",
        // tmux re-quotes an element that holds a space when it prints #{pane_start_command}.
        "env -i \"A=b c\" \"/opt/my oc/opencode\" attach http://127.0.0.1:48123 --dir \"/my p\" \
         --session ses_1",
        // No flags after the URL, or flags in another order: the parser does not need them.
        "/bin/oc attach http://127.0.0.1:48123",
        "/bin/oc attach http://127.0.0.1:48123 --session ses_1 --dir /p",
    ] {
        assert_eq!(attach_port(line), Some(48123), "{line:?}");
    }
    // Another program, OpenCode's server, and an attach to a host that is not loopback.
    for line in [
        "bash",
        "sleep 3600",
        "opencode serve --port 48123",
        "opencode attach http://192.0.2.1:48123",
    ] {
        assert_eq!(attach_port(line), None, "{line:?}");
    }
}

// ---- AC 11a: exact targets ----

#[test]
fn ac11a_every_builder_targets_the_exact_session() {
    let s = name("demo-c1r1");
    assert_eq!(exact_target(&s), "=demo-c1r1:");
    let argv = strings(&["env", "-u", "X", "/bin/oc", "attach"]);
    let mut respawn = strings(&["respawn-pane", "-k", "-t", "=demo-c1r1:", "-c", "/p", "--"]);
    respawn.extend(argv.iter().cloned());
    assert_eq!(respawn_args(&s, "/p", &argv), respawn);
    assert_eq!(
        remain_on_exit_args(&s),
        strings(&[
            "set-option",
            "-p",
            "-t",
            "=demo-c1r1:",
            "remain-on-exit",
            "on"
        ])
    );
    let query = query_args(&s);
    assert_eq!(
        query.get(..4),
        Some(&strings(&["display-message", "-p", "-t", "=demo-c1r1:"])[..])
    );
    assert!(
        query
            .get(4)
            .is_some_and(|f| f.starts_with("#{session_name}")),
        "the query's format starts with #{{session_name}}: {query:?}"
    );
}

#[test]
fn ac11a_no_builder_passes_a_bare_session_name_as_a_target() {
    // `demo` is a prefix of `demo-c1r1`: tmux would match a bare `-t demo` to that session.
    let s = name("demo");
    let argv = tui_argv("/bin/oc", &ProcessEnv::Inherit, 48123, "/p", "ses_1");
    for (builder, args) in [
        ("respawn_args", respawn_args(&s, "/p", &argv)),
        ("remain_on_exit_args", remain_on_exit_args(&s)),
        ("query_args", query_args(&s)),
    ] {
        assert_eq!(targets(&args), ["=demo:"], "{builder}: {args:?}");
        assert!(!args.iter().any(|a| a == "demo"), "{builder}: {args:?}");
    }
}

// ---- AC 11b: escaping ----

#[test]
fn ac11b_escape_arg_guards_only_a_final_semicolon() {
    // Raw values: `x;` becomes `x\;`, `y\;` becomes `y\\;`, `;` becomes `\;`.
    assert_eq!(escape_arg("x;"), r"x\;");
    assert_eq!(escape_arg(r"y\;"), r"y\\;");
    assert_eq!(escape_arg(";"), r"\;");
    for unchanged in ["a b", "#{x}", "{", "}", "~", "-t", ""] {
        assert_eq!(
            escape_arg(unchanged),
            unchanged,
            "{unchanged:?} is left alone"
        );
    }
}

#[test]
fn ac11b_escape_dir_doubles_every_hash_then_escapes() {
    assert_eq!(escape_dir("/p#S;"), r"/p##S\;");
}

#[test]
fn ac11b_respawn_escapes_the_directory_and_every_tui_element() {
    let argv = tui_argv("/bin/oc", &isolated(&[("K", "v;")]), 48123, "/p#S", "ses_1");
    let args = respawn_args(&name("demo-c1r1"), "/p#S", &argv);
    let c = args.iter().position(|a| a == "-c").expect("a -c flag");
    assert_eq!(
        args[c + 1],
        "/p##S",
        "-c is format-expanded, so # is doubled"
    );
    assert!(
        args.iter().any(|a| a == r"K=v\;"),
        "an env value ending in ; is escaped: {args:?}"
    );
    let dir = args
        .iter()
        .position(|a| a == "--dir")
        .expect("a --dir flag");
    assert_eq!(
        args[dir + 1],
        "/p#S",
        "an element after -- is not format-expanded"
    );
}

// ---- AC 11c: one tmux server, never $TMUX's ----

#[test]
fn ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane() {
    let call = strings(&["display-message", "-p"]);
    for (socket, flag) in [
        (
            TmuxSocket::Path(PathBuf::from("/x/sock")),
            vec!["-S", "/x/sock"],
        ),
        (TmuxSocket::Name("hlr".to_owned()), vec!["-L", "hlr"]),
        (TmuxSocket::Default, vec![]),
    ] {
        let tmux = TmuxConfig {
            tmux_bin: PathBuf::from("/opt/tmux"),
            socket: socket.clone(),
        };
        let command = tmux_command(&tmux, &call);
        assert_eq!(command.get_program(), OsStr::new("/opt/tmux"), "{socket:?}");
        let args: Vec<&OsStr> = command.get_args().collect();
        let mut expected: Vec<&OsStr> = flag.iter().map(OsStr::new).collect();
        expected.extend(call.iter().map(OsStr::new));
        assert_eq!(args, expected, "{socket:?}");
        let envs: Vec<(&OsStr, Option<&OsStr>)> = command.get_envs().collect();
        for var in ["TMUX", "TMUX_PANE"] {
            assert!(
                envs.contains(&(OsStr::new(var), None)),
                "{socket:?}: {var} is removed from the child: {envs:?}"
            );
        }
    }
}

// ---- AC 11f: the TUI argv ----

#[test]
fn ac11f_the_tui_argv_under_inherit_and_isolated() {
    let tail = strings(&[
        "/bin/oc",
        "attach",
        "http://127.0.0.1:48123",
        "--dir",
        "/p",
        "--session",
        "ses_1",
    ]);
    let mut inherit = strings(&["env", "-u", "OPENCODE_DISABLE_TERMINAL_TITLE"]);
    inherit.extend(tail.iter().cloned());
    let mut isolated_argv = strings(&["env", "-i", "A=b"]);
    isolated_argv.extend(tail.iter().cloned());
    for (env, expected) in [
        (ProcessEnv::Inherit, inherit),
        (isolated(&[("A", "b")]), isolated_argv),
    ] {
        let argv = tui_argv("/bin/oc", &env, 48123, "/p", "ses_1");
        assert_eq!(argv, expected, "{env:?}");
        assert_eq!(
            attach_port(&argv.join(" ")),
            Some(48123),
            "{env:?}: the port reads back from the line tmux prints"
        );
    }
}

// ---- AC 27: QUERY_FORMAT, query_args and parse_query ----

#[test]
fn ac27_the_query_is_display_message_of_the_five_fields() {
    assert_eq!(QUERY_FORMAT, FORMAT);
    assert_eq!(
        query_args(&name("demo-c1r1")),
        strings(&["display-message", "-p", "-t", "=demo-c1r1:", FORMAT])
    );
}

#[test]
fn ac27_a_live_pane_gives_its_start_command_and_its_whole_title() {
    let s = name("demo-c1r1");
    let start = "env -u OPENCODE_DISABLE_TERMINAL_TITLE /bin/oc attach http://127.0.0.1:48123 \
                 --dir /p --session ses_1";
    assert_eq!(
        parse_query(&s, &format!("demo-c1r1\t0\t\t{start}\tOC | ses_1\n")),
        Query::Live {
            start_command: start.to_owned(),
            title: "OC | ses_1".to_owned(),
        }
    );
    assert_eq!(
        parse_query(&s, "demo-c1r1\t0\t\tsleep 3600\tOC | a\tb\n"),
        Query::Live {
            start_command: "sleep 3600".to_owned(),
            title: "OC | a\tb".to_owned(),
        },
        "the title is everything after the fourth TAB"
    );
}

#[test]
fn ac27_a_dead_pane_gives_its_exit_status() {
    let s = name("demo-c1r1");
    assert_eq!(
        parse_query(&s, "demo-c1r1\t1\t7\tsh -c \"exit 7\"\tanything"),
        Query::Dead(Some(7))
    );
    assert_eq!(
        parse_query(&s, "demo-c1r1\t1\t\tsh\tOpenCode\n"),
        Query::Dead(None),
        "a status that is not a number"
    );
}

#[test]
fn ac27_a_reply_that_is_not_this_sessions_pane_is_no_pane() {
    let s = name("demo-c1r1");
    // The contrast: the same fields under this session's name are a live pane.
    assert!(
        matches!(
            parse_query(&s, "demo-c1r1\t0\t\tsleep 3600\tOC | ses_1\n"),
            Query::Live { .. }
        ),
        "this session's own reply is a live pane"
    );
    for (reply, why) in [
        (
            "\t\t\t\t\n",
            "a missing exact target expands every field to empty",
        ),
        (
            "demo\t0\t\tsleep 3600\tOC | ses_1\n",
            "another session's name",
        ),
        ("demo-c1r1\t0\t\n", "three fields"),
        (
            "demo-c1r1\tx\t\tsleep 3600\tOC | ses_1\n",
            "a pane_dead that is not 0 or 1",
        ),
        ("", "an empty reply"),
    ] {
        assert_eq!(parse_query(&s, reply), Query::NoPane, "{why}: {reply:?}");
    }
}
