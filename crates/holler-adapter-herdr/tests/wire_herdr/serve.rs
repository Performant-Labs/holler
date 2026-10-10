//! A [`WireHerdr`] served on a Unix socket in a temporary directory, as Herdr serves
//! its session: one connection at a time, one request line in, one reply line out, then
//! the connection closes (spike section 3).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tempfile::TempDir;

use super::WireHerdr;

/// How long the server waits for a client's request line before it drops the client.
const CLIENT_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// A served fake. Dropping it stops the server thread and removes the directory.
pub struct Served {
    /// Removed when this drops, after the thread has stopped.
    _dir: TempDir,
    path: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Served {
    /// The socket: `<tempdir>/h.sock`.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Serve `fake` on a new socket. The socket is bound before this returns, so a client
/// may connect at once.
pub fn serve(fake: Arc<WireHerdr>) -> Served {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("h.sock");
    let listener = UnixListener::bind(&path).expect("bind the fake's socket");
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = Arc::clone(&stop);
    let thread = thread::spawn(move || {
        for client in listener.incoming() {
            if stopping.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(client) = client {
                answer_one(&fake, client);
            }
        }
    });
    Served {
        _dir: dir,
        path,
        stop,
        thread: Some(thread),
    }
}

/// Read one line from `client`, answer it, and close the connection.
fn answer_one(fake: &WireHerdr, client: UnixStream) {
    if client.set_read_timeout(Some(CLIENT_READ_TIMEOUT)).is_err() {
        return;
    }
    let mut line = String::new();
    let read = BufReader::new(&client).read_line(&mut line);
    if matches!(read, Ok(n) if n > 0) {
        let reply = format!("{}\n", fake.answer(&line));
        let _ = (&client).write_all(reply.as_bytes());
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocked `accept`; the thread sees the flag and stops.
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
