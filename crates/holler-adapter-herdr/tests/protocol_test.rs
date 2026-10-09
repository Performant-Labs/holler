#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640
//! Herdr's wire with no I/O: the request lines, the reply decoder, the parsers and the
//! version gate (#640 part 1, AC 13-20). Fixtures are hand-written from the bundled
//! schema (protocol 22) and the spike's verbatim `pong`.

mod common;

use common::{id, pane, spike_2x4, split};
use holler_adapter_herdr::layout::{Direction, LayoutNode};
use holler_adapter_herdr::protocol::{
    check_supported, decode_reply, expect_ok, parse_layout_export, parse_pane_info, parse_pong,
    parse_read, parse_snapshot, parse_workspace_created, PaneRef, Request, ServerVersion,
    WorkspaceRef, ALLOWED_METHODS, SUPPORTED_PROTOCOLS, SUPPORTED_VERSIONS,
};
use holler_pane::{Key, PaneError};
use serde_json::{json, Value};

const SECRET: &str = "hunter2-secret-text";

/// One request per variant. The `match` below has no wildcard arm, so a new `Request`
/// variant does not compile until this test covers it (A, W-3).
fn all_requests() -> Vec<Request> {
    vec![
        Request::Ping,
        Request::SessionSnapshot,
        Request::LayoutExport {
            tab_id: "w1:t1".into(),
        },
        Request::WorkspaceCreate {
            label: "scratch".into(),
        },
        Request::Split {
            target: id("w1:p1"),
            direction: Direction::Down,
            ratio: 0.25,
        },
        Request::SendText {
            pane: id("w1:p1"),
            text: "echo hi".into(),
        },
        Request::SendKeys {
            pane: id("w1:p1"),
            keys: vec![Key::new("enter")],
        },
        Request::Read {
            pane: id("w1:p1"),
            lines: 40,
        },
        Request::Close { pane: id("w1:p1") },
    ]
}

fn expected_method(request: &Request) -> &'static str {
    match request {
        Request::Ping => "ping",
        Request::SessionSnapshot => "session.snapshot",
        Request::LayoutExport { .. } => "layout.export",
        Request::WorkspaceCreate { .. } => "workspace.create",
        Request::Split { .. } => "pane.split",
        Request::SendText { .. } => "pane.send_text",
        Request::SendKeys { .. } => "pane.send_keys",
        Request::Read { .. } => "pane.read",
        Request::Close { .. } => "pane.close",
    }
}

fn line_of(request: &Request) -> Value {
    let line = request.to_line();
    assert!(line.ends_with('\n'), "{line:?}");
    assert!(!line[..line.len() - 1].contains('\n'), "{line:?}");
    serde_json::from_str(&line).expect("one JSON object")
}

fn params_of(request: &Request) -> Value {
    let value = line_of(request);
    assert_eq!(value["id"], format!("holler:{}", request.method()));
    assert_eq!(value["method"], request.method());
    value["params"].clone()
}

#[test]
fn ac14_every_request_is_one_json_line_with_its_id_and_method() {
    for request in all_requests() {
        let value = line_of(&request);

        assert_eq!(value["method"], expected_method(&request));
        assert_eq!(value["id"], format!("holler:{}", expected_method(&request)));
        assert_eq!(
            request.id(),
            format!("holler:{}", expected_method(&request))
        );
        assert!(value["params"].is_object(), "{value}");
    }
}

#[test]
fn ac14_params_match_the_schema() {
    let by_variant = |index: usize| params_of(&all_requests()[index]);

    assert_eq!(by_variant(0), json!({}));
    assert_eq!(by_variant(1), json!({}));
    assert_eq!(by_variant(2), json!({"tab_id": "w1:t1"}));
    assert_eq!(by_variant(3), json!({"label": "scratch", "focus": false}));
    assert_eq!(
        by_variant(4),
        json!({"target_pane_id": "w1:p1", "direction": "down", "ratio": 0.25, "focus": false})
    );
    assert_eq!(
        by_variant(5),
        json!({"pane_id": "w1:p1", "text": "echo hi"})
    );
    assert_eq!(
        by_variant(6),
        json!({"pane_id": "w1:p1", "keys": ["enter"]})
    );
    assert_eq!(
        by_variant(7),
        json!({"pane_id": "w1:p1", "source": "recent", "lines": 40, "format": "text"})
    );
    assert_eq!(by_variant(8), json!({"pane_id": "w1:p1"}));
}

#[test]
fn ac14_a_right_split_says_right() {
    let request = Request::Split {
        target: id("w1:p3"),
        direction: Direction::Right,
        ratio: 0.5,
    };

    assert_eq!(params_of(&request)["direction"], "right");
}

#[test]
fn ac14_text_with_newlines_and_quotes_stays_on_one_line() {
    let request = Request::SendText {
        pane: id("w1:p1"),
        text: "a\nb \"c\"".into(),
    };

    assert_eq!(params_of(&request)["text"], "a\nb \"c\"");
}

#[test]
fn ac14_keys_go_out_verbatim_with_no_case_folding() {
    let keys = ["enter", "ctrl+c", "Enter", "C-c"].map(Key::new).to_vec();
    let request = Request::SendKeys {
        pane: id("w1:p1"),
        keys,
    };

    assert_eq!(
        params_of(&request)["keys"],
        json!(["enter", "ctrl+c", "Enter", "C-c"])
    );
}

#[test]
fn ac15_every_method_is_on_the_allow_list_and_nothing_else_is() {
    let mut used: Vec<&str> = all_requests().iter().map(Request::method).collect();
    for request in all_requests() {
        assert_eq!(request.method(), expected_method(&request));
        assert!(
            ALLOWED_METHODS.contains(&request.method()),
            "{}",
            request.method()
        );
    }
    used.sort_unstable();
    let mut listed = ALLOWED_METHODS.to_vec();
    listed.sort_unstable();
    assert_eq!(
        used, listed,
        "an allowed method no request uses guards nothing"
    );
}

#[test]
fn ac15_the_allow_list_has_no_destructive_or_foreign_method() {
    let forbidden = [
        "server.stop",
        "server.live_handoff",
        "layout.apply",
        "pane.move",
        "pane.swap",
    ];

    for method in ALLOWED_METHODS {
        assert!(!forbidden.contains(&method), "{method}");
        assert!(!method.starts_with("plugin."), "{method}");
        assert!(!method.starts_with("integration."), "{method}");
    }
}

#[test]
fn ac15_every_supported_protocol_is_named_in_the_supported_versions_text() {
    for protocol in SUPPORTED_PROTOCOLS {
        assert!(SUPPORTED_VERSIONS.contains(&format!("protocol {protocol}")));
    }
}

#[test]
fn ac13_the_supported_versions_text_is_the_test_kits() {
    assert_eq!(
        SUPPORTED_VERSIONS,
        holler_pane_testkit::herdr::SUPPORTED_VERSIONS
    );
}

fn reply(request: &Request, result: Value) -> String {
    json!({"id": request.id(), "result": result}).to_string()
}

fn error_reply(request: &Request, code: &str, message: &str) -> String {
    json!({"id": request.id(), "error": {"code": code, "message": message}}).to_string()
}

fn unavailable_what(error: PaneError) -> String {
    let PaneError::Unavailable { what } = error else {
        panic!("expected unavailable, got {error:?}");
    };
    assert!(!what.contains('\n'), "{what:?}");
    what
}

#[test]
fn ac16_a_result_reply_gives_the_result_object() {
    let close = Request::Close { pane: id("w1:p5") };

    let result = decode_reply(&close, &reply(&close, json!({"type": "ok"})));

    assert_eq!(result, Ok(json!({"type": "ok"})));
}

#[test]
fn ac16_pane_not_found_names_the_pane_of_the_request() {
    let requests = [
        Request::Close { pane: id("w1:p5") },
        Request::Read {
            pane: id("w1:p5"),
            lines: 3,
        },
        Request::SendText {
            pane: id("w1:p5"),
            text: "x".into(),
        },
        Request::SendKeys {
            pane: id("w1:p5"),
            keys: vec![Key::new("enter")],
        },
        Request::Split {
            target: id("w1:p5"),
            direction: Direction::Right,
            ratio: 0.5,
        },
    ];

    for request in requests {
        let line = error_reply(&request, "pane_not_found", "pane w1:p5 not found");

        assert_eq!(
            decode_reply(&request, &line),
            Err(PaneError::PaneNotFound {
                what: "w1:p5".into()
            }),
            "{}",
            request.method()
        );
    }
}

#[test]
fn ac16_pane_not_found_for_a_request_without_a_pane_is_unavailable() {
    for request in [
        Request::Ping,
        Request::SessionSnapshot,
        Request::WorkspaceCreate { label: "x".into() },
    ] {
        let line = error_reply(&request, "pane_not_found", "pane w1:p5 not found");

        let what = unavailable_what(decode_reply(&request, &line).unwrap_err());

        assert!(what.contains(request.method()), "{what}");
    }
}

#[test]
fn ac16_any_other_herdr_code_is_unavailable_naming_the_method_and_the_code() {
    let request = Request::Close { pane: id("w1:p5") };

    for code in ["invalid_request", "something_new"] {
        let line = error_reply(
            &request,
            code,
            "invalid request: unknown variant `pane.nope`",
        );

        let what = unavailable_what(decode_reply(&request, &line).unwrap_err());

        assert!(what.contains("pane.close") && what.contains(code), "{what}");
    }
}

#[test]
fn ac16_a_garbled_or_mismatched_reply_is_unavailable() {
    let request = Request::Close { pane: id("w1:p5") };
    let other = Request::Ping;
    let lines = [
        "this is not json".to_owned(),
        String::new(),
        reply(&other, json!({"type": "ok"})),
        json!({"id": request.id()}).to_string(),
        json!({"id": request.id(), "result": "ok"}).to_string(),
        "[1,2]".to_owned(),
    ];

    for line in lines {
        let error = decode_reply(&request, &line).expect_err(&line);

        unavailable_what(error);
    }
}

#[test]
fn ac16_no_message_has_a_newline_and_none_echoes_typed_text() {
    let request = Request::SendText {
        pane: id("w1:p5"),
        text: SECRET.into(),
    };
    let lines = [
        error_reply(&request, "invalid_request", &format!("bad text\n{SECRET}")),
        error_reply(&request, "odd\ncode", &format!("echo {SECRET}")),
        format!("{{\"id\":\"holler:pane.send_text\",\"result\":{SECRET}"),
        reply(&Request::Ping, json!({"echo": SECRET})),
        SECRET.to_owned(),
    ];

    for line in lines {
        let error = decode_reply(&request, &line).expect_err(&line);
        let what = unavailable_what(error.clone());

        assert!(!what.contains(SECRET), "{what}");
        assert!(!format!("{error:?}").contains(SECRET), "{error:?}");
        assert!(!error.to_string().contains(SECRET), "{error}");
    }
}

#[test]
fn ac16_a_request_never_prints_its_typed_text() {
    let request = Request::SendText {
        pane: id("w1:p5"),
        text: SECRET.into(),
    };

    assert!(!format!("{request:?}").contains(SECRET));
    assert!(!format!("{request:#?}").contains(SECRET));
}

fn spike_pong() -> Value {
    json!({
        "type": "pong",
        "version": "0.9.1-preview.2026-09-21-0ff0f27e2226",
        "protocol": 22,
        "capabilities": {
            "live_handoff": true,
            "detached_server_daemon": false,
            "endpoint_protocol_generation": 1,
            "surface_interest": true,
            "health_check": true
        }
    })
}

#[test]
fn ac17_the_spikes_pong_is_protocol_22_and_supported() {
    let server = parse_pong(&spike_pong()).unwrap();

    assert_eq!(
        server,
        ServerVersion {
            version: "0.9.1-preview.2026-09-21-0ff0f27e2226".into(),
            protocol: Some(22)
        }
    );
    assert_eq!(check_supported(&server), Ok(()));
}

#[test]
fn ac17_another_protocol_is_refused_naming_the_version_the_protocol_and_the_supported() {
    let pong = json!({"type": "pong", "version": "99.0.0-fake", "protocol": 99});
    let server = parse_pong(&pong).unwrap();

    let Err(PaneError::HerdrVersionUnsupported { message }) = check_supported(&server) else {
        panic!("protocol 99 must be refused");
    };

    assert!(message.contains("99.0.0-fake"), "{message}");
    assert!(message.contains("99"), "{message}");
    assert!(message.contains(SUPPORTED_VERSIONS), "{message}");
    assert!(!message.contains('\n'), "{message}");
}

#[test]
fn ac17_a_pong_without_a_protocol_is_unknown_and_refused() {
    let pong = json!({"type": "pong", "version": "0.9.1-preview"});
    let server = parse_pong(&pong).unwrap();
    assert_eq!(server.protocol, None);

    let Err(PaneError::HerdrVersionUnsupported { message }) = check_supported(&server) else {
        panic!("an unknown protocol must be refused");
    };

    assert!(message.contains("0.9.1-preview"), "{message}");
    assert!(message.to_lowercase().contains("unknown"), "{message}");
    assert!(message.contains(SUPPORTED_VERSIONS), "{message}");
    assert!(!message.contains('\n'), "{message}");
}

#[test]
fn ac17_a_result_that_is_not_a_pong_is_unavailable() {
    let error = parse_pong(&json!({"type": "ok"})).unwrap_err();

    unavailable_what(error);
}

/// A `session_snapshot` result with the schema's required fields. Workspace `w1`
/// (`scratch`) has tabs numbered 2 and 1, in that order.
fn snapshot_result(second_label: &str) -> Value {
    let workspace = |workspace_id: &str, number: u32, label: &str, active_tab: &str| {
        json!({
            "workspace_id": workspace_id, "number": number, "label": label, "focused": false,
            "pane_count": 1, "tab_count": 1, "active_tab_id": active_tab,
            "agent_status": "idle", "unknown_extra": true
        })
    };
    let tab = |tab_id: &str, workspace_id: &str, number: u32| {
        json!({
            "tab_id": tab_id, "workspace_id": workspace_id, "number": number, "label": "t",
            "focused": false, "pane_count": 1, "agent_status": "idle"
        })
    };
    let pane = |pane_id: &str, workspace_id: &str, tab_id: &str| {
        json!({
            "pane_id": pane_id, "workspace_id": workspace_id, "tab_id": tab_id,
            "focused": false, "cwd": "/tmp/x", "agent_status": "idle", "revision": 3
        })
    };
    json!({"type": "session_snapshot", "snapshot": {
        "version": "0.9.1-preview", "protocol": 22,
        "workspaces": [
            workspace("w1", 1, "scratch", "w1:t2"),
            workspace("w2", 2, second_label, "w2:t1")
        ],
        "tabs": [tab("w1:t2", "w1", 2), tab("w1:t1", "w1", 1), tab("w2:t1", "w2", 1)],
        "panes": [
            pane("w1:p1", "w1", "w1:t1"),
            pane("w1:p2", "w1", "w1:t1"),
            pane("w2:p1", "w2", "w2:t1")
        ],
        "layouts": [],
        "agents": []
    }})
}

#[test]
fn ac18_a_snapshot_gives_workspaces_by_label_with_the_lowest_numbered_tab() {
    let state = parse_snapshot(&snapshot_result("other")).unwrap();

    assert_eq!(
        state.workspace("scratch").unwrap(),
        Some(&WorkspaceRef {
            workspace_id: "w1".into(),
            label: "scratch".into(),
            grid_tab: Some("w1:t1".into())
        })
    );
    assert_eq!(
        state.workspace("other").unwrap().unwrap().workspace_id,
        "w2"
    );
    assert_eq!(state.workspace("absent"), Ok(None));
}

#[test]
fn ac18_a_snapshot_lists_each_pane_with_its_workspace_and_tab() {
    let state = parse_snapshot(&snapshot_result("other")).unwrap();
    let pane_ref = |pane: &str, workspace: &str, tab: &str| PaneRef {
        pane_id: id(pane),
        workspace_id: workspace.into(),
        tab_id: tab.into(),
    };

    assert_eq!(
        state.panes,
        vec![
            pane_ref("w1:p1", "w1", "w1:t1"),
            pane_ref("w1:p2", "w1", "w1:t1"),
            pane_ref("w2:p1", "w2", "w2:t1"),
        ]
    );
}

#[test]
fn ac18_two_workspaces_with_one_label_are_unavailable_naming_both() {
    let state = parse_snapshot(&snapshot_result("scratch")).unwrap();

    let what = unavailable_what(state.workspace("scratch").unwrap_err());

    assert!(
        what.contains("scratch") && what.contains("w1") && what.contains("w2"),
        "{what}"
    );
    assert_eq!(
        state.workspace("other"),
        Ok(None),
        "only the duplicated label is refused"
    );
}

#[test]
fn ac18_a_garbled_snapshot_is_unavailable() {
    let mut not_a_list = snapshot_result("other");
    not_a_list["snapshot"]["workspaces"] = json!("w1");
    let mut no_panes = snapshot_result("other");
    no_panes["snapshot"]
        .as_object_mut()
        .unwrap()
        .remove("panes");

    for result in [
        json!({"type": "ok"}),
        json!({"type": "session_snapshot"}),
        not_a_list,
        no_panes,
    ] {
        unavailable_what(parse_snapshot(&result).unwrap_err());
    }
}

fn pane_json(pane_id: &str) -> Value {
    json!({"type": "pane", "pane_id": pane_id, "label": "r1c1", "cwd": "/tmp/x", "command": null})
}

/// A `layout_export` result around `root`, with every field the schema lists.
fn layout_of(root: Value) -> Value {
    json!({"type": "layout_export", "layout": {
        "workspace_id": "w1", "tab_id": "w1:t1", "zoomed": false,
        "focused_pane_id": "w1:p1", "root": root
    }})
}

/// The spike's 2x4 tree as `layout.export` shows it.
fn export_2x4() -> Value {
    let row = |ids: [&str; 4]| {
        let split_json = |ratio: f64, first: Value, second: Value| json!({"type": "split", "direction": "right", "ratio": ratio, "first": first, "second": second});
        let tail = split_json(0.5, pane_json(ids[2]), pane_json(ids[3]));
        let mid = split_json(0.3333, pane_json(ids[1]), tail);
        split_json(0.25, pane_json(ids[0]), mid)
    };
    let root = json!({
        "type": "split", "direction": "down", "ratio": 0.5,
        "first": row(["w1:p1", "w1:p3", "w1:p4", "w1:p5"]),
        "second": row(["w1:p2", "w1:p6", "w1:p7", "w1:p8"])
    });
    layout_of(root)
}

#[test]
fn ac19_the_2x4_export_is_the_2x4_tree() {
    let tree = parse_layout_export(&export_2x4()).unwrap();

    assert_eq!(tree, spike_2x4());
}

#[test]
fn ac19_a_lone_pane_export_is_a_pane() {
    let result = layout_of(pane_json("w1:p1"));

    assert_eq!(parse_layout_export(&result).unwrap(), pane("w1:p1"));
}

#[test]
fn ac19_a_split_keeps_its_direction_and_ratio() {
    let root = json!({"type": "split", "direction": "right", "ratio": 0.7,
        "first": pane_json("A"), "second": pane_json("B")});
    let result = layout_of(root);

    let expected: LayoutNode = split(Direction::Right, 0.7, pane("A"), pane("B"));

    assert_eq!(parse_layout_export(&result).unwrap(), expected);
}

#[test]
fn ac19_a_pane_without_an_id_an_unknown_direction_or_another_type_is_unavailable() {
    let with_root = layout_of;
    let split_of = |direction: &str| {
        json!({"type": "split", "direction": direction, "ratio": 0.5,
            "first": pane_json("A"), "second": pane_json("B")})
    };
    let results = [
        with_root(json!({"type": "pane", "pane_id": null})),
        with_root(json!({"type": "pane"})),
        with_root(split_of("left")),
        {
            let mut other_type = layout_of(pane_json("A"));
            other_type["type"] = json!("session_snapshot");
            other_type
        },
        json!({"type": "layout_export"}),
    ];

    for result in results {
        unavailable_what(parse_layout_export(&result).unwrap_err());
    }
}

fn workspace_created() -> Value {
    json!({"type": "workspace_created",
        "workspace": {"workspace_id": "w3", "number": 3, "label": "fresh", "focused": false,
            "pane_count": 1, "tab_count": 1, "active_tab_id": "w3:t1", "agent_status": "idle"},
        "tab": {"tab_id": "w3:t1", "workspace_id": "w3", "number": 1, "label": "t",
            "focused": false, "pane_count": 1, "agent_status": "idle"},
        "root_pane": {"pane_id": "w3:p1", "workspace_id": "w3", "tab_id": "w3:t1"}})
}

#[test]
fn ac20_a_created_workspace_gives_its_ref_and_its_root_pane() {
    let (workspace, root) = parse_workspace_created(&workspace_created()).unwrap();

    assert_eq!(
        workspace,
        WorkspaceRef {
            workspace_id: "w3".into(),
            label: "fresh".into(),
            grid_tab: Some("w3:t1".into())
        }
    );
    assert_eq!(root, id("w3:p1"));
}

#[test]
fn ac20_pane_info_gives_the_pane_id() {
    let result = json!({"type": "pane_info", "pane": {"pane_id": "w1:pA", "workspace_id": "w1", "tab_id": "w1:t1"}});

    assert_eq!(parse_pane_info(&result), Ok(id("w1:pA")));
}

fn read_result(text: &str) -> Value {
    json!({"type": "pane_read", "read": {
        "pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "source": "recent",
        "format": "text", "text": text, "revision": 7, "truncated": false}})
}

#[test]
fn ac20_read_gives_the_last_lines_joined_without_a_trailing_newline() {
    let result = read_result("a\nb\nc\nd\n");

    assert_eq!(parse_read(&result, 2).unwrap(), "c\nd");
    assert_eq!(parse_read(&result, 10).unwrap(), "a\nb\nc\nd");
    assert_eq!(parse_read(&result, 0).unwrap(), "");
    assert_eq!(parse_read(&read_result("a\n\nb"), 2).unwrap(), "\nb");
}

#[test]
fn ac20_expect_ok_accepts_only_ok() {
    assert_eq!(expect_ok(&json!({"type": "ok"})), Ok(()));
    unavailable_what(expect_ok(&json!({"type": "pane_info"})).unwrap_err());
    unavailable_what(expect_ok(&json!({})).unwrap_err());
}

#[test]
fn a_result_of_the_wrong_type_is_unavailable_for_every_parser() {
    let wrong = json!({"type": "ok"});

    unavailable_what(parse_workspace_created(&wrong).unwrap_err());
    unavailable_what(parse_pane_info(&wrong).unwrap_err());
    unavailable_what(parse_read(&wrong, 3).unwrap_err());
}
