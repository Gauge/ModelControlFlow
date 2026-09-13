#![allow(unsafe_code, reason = "SDL is a C interface and has no safe form")]

use std::ffi::c_void;

pub const INIT_VIDEO: u32 = 0x0000_0020;

pub const EVENT_QUIT: u32 = 0x100;
pub const EVENT_KEY_DOWN: u32 = 0x300;
pub const EVENT_WINDOW_RESIZED: u32 = 0x206;
pub const EVENT_WINDOW_PIXEL_SIZE_CHANGED: u32 = 0x207;

pub const EVENT_MOUSE_MOTION: u32 = 0x400;
pub const EVENT_MOUSE_BUTTON_DOWN: u32 = 0x401;
pub const EVENT_MOUSE_BUTTON_UP: u32 = 0x402;
pub const EVENT_MOUSE_WHEEL: u32 = 0x403;

pub const MOUSE_X_OFFSET: usize = 28;
pub const MOUSE_Y_OFFSET: usize = 32;
pub const WHEEL_Y_OFFSET: usize = 28;
pub const MOUSE_BUTTON_OFFSET: usize = 24;

pub const BLEND: u32 = 1;
pub const TEXTURE_STATIC: u32 = 0;
pub const PIXELFORMAT_RGBA32: u32 = 0x1676_2004;

pub const EVENT_BYTES: usize = 128;
pub const KEY_OFFSET: usize = 28;

pub const MOD_OFFSET: usize = 32;
pub const KMOD_CTRL: u16 = 0x00C0;
pub const KMOD_SHIFT: u16 = 0x0003;

pub const KEY_MASK: u32 = 0x4000_0000;
pub const KEY_RIGHT: u32 = KEY_MASK | 0x4F;
pub const KEY_LEFT: u32 = KEY_MASK | 0x50;
pub const KEY_DOWN: u32 = KEY_MASK | 0x51;
pub const KEY_UP: u32 = KEY_MASK | 0x52;
pub const KEY_HOME: u32 = KEY_MASK | 0x4A;
pub const KEY_END: u32 = KEY_MASK | 0x4D;
pub const KEY_DELETE: u32 = 0x7F;
pub const KEY_RETURN: u32 = 0x0D;
pub const KEY_ESCAPE: u32 = 0x1B;
pub const KEY_BACKSPACE: u32 = 0x08;

pub const EVENT_TEXT_INPUT: u32 = 0x303;

pub const TEXT_OFFSET: usize = 24;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
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
    fn SDL_WaitEventTimeout(event: *mut u8, timeout_ms: i32) -> bool;
    fn SDL_SetRenderDrawColor(renderer: *mut c_void, r: u8, g: u8, b: u8, a: u8) -> bool;
    fn SDL_RenderClear(renderer: *mut c_void) -> bool;
    fn SDL_RenderFillRect(renderer: *mut c_void, rect: *const Rect) -> bool;
    fn SDL_SetRenderClipRect(renderer: *mut c_void, rect: *const ClipRect) -> bool;
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
    fn SDL_StartTextInput(window: *mut c_void) -> bool;
    fn SDL_GetClipboardText() -> *mut u8;
    fn SDL_SetClipboardText(text: *const u8) -> bool;
    fn SDL_HasClipboardText() -> bool;
    fn SDL_free(mem: *mut c_void);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held(usize);

impl Held {
    #[must_use]
    pub const fn at(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct Window {
    handle: *mut c_void,
    renderer: *mut c_void,
    textures: Vec<*mut c_void>,
}

pub const GLYPH: f32 = 8.0;

impl Window {
    #[cfg(have_sdl)]
    pub fn open(title: &str, width: i32, height: i32) -> Result<Self, String> {
        let title =
            std::ffi::CString::new(title).map_err(|_| "the title is not text".to_owned())?;
        let mut window: *mut c_void = std::ptr::null_mut();
        let mut renderer: *mut c_void = std::ptr::null_mut();
        unsafe {
            if !SDL_Init(INIT_VIDEO) {
                return Err(said());
            }
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

    #[cfg(not(have_sdl))]
    pub fn open(_title: &str, _width: i32, _height: i32) -> Result<Self, String> {
        Err("the window library is not installed — `mcf provision SDL3` builds it".to_owned())
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn next_event(&self) -> Option<[u8; EVENT_BYTES]> {
        let mut event = [0_u8; EVENT_BYTES];
        let had = unsafe { SDL_PollEvent(event.as_mut_ptr()) };
        had.then_some(event)
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn wait_event(&self, within: std::time::Duration) -> Option<[u8; EVENT_BYTES]> {
        let mut event = [0_u8; EVENT_BYTES];
        let timeout = i32::try_from(within.as_millis()).unwrap_or(i32::MAX);
        let had = unsafe { SDL_WaitEventTimeout(event.as_mut_ptr(), timeout) };
        had.then_some(event)
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn clipboard_text(&self) -> Option<String> {
        unsafe {
            if !SDL_HasClipboardText() {
                return None;
            }
            let held = SDL_GetClipboardText();
            if held.is_null() {
                return None;
            }
            let text = std::ffi::CStr::from_ptr(held.cast())
                .to_str()
                .ok()
                .map(str::to_owned);
            SDL_free(held.cast());
            text
        }
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn put_on_clipboard(&self, text: &str) -> bool {
        let Ok(held) = std::ffi::CString::new(text) else {
            return false;
        };
        unsafe { SDL_SetClipboardText(held.as_ptr().cast()) }
    }

    #[cfg(have_sdl)]
    pub fn clear(&self, colour: (u8, u8, u8)) {
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _cleared = SDL_RenderClear(self.renderer);
        }
    }

    #[cfg(have_sdl)]
    pub fn fill(&self, rect: Rect, colour: (u8, u8, u8)) {
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _filled = SDL_RenderFillRect(self.renderer, &raw const rect);
        }
    }

    #[cfg(have_sdl)]
    pub fn text(&self, x: f32, y: f32, text: &str, colour: (u8, u8, u8)) {
        let Ok(text) = std::ffi::CString::new(text) else {
            return;
        };
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, 255);
            let _drawn = SDL_RenderDebugText(self.renderer, x, y, text.as_ptr().cast());
        }
    }

    #[cfg(have_sdl)]
    pub fn present(&self) {
        unsafe {
            let _shown = SDL_RenderPresent(self.renderer);
        }
    }

    #[cfg(have_sdl)]
    pub fn clip(&self, rect: Option<ClipRect>) {
        unsafe {
            let _set = match rect {
                Some(rect) => SDL_SetRenderClipRect(self.renderer, &raw const rect),
                None => SDL_SetRenderClipRect(self.renderer, std::ptr::null()),
            };
        }
    }

    #[cfg(have_sdl)]
    pub fn fill_with(&self, rect: Rect, colour: (u8, u8, u8), alpha: u8) {
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, alpha);
            let _filled = SDL_RenderFillRect(self.renderer, &raw const rect);
        }
    }

    #[cfg(have_sdl)]
    pub fn line(&self, from: (f32, f32), to: (f32, f32), colour: (u8, u8, u8), alpha: u8) {
        unsafe {
            let _set = SDL_SetRenderDrawColor(self.renderer, colour.0, colour.1, colour.2, alpha);
            let _drawn = SDL_RenderLine(self.renderer, from.0, from.1, to.0, to.1);
        }
    }

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

    #[cfg(have_sdl)]
    pub fn blit(&self, held: Held, from: Rect, to: Rect, colour: (u8, u8, u8), alpha: u8) {
        let Some(&texture) = self.textures.get(held.0) else {
            return;
        };
        unsafe {
            let _tinted = SDL_SetTextureColorMod(texture, colour.0, colour.1, colour.2);
            let _faded = SDL_SetTextureAlphaMod(texture, alpha);
            let _drawn = SDL_RenderTexture(self.renderer, texture, &raw const from, &raw const to);
        }
    }

    #[cfg(have_sdl)]
    pub fn start_typing(&self) {
        unsafe {
            let _started = SDL_StartTextInput(self.handle);
        }
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn scale(&self) -> f32 {
        let scale = unsafe { SDL_GetWindowDisplayScale(self.handle) };
        if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        }
    }

    #[cfg(have_sdl)]
    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        let (mut width, mut height) = (0_i32, 0_i32);
        unsafe {
            let _got = SDL_GetRenderOutputSize(self.renderer, &raw mut width, &raw mut height);
        }
        (width, height)
    }
}

#[cfg(not(have_sdl))]
#[allow(clippy::unused_self, reason = "the shape must match the working one")]
impl Window {
    #[must_use]
    pub fn next_event(&self) -> Option<[u8; EVENT_BYTES]> {
        None
    }
    #[must_use]
    pub fn wait_event(&self, within: std::time::Duration) -> Option<[u8; EVENT_BYTES]> {
        std::thread::sleep(within);
        None
    }
    #[must_use]
    pub fn clipboard_text(&self) -> Option<String> {
        None
    }
    #[must_use]
    pub fn put_on_clipboard(&self, _text: &str) -> bool {
        false
    }
    pub fn clear(&self, _colour: (u8, u8, u8)) {}
    pub fn fill(&self, _rect: Rect, _colour: (u8, u8, u8)) {}
    pub fn text(&self, _x: f32, _y: f32, _text: &str, _colour: (u8, u8, u8)) {}
    pub fn present(&self) {}
    pub fn clip(&self, _rect: Option<ClipRect>) {}
    pub fn fill_with(&self, _rect: Rect, _colour: (u8, u8, u8), _alpha: u8) {}
    pub fn line(&self, _from: (f32, f32), _to: (f32, f32), _colour: (u8, u8, u8), _alpha: u8) {}
    pub fn upload(&mut self, _width: u32, _height: u32, _rgba: &[u8]) -> Option<Held> {
        None
    }
    pub fn blit(&self, _held: Held, _from: Rect, _to: Rect, _colour: (u8, u8, u8), _alpha: u8) {}
    #[must_use]
    pub fn scale(&self) -> f32 {
        1.0
    }
    pub fn start_typing(&self) {}
    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        (0, 0)
    }
}

#[cfg(have_sdl)]
impl Drop for Window {
    fn drop(&mut self) {
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

#[cfg(have_sdl)]
fn said() -> String {
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

#[must_use]
pub fn event_type(event: &[u8; EVENT_BYTES]) -> u32 {
    four_at(event, 0)
}

fn four_at(event: &[u8; EVENT_BYTES], at: usize) -> u32 {
    let Some(held) = event.get(at..at + 4) else {
        return 0;
    };
    let mut four = [0_u8; 4];
    four.copy_from_slice(held);
    u32::from_ne_bytes(four)
}

fn two_at(event: &[u8; EVENT_BYTES], at: usize) -> u16 {
    let Some(held) = event.get(at..at + 2) else {
        return 0;
    };
    let mut two = [0_u8; 2];
    two.copy_from_slice(held);
    u16::from_ne_bytes(two)
}

#[must_use]
pub fn event_key(event: &[u8; EVENT_BYTES]) -> u32 {
    four_at(event, KEY_OFFSET)
}

#[must_use]
pub fn event_mod(event: &[u8; EVENT_BYTES]) -> u16 {
    two_at(event, MOD_OFFSET)
}

#[must_use]
pub fn event_has_ctrl(event: &[u8; EVENT_BYTES]) -> bool {
    event_mod(event) & KMOD_CTRL != 0
}

#[must_use]
pub fn event_has_shift(event: &[u8; EVENT_BYTES]) -> bool {
    event_mod(event) & KMOD_SHIFT != 0
}

fn float_at(event: &[u8; EVENT_BYTES], at: usize) -> f32 {
    f32::from_bits(four_at(event, at))
}

#[must_use]
pub fn event_mouse(event: &[u8; EVENT_BYTES]) -> (f32, f32) {
    (
        float_at(event, MOUSE_X_OFFSET),
        float_at(event, MOUSE_Y_OFFSET),
    )
}

#[must_use]
pub fn event_wheel(event: &[u8; EVENT_BYTES]) -> f32 {
    float_at(event, WHEEL_Y_OFFSET)
}

#[must_use]
pub fn event_text(event: &[u8; EVENT_BYTES]) -> Option<String> {
    let held = event.get(TEXT_OFFSET..TEXT_OFFSET + 8)?;
    let mut eight = [0_u8; 8];
    eight.copy_from_slice(held);
    let pointer = usize::from_ne_bytes(eight) as *const std::ffi::c_char;
    if pointer.is_null() {
        return None;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(pointer) };
    Some(text.to_string_lossy().into_owned())
}

#[must_use]
pub fn event_is_left_button(event: &[u8; EVENT_BYTES]) -> bool {
    event.get(MOUSE_BUTTON_OFFSET) == Some(&1)
}

#[cfg(test)]
mod tests;
