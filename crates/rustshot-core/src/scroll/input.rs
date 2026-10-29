//! Moving the pointer and sending scroll-wheel events to whatever is under
//! it. macOS needs the Accessibility permission for this; Windows needs
//! nothing; Linux needs X11 (Wayland doesn't let apps send input to others).
//!
//! Positions are in the same units as [`crate::pointer`]: global points on
//! macOS, global pixels elsewhere.

use anyhow::Result;

/// Whether this app may send scroll events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Granted,
    /// macOS: Accessibility isn't allowed yet.
    NeedsPermission,
    /// Linux without X11.
    Unsupported,
}

pub fn access() -> Access {
    #[cfg(target_os = "macos")]
    return if macos::trusted() { Access::Granted } else { Access::NeedsPermission };
    #[cfg(windows)]
    return Access::Granted;
    #[cfg(all(unix, not(target_os = "macos")))]
    return if x11::connect().is_some() { Access::Granted } else { Access::Unsupported };
}

/// macOS: show the system's Accessibility prompt (and list Rustshot in
/// System Settings → Privacy & Security → Accessibility). Returns at once.
pub fn request_access() {
    #[cfg(target_os = "macos")]
    macos::prompt();
}

/// Sends the scrolling. Created on the thread that uses it.
pub struct Scroller {
    #[cfg(all(unix, not(target_os = "macos")))]
    x: x11::Conn,
}

impl Scroller {
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(all(unix, not(target_os = "macos")))]
            x: x11::connect().ok_or_else(|| anyhow::anyhow!("scrolling capture needs an X11 session"))?,
        })
    }

    /// Put the pointer at `(x, y)`.
    pub fn move_to(&mut self, x: f64, y: f64) {
        #[cfg(target_os = "macos")]
        macos::warp(x, y);
        #[cfg(windows)]
        windows_input::move_to(x, y);
        #[cfg(all(unix, not(target_os = "macos")))]
        self.x.move_to(x, y);
    }

    /// Scroll the content under the pointer down: `amount` is points on
    /// macOS, wheel notches elsewhere.
    pub fn scroll_down(&mut self, amount: u32) {
        #[cfg(target_os = "macos")]
        macos::scroll(amount);
        #[cfg(windows)]
        windows_input::scroll(amount);
        #[cfg(all(unix, not(target_os = "macos")))]
        self.x.scroll(amount);
    }

    /// Whether scrolling is measured in points (macOS) rather than wheel
    /// notches (elsewhere).
    pub fn scrolls_by_points() -> bool {
        cfg!(target_os = "macos")
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSDictionary, NSNumber, NSString};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    /// kCGScrollEventUnitPixel
    const UNIT_PIXEL: u32 = 0;
    /// kCGHIDEventTap
    const HID_EVENT_TAP: u32 = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> u8;
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
        fn CGEventCreateScrollWheelEvent(source: *const c_void, units: u32, wheel_count: u32, wheel1: i32, ...) -> *mut c_void;
        fn CGEventPost(tap: u32, event: *mut c_void);
        fn CGWarpMouseCursorPosition(point: CGPoint) -> i32;
        fn CGAssociateMouseAndMouseCursorPosition(connected: u8) -> i32;
        fn CFRelease(object: *const c_void);
    }

    pub fn trusted() -> bool {
        unsafe { AXIsProcessTrusted() != 0 }
    }

    pub fn prompt() {
        // kAXTrustedCheckOptionPrompt's value, toll-free bridged.
        let key = NSString::from_str("AXTrustedCheckOptionPrompt");
        let yes = NSNumber::new_bool(true);
        let options: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::from_slices(&[&*key], &[&**yes]);
        unsafe {
            AXIsProcessTrustedWithOptions(Retained::as_ptr(&options) as *const c_void);
        }
    }

    pub fn warp(x: f64, y: f64) {
        unsafe {
            CGWarpMouseCursorPosition(CGPoint { x, y });
            // Warping briefly detaches the mouse from the pointer; reattach.
            CGAssociateMouseAndMouseCursorPosition(1);
        }
    }

    pub fn scroll(points: u32) {
        unsafe {
            // Negative is down (content moves up).
            let event = CGEventCreateScrollWheelEvent(std::ptr::null(), UNIT_PIXEL, 1, -(points.min(i32::MAX as u32) as i32));
            if !event.is_null() {
                CGEventPost(HID_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }
}

#[cfg(windows)]
mod windows_input {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetCursorPos, WHEEL_DELTA};

    pub fn move_to(x: f64, y: f64) {
        unsafe {
            SetCursorPos(x.round() as i32, y.round() as i32);
        }
    }

    pub fn scroll(notches: u32) {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    // Negative is down.
                    mouseData: (-(WHEEL_DELTA as i32) * notches.min(100) as i32) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod x11 {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt as _;
    use x11rb::protocol::xtest::ConnectionExt as _;
    use x11rb::rust_connection::RustConnection;

    pub struct Conn {
        conn: RustConnection,
        root: u32,
    }

    /// X11 with the XTest extension, or `None` (Wayland, or no XTest).
    pub fn connect() -> Option<Conn> {
        let (conn, screen) = x11rb::connect(None).ok()?;
        conn.xtest_get_version(2, 2).ok()?.reply().ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        Some(Conn { conn, root })
    }

    impl Conn {
        pub fn move_to(&self, x: f64, y: f64) {
            let _ = self.conn.warp_pointer(x11rb::NONE, self.root, 0, 0, 0, 0, x.round() as i16, y.round() as i16);
            let _ = self.conn.flush();
        }

        pub fn scroll(&self, notches: u32) {
            // Button 5 is the wheel turning down.
            const BUTTON_PRESS: u8 = 4;
            const BUTTON_RELEASE: u8 = 5;
            for _ in 0..notches.min(100) {
                let _ = self.conn.xtest_fake_input(BUTTON_PRESS, 5, x11rb::CURRENT_TIME, self.root, 0, 0, 0);
                let _ = self.conn.xtest_fake_input(BUTTON_RELEASE, 5, x11rb::CURRENT_TIME, self.root, 0, 0, 0);
            }
            let _ = self.conn.flush();
        }
    }
}
