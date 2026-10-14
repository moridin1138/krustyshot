//! Work around an AppKit crash on macOS.
//!
//! AppKit's Touch Bar support (present even on Macs without a Touch Bar)
//! creates `_NSTouchBarFinderObservation` objects that watch windows and
//! responders with key-value observing. Occasionally one of them tries, in
//! `-invalidate`, to stop observing a key path it is no longer registered
//! for. That raises an `NSRangeException`, which AppKit turns into a crash
//! (`-[NSApplication _crashOnException:]`). Rustshot hit this after a few
//! minutes to an hour of normal use.
//!
//! The fix wraps that one method: the original implementation still runs,
//! but an Objective-C exception it raises is caught and logged instead of
//! terminating the app. Nothing is left registered that would not have been
//! anyway, because the exception means the observer was already gone.

#[cfg(target_os = "macos")]
mod imp {
    use std::panic::AssertUnwindSafe;
    use std::sync::OnceLock;

    use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
    use objc2::sel;

    type Invalidate = unsafe extern "C-unwind" fn(*mut AnyObject, Sel);

    static ORIGINAL: OnceLock<Invalidate> = OnceLock::new();

    unsafe extern "C-unwind" fn guarded_invalidate(this: *mut AnyObject, cmd: Sel) {
        let Some(original) = ORIGINAL.get().copied() else { return };
        let result = objc2::exception::catch(AssertUnwindSafe(|| unsafe { original(this, cmd) }));
        if let Err(exception) = result {
            log::warn!("ignored AppKit Touch Bar exception: {exception:?}");
        }
    }

    /// Install the guard. Safe to call more than once.
    pub fn install() -> bool {
        if ORIGINAL.get().is_some() {
            return true;
        }
        // Make sure AppKit is loaded so the private class can be found.
        let _ = <objc2_app_kit::NSApplication as objc2::ClassType>::class();
        let Some(cls) = AnyClass::get(c"_NSTouchBarFinderObservation") else {
            log::debug!("_NSTouchBarFinderObservation not present; no Touch Bar guard needed");
            return false;
        };
        let Some(method) = cls.instance_method(sel!(invalidate)) else {
            log::debug!("-[_NSTouchBarFinderObservation invalidate] not present");
            return false;
        };
        // SAFETY: `invalidate` takes no arguments and returns void, matching
        // `Invalidate`; the replacement has the same signature and calls the
        // original. The original is stored before the swap so the
        // replacement can always find it.
        unsafe {
            let original: Invalidate = std::mem::transmute::<Imp, Invalidate>(method.implementation());
            let _ = ORIGINAL.set(original);
            let replacement = std::mem::transmute::<Invalidate, Imp>(guarded_invalidate as Invalidate);
            method.set_implementation(replacement);
        }
        log::debug!("installed Touch Bar crash guard");
        true
    }
}

#[cfg(target_os = "macos")]
pub use imp::install;

#[cfg(not(target_os = "macos"))]
pub fn install() -> bool {
    false
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, NSObject};
    use objc2_foundation::{NSSet, NSString};

    /// An observation that believes it watches a key path it never
    /// registered, so `-invalidate` raises the same `NSRangeException`
    /// seen in Rustshot's crash reports.
    fn broken_observation(cls: &AnyClass) -> Retained<AnyObject> {
        let target = NSObject::new();
        let paths = NSSet::from_retained_slice(&[NSString::from_str("description")]);
        let block = block2::RcBlock::new(|| {});
        let obs: Retained<AnyObject> =
            unsafe { msg_send![cls, observationForObject: &*target, keyPaths: &*paths, observer: &*block] };
        let bogus = NSSet::from_retained_slice(&[NSString::from_str("description"), NSString::from_str("bogus")]);
        let ivar = cls.instance_variable(c"_keyPaths").expect("_keyPaths ivar");
        unsafe {
            let slot = ivar.load_ptr::<*mut AnyObject>(&obs);
            *slot = Retained::into_raw(bogus) as *mut AnyObject;
        }
        // Keep the observed object alive for the rest of the test process.
        std::mem::forget(target);
        obs
    }

    #[test]
    fn guard_swallows_touch_bar_exception() {
        let _ = <objc2_app_kit::NSApplication as objc2::ClassType>::class();
        let Some(cls) = AnyClass::get(c"_NSTouchBarFinderObservation") else {
            eprintln!("skipping: this macOS has no _NSTouchBarFinderObservation");
            return;
        };
        // Sanity check: without the guard this really throws.
        let unguarded = broken_observation(cls);
        let raised = objc2::exception::catch(std::panic::AssertUnwindSafe(|| unsafe {
            let _: () = msg_send![&*unguarded, invalidate];
        }));
        assert!(raised.is_err(), "expected the broken observation to raise");

        assert!(super::install());
        let guarded = broken_observation(cls);
        // Called with no handler around it, like AppKit does. Without the
        // guard this would abort the test process.
        unsafe {
            let _: () = msg_send![&*guarded, invalidate];
        }
    }
}
