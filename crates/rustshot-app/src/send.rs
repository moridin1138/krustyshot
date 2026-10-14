//! Printing, email and the Share menu. On macOS these are AppKit panels,
//! which must run on the main thread; elsewhere they hand the file to the
//! OS (`rustshot_core::share`).

use std::path::Path;

use anyhow::Result;

/// Show the print panel for the image at `path` (macOS), or print it the
/// platform's way.
pub fn print(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::print(path);
    #[cfg(not(target_os = "macos"))]
    rustshot_core::share::print_file(path)
}

/// A new message in the default mail app with `path` attached.
pub fn email(path: &Path, subject: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::email(path, subject);
    #[cfg(not(target_os = "macos"))]
    rustshot_core::share::email_file(path, subject)
}

/// The system Share menu for `path`, next to the mouse pointer in the
/// active window (macOS only).
pub fn share(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::share(path);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        anyhow::bail!("sharing isn't available on this platform")
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::Path;

    use anyhow::{anyhow, bail, Context, Result};
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::msg_send;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSRect, NSSize, NSString, NSURL};

    fn class(name: &std::ffi::CStr) -> Result<&'static AnyClass> {
        AnyClass::get(name).ok_or_else(|| anyhow!("{} is missing", name.to_string_lossy()))
    }

    fn file_url(path: &Path) -> Retained<NSURL> {
        NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
    }

    fn bring_forward() {
        crate::platform::activate_app();
    }

    pub fn print(path: &Path) -> Result<()> {
        MainThreadMarker::new().context("printing must start on the main thread")?;
        bring_forward();
        unsafe {
            let image: Allocated<AnyObject> = msg_send![class(c"NSImage")?, alloc];
            let image: Option<Retained<AnyObject>> = msg_send![image, initWithContentsOfURL: &*file_url(path)];
            let image = image.context("couldn't read the image")?;
            // One point per pixel; fit-to-page scales it down to the paper.
            let rep: *mut AnyObject = msg_send![&image, bestRepresentationForRect: std::ptr::null::<NSRect>(), context: std::ptr::null::<AnyObject>(), hints: std::ptr::null::<AnyObject>()];
            let size = match rep.as_ref() {
                Some(rep) => {
                    let (w, h): (isize, isize) = (msg_send![rep, pixelsWide], msg_send![rep, pixelsHigh]);
                    NSSize::new(w as f64, h as f64)
                }
                None => msg_send![&image, size],
            };
            let _: () = msg_send![&image, setSize: size];

            let view: Allocated<AnyObject> = msg_send![class(c"NSImageView")?, alloc];
            let view: Retained<AnyObject> = msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), size)];
            let _: () = msg_send![&view, setImage: &*image];
            // NSImageScaleProportionallyUpOrDown
            let _: () = msg_send![&view, setImageScaling: 3usize];

            let shared: Retained<AnyObject> = msg_send![class(c"NSPrintInfo")?, sharedPrintInfo];
            let info: Retained<AnyObject> = msg_send![&shared, copy];
            // NSPrintingPaginationModeFit
            let _: () = msg_send![&info, setHorizontalPagination: 1isize];
            let _: () = msg_send![&info, setVerticalPagination: 1isize];
            let _: () = msg_send![&info, setHorizontallyCentered: true];
            let _: () = msg_send![&info, setVerticallyCentered: true];
            if size.width > size.height {
                // NSPaperOrientationLandscape
                let _: () = msg_send![&info, setOrientation: 1isize];
            }

            let op: Retained<AnyObject> = msg_send![class(c"NSPrintOperation")?, printOperationWithView: &*view, printInfo: &*info];
            let _: () = msg_send![&op, setShowsPrintPanel: true];
            let _: () = msg_send![&op, setShowsProgressPanel: true];
            let _: bool = msg_send![&op, runOperation];
        }
        Ok(())
    }

    pub fn email(path: &Path, subject: &str) -> Result<()> {
        MainThreadMarker::new().context("email must start on the main thread")?;
        unsafe {
            // NSSharingServiceNameComposeEmail
            let name = NSString::from_str("com.apple.share.Mail.compose");
            let service: Option<Retained<AnyObject>> = msg_send![class(c"NSSharingService")?, sharingServiceNamed: &*name];
            let service = service.context("no mail app is set up")?;
            let _: () = msg_send![&service, setSubject: &*NSString::from_str(subject)];
            let items = NSArray::from_retained_slice(&[Retained::into_super(Retained::into_super(file_url(path)))]);
            let ok: bool = msg_send![&service, canPerformWithItems: &*items];
            if !ok {
                bail!("the mail app can't attach the image");
            }
            let _: () = msg_send![&service, performWithItems: &*items];
        }
        Ok(())
    }

    pub fn share(path: &Path) -> Result<()> {
        let mtm = MainThreadMarker::new().context("sharing must start on the main thread")?;
        let app = NSApplication::sharedApplication(mtm);
        let window = app.keyWindow().or_else(|| app.mainWindow()).context("no window to show the Share menu in")?;
        let view = window.contentView().context("no window content")?;
        unsafe {
            let items = NSArray::from_retained_slice(&[Retained::into_super(Retained::into_super(file_url(path)))]);
            let picker: Allocated<AnyObject> = msg_send![class(c"NSSharingServicePicker")?, alloc];
            let picker: Retained<AnyObject> = msg_send![picker, initWithItems: &*items];
            // Next to the pointer, which is on the button just clicked.
            let at = window.mouseLocationOutsideOfEventStream();
            let at = view.convertPoint_fromView(at, None);
            let rect = NSRect::new(NSPoint::new(at.x - 2.0, at.y - 2.0), NSSize::new(4.0, 4.0));
            // NSRectEdgeMinY: below the point.
            let _: () = msg_send![&picker, showRelativeToRect: rect, ofView: &*view, preferredEdge: 1usize];
        }
        Ok(())
    }
}
