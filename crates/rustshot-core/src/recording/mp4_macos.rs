//! H.264 MP4 encoding with macOS's own (hardware) encoder via AVAssetWriter.

use std::path::Path;
use std::ptr::NonNull;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVAssetWriter, AVAssetWriterInput, AVAssetWriterInputPixelBufferAdaptor, AVAssetWriterStatus, AVFileTypeMPEG4,
    AVMediaTypeAudio, AVMediaTypeVideo, AVVideoAverageBitRateKey, AVVideoCodecKey, AVVideoCodecTypeH264, AVVideoCompressionPropertiesKey,
    AVVideoExpectedSourceFrameRateKey, AVVideoHeightKey, AVVideoMaxKeyFrameIntervalKey, AVVideoScalingModeKey,
    AVVideoScalingModeResizeAspect, AVVideoWidthKey,
};
use objc2_avf_audio::{AVEncoderBitRateKey, AVFormatIDKey, AVNumberOfChannelsKey, AVSampleRateKey};
use objc2_core_audio_types::{
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatLinearPCM, kAudioFormatMPEG4AAC, AudioStreamBasicDescription,
};
use objc2_core_foundation::{CFRetained, CFString};
use objc2_core_media::{
    kCMBlockBufferAssureMemoryNowFlag, CMAudioFormatDescriptionCreate, CMAudioSampleBufferCreateReadyWithPacketDescriptions,
    CMBlockBuffer, CMFormatDescription, CMSampleBuffer, CMTime,
};
use objc2_core_video::{
    kCVPixelBufferHeightKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelFormatType_32BGRA,
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferPool, CVPixelBufferUnlockBaseAddress,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use super::audio::{AudioChunk, RATE};
use crate::config::RecordingSettings;
use super::{VideoEncoder, VideoFrame};

struct AudioTrack {
    input: Retained<AVAssetWriterInput>,
    /// The format of the PCM handed to the writer: 48 kHz stereo float.
    format: CFRetained<CMFormatDescription>,
}

pub struct Mp4Encoder {
    writer: Retained<AVAssetWriter>,
    input: Retained<AVAssetWriterInput>,
    adaptor: Retained<AVAssetWriterInputPixelBufferAdaptor>,
    pool: Option<Retained<CVPixelBufferPool>>,
    source: (u32, u32),
    audio: Option<AudioTrack>,
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
    /// (the writer scales); `bit_rate` in bits per second. With `audio`,
    /// the file also gets an AAC sound track.
    pub fn create(path: &Path, source: (u32, u32), output: (u32, u32), fps: u32, bit_rate: u64, audio: bool) -> Result<Self> {
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
        let audio = if audio { Some(Self::audio_track(&writer)?) } else { None };
        Ok(Self { writer, input, adaptor, pool: None, source, audio, started: false })
    }

    fn audio_track(writer: &AVAssetWriter) -> Result<AudioTrack> {
        let keys = unsafe {
            [AVFormatIDKey.unwrap(), AVSampleRateKey.unwrap(), AVNumberOfChannelsKey.unwrap(), AVEncoderBitRateKey.unwrap()]
        };
        let (format_id, rate, channels, bit_rate) = (
            NSNumber::new_u32(kAudioFormatMPEG4AAC),
            NSNumber::new_u32(RATE),
            NSNumber::new_u32(2),
            NSNumber::new_u64(RecordingSettings::AUDIO_BIT_RATE),
        );
        let settings = dict(&keys, &[&*format_id, &*rate, &*channels, &*bit_rate]);
        let input = unsafe { AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(AVMediaTypeAudio.unwrap(), Some(&settings)) };
        unsafe {
            input.setExpectsMediaDataInRealTime(true);
            if !writer.canAddInput(&input) {
                bail!("the video writer rejected the sound settings");
            }
            writer.addInput(&input);
        }
        let asbd = AudioStreamBasicDescription {
            mSampleRate: RATE as f64,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 8,
            mFramesPerPacket: 1,
            mBytesPerFrame: 8,
            mChannelsPerFrame: 2,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        let mut format: *const CMFormatDescription = std::ptr::null();
        let status = unsafe {
            CMAudioFormatDescriptionCreate(None, NonNull::from(&asbd), 0, std::ptr::null(), 0, std::ptr::null(), None, NonNull::from(&mut format))
        };
        let format = NonNull::new(format.cast_mut()).filter(|_| status == 0).context("can't describe the sound format")?;
        Ok(AudioTrack { input, format: unsafe { CFRetained::from_raw(format) } })
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

    fn push_audio(&mut self, chunk: &AudioChunk) -> Result<()> {
        if !self.started {
            self.start()?;
        }
        let Some(track) = &self.audio else { return Ok(()) };
        if chunk.frames.is_empty() || !unsafe { track.input.isReadyForMoreMediaData() } {
            return Ok(());
        }
        let len = chunk.frames.len() * 8;
        unsafe {
            let mut block: *mut CMBlockBuffer = std::ptr::null_mut();
            let r = CMBlockBuffer::create_with_memory_block(
                None,
                std::ptr::null_mut(),
                len,
                None,
                std::ptr::null(),
                0,
                len,
                kCMBlockBufferAssureMemoryNowFlag,
                NonNull::from(&mut block),
            );
            let Some(block) = NonNull::new(block).filter(|_| r == 0) else { bail!("no sound buffer ({r})") };
            let block = CFRetained::from_raw(block);
            let r = CMBlockBuffer::replace_data_bytes(NonNull::from(&chunk.frames[0]).cast(), &block, 0, len);
            if r != 0 {
                bail!("can't fill the sound buffer ({r})");
            }
            let mut sample: *mut CMSampleBuffer = std::ptr::null_mut();
            let r = CMAudioSampleBufferCreateReadyWithPacketDescriptions(
                None,
                &block,
                &track.format,
                chunk.frames.len() as isize,
                CMTime::new(chunk.start as i64, RATE as i32),
                std::ptr::null(),
                NonNull::from(&mut sample),
            );
            let Some(sample) = NonNull::new(sample).filter(|_| r == 0) else { bail!("no sound sample ({r})") };
            let sample = CFRetained::from_raw(sample);
            if !track.input.appendSampleBuffer(&sample) {
                bail!("the sound encoder stopped: {:?}", self.writer.error().map(|e| e.localizedDescription()));
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
            if let Some(track) = &self.audio {
                track.input.markAsFinished();
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two seconds of a moving bar and a 440 Hz tone: the file must have
    /// both tracks, each about two seconds long.
    #[test]
    fn writes_video_and_sound() {
        let path = std::env::temp_dir().join(format!("rustshot-mp4-audio-{}.mp4", std::process::id()));
        let (w, h, fps) = (320u32, 240u32, 30u32);
        let mut enc: Box<dyn VideoEncoder> = Box::new(Mp4Encoder::create(&path, (w, h), (w, h), fps, 1_000_000, true).unwrap());
        let chunk = RATE as usize / fps as usize;
        for i in 0..fps * 2 {
            let rgba = (0..w * h).flat_map(|p| if p % w == i * 5 { [255, 0, 0, 255] } else { [20, 20, 20, 255] }).collect();
            let pts = Duration::from_secs_f64(i as f64 / fps as f64);
            enc.push(&VideoFrame { width: w, height: h, rgba, pts }).unwrap();
            let start = i as u64 * chunk as u64;
            let frames = (0..chunk)
                .map(|j| {
                    let t = (start + j as u64) as f32 / RATE as f32;
                    let v = (t * 440.0 * std::f32::consts::TAU).sin() * 0.3;
                    [v, v]
                })
                .collect();
            enc.push_audio(&AudioChunk { start, frames }).unwrap();
            // Real time, so the writer's inputs are ready.
            std::thread::sleep(Duration::from_millis(5));
        }
        enc.finish().unwrap();

        let asset = unsafe {
            objc2_av_foundation::AVURLAsset::URLAssetWithURL_options(
                &NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy())),
                None,
            )
        };
        #[allow(deprecated)]
        let (video, sound) = unsafe {
            (asset.tracksWithMediaType(AVMediaTypeVideo.unwrap()).count(), asset.tracksWithMediaType(AVMediaTypeAudio.unwrap()).count())
        };
        let _ = std::fs::remove_file(&path);
        assert_eq!((video, sound), (1, 1));
    }
}
