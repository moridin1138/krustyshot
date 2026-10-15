//! Platform-independent audio plumbing: converting sources to one format,
//! timestamping them against the recording clock, and mixing them.

use std::collections::VecDeque;
use std::time::Duration;

/// Everything is mixed and encoded as 48 kHz stereo float.
pub const RATE: u32 = 48_000;

/// A block of mixed audio on the recording's timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioChunk {
    /// Index of the first frame, counting from the start of the recording at [`RATE`].
    pub start: u64,
    /// Interleaved stereo frames.
    pub frames: Vec<[f32; 2]>,
}

impl AudioChunk {
    pub fn pts(&self) -> Duration {
        Duration::from_secs_f64(self.start as f64 / RATE as f64)
    }
}

/// Audio from one source, already converted to [`RATE`] stereo.
#[derive(Debug)]
pub struct SourceChunk {
    pub start: u64,
    pub frames: Vec<[f32; 2]>,
}

/// Convert interleaved samples with any channel count and rate to [`RATE`]
/// stereo. Mono is copied to both sides; extra channels are dropped.
/// Linear interpolation is plenty for voice and system sound.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
pub fn to_stereo_48k(samples: &[f32], channels: usize, rate: u32) -> Vec<[f32; 2]> {
    let channels = channels.max(1);
    let frames: Vec<[f32; 2]> = samples
        .chunks_exact(channels)
        .map(|f| if channels == 1 { [f[0], f[0]] } else { [f[0], f[1]] })
        .collect();
    if rate == RATE || frames.is_empty() {
        return frames;
    }
    let out_len = (frames.len() as u64 * RATE as u64 / rate as u64) as usize;
    let step = rate as f64 / RATE as f64;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * step;
            let j = pos as usize;
            let t = (pos - j as f64) as f32;
            let a = frames[j.min(frames.len() - 1)];
            let b = frames[(j + 1).min(frames.len() - 1)];
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
        })
        .collect()
}

/// Places one source's chunks on the recording timeline. Counts samples from
/// the first chunk (smooth and drift-free over minutes), but jumps forward if
/// the source went quiet and came back, which loopback capture does when
/// nothing is playing.
#[derive(Default)]
pub struct SourceClock {
    /// Timeline position (in [`RATE`] frames) of the next chunk.
    next: Option<u64>,
}

impl SourceClock {
    /// `now` is time since the recording started, when the chunk of
    /// `frames` (at [`RATE`]) arrived. Returns the chunk's start position.
    pub fn place(&mut self, now: Duration, frames: usize) -> u64 {
        let arrived_start = (now.as_secs_f64() * RATE as f64) as u64;
        let arrived_start = arrived_start.saturating_sub(frames as u64);
        let start = match self.next {
            // More than 100 ms behind real time: a gap; resynchronise.
            Some(next) if arrived_start > next + RATE as u64 / 10 => arrived_start,
            Some(next) => next,
            None => arrived_start,
        };
        self.next = Some(start + frames as u64);
        start
    }
}

/// Sums sources onto one timeline and hands out finished stretches.
#[derive(Default)]
pub struct Mixer {
    /// Timeline position of `buf[0]`; everything before it has been emitted.
    base: u64,
    buf: VecDeque<[f32; 2]>,
}

impl Mixer {
    pub fn add(&mut self, chunk: &SourceChunk) {
        for (i, f) in chunk.frames.iter().enumerate() {
            let pos = chunk.start + i as u64;
            if pos < self.base {
                continue; // too late: already emitted
            }
            let idx = (pos - self.base) as usize;
            if idx >= self.buf.len() {
                self.buf.resize(idx + 1, [0.0; 2]);
            }
            self.buf[idx][0] += f[0];
            self.buf[idx][1] += f[1];
        }
    }

    /// Emit everything before timeline position `end` (silence where no
    /// source had sound), clipped to [-1, 1].
    pub fn take_until(&mut self, end: u64) -> Option<AudioChunk> {
        if end <= self.base {
            return None;
        }
        let n = (end - self.base) as usize;
        let mut frames: Vec<[f32; 2]> = self.buf.drain(..n.min(self.buf.len())).collect();
        frames.resize(n, [0.0; 2]);
        for f in &mut frames {
            f[0] = f[0].clamp(-1.0, 1.0);
            f[1] = f[1].clamp(-1.0, 1.0);
        }
        let chunk = AudioChunk { start: self.base, frames };
        self.base = end;
        Some(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_44k_to_stereo_48k() {
        let mono: Vec<f32> = (0..441).map(|i| i as f32 / 441.0).collect();
        let out = to_stereo_48k(&mono, 1, 44_100);
        assert_eq!(out.len(), 480);
        assert_eq!(out[0], [0.0, 0.0]);
        assert!(out.iter().all(|f| f[0] == f[1]));
        // Still a rising ramp.
        assert!(out.windows(2).all(|w| w[1][0] >= w[0][0]));
    }

    #[test]
    fn extra_channels_dropped() {
        let quad = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        assert_eq!(to_stereo_48k(&quad, 4, RATE), vec![[0.1, 0.2], [0.5, 0.6]]);
    }

    #[test]
    fn clock_counts_then_resyncs_after_a_gap() {
        let mut c = SourceClock::default();
        let ms = |m: u64| Duration::from_millis(m);
        // 10 ms chunks (480 frames) arriving on time.
        assert_eq!(c.place(ms(10), 480), 0);
        assert_eq!(c.place(ms(21), 480), 480); // a little jitter is ignored
        assert_eq!(c.place(ms(30), 480), 960);
        // Silence for a second, then sound again: jump to real time.
        assert_eq!(c.place(ms(1500), 480), 1500 * 48 - 480);
    }

    #[test]
    fn mixer_sums_fills_gaps_and_clips() {
        let mut m = Mixer::default();
        m.add(&SourceChunk { start: 0, frames: vec![[0.5, 0.5]; 4] });
        m.add(&SourceChunk { start: 2, frames: vec![[0.75, -0.25]; 4] });
        let a = m.take_until(4).unwrap();
        assert_eq!(a.start, 0);
        assert_eq!(a.frames, vec![[0.5, 0.5], [0.5, 0.5], [1.0, 0.25], [1.0, 0.25]]);
        // Gap after the data: silence.
        let b = m.take_until(8).unwrap();
        assert_eq!(b.frames, vec![[0.75, -0.25], [0.75, -0.25], [0.0, 0.0], [0.0, 0.0]]);
        // Late audio is ignored.
        m.add(&SourceChunk { start: 1, frames: vec![[0.9; 2]] });
        assert!(m.take_until(8).is_none());
    }
}
