//! Ring buffer with jitter compensation for audio samples
//!
//! Provides a lock-free ring buffer that can absorb jitter in audio
//! sample delivery, preventing underruns and overruns.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Ring buffer for audio samples with jitter compensation
///
/// Designed for a single producer (decoder) and single consumer (audio output).
/// Uses atomic operations for lock-free access.
///
/// The buffer maintains a read and write position, with the invariant that
/// the writer never overtakes the reader (drops samples if buffer is full)
/// and the reader outputs silence when the buffer is empty.
pub struct AudioRingBuffer {
    /// The underlying sample buffer
    buffer: Vec<f32>,
    /// Current write position
    write_pos: AtomicUsize,
    /// Current read position
    read_pos: AtomicUsize,
    /// Buffer capacity
    capacity: usize,
    /// Whether the buffer has data available
    has_data: AtomicBool,
}

// Safety: AudioRingBuffer can be shared between threads
unsafe impl Send for AudioRingBuffer {}
unsafe impl Sync for AudioRingBuffer {}

impl AudioRingBuffer {
    /// Create a new ring buffer with the given capacity (in samples)
    ///
    /// For stereo 48kHz with 100ms of buffering:
    /// capacity = 48000 * 2 * 0.1 = 9600 samples
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            capacity,
            has_data: AtomicBool::new(false),
        }
    }

    /// Write samples into the ring buffer
    ///
    /// Returns the number of samples actually written.
    /// If the buffer is full, excess samples are dropped.
    pub fn write(&self, samples: &[f32]) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);

        // Available space
        let available = if write >= read {
            self.capacity - (write - read) - 1
        } else {
            read - write - 1
        };

        let to_write = samples.len().min(available);
        if to_write == 0 {
            return 0;
        }

        // Write samples (we need interior mutability without locks)
        // Safety: only one writer exists, and we don't overlap with the reader's region
        let buf_ptr = self.buffer.as_ptr() as *mut f32;
        for (i, &sample) in samples.iter().enumerate().take(to_write) {
            let pos = (write + i) % self.capacity;
            unsafe {
                *buf_ptr.add(pos) = sample;
            }
        }

        self.write_pos
            .store((write + to_write) % self.capacity, Ordering::Release);
        self.has_data.store(true, Ordering::Release);

        to_write
    }

    /// Read samples from the ring buffer
    ///
    /// Fills the output buffer. If not enough samples are available,
    /// the remaining positions are filled with silence (0.0).
    /// Returns the number of actual samples read (not silence).
    pub fn read(&self, output: &mut [f32]) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Relaxed);

        // Available samples
        let available = if write >= read {
            write - read
        } else {
            self.capacity - read + write
        };

        let to_read = output.len().min(available);

        // Read samples
        for (i, sample) in output.iter_mut().enumerate().take(to_read) {
            let pos = (read + i) % self.capacity;
            *sample = self.buffer[pos];
        }

        // Fill remaining with silence
        for sample in output[to_read..].iter_mut() {
            *sample = 0.0;
        }

        if to_read > 0 {
            self.read_pos
                .store((read + to_read) % self.capacity, Ordering::Release);
        }

        if available <= to_read {
            self.has_data.store(false, Ordering::Release);
        }

        to_read
    }

    /// Check if the buffer has data available
    pub fn has_data(&self) -> bool {
        self.has_data.load(Ordering::Acquire)
    }

    /// Get the number of samples currently in the buffer
    pub fn available(&self) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the fill level as a percentage
    pub fn fill_level(&self) -> f32 {
        self.available() as f32 / self.capacity as f32
    }

    /// Reset the buffer (clear all data)
    pub fn reset(&self) {
        self.write_pos.store(0, Ordering::Release);
        self.read_pos.store(0, Ordering::Release);
        self.has_data.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write_read() {
        let buf = AudioRingBuffer::new(1024);

        let samples = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(buf.write(&samples), 4);
        assert_eq!(buf.available(), 4);

        let mut output = [0.0f32; 4];
        assert_eq!(buf.read(&mut output), 4);
        assert_eq!(output, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_underrun_silence() {
        let buf = AudioRingBuffer::new(1024);

        let samples = [1.0, 2.0];
        buf.write(&samples);

        let mut output = [0.0f32; 4];
        let read = buf.read(&mut output);
        assert_eq!(read, 2);
        assert_eq!(output, [1.0, 2.0, 0.0, 0.0]); // Last two are silence
    }

    #[test]
    fn test_overrun_drops() {
        let buf = AudioRingBuffer::new(4); // Very small buffer

        let samples = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let written = buf.write(&samples);
        assert!(written < 6); // Not all samples fit
    }

    #[test]
    fn test_wrap_around() {
        let buf = AudioRingBuffer::new(8);

        // Fill partially
        let s1 = [1.0, 2.0, 3.0, 4.0, 5.0];
        buf.write(&s1);

        // Read some
        let mut out = [0.0f32; 3];
        buf.read(&mut out);
        assert_eq!(out, [1.0, 2.0, 3.0]);

        // Write more (wraps around)
        let s2 = [6.0, 7.0, 8.0, 9.0];
        let written = buf.write(&s2);
        assert!(written > 0);

        // Read all
        let mut out2 = [0.0f32; 6];
        let read = buf.read(&mut out2);
        assert!(read > 0);
        assert_eq!(out2[0], 4.0);
        assert_eq!(out2[1], 5.0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let buf = Arc::new(AudioRingBuffer::new(4800)); // 100ms at 48kHz
        let buf_writer = buf.clone();
        let buf_reader = buf.clone();

        let writer = thread::spawn(move || {
            let samples: Vec<f32> = (0..48000).map(|i| (i as f32) / 48000.0).collect();
            let mut total_written = 0;
            for chunk in samples.chunks(480) {
                total_written += buf_writer.write(chunk);
                thread::sleep(Duration::from_micros(100));
            }
            total_written
        });

        let reader = thread::spawn(move || {
            let mut total_read = 0;
            let mut output = [0.0f32; 480];
            for _ in 0..100 {
                total_read += buf_reader.read(&mut output);
                thread::sleep(Duration::from_micros(200));
            }
            total_read
        });

        let written = writer.join().unwrap();
        let read = reader.join().unwrap();

        assert!(written > 0);
        assert!(read > 0);
    }
}
