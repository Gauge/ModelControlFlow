//! The window library, declared rather than depended on.
//!
//! **Twelve functions.** A window, an event pump, a few rectangles and a line
//! of text — that is the whole of what MCF asks of SDL, because every panel,
//! table, column and button is drawn by MCF's own layout code, the same code
//! the terminal console uses. A widget toolkit would have been a second
//! implementation of something that already exists and is tested.
//!
//! **The opt-in the workspace anticipated, for the second time.** `unsafe_code`
//! is denied rather than forbidden precisely so C-ABI work can opt in per
//! module with the reason at the site. The terminal does it for `tcsetattr`;
//! this does it for SDL. What crosses the boundary is integers, a byte buffer
//! MCF owns, and pointers that live no longer than the call they are passed to.

#![allow(unsafe_code, reason = "SDL is a C interface and has no safe form")]

use std::ffi::c_void;

/// What `SDL_Init` is asked for. Video only — MCF has no use for audio, a
/// joystick or a sensor, and the provisioned build has none of them compiled.
pub const INIT_VIDEO: u32 = 0x0000_0020;

/// The events MCF acts on. Everything else is read and dropped.
pub const EVENT_QUIT: u32 = 0x100;
/// A key going down.
pub const EVENT_KEY_DOWN: u32 = 0x300;
/// The window being resized, which changes how much fits.
pub const EVENT_WINDOW_RESIZED: u32 = 0x203;

/// An `SDL_Event` is a union; this is its size, and MCF reads two fields out
/// of it by offset rather than describing the whole shape.
pub const EVENT_BYTES: usize = 128;
/// Where the keycode sits in `SDL_KeyboardEvent`: type, reserved, timestamp,
/// window, keyboard, scancode — then the key.
pub const KEY_OFFSET: usize = 28;

/// Keycodes. Printable keys are their own character; the rest carry a mask.
pub const KEY_MASK: u32 = 0x4000_0000;
/// Cursor right, from its scancode.
pub const KEY_RIGHT: u32 = KEY_MASK | 0x4F;
/// Cursor left.
pub const KEY_LEFT: u32 = KEY_MASK | 0x50;
/// Cursor down.
pub const KEY_DOWN: u32 = KEY_MASK | 0x51;
/// Cursor up.
pub const KEY_UP: u32 = KEY_MASK | 0x52;
/// Return.
pub const KEY_RETURN: u32 = 0x0D;
/// Escape.
pub const KEY_ESCAPE: u32 = 0x1B;

/// A rectangle in the renderer's own floating-point coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

#[cfg(have_sdl)]
unsafe extern "C" {
    fn SDL_Init(flags: u32) -> bool;
    fn SDL_Quit();
    fn SDL_CreateWindowAndRenderer(
        title: *const u8,
        width: i32,
        height: i32,
        flags: u64,
        window: *mut *mut c_void,
        renderer: *mut *mut c_void,
    ) -> bool;
    fn SDL_DestroyRenderer(renderer: *mut c_void);
    fn SDL_DestroyWindow(window: *mut c_void);
    fn SDL_PollEvent(event: *mut u8) -> bool;
    fn SDL_SetRenderDrawColor(renderer: *mut c_void, r: u8, g: u8, b: u8, a: u8) -> bool;
    fn SDL_RenderClear(renderer: *mut c_void) -> bool;
    fn SDL_RenderFillRect(renderer: *mut c_void, rect: *const Rect) -> bool;
    fn SDL_RenderDebugText(renderer: *mut c_void, x: f32, y: f32, text: *const u8) -> bool;
    fn SDL_RenderPresent(renderer: *mut c_void) -> bool;
    fn SDL_GetRenderOutputSize(renderer: *mut c_void, w: *mut i32, h: *mut i32) -> bool;
    fn SDL_GetError() -> *const u8;
}

/// A window and the renderer that draws into it.
#[derive(Debug)]
pub struct Window {
    window: *mut c_void,
    renderer: *mut c_void,
}

/// How wide and tall one character of SDL's built-in font is.
pub const GLYPH: f32 = 8.0;

impl Window {
    /// Opens a window, or says why it could not.
    ///
    /// # Errors
    ///
    /// What SDL said, or a sentence saying the library is not provisioned.
    #[cfg(have_sdl)]
    pub fn open(title: &str, width: i32, height: i32) -> Result<Self, String> {
        let title =
            std::ffi::CString::new(title).map_err(|_| "the title is not text".to_owned())?;
        let mut window: *mut c_void = std::ptr::null_mut();
        let mut renderer: *mut c_void = std::ptr::null_mut();
        // SAFETY: `SDL_Init` takes an integer. Every pointer below is to a
        // local that outlives the call, and the title is a C string alive for
        // the whole of it.
        unsafe {
            if !SDL_Init(INIT_VIDEO) {
                return Err(said());
            }
            // 0x20 is SDL_WINDOW_RESIZABLE.
            if !SDL_CreateWindowAndRenderer(
                title.as_ptr().cast(),
                width,
                height,
                0x20,
                &raw mut window,
                &raw mut renderer,
            ) {
                let why = said();
                SDL_Quit();
                return Err(why);
            }
        }
        Ok(Self { window, renderer })
    }

    /// Where there is no provisioned library, opening says so and says what to
    /// run — rather than failing to compile for everybody who has not.
    #[cfg(not(have_sdl))]
    pub fn open(_title: &str, _width: i32, _height: i32) -> Result<Self, String> {
        Err("the window library is not installed — `mcf provision SDL3` builds it".to_owned())
    }

    /// The next event, or `None` when there are no more waiting.
    #[cfg(have_sdl)]
    #[must_use]
    pub fn next_event(&self) -> Option<[u8; EVENT_BYTES]> {
        let mut event = [0_u8; EVENT_BYTES];
        // SAFETY: SDL writes at most `sizeof(SDL_Event)` bytes, which is what
        // this buffer is sized for and why it is not smaller.
        let had = unsafe { SDL_PollEvent(event.as_mut_ptr()) };
        had.then_some(event)
    }

    /// Clears the window to one colour.
    #[cfg(have_sdl)]
    pub fn clear(&self, colour: (u8, u8, u8)) {
        // SAFETY: the renderer came from `SDL_CreateWindowAndRenderer` and is
        // dropped only in `Drop`.
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _cleared = SDL_RenderClear(self.renderer);
        }
    }

    /// Fills a rectangle.
    #[cfg(have_sdl)]
    pub fn fill(&self, rect: Rect, colour: (u8, u8, u8)) {
        // SAFETY: `rect` is a local of the layout SDL documents, passed by
        // pointer for the duration of the call.
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _filled = SDL_RenderFillRect(self.renderer, &raw const rect);
        }
    }

    /// Draws one line of text in SDL's own font.
    #[cfg(have_sdl)]
    pub fn text(&self, x: f32, y: f32, text: &str, colour: (u8, u8, u8)) {
        let Ok(text) = std::ffi::CString::new(text) else {
            return;
        };
        // SAFETY: the C string outlives the call.
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _drawn = SDL_RenderDebugText(self.renderer, x, y, text.as_ptr().cast());
        }
    }

    /// Shows what has been drawn.
    #[cfg(have_sdl)]
    pub fn present(&self) {
        // SAFETY: as above.
        unsafe {
            let _shown = SDL_RenderPresent(self.renderer);
        }
    }

    /// How many pixels the window has now.
    #[cfg(have_sdl)]
    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        let (mut width, mut height) = (0_i32, 0_i32);
        // SAFETY: two locals, written by SDL, read after the call.
        unsafe {
            let _got = SDL_GetRenderOutputSize(self.renderer, &raw mut width, &raw mut height);
        }
        (width, height)
    }
}

/// Without the library there is no window, so there is nothing to draw into.
///
/// These exist so the crate COMPILES on a machine that has not provisioned it —
/// `open` has already refused, so none of them can be reached. A crate that
/// would not build without a provisioned component would break the build for
/// everybody who has not provisioned one, which is a worse failure than a
/// window that will not open.
#[cfg(not(have_sdl))]
#[allow(clippy::unused_self, reason = "the shape must match the working one")]
impl Window {
    /// Unreachable: `open` refused.
    #[must_use]
    pub fn next_event(&self) -> Option<[u8; EVENT_BYTES]> {
        None
    }
    /// Unreachable: `open` refused.
    pub fn clear(&self, _colour: (u8, u8, u8)) {}
    /// Unreachable: `open` refused.
    pub fn fill(&self, _rect: Rect, _colour: (u8, u8, u8)) {}
    /// Unreachable: `open` refused.
    pub fn text(&self, _x: f32, _y: f32, _text: &str, _colour: (u8, u8, u8)) {}
    /// Unreachable: `open` refused.
    pub fn present(&self) {}
    /// Unreachable: `open` refused.
    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        (0, 0)
    }
}

#[cfg(have_sdl)]
impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: each pointer is dropped once, and the struct is not `Copy`.
        unsafe {
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.window);
            SDL_Quit();
        }
    }
}

/// What SDL last complained about.
#[cfg(have_sdl)]
fn said() -> String {
    // SAFETY: SDL owns the string and keeps it until the next call that sets
    // it; it is copied here before anything else happens.
    unsafe {
        let held = SDL_GetError();
        if held.is_null() {
            return "SDL did not say".to_owned();
        }
        std::ffi::CStr::from_ptr(held.cast())
            .to_string_lossy()
            .into_owned()
    }
}

/// The type of an event, from the buffer SDL filled.
#[must_use]
pub fn event_type(event: &[u8; EVENT_BYTES]) -> u32 {
    four_at(event, 0)
}

/// Four bytes at an offset, as SDL wrote them.
fn four_at(event: &[u8; EVENT_BYTES], at: usize) -> u32 {
    let Some(held) = event.get(at..at + 4) else {
        return 0;
    };
    let mut four = [0_u8; 4];
    four.copy_from_slice(held);
    u32::from_ne_bytes(four)
}

/// The keycode of a key event.
#[must_use]
pub fn event_key(event: &[u8; EVENT_BYTES]) -> u32 {
    four_at(event, KEY_OFFSET)
}

#[cfg(test)]
mod tests;
