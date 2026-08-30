//! The terminal itself: raw input, an alternate screen, and getting both back.
//!
//! **Written rather than depended on, and it is not much to write.** Raw input
//! is one call on each platform — `tcsetattr` where there is a termios and
//! `SetConsoleMode` where there is a console — and both live in libraries the
//! standard library already links, so they can be declared here. Drawing is
//! escape sequences, which are bytes. P3 keeps every dependency out of the
//! measuring crates; this one does not need an exception.
//!
//! **Restoration is the whole risk.** A terminal left in raw mode is a terminal
//! that no longer echoes what the operator types, and they will not know why.
//! So it is given back on three paths and not one: the [`Restored`] guard on
//! every ordinary return, a panic hook for the path `Drop` does not run on, and
//! an explicit call before the process exits. A22's *every action is available
//! with no display attached* is about capability; this is the other side of the
//! same care — a display that was attached is put back the way it was found.

use std::io::Write;

/// What the terminal was before MCF touched it.
///
/// Holding this is what makes the change reversible. It is deliberately not
/// `Copy`: there is one saved state, and a second copy restored later would put
/// back a terminal that had moved on.
#[derive(Debug)]
pub struct Restored {
    #[cfg(unix)]
    saved: unix::Termios,
    #[cfg(windows)]
    saved: (u32, u32),
    /// Set once the terminal has been given back, so a guard that has already
    /// done its work does not do it twice.
    done: bool,
}

impl Restored {
    /// Gives the terminal back. Safe to call more than once.
    pub fn now(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        // The order matters: leave the alternate screen last, so anything the
        // caller printed on the way out lands on the screen the operator keeps.
        let mut out = std::io::stdout();
        let _shown = out.write_all(b"\x1b[?25h\x1b[?1049l");
        let _flushed = out.flush();
        #[cfg(unix)]
        unix::restore(&self.saved);
        #[cfg(windows)]
        windows::restore(self.saved);
    }
}

impl Drop for Restored {
    fn drop(&mut self) {
        self.now();
    }
}

/// Takes the terminal: raw input, alternate screen, cursor hidden.
///
/// # Errors
///
/// A string saying what could not be done. The commonest is not a fault: MCF's
/// output is a pipe or a file, there is no terminal to take, and a caller is
/// told so rather than drawing a screen nobody is looking at (A7).
pub fn take() -> Result<Restored, String> {
    #[cfg(unix)]
    let saved = unix::raw()?;
    #[cfg(windows)]
    let saved = windows::raw()?;

    // The alternate screen means the operator's scrollback is not scribbled
    // over: what was in the terminal before is exactly what is in it after.
    let mut out = std::io::stdout();
    out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J")
        .and_then(|()| out.flush())
        .map_err(|error| format!("the alternate screen could not be entered: {error}"))?;

    // The path `Drop` does not run on. A panic while the terminal is raw would
    // otherwise leave it raw, and the message printed by the panic would be the
    // last legible thing on the screen.
    let existing = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = std::io::stdout();
        let _shown = out.write_all(b"\x1b[?25h\x1b[?1049l");
        let _flushed = out.flush();
        existing(info);
    }));

    Ok(Restored { saved, done: false })
}

/// How many columns and rows there are, or a usable pair if the platform will
/// not say.
#[must_use]
pub fn size() -> (u16, u16) {
    #[cfg(unix)]
    let got = unix::size();
    #[cfg(windows)]
    let got = windows::size();
    // Eighty by twenty-four is the size a terminal has when nothing says
    // otherwise. It is a fallback, not a measurement, and the screen is drawn
    // to fit whatever it turns out to be.
    got.unwrap_or((80, 24))
}

/// Raw input on a platform that has a termios.
///
/// **The opt-in the workspace anticipated.** `unsafe_code` is denied rather
/// than forbidden precisely so that C-ABI work can opt in per module with the
/// reason written here, and this is that work: putting a terminal into raw
/// mode is one call into the C library and there is no safe spelling of it.
///
/// **What the unsafety is, exactly.** Four foreign functions, each called with
/// a pointer to a stack local of the layout the platform documents, and a file
/// descriptor that is the constant 0. Nothing is allocated, nothing is freed,
/// no pointer outlives the call it is passed to, and no value crosses the
/// boundary except integers and the `Termios` this module owns. The one thing
/// that could be got wrong is the struct layout, which is why it is `repr(C)`
/// and why `tcgetattr` fills it before anything reads it.
#[allow(unsafe_code, reason = "termios is a C interface and has no safe form")]
#[cfg(unix)]
mod unix {
    /// `struct termios`, as the platform lays it out.
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub(super) struct Termios {
        input: u32,
        output: u32,
        control: u32,
        local: u32,
        line: u8,
        characters: [u8; 32],
        input_speed: u32,
        output_speed: u32,
    }

    #[repr(C)]
    struct WindowSize {
        rows: u16,
        columns: u16,
        width_pixels: u16,
        height_pixels: u16,
    }

    // The standard library links libc already; these are declarations, not a
    // dependency.
    unsafe extern "C" {
        fn tcgetattr(fd: i32, held: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, when: i32, held: *const Termios) -> i32;
        fn ioctl(fd: i32, request: u64, ...) -> i32;
        fn isatty(fd: i32) -> i32;
    }

    const STDIN: i32 = 0;
    const TCSANOW: i32 = 0;
    const TIOCGWINSZ: u64 = 0x5413;

    // Local flags. ISIG is cleared so that ctrl-c arrives as a key: the
    // application decides to stop and gives the terminal back on its way out,
    // where a signal would have ended the process with the terminal still raw.
    const ISIG: u32 = 0x0001;
    const ICANON: u32 = 0x0002;
    const ECHO: u32 = 0x0008;
    const IEXTEN: u32 = 0x8000;
    // Input flags: ctrl-s must not stop the output, and a carriage return must
    // arrive as itself.
    const IXON: u32 = 0x0400;
    const ICRNL: u32 = 0x0100;

    const VMIN: usize = 6;
    const VTIME: usize = 5;

    pub(super) fn raw() -> Result<Termios, String> {
        if unsafe { isatty(STDIN) } != 1 {
            return Err("MCF's input is not a terminal, so there is no screen to draw".to_owned());
        }
        let mut held = Termios {
            input: 0,
            output: 0,
            control: 0,
            local: 0,
            line: 0,
            characters: [0; 32],
            input_speed: 0,
            output_speed: 0,
        };
        if unsafe { tcgetattr(STDIN, &raw mut held) } != 0 {
            return Err("the terminal would not say what state it is in".to_owned());
        }
        let saved = held;
        held.local &= !(ISIG | ICANON | ECHO | IEXTEN);
        held.input &= !(IXON | ICRNL);
        // Block until there is a key, and wait no longer than there is one for.
        // This is what makes the application cost nothing while nobody types.
        held.characters[VMIN] = 1;
        held.characters[VTIME] = 0;
        if unsafe { tcsetattr(STDIN, TCSANOW, &raw const held) } != 0 {
            return Err("the terminal would not be put into raw mode".to_owned());
        }
        Ok(saved)
    }

    pub(super) fn restore(saved: &Termios) {
        let _put_back = unsafe { tcsetattr(STDIN, TCSANOW, std::ptr::from_ref(saved)) };
    }

    pub(super) fn size() -> Option<(u16, u16)> {
        let mut found = WindowSize {
            rows: 0,
            columns: 0,
            width_pixels: 0,
            height_pixels: 0,
        };
        if unsafe { ioctl(STDIN, TIOCGWINSZ, &raw mut found) } != 0 || found.columns == 0 {
            return None;
        }
        Some((found.columns, found.rows))
    }
}

/// Raw input on a platform that has a console.
///
/// The same opt-in as `unix` above and for the same reason: `SetConsoleMode`
/// is a C interface. The calls take a handle obtained from `GetStdHandle` and
/// a pointer to a stack local; nothing is allocated and nothing outlives the
/// call.
#[allow(
    unsafe_code,
    reason = "the console API is a C interface and has no safe form"
)]
#[cfg(windows)]
mod windows {
    #[repr(C)]
    #[derive(Default)]
    struct Coordinate {
        x: i16,
        y: i16,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Rectangle {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ScreenBufferInfo {
        size: Coordinate,
        cursor: Coordinate,
        attributes: u16,
        window: Rectangle,
        maximum: Coordinate,
    }

    // kernel32 is already linked by the standard library.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> isize;
        fn GetConsoleMode(handle: isize, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: isize, mode: u32) -> i32;
        fn GetConsoleScreenBufferInfo(handle: isize, info: *mut ScreenBufferInfo) -> i32;
    }

    const STD_INPUT: u32 = -10_i32 as u32;
    const STD_OUTPUT: u32 = -11_i32 as u32;

    const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    /// Without this, arrow keys arrive as console records rather than as the
    /// escape sequences every other platform sends, and the key decoder would
    /// need a second implementation.
    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
    /// Without this, the escape sequences MCF writes are printed rather than
    /// obeyed. Windows 10 and later understand them once asked.
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    pub(super) fn raw() -> Result<(u32, u32), String> {
        let input = unsafe { GetStdHandle(STD_INPUT) };
        let output = unsafe { GetStdHandle(STD_OUTPUT) };
        let (mut was_in, mut was_out) = (0_u32, 0_u32);
        if unsafe { GetConsoleMode(input, &raw mut was_in) } == 0
            || unsafe { GetConsoleMode(output, &raw mut was_out) } == 0
        {
            return Err("MCF's input is not a console, so there is no screen to draw".to_owned());
        }
        let raw_in = (was_in & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT))
            | ENABLE_VIRTUAL_TERMINAL_INPUT;
        let raw_out = was_out | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        if unsafe { SetConsoleMode(input, raw_in) } == 0
            || unsafe { SetConsoleMode(output, raw_out) } == 0
        {
            return Err("the console would not be put into raw mode".to_owned());
        }
        Ok((was_in, was_out))
    }

    pub(super) fn restore(saved: (u32, u32)) {
        let input = unsafe { GetStdHandle(STD_INPUT) };
        let output = unsafe { GetStdHandle(STD_OUTPUT) };
        let _put_back = unsafe { SetConsoleMode(input, saved.0) };
        let _also = unsafe { SetConsoleMode(output, saved.1) };
    }

    pub(super) fn size() -> Option<(u16, u16)> {
        let output = unsafe { GetStdHandle(STD_OUTPUT) };
        let mut info = ScreenBufferInfo::default();
        if unsafe { GetConsoleScreenBufferInfo(output, &raw mut info) } == 0 {
            return None;
        }
        let columns = u16::try_from(info.window.right - info.window.left + 1).ok()?;
        let rows = u16::try_from(info.window.bottom - info.window.top + 1).ok()?;
        (columns > 0 && rows > 0).then_some((columns, rows))
    }
}
