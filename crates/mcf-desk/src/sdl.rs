//! The window library, declared rather than depended on.
//!
//! **Twenty-three functions.** A window, an event pump, rectangles, lines and
//! textures — that is the whole of what MCF asks of SDL, because every panel,
//! card, button and chart is drawn above this by MCF's own code. A widget
//! toolkit was refused, and the refusal held: what came instead is a
//! rasteriser (`font.rs`) and a painter (`paint.rs`), both MCF's.
//!
//! **Text is no longer SDL's.** The first design drew with `SDL_RenderDebugText`
//! and its 8×8 bitmap font, which is why the window looked like a terminal.
//! That call is still bound, and is now used for one thing: saying that no font
//! could be found, which is the one message that must survive having no font.
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
///
/// **This was `0x203` and `0x203` is `SDL_EVENT_WINDOW_HIDDEN`.** The number
/// was written from memory rather than read out of the header, so the window
/// re-laid itself out when it was hidden and never when it was resized. Every
/// constant in this file has since been read from the provisioned headers, and
/// a test reads them again (F131, A6).
pub const EVENT_WINDOW_RESIZED: u32 = 0x206;
/// The backing store changing size — a move between displays of different
/// pixel densities, which resizes nothing but changes what a pixel is.
pub const EVENT_WINDOW_PIXEL_SIZE_CHANGED: u32 = 0x207;

/// The pointer moving.
pub const EVENT_MOUSE_MOTION: u32 = 0x400;
/// A mouse button going down.
pub const EVENT_MOUSE_BUTTON_DOWN: u32 = 0x401;
/// A mouse button coming back up. A click is a press and a release inside the
/// same thing, which is why both are needed.
pub const EVENT_MOUSE_BUTTON_UP: u32 = 0x402;
/// The wheel turning.
pub const EVENT_MOUSE_WHEEL: u32 = 0x403;

/// Where the pointer's position sits in `SDL_MouseMotionEvent` and in
/// `SDL_MouseButtonEvent`. The two agree, which is why one offset serves both:
/// type, reserved, timestamp, window, which — then, in the button event,
/// button, down, clicks and padding filling the same four bytes that the
/// motion event spends on its state mask.
pub const MOUSE_X_OFFSET: usize = 28;
/// As `MOUSE_X_OFFSET`.
pub const MOUSE_Y_OFFSET: usize = 32;
/// Where the wheel's vertical travel sits in `SDL_MouseWheelEvent`.
pub const WHEEL_Y_OFFSET: usize = 28;
/// Which button, in `SDL_MouseButtonEvent`. One is the left one.
pub const MOUSE_BUTTON_OFFSET: usize = 24;

/// Alpha blending, so that anything drawn over anything else combines rather
/// than replacing. Everything MCF draws is blended: the corner masks, the
/// glyph coverage and every panel that sits on a shade of the ground.
pub const BLEND: u32 = 1;
/// A texture MCF uploads once and never changes.
pub const TEXTURE_STATIC: u32 = 0;
/// Four bytes per pixel, red first in memory. On a little-endian machine this
/// is what SDL calls `SDL_PIXELFORMAT_RGBA32`; the number is
/// `SDL_PIXELFORMAT_ABGR8888`, read from `SDL_pixels.h`.
pub const PIXELFORMAT_RGBA32: u32 = 0x1676_2004;

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
    fn SDL_SetRenderDrawBlendMode(renderer: *mut c_void, mode: u32) -> bool;
    fn SDL_RenderLine(renderer: *mut c_void, x1: f32, y1: f32, x2: f32, y2: f32) -> bool;
    fn SDL_CreateTexture(
        renderer: *mut c_void,
        format: u32,
        access: u32,
        w: i32,
        h: i32,
    ) -> *mut c_void;
    fn SDL_UpdateTexture(
        texture: *mut c_void,
        rect: *const c_void,
        pixels: *const u8,
        pitch: i32,
    ) -> bool;
    fn SDL_SetTextureBlendMode(texture: *mut c_void, mode: u32) -> bool;
    fn SDL_SetTextureColorMod(texture: *mut c_void, r: u8, g: u8, b: u8) -> bool;
    fn SDL_SetTextureAlphaMod(texture: *mut c_void, alpha: u8) -> bool;
    fn SDL_RenderTexture(
        renderer: *mut c_void,
        texture: *mut c_void,
        src: *const Rect,
        dst: *const Rect,
    ) -> bool;
    fn SDL_DestroyTexture(texture: *mut c_void);
    fn SDL_GetWindowDisplayScale(window: *mut c_void) -> f32;
    fn SDL_SetWindowMinimumSize(window: *mut c_void, w: i32, h: i32) -> bool;
}

/// A texture the window owns.
///
/// The window keeps them and destroys them before its renderer, which is why
/// this is a handle and not a pointer with a lifetime: a texture outliving its
/// renderer is a crash, and the ownership that prevents it is easier to state
/// than to prove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held(usize);

impl Held {
    /// A handle to the image at a position. The software surface issues these
    /// too, so that one painter can draw to either.
    #[must_use]
    pub const fn at(index: usize) -> Self {
        Self(index)
    }

    /// Which image this is.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// A window and the renderer that draws into it.
#[derive(Debug)]
pub struct Window {
    handle: *mut c_void,
    renderer: *mut c_void,
    textures: Vec<*mut c_void>,
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
        // SAFETY: both pointers came back non-null from a call that succeeded.
        unsafe {
            let _blended = SDL_SetRenderDrawBlendMode(renderer, BLEND);
            let _floor = SDL_SetWindowMinimumSize(window, 880, 560);
        }
        Ok(Self {
            handle: window,
            renderer,
            textures: Vec::new(),
        })
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

    /// Fills a rectangle, with an alpha.
    #[cfg(have_sdl)]
    pub fn fill_with(&self, rect: Rect, colour: (u8, u8, u8), alpha: u8) {
        // SAFETY: `rect` is a local, passed by pointer for the call only.
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, alpha);
            let _filled = SDL_RenderFillRect(self.renderer, &raw const rect);
        }
    }

    /// Draws a straight line one pixel wide.
    #[cfg(have_sdl)]
    pub fn line(&self, from: (f32, f32), to: (f32, f32), colour: (u8, u8, u8), alpha: u8) {
        // SAFETY: four floats and a renderer this struct owns.
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, alpha);
            let _drawn = SDL_RenderLine(self.renderer, from.0, from.1, to.0, to.1);
        }
    }

    /// Uploads an image and keeps it until the window closes.
    ///
    /// `rgba` is four bytes a pixel, red first, `width × height` of them.
    /// Returns `None` if SDL would not make the texture or the slice is not
    /// the size it claims — a texture built from the wrong number of bytes
    /// reads past the end of somebody's buffer, so the length is checked here
    /// rather than trusted (A2).
    #[cfg(have_sdl)]
    pub fn upload(&mut self, width: u32, height: u32, rgba: &[u8]) -> Option<Held> {
        let wanted = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if rgba.len() != wanted {
            return None;
        }
        let (Ok(w), Ok(h)) = (i32::try_from(width), i32::try_from(height)) else {
            return None;
        };
        let pitch = w.checked_mul(4)?;
        // SAFETY: the renderer is live; the pixel pointer is to a slice whose
        // length has just been checked against the dimensions passed beside
        // it, and SDL copies out of it before returning.
        let texture = unsafe {
            let made = SDL_CreateTexture(self.renderer, PIXELFORMAT_RGBA32, TEXTURE_STATIC, w, h);
            if made.is_null() {
                return None;
            }
            if !SDL_UpdateTexture(made, std::ptr::null(), rgba.as_ptr(), pitch) {
                SDL_DestroyTexture(made);
                return None;
            }
            let _blended = SDL_SetTextureBlendMode(made, BLEND);
            made
        };
        self.textures.push(texture);
        Some(Held(self.textures.len().saturating_sub(1)))
    }

    /// Draws part of an uploaded image into a rectangle, tinted.
    ///
    /// The tint is what makes one grey atlas of glyph coverage serve every
    /// colour of text on every screen.
    #[cfg(have_sdl)]
    pub fn blit(&self, held: Held, from: Rect, to: Rect, colour: (u8, u8, u8), alpha: u8) {
        let Some(&texture) = self.textures.get(held.0) else {
            return;
        };
        // SAFETY: the handle indexed a vector this struct owns, so the pointer
        // is one SDL gave back and has not been destroyed — `Drop` is the only
        // thing that destroys them, and it consumes the window.
        unsafe {
            let _tinted = SDL_SetTextureColorMod(texture, colour.0, colour.1, colour.2);
            let _faded = SDL_SetTextureAlphaMod(texture, alpha);
            let _drawn = SDL_RenderTexture(self.renderer, texture, &raw const from, &raw const to);
        }
    }

    /// How much larger this display draws things than a low-density one.
    ///
    /// One on an ordinary screen, two on a dense one. Every size in the
    /// interface is multiplied by it, which is the difference between an
    /// application and an application that is unreadable on a good monitor.
    #[cfg(have_sdl)]
    #[must_use]
    pub fn scale(&self) -> f32 {
        // SAFETY: the window pointer is live for the life of this struct.
        let scale = unsafe { SDL_GetWindowDisplayScale(self.handle) };
        if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
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
    pub fn fill_with(&self, _rect: Rect, _colour: (u8, u8, u8), _alpha: u8) {}
    /// Unreachable: `open` refused.
    pub fn line(&self, _from: (f32, f32), _to: (f32, f32), _colour: (u8, u8, u8), _alpha: u8) {}
    /// Unreachable: `open` refused.
    pub fn upload(&mut self, _width: u32, _height: u32, _rgba: &[u8]) -> Option<Held> {
        None
    }
    /// Unreachable: `open` refused.
    pub fn blit(&self, _held: Held, _from: Rect, _to: Rect, _colour: (u8, u8, u8), _alpha: u8) {}
    /// Unreachable: `open` refused.
    #[must_use]
    pub fn scale(&self) -> f32 {
        1.0
    }
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
        // Textures go before the renderer that made them, which is the order
        // SDL requires and the reason the window owns them at all.
        unsafe {
            for &texture in &self.textures {
                SDL_DestroyTexture(texture);
            }
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.handle);
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

/// A float at an offset, as SDL wrote it.
fn float_at(event: &[u8; EVENT_BYTES], at: usize) -> f32 {
    f32::from_bits(four_at(event, at))
}

/// Where the pointer was when this event happened, in window coordinates.
#[must_use]
pub fn event_mouse(event: &[u8; EVENT_BYTES]) -> (f32, f32) {
    (
        float_at(event, MOUSE_X_OFFSET),
        float_at(event, MOUSE_Y_OFFSET),
    )
}

/// How far the wheel turned. Positive is away from the hand.
#[must_use]
pub fn event_wheel(event: &[u8; EVENT_BYTES]) -> f32 {
    float_at(event, WHEEL_Y_OFFSET)
}

/// Whether a button event is the left button — the only one MCF acts on.
#[must_use]
pub fn event_is_left_button(event: &[u8; EVENT_BYTES]) -> bool {
    event.get(MOUSE_BUTTON_OFFSET) == Some(&1)
}

#[cfg(test)]
mod tests;
