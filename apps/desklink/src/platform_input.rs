use anyhow::{bail, Result};
use link_core::InputEvent;

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::ffi::c_void;
    type Event = *mut c_void;
    #[repr(C)] #[derive(Clone, Copy)] struct Point { x: f64, y: f64 }
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn CGEventCreate(source: *mut c_void) -> Event;
        fn CGEventGetLocation(event: Event) -> Point;
        fn CGEventCreateMouseEvent(source: *mut c_void, kind: u32, point: Point, button: u32) -> Event;
        fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> Event;
        fn CGEventCreateScrollWheelEvent(source: *mut c_void, units: u32, count: u32, delta1: i32, delta2: i32, delta3: i32) -> Event;
        fn CGEventPost(tap: u32, event: Event);
        fn CFRelease(object: *mut c_void);
    }
    const LEFT: u32 = 0; const RIGHT: u32 = 1; const CENTER: u32 = 2;
    const MOVED: u32 = 5; const LEFT_DOWN: u32 = 1; const LEFT_UP: u32 = 2;
    const RIGHT_DOWN: u32 = 3; const RIGHT_UP: u32 = 4; const OTHER_DOWN: u32 = 25; const OTHER_UP: u32 = 26;
    const SCROLL_PIXEL: u32 = 0; const HID_TAP: u32 = 0;

    pub fn inject(event: InputEvent) -> Result<()> {
        unsafe {
            if !AXIsProcessTrusted() { bail!("Enable DeskLink in System Settings → Privacy & Security → Accessibility"); }
            let out = match event {
                InputEvent::PointerMotion(m) => {
                    let current = CGEventCreate(std::ptr::null_mut());
                    if current.is_null() { bail!("Could not read cursor position"); }
                    let p = CGEventGetLocation(current); CFRelease(current);
                    CGEventCreateMouseEvent(std::ptr::null_mut(), MOVED, Point { x: p.x + m.dx as f64, y: p.y + m.dy as f64 }, LEFT)
                }
                InputEvent::Button { button, pressed } => {
                    let (kind, btn) = match button { 0 => (if pressed { LEFT_DOWN } else { LEFT_UP }, LEFT), 1 => (if pressed { RIGHT_DOWN } else { RIGHT_UP }, RIGHT), _ => (if pressed { OTHER_DOWN } else { OTHER_UP }, CENTER) };
                    let current = CGEventCreate(std::ptr::null_mut()); if current.is_null() { bail!("Could not read cursor position"); }
                    let p = CGEventGetLocation(current); CFRelease(current);
                    CGEventCreateMouseEvent(std::ptr::null_mut(), kind, p, btn)
                }
                InputEvent::Scroll { horizontal, vertical } => CGEventCreateScrollWheelEvent(std::ptr::null_mut(), SCROLL_PIXEL, 2, vertical as i32, horizontal as i32, 0),
                InputEvent::Key { scan_code, pressed } => {
                    let Some(code) = mac_keycode(scan_code) else { bail!("Unsupported HID key usage {scan_code}"); };
                    CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, pressed)
                }
            };
            if out.is_null() { bail!("Could not construct native input event"); }
            CGEventPost(HID_TAP, out); CFRelease(out);
        }
        Ok(())
    }

    fn mac_keycode(hid: u16) -> Option<u16> {
        let letters = [0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46, 45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16, 6];
        if (0x04..=0x1d).contains(&hid) { return Some(letters[(hid - 0x04) as usize]); }
        Some(match hid {
            0x1e => 18, 0x1f => 19, 0x20 => 20, 0x21 => 21, 0x22 => 23, 0x23 => 22, 0x24 => 26, 0x25 => 28, 0x26 => 25, 0x27 => 29,
            0x28 => 36, 0x29 => 53, 0x2a => 51, 0x2b => 48, 0x2c => 49,
            0x4f => 124, 0x50 => 123, 0x51 => 125, 0x52 => 126,
            0xe0 => 59, 0xe1 => 56, 0xe2 => 58, 0xe3 => 55, 0xe4 => 62, 0xe5 => 60, 0xe6 => 61, 0xe7 => 54,
            _ => return None,
        })
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, INPUT_u, KEYBDINPUT, MOUSEINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_MOVE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_WHEEL};
    fn send(input: INPUT) -> Result<()> { let sent = unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) }; if sent != 1 { bail!("Windows rejected the input event"); } Ok(()) }
    fn mouse(dx: i32, dy: i32, flags: u32, data: u32) -> Result<()> {
        send(INPUT { type_: INPUT_MOUSE, u: INPUT_u { mi: MOUSEINPUT { dx, dy, mouseData: data, dwFlags: flags, time: 0, dwExtraInfo: 0 } } })
    }
    pub fn inject(event: InputEvent) -> Result<()> {
        match event {
            InputEvent::PointerMotion(m) => mouse(m.dx, m.dy, MOUSEEVENTF_MOVE, 0),
            InputEvent::Button { button: 0, pressed } => mouse(0, 0, if pressed { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP }, 0),
            InputEvent::Button { button: 1, pressed } => mouse(0, 0, if pressed { MOUSEEVENTF_RIGHTDOWN } else { MOUSEEVENTF_RIGHTUP }, 0),
            InputEvent::Button { pressed, .. } => mouse(0, 0, if pressed { MOUSEEVENTF_MIDDLEDOWN } else { MOUSEEVENTF_MIDDLEUP }, 0),
            InputEvent::Scroll { vertical, .. } => mouse(0, 0, MOUSEEVENTF_WHEEL, vertical as i32 as u32),
            InputEvent::Key { scan_code, pressed } => {
                let Some(code) = windows_scancode(scan_code) else { bail!("Unsupported HID key usage {scan_code}"); };
                send(INPUT { type_: INPUT_KEYBOARD, u: INPUT_u { ki: KEYBDINPUT { wVk: 0, wScan: code, dwFlags: KEYEVENTF_SCANCODE | if pressed { 0 } else { KEYEVENTF_KEYUP }, time: 0, dwExtraInfo: 0 } } })
            }
        }
    }
    fn windows_scancode(hid: u16) -> Option<u16> {
        let letters = [0x1e,0x30,0x2e,0x20,0x12,0x21,0x22,0x23,0x17,0x24,0x25,0x26,0x32,0x31,0x18,0x19,0x10,0x13,0x1f,0x14,0x16,0x2f,0x11,0x2d,0x15,0x2c];
        if (0x04..=0x1d).contains(&hid) { return Some(letters[(hid - 0x04) as usize]); }
        Some(match hid {
            0x1e => 0x02,0x1f => 0x03,0x20 => 0x04,0x21 => 0x05,0x22 => 0x06,0x23 => 0x07,0x24 => 0x08,0x25 => 0x09,0x26 => 0x0a,0x27 => 0x0b,
            0x28 => 0x1c,0x29 => 0x01,0x2a => 0x0e,0x2b => 0x0f,0x2c => 0x39,0x4f => 0x4d,0x50 => 0x4b,0x51 => 0x50,0x52 => 0x48,
            0xe0 => 0x1d,0xe1 => 0x2a,0xe2 => 0x38,0xe3 => 0x5b,0xe4 => 0x1d,0xe5 => 0x36,0xe6 => 0x38,0xe7 => 0x5c,
            _ => return None,
        })
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod native { use super::*; pub fn inject(_: InputEvent) -> Result<()> { bail!("Native input injection is not implemented on this operating system") } }

pub fn inject(event: InputEvent) -> Result<()> { native::inject(event) }
