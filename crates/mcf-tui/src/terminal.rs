use std::io::Write;

#[derive(Debug)]
pub struct Restored {
    #[cfg(unix)]
    saved: unix::Termios,
    #[cfg(windows)]
    saved: (u32, u32),
    done: bool,
}

impl Restored {
    pub fn now(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
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

pub fn take() -> Result<Restored, String> {
    #[cfg(unix)]
    let saved = unix::raw()?;
    #[cfg(windows)]
    let saved = windows::raw()?;

    let mut out = std::io::stdout();
    out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J")
        .and_then(|()| out.flush())
        .map_err(|error| format!("the alternate screen could not be entered: {error}"))?;

    let existing = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = std::io::stdout();
        let _shown = out.write_all(b"\x1b[?25h\x1b[?1049l");
        let _flushed = out.flush();
        existing(info);
    }));

    Ok(Restored { saved, done: false })
}

pub fn wait_for_a_key(should: bool) {
    #[cfg(unix)]
    unix::wait_for_a_key(should);
    #[cfg(windows)]
    let _ = should;
}

#[must_use]
pub fn size() -> (u16, u16) {
    #[cfg(unix)]
    let got = unix::size();
    #[cfg(windows)]
    let got = windows::size();
    got.unwrap_or((80, 24))
}

#[allow(unsafe_code, reason = "termios is a C interface and has no safe form")]
#[cfg(unix)]
mod unix {
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

    unsafe extern "C" {
        fn tcgetattr(fd: i32, held: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, when: i32, held: *const Termios) -> i32;
        fn ioctl(fd: i32, request: u64, ...) -> i32;
        fn isatty(fd: i32) -> i32;
    }

    const STDIN: i32 = 0;
    const TCSANOW: i32 = 0;
    const TIOCGWINSZ: u64 = 0x5413;

    const ISIG: u32 = 0x0001;
    const ICANON: u32 = 0x0002;
    const ECHO: u32 = 0x0008;
    const IEXTEN: u32 = 0x8000;
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
        held.characters[VMIN] = 1;
        held.characters[VTIME] = 0;
        if unsafe { tcsetattr(STDIN, TCSANOW, &raw const held) } != 0 {
            return Err("the terminal would not be put into raw mode".to_owned());
        }
        Ok(saved)
    }

    pub(super) fn wait_for_a_key(should: bool) {
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
            return;
        }
        held.characters[VMIN] = u8::from(should);
        held.characters[VTIME] = if should { 0 } else { 10 };
        let _set = unsafe { tcsetattr(STDIN, TCSANOW, &raw const held) };
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
    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
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
