//! A minimal, hand-rolled fake OpenCode HTTP server for
//! `http_attach_driver_test.rs` (issue #194). Hand-rolled rather than
//! pulling in `axum`/`hyper` as a new dev-dependency — this workspace's own
//! convention (`holler-hub::serve` hand-rolls its WebSocket upgrade with
//! `httparse`/`http`, per issue #155 §7 "declare only what's consumed") and
//! this fake server's needs (a handful of fixed routes, one long-lived SSE
//! connection) are simple enough not to need a real HTTP framework either.
//!
//! Routing and (dis)connection are driven live by the test via [`FakeState`]:
//! `push_event` broadcasts one SSE frame to every currently-connected
//! `/event` listener; `force_disconnect` closes every such connection right
//! now (driving `HttpAttachDriver`'s reconnect path). Every other route
//! records what it received (for `attach_404_fails_closed_no_session_new_call`
//! and friends) and/or serves from mutable state (`exists_v1`/`exists_v2`,
//! `questions`, `permissions`, `child_sessions`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[derive(Debug, Default, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

pub struct FakeState {
    pub exists_v1: bool,
    pub exists_v2: bool,
    pub questions: Vec<Value>,
    pub permissions: Vec<Value>,
    pub child_sessions: Vec<Value>,
    pub requests: Vec<RecordedRequest>,
    pub session_new_called: bool,
    event_tx: broadcast::Sender<String>,
    /// Bumped by `force_disconnect` — every live `/event` handler compares
    /// its own captured generation against the current one each loop tick
    /// and closes the connection the moment they differ.
    generation: Arc<AtomicU64>,
}

impl FakeState {
    fn new() -> Self {
        let (event_tx, _rx) = broadcast::channel(256);
        Self {
            exists_v1: true,
            exists_v2: false,
            questions: Vec::new(),
            permissions: Vec::new(),
            child_sessions: Vec::new(),
            requests: Vec::new(),
            session_new_called: false,
            event_tx,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub struct FakeServer {
    pub addr: std::net::SocketAddr,
    pub state: Arc<Mutex<FakeState>>,
    _task: tokio::task::JoinHandle<()>,
}

impl FakeServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake server must bind a loopback port");
        let addr = listener.local_addr().expect("bound listener has a local addr");
        let state = Arc::new(Mutex::new(FakeState::new()));
        let accept_state = state.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((socket, _peer)) = listener.accept().await else { return };
                let conn_state = accept_state.clone();
                tokio::spawn(async move {
                    let _ = handle_connection(socket, conn_state).await;
                });
            }
        });
        FakeServer { addr, state, _task: task }
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Broadcasts one already-JSON-encoded OpenCode event to every currently
    /// connected `/event` listener.
    pub fn push_event(&self, event: Value) {
        let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = state.event_tx.send(event.to_string());
    }

    /// Closes every currently-open `/event` connection right now.
    pub fn force_disconnect(&self) {
        let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .requests
            .clone()
    }

    pub fn set_questions(&self, questions: Vec<Value>) {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).questions = questions;
    }

    pub fn set_permissions(&self, permissions: Vec<Value>) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .permissions = permissions;
    }

    pub fn set_child_sessions(&self, sessions: Vec<Value>) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .child_sessions = sessions;
    }
}

/// One assistant text-delta SSE event, OpenCode's real shape (see
/// `http_attach_driver.rs`'s own doc comment for the pinned fields).
pub fn message_updated_assistant(message_id: &str, session_id: &str) -> Value {
    json!({
        "type": "message.updated",
        "properties": {"sessionID": session_id, "info": {"id": message_id, "role": "assistant"}}
    })
}

pub fn part_updated_text(message_id: &str, session_id: &str, text: &str) -> Value {
    json!({
        "type": "message.part.updated",
        "properties": {
            "sessionID": session_id,
            "part": {"type": "text", "text": text, "messageID": message_id}
        }
    })
}

pub fn session_idle(session_id: &str) -> Value {
    json!({"type": "session.idle", "properties": {"sessionID": session_id}})
}

pub fn session_error(session_id: &str) -> Value {
    json!({"type": "session.error", "properties": {"sessionID": session_id}})
}

pub fn question_request(id: &str, session_id: &str, question: &str, options: &[&str]) -> Value {
    json!({
        "id": id,
        "sessionID": session_id,
        "questions": [{
            "question": question,
            "options": options.iter().map(|o| json!({"label": o})).collect::<Vec<_>>(),
        }]
    })
}

pub fn multi_question_request(id: &str, session_id: &str, questions: &[(&str, &[&str])]) -> Value {
    json!({
        "id": id,
        "sessionID": session_id,
        "questions": questions.iter().map(|(q, opts)| json!({
            "question": q,
            "options": opts.iter().map(|o| json!({"label": o})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()
    })
}

pub fn permission_request(id: &str, session_id: &str, permission: &str) -> Value {
    json!({"id": id, "sessionID": session_id, "permission": permission})
}

async fn handle_connection(
    mut socket: TcpStream,
    state: Arc<Mutex<FakeState>>,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = socket.split();
    let mut reader = BufReader::new(read_half);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await? == 0 {
        return Ok(());
    }
    let mut parts = request_line.trim_end().splitn(3, ' ');
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line).await? == 0 {
            break;
        }
        let trimmed = header_line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed
            .split_once(':')
            .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        {
            content_length = value.1.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }
    let body = String::from_utf8_lossy(&body).to_string();

    if path == "/event" {
        return serve_sse(&mut write_half, &state).await;
    }

    let response = route(&method, &path, &body, &state);
    write_half.write_all(response.as_bytes()).await?;
    write_half.flush().await?;
    Ok(())
}

async fn serve_sse(
    write_half: &mut tokio::net::tcp::WriteHalf<'_>,
    state: &Arc<Mutex<FakeState>>,
) -> std::io::Result<()> {
    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
    write_half.write_all(header.as_bytes()).await?;
    write_half.flush().await?;

    let (mut rx, my_generation, generation) = {
        let state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            state.event_tx.subscribe(),
            state.generation.load(Ordering::SeqCst),
            state.generation.clone(),
        )
    };

    loop {
        if generation.load(Ordering::SeqCst) != my_generation {
            return Ok(());
        }
        let recv = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
        match recv {
            Ok(Ok(frame)) => {
                let payload = format!("data: {frame}\n\n");
                if write_half.write_all(payload.as_bytes()).await.is_err() {
                    return Ok(());
                }
                if write_half.flush().await.is_err() {
                    return Ok(());
                }
            }
            Ok(Err(broadcast::error::RecvError::Closed)) => return Ok(()),
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Err(_timeout) => continue,
        }
    }
}

fn route(method: &str, path: &str, body: &str, state: &Arc<Mutex<FakeState>>) -> String {
    let mut state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    state.requests.push(RecordedRequest {
        method: method.to_string(),
        path: path.to_string(),
        body: body.to_string(),
    });

    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    match (method, segments.as_slice()) {
        ("GET", ["session", id]) if !id.is_empty() => {
            if state.exists_v1 {
                ok_json("{}")
            } else {
                not_found()
            }
        }
        ("GET", ["api", "session", id]) if !id.is_empty() => {
            if state.exists_v2 {
                ok_json("{}")
            } else {
                not_found()
            }
        }
        ("GET", ["session"]) => {
            ok_json(&Value::Array(state.child_sessions.clone()).to_string())
        }
        ("POST", ["session"]) => {
            state.session_new_called = true;
            ok_json("{}")
        }
        // Only the routes independently confirmed live against a real
        // `opencode serve` v1.18.20 are wired up here (see
        // `http_attach_driver.rs`'s module doc): `prompt_async` is always
        // bare; `interrupt` is always `/api`-prefixed. The other prefix of
        // each falls through to `not_found` here, standing in for OpenCode's
        // own real behavior of silently serving its SPA shell instead (a
        // driver bug calling the wrong form is still caught either way,
        // since neither shape is the `204` this fake returns for the real
        // route).
        ("POST", ["session", _id, "prompt_async"]) => no_content(),
        ("POST", ["api", "session", _id, "interrupt"]) => no_content(),
        ("GET", ["question"]) => ok_json(&Value::Array(state.questions.clone()).to_string()),
        ("GET", ["permission"]) => ok_json(&Value::Array(state.permissions.clone()).to_string()),
        ("POST", ["question", _id, "reply"]) => ok_json("{}"),
        ("POST", ["permission", _id, "reply"]) => ok_json("{}"),
        _ => not_found(),
    }
}

fn ok_json(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn no_content() -> String {
    "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_string()
}

fn not_found() -> String {
    let body = "{}";
    format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
