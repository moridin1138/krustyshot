//! H.264 MP4 encoding with Windows' own encoder (Media Foundation Sink Writer).
//!
//! Media Foundation is set up lazily on the encoder thread, because COM
//! objects belong to the thread (apartment) that creates them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::{imageops, RgbaImage};
use windows::core::HSTRING;
use windows::Win32::Media::MediaFoundation::{
    IMFAttributes, IMFMediaType, IMFSinkWriter, MFAudioFormat_AAC, MFAudioFormat_PCM, MFCreateAttributes, MFCreateMediaType,
    MFCreateMemoryBuffer, MFCreateSample, MFCreateSinkWriterFromURL, MFMediaType_Audio, MFMediaType_Video, MFShutdown, MFStartup,
    MFVideoFormat_H264, MFVideoFormat_RGB32, MFVideoInterlace_Progressive, MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
    MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND,
    MF_MT_AVG_BITRATE, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,
    MF_VERSION, MFSTARTUP_LITE,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

use super::audio::{AudioChunk, RATE};
use crate::config::RecordingSettings;
use super::{VideoEncoder, VideoFrame};

/// 100-nanosecond units per second, Media Foundation's time base.
const HNS_PER_SECOND: i64 = 10_000_000;
/// AAC bit rate in bytes per second (the encoder accepts only a few values).
const AAC_BYTES_PER_SECOND: u32 = (RecordingSettings::AUDIO_BIT_RATE / 8) as u32;

struct Writer {
    sink: IMFSinkWriter,
    stream: u32,
    audio_stream: Option<u32>,
}

pub struct Mp4Encoder {
    path: PathBuf,
    output: (u32, u32),
    fps: u32,
    bit_rate: u64,
    audio: bool,
    writer: Option<Writer>,
    /// Media Foundation and COM were started on this thread and need shutting down.
    started: bool,
    frames: u64,
}

// Only used from the encoder thread (set up lazily there).
unsafe impl Send for Mp4Encoder {}

fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

impl Mp4Encoder {
    /// `output` is the video size (frames are scaled to it); `bit_rate` in
    /// bits per second. With `audio`, the file also gets an AAC sound track.
    pub fn create(path: &Path, output: (u32, u32), fps: u32, bit_rate: u64, audio: bool) -> Result<Self> {
        let _ = std::fs::remove_file(path);
        Ok(Self { path: path.to_path_buf(), output, fps, bit_rate, audio, writer: None, started: false, frames: 0 })
    }

    /// 48 kHz stereo 16-bit: PCM in, or AAC out (with its bit rate).
    fn audio_type(subtype: &windows::core::GUID, bytes_per_second: u32) -> Result<IMFMediaType> {
        unsafe {
            let t = MFCreateMediaType()?;
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            t.SetGUID(&MF_MT_SUBTYPE, subtype)?;
            t.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
            t.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RATE)?;
            t.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
            t.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, bytes_per_second)?;
            if *subtype == MFAudioFormat_PCM {
                t.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4)?;
            }
            Ok(t)
        }
    }

    fn video_type(subtype: &windows::core::GUID, (w, h): (u32, u32), fps: u32) -> Result<IMFMediaType> {
        unsafe {
            let t = MFCreateMediaType()?;
            t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            t.SetGUID(&MF_MT_SUBTYPE, subtype)?;
            t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            t.SetUINT64(&MF_MT_FRAME_SIZE, pack(w, h))?;
            t.SetUINT64(&MF_MT_FRAME_RATE, pack(fps, 1))?;
            t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
            Ok(t)
        }
    }

    fn start(&mut self) -> Result<()> {
        unsafe {
            // S_FALSE (already initialised) is fine too.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            MFStartup(MF_VERSION, MFSTARTUP_LITE).context("starting Media Foundation")?;
            self.started = true;

            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 1)?;
            let attrs = attrs.context("no attributes")?;
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;

            let url = HSTRING::from(self.path.as_os_str());
            let sink = MFCreateSinkWriterFromURL(&url, None, &attrs).context("creating the video file")?;

            let out = Self::video_type(&MFVideoFormat_H264, self.output, self.fps)?;
            out.SetUINT32(&MF_MT_AVG_BITRATE, self.bit_rate.min(u32::MAX as u64) as u32)?;
            let stream = sink.AddStream(&out).context("the H.264 encoder rejected the settings")?;

            let input = Self::video_type(&MFVideoFormat_RGB32, self.output, self.fps)?;
            // A positive stride means rows run top to bottom (RGB32 defaults to bottom-up).
            input.SetUINT32(&MF_MT_DEFAULT_STRIDE, self.output.0 * 4)?;
            sink.SetInputMediaType(stream, &input, None).context("setting the encoder's input format")?;

            let audio_stream = if self.audio {
                let out = Self::audio_type(&MFAudioFormat_AAC, AAC_BYTES_PER_SECOND)?;
                let s = sink.AddStream(&out).context("the AAC encoder rejected the settings")?;
                let input = Self::audio_type(&MFAudioFormat_PCM, RATE * 4)?;
                sink.SetInputMediaType(s, &input, None).context("setting the sound encoder's input format")?;
                Some(s)
            } else {
                None
            };
            sink.BeginWriting().context("starting to write the video")?;
            self.writer = Some(Writer { sink, stream, audio_stream });
        }
        Ok(())
    }
}

impl VideoEncoder for Mp4Encoder {
    fn push(&mut self, frame: &VideoFrame) -> Result<()> {
        if self.writer.is_none() {
            self.start()?;
        }
        let (w, h) = self.output;
        // Scale to the output size if needed (Media Foundation's sink writer doesn't).
        let scaled;
        let rgba: &[u8] = if (frame.width, frame.height) == (w, h) {
            &frame.rgba
        } else {
            let Some(full) = RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone()) else { return Ok(()) };
            scaled = imageops::thumbnail(&full, w, h);
            scaled.as_raw()
        };
        let Writer { sink, stream, .. } = self.writer.as_ref().unwrap();
        let len = w * h * 4;
        unsafe {
            let buffer = MFCreateMemoryBuffer(len)?;
            let mut data: *mut u8 = std::ptr::null_mut();
            buffer.Lock(&mut data, None, None)?;
            let dst = std::slice::from_raw_parts_mut(data, len as usize);
            // RGBA → BGRX.
            for (s, d) in rgba.as_chunks::<4>().0.iter().zip(dst.as_chunks_mut::<4>().0.iter_mut()) {
                *d = [s[2], s[1], s[0], 255];
            }
            buffer.Unlock()?;
            buffer.SetCurrentLength(len)?;

            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(frame.pts.as_nanos() as i64 / 100)?;
            sample.SetSampleDuration(HNS_PER_SECOND / self.fps as i64)?;
            sink.WriteSample(*stream, &sample).context("the encoder stopped")?;
        }
        self.frames += 1;
        Ok(())
    }

    fn push_audio(&mut self, chunk: &AudioChunk) -> Result<()> {
        if self.writer.is_none() {
            self.start()?;
        }
        let Some(Writer { sink, audio_stream: Some(stream), .. }) = self.writer.as_ref() else { return Ok(()) };
        if chunk.frames.is_empty() {
            return Ok(());
        }
        let len = (chunk.frames.len() * 4) as u32;
        unsafe {
            let buffer = MFCreateMemoryBuffer(len)?;
            let mut data: *mut u8 = std::ptr::null_mut();
            buffer.Lock(&mut data, None, None)?;
            let dst = std::slice::from_raw_parts_mut(data as *mut i16, chunk.frames.len() * 2);
            for (f, d) in chunk.frames.iter().zip(dst.as_chunks_mut::<2>().0.iter_mut()) {
                *d = [(f[0] * i16::MAX as f32) as i16, (f[1] * i16::MAX as f32) as i16];
            }
            buffer.Unlock()?;
            buffer.SetCurrentLength(len)?;

            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            let to_hns = |frames: u64| (frames as i128 * HNS_PER_SECOND as i128 / RATE as i128) as i64;
            sample.SetSampleTime(to_hns(chunk.start))?;
            sample.SetSampleDuration(to_hns(chunk.frames.len() as u64))?;
            sink.WriteSample(*stream, &sample).context("the sound encoder stopped")?;
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<()> {
        let result = match self.writer.take() {
            // Media Foundation can't finish a file with no frames; leave none.
            None => Ok(()),
            Some(Writer { sink, .. }) => unsafe { sink.Finalize().context("finishing the video") },
        };
        if self.started {
            unsafe {
                let _ = MFShutdown();
                CoUninitialize();
            }
        }
        result
    }
}
