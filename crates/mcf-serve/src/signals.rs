#![allow(unsafe_code)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::control::Request;

const HANGUP: i32 = 1;
const INTERRUPT: i32 = 2;
const TERMINATE: i32 = 15;
const STOPPING: [i32; 3] = [HANGUP, INTERRUPT, TERMINATE];
const DEFAULT: usize = 0;
const PATIENCE: Duration = Duration::from_secs(30);

static WRITER: AtomicI32 = AtomicI32::new(-1);
static LAST: AtomicI32 = AtomicI32::new(0);
static WATCHED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static PIPE: OnceLock<Option<UnixStream>> = OnceLock::new();

#[cfg(unix)]
unsafe extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn write(fd: i32, buffer: *const u8, count: usize) -> isize;
    fn raise(signum: i32) -> i32;
}

extern "C" fn on_signal(which: i32) {
    LAST.store(which, Ordering::Relaxed);
    let descriptor = WRITER.load(Ordering::Relaxed);
    if descriptor < 0 {
        return;
    }
    let byte = [1_u8];
    let _written = unsafe { write(descriptor, byte.as_ptr(), 1) };
}

#[derive(Debug)]
pub struct Watch {
    socket: PathBuf,
}

impl Watch {
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

#[must_use]
pub fn watched() -> Vec<PathBuf> {
    WATCHED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn open_pipe() -> Option<UnixStream> {
    let (writer, mut reader) = UnixStream::pair().ok()?;
    std::thread::Builder::new()
        .name("mcf-signals".to_owned())
        .spawn(move || {
            let mut byte = [0_u8; 1];
            while reader.read(&mut byte).is_ok_and(|read| read == 1) {
                let which = LAST.load(Ordering::Relaxed);
                let mut asked = false;
                for socket in watched() {
                    asked |= ask_to_stop(&socket, which);
                }
                if !asked {
                    stop_without_being_asked(which);
                }
            }
        })
        .ok()?;
    WRITER.store(writer.as_raw_fd(), Ordering::Relaxed);
    Some(writer)
}

#[cfg(unix)]
fn install(which: i32, handler: usize) {
    let _previous = unsafe { signal(which, handler) };
}

#[cfg(not(unix))]
fn install(_which: i32, _handler: usize) {}

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

fn ask_to_stop(socket: &Path, which: i32) -> bool {
    let Ok(mut connection) = UnixStream::connect(socket) else {
        return false;
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
        return false;
    }
    let mut line = String::new();
    let _answered = BufReader::new(&connection).read_line(&mut line);
    true
}

fn stop_without_being_asked(which: i32) {
    for each in STOPPING {
        install(each, DEFAULT);
    }
    let _raised = unsafe { raise(which) };
}

#[cfg(test)]
mod tests;
