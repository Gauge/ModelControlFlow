//! A termination signal stops the daemon the way `mcf stop` does: the server
//! it holds is let go, and the record says why it stopped (B-584, A27).
//!
//! **What was observed.** A daemon ended by `SIGTERM` — a scope being stopped,
//! a session ending, somebody's `kill` — or by `SIGINT` from the keyboard in
//! a foreground `mcf serve` died where it stood. The drop that stops its
//! engine server never ran, the record never said it stopped, and the server
//! sat holding its model until the next daemon's start found it (F262).
//! B-574 built that sweep; this closes the gap between the two starts.
//!
//! **What is done.** The signal is turned into the request a client would
//! send. A handler may do almost nothing — it runs between two instructions
//! of whatever the process was doing — so it does one thing that is safe
//! there: it writes a byte to a socket pair this module holds. A thread
//! waiting on the other end reads the byte and, as an ordinary client,
//! connects to the daemon's own control socket and asks it to stop, naming
//! the signal as the reason. From there the stop is the stop `mcf stop`
//! makes: the requests in flight close, the hold is let go in writing, the
//! server is dropped, `daemon_stopped` is recorded, and `serve` returns
//! `Stopped::Asked`.
//!
//! **What is not done.** `SIGKILL` cannot be handled by anything, and a
//! daemon ended that way is still the next daemon's to clean up after. A
//! second signal while the stop is under way is not a harder stop: the
//! request has been sent, and sending it again would be answered by a daemon
//! already closing. Nothing polls: the thread sleeps on a read, which costs
//! nothing while no signal comes (B-071).
//!
//! **Why this module takes the `unsafe_code` opt-out.** Installing a signal
//! handler and writing from inside one are two calls the standard library
//! does not offer: `signal(2)` and `write(2)`. The handler touches one
//! atomic and one file descriptor; everything else happens on the thread,
//! in safe Rust. The fourth such module in the workspace (build.md §4).

// The reason is above.
#![allow(unsafe_code)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::control::Request;

/// `SIGHUP`: the session that started a foreground daemon went away.
const HANGUP: i32 = 1;
/// `SIGINT`: the keyboard.
const INTERRUPT: i32 = 2;
/// `SIGTERM`: what `kill`, a scope and a session manager send.
const TERMINATE: i32 = 15;
/// The signals a stop is asked for on.
const STOPPING: [i32; 3] = [HANGUP, INTERRUPT, TERMINATE];
/// `SIG_DFL`: the platform's own handling, restored when the watch ends.
const DEFAULT: usize = 0;
/// How long the stop request is given to be answered.
const PATIENCE: Duration = Duration::from_secs(30);

/// The descriptor the handler writes to, or `-1` until the pipe is made.
static WRITER: AtomicI32 = AtomicI32::new(-1);
/// The last signal the handler saw, for the reason the stop carries.
static LAST: AtomicI32 = AtomicI32::new(0);
/// The sockets of the daemons this process is serving. A signal to the
/// process is a stop for every one of them — one, outside a test.
static WATCHED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// The one pipe and reader thread this process has, made on the first
/// watch and kept: a handler is process-wide, so what it writes to is too.
static PIPE: OnceLock<Option<UnixStream>> = OnceLock::new();

#[cfg(unix)]
unsafe extern "C" {
    /// `sighandler_t signal(int signum, sighandler_t handler)`.
    fn signal(signum: i32, handler: usize) -> usize;
    /// `ssize_t write(int fd, const void *buf, size_t count)`.
    fn write(fd: i32, buffer: *const u8, count: usize) -> isize;
}

/// The handler: one atomic store and one byte written. Nothing here
/// allocates, locks or formats, because a handler runs at any instruction
/// of any thread and may not wait on anything that thread holds.
extern "C" fn on_signal(which: i32) {
    LAST.store(which, Ordering::Relaxed);
    let descriptor = WRITER.load(Ordering::Relaxed);
    if descriptor < 0 {
        return;
    }
    let byte = [1_u8];
    // SAFETY: `write` is async-signal-safe by the platform's own list; the
    // buffer is one byte on this frame, and the descriptor is one this
    // module opened and keeps for the life of the process.
    let _written = unsafe { write(descriptor, byte.as_ptr(), 1) };
}

/// A watch over the stopping signals for the life of one `serve`.
///
/// While any watch is up the handlers are installed; dropping the last one
/// restores the platform's handling. The pipe and its reader thread are
/// made once and kept, asleep on a read that costs nothing until a signal
/// comes.
#[derive(Debug)]
pub struct Watch {
    /// The socket this watch asks to stop.
    socket: PathBuf,
}

impl Watch {
    /// Registers `socket` as one a signal stops, installing the handlers if
    /// this is the first. `None` where the pipe cannot be made, in which
    /// case the daemon runs without a watch and a signal ends it as before.
    #[must_use]
    pub fn over(socket: &Path) -> Option<Self> {
        PIPE.get_or_init(open_pipe).as_ref()?;
        let mut watched = WATCHED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if watched.is_empty() {
            for which in STOPPING {
                install(which, on_signal as *const () as usize);
            }
        }
        watched.push(socket.to_path_buf());
        Some(Self {
            socket: socket.to_path_buf(),
        })
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let mut watched = WATCHED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(at) = watched.iter().position(|held| *held == self.socket) {
            watched.remove(at);
        }
        if watched.is_empty() {
            for which in STOPPING {
                install(which, DEFAULT);
            }
        }
    }
}

/// The sockets a signal would stop right now.
#[must_use]
pub fn watched() -> Vec<PathBuf> {
    WATCHED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Makes the pipe and starts the thread that turns each byte on it into
/// stop requests. The writer is kept in the static for the handler; the
/// reader goes to the thread, which lives as long as the process.
fn open_pipe() -> Option<UnixStream> {
    let (writer, mut reader) = UnixStream::pair().ok()?;
    std::thread::Builder::new()
        .name("mcf-signals".to_owned())
        .spawn(move || {
            let mut byte = [0_u8; 1];
            while reader.read(&mut byte).is_ok_and(|read| read == 1) {
                let which = LAST.load(Ordering::Relaxed);
                for socket in watched() {
                    ask_to_stop(&socket, which);
                }
            }
        })
        .ok()?;
    WRITER.store(writer.as_raw_fd(), Ordering::Relaxed);
    Some(writer)
}

/// Sets a signal's handling.
#[cfg(unix)]
fn install(which: i32, handler: usize) {
    // SAFETY: `signal` takes a signal number and a handler address; the
    // handler is this module's `extern "C"` function or the platform's
    // default, and neither is unwound into.
    let _previous = unsafe { signal(which, handler) };
}

#[cfg(not(unix))]
fn install(_which: i32, _handler: usize) {}

/// The words the stop carries for a signal.
#[must_use]
pub fn reason_for(which: i32) -> String {
    let name = match which {
        HANGUP => "SIGHUP",
        INTERRUPT => "SIGINT",
        TERMINATE => "SIGTERM",
        _ => "a signal",
    };
    format!("the process received {name}")
}

/// As a client would: connects to the daemon's own socket and sends the stop
/// request, then reads the answer so the daemon is not left writing to a
/// closed connection. The outcome is not acted on — a daemon that could not
/// be reached is one that has already stopped.
fn ask_to_stop(socket: &Path, which: i32) {
    let Ok(mut connection) = UnixStream::connect(socket) else {
        return;
    };
    let _deadline = connection.set_read_timeout(Some(PATIENCE));
    let _writing = connection.set_write_timeout(Some(PATIENCE));
    let request = Request::Stop {
        reason: reason_for(which),
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return;
    }
    let mut line = String::new();
    let _answered = BufReader::new(&connection).read_line(&mut line);
}

#[cfg(test)]
mod tests;
