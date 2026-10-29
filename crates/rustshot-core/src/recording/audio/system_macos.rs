//! macOS system sound through ScreenCaptureKit (macOS 13 or later).
//!
//! The framework is loaded at run time and used through messages rather
//! than linked, so the app still starts on older macOS, where this source
//! simply isn't offered. The stream captures sound from the whole display
//! (everything the Mac plays) except Rustshot's own; its video is kept as
//! small and slow as possible and ignored.

use std::ptr::{null_mut, NonNull};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use block2::RcBlock;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass};
use objc2_core_audio_types::{kAudioFormatFlagIsFloat, kAudioFormatFlagIsNonInterleaved, AudioBuffer, AudioBufferList};
use objc2_core_foundation::CFRetained;
use objc2_core_media::{CMAudioFormatDescriptionGetStreamBasicDescription, CMBlockBuffer, CMSampleBuffer, CMTime};
use objc2_foundation::NSError;

use super::SourceSender;
use crate::sck::{self, class, wait_for, Owned};

/// `SCStreamOutputType`.
const OUTPUT_SCREEN: isize = 0;
const OUTPUT_AUDIO: isize = 1;
/// `kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment`.
const ASSURE_16_BYTE_ALIGNMENT: u32 = 1 << 0;


/// macOS 13+ and the framework loads.
pub fn supported() -> bool {
    sck::at_least(13) && sck::loaded()
}


pub struct Ivars {
    sender: Mutex<SourceSender>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and this type has no Drop.
    #[unsafe(super(NSObject))]
    #[name = "RustshotSystemAudioOutput"]
    #[ivars = Ivars]
    struct Output;

    impl Output {
        /// `SCStreamOutput`'s only method.
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn did_output(&self, _stream: *mut AnyObject, buffer: *mut CMSampleBuffer, kind: isize) {
            if kind != OUTPUT_AUDIO || buffer.is_null() {
                return;
            }
            if let Some((samples, channels, rate)) = unsafe { read_samples(&*buffer) }
                && let Ok(mut sender) = self.ivars().sender.lock()
            {
                sender.send(&samples, channels, rate);
            }
        }
    }
);

impl Output {
    fn new(sender: SourceSender) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { sender: Mutex::new(sender) });
        unsafe { msg_send![super(this), init] }
    }
}

/// Float samples out of an audio sample buffer, interleaved, with the
/// channel count and rate.
unsafe fn read_samples(buffer: &CMSampleBuffer) -> Option<(Vec<f32>, usize, u32)> {
    unsafe {
        let desc = buffer.format_description()?;
        let asbd = CMAudioFormatDescriptionGetStreamBasicDescription(&desc).as_ref()?;
        if asbd.mFormatFlags & kAudioFormatFlagIsFloat == 0 || asbd.mBitsPerChannel != 32 {
            return None;
        }
        let planar = asbd.mFormatFlags & kAudioFormatFlagIsNonInterleaved != 0;
        let channels = asbd.mChannelsPerFrame.max(1) as usize;

        // Ask how big the buffer list is, then fetch it.
        let mut needed = 0usize;
        buffer.audio_buffer_list_with_retained_block_buffer(&mut needed, null_mut(), 0, None, None, ASSURE_16_BYTE_ALIGNMENT, null_mut());
        if needed < size_of::<AudioBufferList>() {
            return None;
        }
        let mut storage = vec![0u64; needed.div_ceil(8)];
        let list = storage.as_mut_ptr() as *mut AudioBufferList;
        let mut block: *mut CMBlockBuffer = null_mut();
        let status =
            buffer.audio_buffer_list_with_retained_block_buffer(null_mut(), list, needed, None, None, ASSURE_16_BYTE_ALIGNMENT, &mut block);
        // Released when this goes out of scope; it keeps the sample data alive until then.
        let _block = NonNull::new(block).map(|b| CFRetained::from_raw(b));
        if status != 0 {
            return None;
        }
        let buffers: &[AudioBuffer] = std::slice::from_raw_parts((*list).mBuffers.as_ptr(), (*list).mNumberBuffers as usize);
        let plane = |b: &AudioBuffer| -> &[f32] {
            if b.mData.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(b.mData as *const f32, b.mDataByteSize as usize / 4)
            }
        };
        if planar {
            // One buffer per channel: interleave the first two.
            let left = plane(buffers.first()?);
            let right = buffers.get(1).map(plane).unwrap_or(left);
            let n = left.len().min(right.len());
            let mut out = Vec::with_capacity(n * 2);
            for i in 0..n {
                out.push(left[i]);
                out.push(right[i]);
            }
            Some((out, 2, asbd.mSampleRate as u32))
        } else {
            Some((plane(buffers.first()?).to_vec(), channels, asbd.mSampleRate as u32))
        }
    }
}

/// A running system-sound stream; stops when dropped.
pub struct SystemAudio {
    stream: Retained<AnyObject>,
    _output: Retained<Output>,
}

impl SystemAudio {
    pub fn start(sender: SourceSender) -> Result<Self> {
        if !supported() {
            bail!("needs macOS 13 or later");
        }
        unsafe {
            // Any display will do: audio isn't tied to one.
            let display = wait_for(
                |block| {
                    let _: () = msg_send![class(c"SCShareableContent").unwrap(), getShareableContentWithCompletionHandler: &**block];
                },
                |content| {
                    let content = content.as_ref()?;
                    let displays: *mut AnyObject = msg_send![content, displays];
                    let first: *mut AnyObject = msg_send![displays, firstObject];
                    Retained::retain(first).map(|d| Owned(Retained::into_raw(d)))
                },
            )
            .context("listing displays")?;
            let display = Retained::from_raw(display.0).context("no display")?;

            let no_windows: Retained<AnyObject> = msg_send![class(c"NSArray")?, array];
            let filter: Allocated<AnyObject> = msg_send![class(c"SCContentFilter")?, alloc];
            let filter: Retained<AnyObject> = msg_send![filter, initWithDisplay: &*display, excludingWindows: &*no_windows];

            let config: Retained<AnyObject> = msg_send![class(c"SCStreamConfiguration")?, new];
            let _: () = msg_send![&config, setCapturesAudio: true];
            let _: () = msg_send![&config, setExcludesCurrentProcessAudio: true];
            let _: () = msg_send![&config, setSampleRate: super::RATE as isize];
            let _: () = msg_send![&config, setChannelCount: 2isize];
            // The video side is unused: keep it tiny and slow.
            let _: () = msg_send![&config, setWidth: 2usize];
            let _: () = msg_send![&config, setHeight: 2usize];
            let _: () = msg_send![&config, setMinimumFrameInterval: CMTime::new(1, 1)];
            let _: () = msg_send![&config, setQueueDepth: 3isize];

            let stream: Allocated<AnyObject> = msg_send![class(c"SCStream")?, alloc];
            let nil: *mut AnyObject = null_mut();
            let stream: Retained<AnyObject> =
                msg_send![stream, initWithFilter: &*filter, configuration: &*config, delegate: nil];

            let output = Output::new(sender);
            let queue: *mut AnyObject = null_mut();
            for kind in [OUTPUT_SCREEN, OUTPUT_AUDIO] {
                let mut error: *mut NSError = null_mut();
                let ok: bool =
                    msg_send![&stream, addStreamOutput: &*output, type: kind, sampleHandlerQueue: queue, error: &mut error];
                if !ok {
                    let why = error.as_ref().map(|e| e.localizedDescription().to_string()).unwrap_or_default();
                    bail!("adding the stream output failed: {why}");
                }
            }

            wait_for(
                |block| {
                    let block = RcBlock::new({
                        let block = block.clone();
                        move |err: *mut NSError| block.call((null_mut(), err))
                    });
                    let _: () = msg_send![&stream, startCaptureWithCompletionHandler: &*block];
                },
                |_| Some(()),
            )
            .context("starting system sound capture")?;
            log::info!("recording system sound");
            Ok(Self { stream, _output: output })
        }
    }
}

impl Drop for SystemAudio {
    fn drop(&mut self) {
        let (tx, rx) = channel::<()>();
        let tx = Mutex::new(Some(tx));
        let block = RcBlock::new(move |_err: *mut NSError| {
            if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
                let _ = tx.send(());
            }
        });
        unsafe {
            let _: () = msg_send![&self.stream, stopCaptureWithCompletionHandler: &*block];
        }
        let _ = rx.recv_timeout(Duration::from_secs(2));
    }
}
