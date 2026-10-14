//! H.264 MP4 encoding with macOS's own (hardware) encoder via AVAssetWriter.

use std::path::Path;
use std::ptr::NonNull;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVAssetWriter, AVAssetWriterInput, AVAssetWriterInputPixelBufferAdaptor, AVAssetWriterStatus, AVFileTypeMPEG4,
    AVMediaTypeVideo, AVVideoAverageBitRateKey, AVVideoCodecKey, AVVideoCodecTypeH264, AVVideoCompressionPropertiesKey,
    AVVideoExpectedSourceFrameRateKey, AVVideoHeightKey, AVVideoMaxKeyFrameIntervalKey, AVVideoScalingModeKey,
    AVVideoScalingModeResizeAspect, AVVideoWidthKey,
};
use objc2_core_foundation::{CFRetained, CFString};
use objc2_core_media::CMTime;
use objc2_core_video::{
    kCVPixelBufferHeightKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelFormatType_32BGRA,
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferPool, CVPixelBufferUnlockBaseAddress,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use super::{VideoEncoder, VideoFrame};

pub struct Mp4Encoder {
    writer: Retained<AVAssetWriter>,
    input: Retained<AVAssetWriterInput>,
    adaptor: Retained<AVAssetWriterInputPixelBufferAdaptor>,
    pool: Option<Retained<CVPixelBufferPool>>,
    source: (u32, u32),
    started: bool,
}

// The AVFoundation objects are only used from the encoder thread.
unsafe impl Send for Mp4Encoder {}

fn ns_key(k: &CFString) -> &NSString {
    // CFString and NSString are toll-free bridged.
    unsafe { &*(k as *const CFString as *const NSString) }
}

fn dict(keys: &[&NSString], values: &[&AnyObject]) -> Retained<NSDictionary<NSString, AnyObject>> {
    NSDictionary::from_slices(keys, values)
}

impl Mp4Encoder {
    /// `source` is the size of the frames pushed; `output` the video size
    /// (the writer scales); `bit_rate` in bits per second.
    pub fn create(path: &Path, source: (u32, u32), output: (u32, u32), fps: u32, bit_rate: u64) -> Result<Self> {
        let _ = std::fs::remove_file(path);
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let writer = unsafe { AVAssetWriter::assetWriterWithURL_fileType_error(&url, AVFileTypeMPEG4.unwrap()) }
            .map_err(|e| anyhow!("can't create the video file: {}", e.localizedDescription()))?;

        let (rate, fps_n, keyframes) = (NSNumber::new_u64(bit_rate), NSNumber::new_u32(fps), NSNumber::new_u32(fps * 2));
        let compression_keys = unsafe {
            [AVVideoAverageBitRateKey.unwrap(), AVVideoExpectedSourceFrameRateKey.unwrap(), AVVideoMaxKeyFrameIntervalKey.unwrap()]
        };
        let compression = dict(&compression_keys, &[&*rate, &*fps_n, &*keyframes]);
        let width = NSNumber::new_u32(output.0);
        let height = NSNumber::new_u32(output.1);
        let setting_keys = unsafe {
            [
                AVVideoCodecKey.unwrap(),
                AVVideoWidthKey.unwrap(),
                AVVideoHeightKey.unwrap(),
                AVVideoScalingModeKey.unwrap(),
                AVVideoCompressionPropertiesKey.unwrap(),
            ]
        };
        let (codec, scaling): (&AnyObject, &AnyObject) =
            unsafe { (AVVideoCodecTypeH264.unwrap(), AVVideoScalingModeResizeAspect.unwrap()) };
        let settings = dict(&setting_keys, &[codec, &*width, &*height, scaling, &*compression]);
        let input = unsafe { AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(AVMediaTypeVideo.unwrap(), Some(&settings)) };
        unsafe { input.setExpectsMediaDataInRealTime(true) };

        let attr_keys = unsafe {
            [ns_key(kCVPixelBufferPixelFormatTypeKey), ns_key(kCVPixelBufferWidthKey), ns_key(kCVPixelBufferHeightKey)]
        };
        let (format, src_w, src_h) =
            (NSNumber::new_u32(kCVPixelFormatType_32BGRA), NSNumber::new_u32(source.0), NSNumber::new_u32(source.1));
        let attrs = dict(&attr_keys, &[&*format, &*src_w, &*src_h]);
        let adaptor = unsafe {
            AVAssetWriterInputPixelBufferAdaptor::assetWriterInputPixelBufferAdaptorWithAssetWriterInput_sourcePixelBufferAttributes(
                &input,
                Some(&attrs),
            )
        };
        unsafe {
            if !writer.canAddInput(&input) {
                bail!("the video writer rejected the settings");
            }
            writer.addInput(&input);
        }
        Ok(Self { writer, input, adaptor, pool: None, source, started: false })
    }

    fn start(&mut self) -> Result<()> {
        unsafe {
            if !self.writer.startWriting() {
                bail!("can't start writing the video: {:?}", self.writer.error().map(|e| e.localizedDescription()));
            }
            self.writer.startSessionAtSourceTime(CMTime::new(0, 1000));
        }
        self.pool = Some(unsafe { self.adaptor.pixelBufferPool() }.context("no pixel buffer pool")?);
        self.started = true;
        Ok(())
    }
}

impl VideoEncoder for Mp4Encoder {
    fn push(&mut self, frame: &VideoFrame) -> Result<()> {
        if !self.started {
            self.start()?;
        }
        if (frame.width, frame.height) != self.source {
            return Ok(());
        }
        // In real time, a busy encoder means skip this frame.
        if !unsafe { self.input.isReadyForMoreMediaData() } {
            return Ok(());
        }
        let pool = self.pool.as_ref().unwrap();
        unsafe {
            let mut raw: *mut CVPixelBuffer = std::ptr::null_mut();
            let r = CVPixelBufferPool::create_pixel_buffer(None, pool, NonNull::from(&mut raw));
            let Some(ptr) = NonNull::new(raw).filter(|_| r == 0) else { bail!("no pixel buffer ({r})") };
            let buf = CFRetained::from_raw(ptr);
            CVPixelBufferLockBaseAddress(&buf, CVPixelBufferLockFlags(0));
            let base = CVPixelBufferGetBaseAddress(&buf) as *mut u8;
            let stride = CVPixelBufferGetBytesPerRow(&buf);
            let row = frame.width as usize * 4;
            for y in 0..frame.height as usize {
                let src = &frame.rgba[y * row..][..row];
                let dst = std::slice::from_raw_parts_mut(base.add(y * stride), row);
                // RGBA → BGRA.
                for (s, d) in src.as_chunks::<4>().0.iter().zip(dst.as_chunks_mut::<4>().0.iter_mut()) {
                    *d = [s[2], s[1], s[0], 255];
                }
            }
            CVPixelBufferUnlockBaseAddress(&buf, CVPixelBufferLockFlags(0));
            let pts = CMTime::new(frame.pts.as_millis() as i64, 1000);
            if !self.adaptor.appendPixelBuffer_withPresentationTime(&buf, pts) {
                bail!("the encoder stopped: {:?}", self.writer.error().map(|e| e.localizedDescription()));
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<()> {
        if !self.started {
            // No frames at all: still produce a valid (empty) file.
            self.start()?;
        }
        let (tx, done) = std::sync::mpsc::channel();
        let block = block2::RcBlock::new(move || {
            let _ = tx.send(());
        });
        unsafe {
            self.input.markAsFinished();
            self.writer.finishWritingWithCompletionHandler(&block);
        }
        done.recv_timeout(Duration::from_secs(30)).context("finishing the video timed out")?;
        let status = unsafe { self.writer.status() };
        if status != AVAssetWriterStatus::Completed {
            bail!("the video wasn't finished ({status:?}): {:?}", unsafe { self.writer.error() }.map(|e| e.localizedDescription()));
        }
        Ok(())
    }
}
