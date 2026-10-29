//! ScreenCaptureKit, loaded at run time and driven by messages, so the app
//! still starts on macOS versions that lack it (the framework arrived in
//! 12.3; screenshots without the pointer need 14, system sound 13).

use std::ffi::{c_char, c_int, c_void};
use std::ptr::NonNull;
use std::sync::mpsc::channel;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use block2::RcBlock;
use image::RgbaImage;
use objc2::msg_send;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGDataProvider, CGImage, CGImageAlphaInfo, CGImageByteOrderInfo};
use objc2_foundation::{NSError, NSOperatingSystemVersion, NSProcessInfo};

const FRAMEWORK: &std::ffi::CStr = c"/System/Library/Frameworks/ScreenCaptureKit.framework/ScreenCaptureKit";

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn CFRetain(object: *const c_void) -> *const c_void;
}
const RTLD_LAZY: c_int = 1;

/// The framework loads.
pub fn loaded() -> bool {
    static LOADED: OnceLock<bool> = OnceLock::new();
    *LOADED.get_or_init(|| unsafe { !dlopen(FRAMEWORK.as_ptr(), RTLD_LAZY).is_null() })
}

/// macOS `major` or later.
pub fn at_least(major: isize) -> bool {
    NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(NSOperatingSystemVersion { majorVersion: major, minorVersion: 0, patchVersion: 0 })
}

pub fn class(name: &std::ffi::CStr) -> Result<&'static AnyClass> {
    AnyClass::get(name).ok_or_else(|| anyhow!("{} is missing", name.to_string_lossy()))
}

/// An object pointer (already retained) handed back from a completion block.
pub struct Owned(pub *mut AnyObject);
// The object is only used again on the thread that waits for it.
unsafe impl Send for Owned {}

/// Run an asynchronous ScreenCaptureKit call that reports `(result, error)`
/// to a completion block, and wait for it.
pub fn wait_for<R: Send + 'static>(call: impl FnOnce(&RcBlock<dyn Fn(*mut AnyObject, *mut NSError)>), map: fn(*mut AnyObject) -> Option<R>) -> Result<R> {
    let (tx, rx) = channel::<Result<R, String>>();
    let tx = Mutex::new(Some(tx));
    let block = RcBlock::new(move |obj: *mut AnyObject, err: *mut NSError| {
        let outcome = match unsafe { err.as_ref() } {
            Some(e) => Err(e.localizedDescription().to_string()),
            None => map(obj).ok_or_else(|| "no result".to_string()),
        };
        if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
            let _ = tx.send(outcome);
        }
    });
    call(&block);
    rx.recv_timeout(Duration::from_secs(5)).context("ScreenCaptureKit didn't answer")?.map_err(|e| anyhow!(e))
}

/// Screenshots that leave out the mouse pointer (macOS 14+). The older
/// capture call includes it on recent macOS, whatever the app asks.
pub fn screenshots_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| at_least(14) && loaded() && AnyClass::get(c"SCScreenshotManager").is_some())
}

/// A screenshot of the display `display_id` (a CGDirectDisplayID) at
/// `width`×`height` pixels, without the pointer.
pub fn screenshot(display_id: u32, width: u32, height: u32) -> Result<RgbaImage> {
    if !screenshots_supported() {
        bail!("needs macOS 14 or later");
    }
    unsafe {
        let content = wait_for(
            |block| {
                let _: () = msg_send![class(c"SCShareableContent").unwrap(), getShareableContentWithCompletionHandler: &**block];
            },
            |content| Retained::retain(content).map(|c| Owned(Retained::into_raw(c))),
        )
        .context("listing displays")?;
        let content = Retained::from_raw(content.0).context("no displays")?;
        let displays: Retained<AnyObject> = msg_send![&content, displays];
        let count: usize = msg_send![&displays, count];
        let display = (0..count)
            .map(|i| -> Retained<AnyObject> { msg_send![&displays, objectAtIndex: i] })
            .find(|d| {
                let id: u32 = msg_send![&**d, displayID];
                id == display_id
            })
            .context("that display isn't available to capture")?;

        let no_windows: Retained<AnyObject> = msg_send![class(c"NSArray")?, array];
        let filter: Allocated<AnyObject> = msg_send![class(c"SCContentFilter")?, alloc];
        let filter: Retained<AnyObject> = msg_send![filter, initWithDisplay: &*display, excludingWindows: &*no_windows];
        let config: Retained<AnyObject> = msg_send![class(c"SCStreamConfiguration")?, new];
        let _: () = msg_send![&config, setWidth: width as usize];
        let _: () = msg_send![&config, setHeight: height as usize];
        let _: () = msg_send![&config, setShowsCursor: false];

        let image = wait_for(
            |block| {
                let _: () = msg_send![
                    class(c"SCScreenshotManager").unwrap(),
                    captureImageWithFilter: &*filter,
                    configuration: &*config,
                    completionHandler: &**block
                ];
            },
            // A CGImage, only valid during the block: keep it.
            |image| (!image.is_null()).then(|| Owned(CFRetain(image as *const c_void) as *mut AnyObject)),
        )
        .context("taking the screenshot")?;
        let image: CFRetained<CGImage> = CFRetained::from_raw(NonNull::new(image.0 as *mut CGImage).context("no image")?);
        to_rgba(&image)
    }
}

/// A 32-bit CGImage's pixels as RGBA.
fn to_rgba(image: &CGImage) -> Result<RgbaImage> {
    let (w, h) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    if CGImage::bits_per_pixel(Some(image)) != 32 {
        bail!("unexpected screenshot format");
    }
    let stride = CGImage::bytes_per_row(Some(image));
    let provider = CGImage::data_provider(Some(image)).context("no image data")?;
    let data = CGDataProvider::data(Some(&provider)).context("no image data")?.to_vec();
    // Screenshots are BGRA (32-bit little-endian, alpha first) in practice;
    // handle RGBA too.
    let bgra = CGImage::byte_order_info(Some(image)) == CGImageByteOrderInfo::Order32Little;
    let alpha_first = matches!(
        CGImage::alpha_info(Some(image)),
        CGImageAlphaInfo::PremultipliedFirst | CGImageAlphaInfo::First | CGImageAlphaInfo::NoneSkipFirst
    );
    let mut out = Vec::with_capacity(w * h * 4);
    for row in data.chunks(stride).take(h) {
        for p in row[..w * 4].as_chunks::<4>().0 {
            let (r, g, b) = match (bgra, alpha_first) {
                // Little-endian ARGB is stored B, G, R, A.
                (true, true) => (p[2], p[1], p[0]),
                (true, false) => (p[3], p[2], p[1]),
                (false, true) => (p[1], p[2], p[3]),
                (false, false) => (p[0], p[1], p[2]),
            };
            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
    RgbaImage::from_raw(w as u32, h as u32, out).context("screenshot size")
}
