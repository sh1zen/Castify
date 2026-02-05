use ac_ffmpeg::codec::Decoder;
use ac_ffmpeg::codec::audio::AudioDecoder;
use ac_ffmpeg::packet::PacketMut;
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Maximum samples in the ring buffer (at 48kHz stereo, this is ~170ms of audio)
/// This prevents unbounded memory growth and limits audio latency
const MAX_BUFFER_SAMPLES: usize = 16384;
const I16_TO_F32: f32 = 1.0 / 32768.0;

pub struct AudioPlayer {
    sample_buffer: Arc<Mutex<AudioRingBuffer>>,
    decoder: AudioDecoder,
    _stream: cpal::Stream, // kept alive
}

/// A lock-free ring buffer for audio samples with overflow protection
struct AudioRingBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    len: usize,
    capacity: usize,
    samples_dropped: u64,
}

impl AudioRingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0f32; capacity],
            write_pos: 0,
            read_pos: 0,
            len: 0,
            capacity,
            samples_dropped: 0,
        }
    }

    /// Push samples to the buffer, dropping oldest if full
    fn push(&mut self, samples: &[f32]) {
        for &sample in samples {
            if self.len >= self.capacity {
                self.read_pos = (self.read_pos + 1) % self.capacity;
                self.samples_dropped += 1;
            } else {
                self.len += 1;
            }
            self.buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.capacity;
        }
    }

    /// Read samples from the buffer
    fn read(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            if self.len > 0 {
                *sample = self.buffer[self.read_pos];
                self.read_pos = (self.read_pos + 1) % self.capacity;
                self.len -= 1;
            } else {
                *sample = 0.0;
            }
        }
    }
}

unsafe impl Send for AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let decoder = AudioDecoder::new("libopus").or_else(|e| {
            log::warn!(
                "libopus decoder not available ({}), trying built-in opus decoder",
                e
            );
            AudioDecoder::new("opus")
        })?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output audio device"))?;
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: 48000,
            buffer_size: cpal::BufferSize::Default,
        };

        let sample_buffer = Arc::new(Mutex::new(AudioRingBuffer::new(MAX_BUFFER_SAMPLES)));
        let buffer_clone = Arc::clone(&sample_buffer);

        let stream = device.build_output_stream(
            &config,
            move |output: &mut [f32], _| {
                if let Ok(mut buf) = buffer_clone.lock() {
                    buf.read(output);
                } else {
                    output.fill(0.0);
                }
            },
            |err| log::error!("Audio output error: {}", err),
            None,
        )?;
        stream.play()?;

        Ok(Self {
            sample_buffer,
            decoder,
            _stream: stream,
        })
    }

    pub fn play(&mut self, opus_data: &[u8]) -> Result<()> {
        let packet = PacketMut::from(opus_data).freeze();
        match self.decoder.try_push(packet) {
            Ok(()) => {}
            Err(e) => {
                if e.is_again() {
                    self.drain_frames();
                    let retry = PacketMut::from(opus_data).freeze();
                    if let Err(e) = self.decoder.try_push(retry) {
                        log::warn!("Audio decode retry failed: {}", e);
                    }
                } else {
                    log::warn!("Audio decode error: {}", e);
                }
            }
        }
        self.drain_frames();
        Ok(())
    }

    fn drain_frames(&mut self) {
        let mut all_samples: Vec<f32> = Vec::new();

        while let Ok(Some(frame)) = self.decoder.take() {
            let planes = frame.planes();
            let sample_count = frame.samples();
            if sample_count == 0 {
                continue;
            }

            if planes.len() >= 2 {
                let left = planes[0].data();
                let right = planes[1].data();
                if !append_planar_stereo(&mut all_samples, left, right, sample_count) {
                    log::warn!(
                        "Audio plane too small ({}+{} bytes for {} samples)",
                        left.len(),
                        right.len(),
                        sample_count
                    );
                }
                continue;
            }

            if let Some(data) = planes.first().map(|p| p.data())
                && !append_interleaved_stereo(&mut all_samples, data, sample_count)
            {
                log::warn!(
                    "Interleaved audio too small ({} bytes for {} samples)",
                    data.len(),
                    sample_count
                );
            }
        }

        if !all_samples.is_empty()
            && let Ok(mut buf) = self.sample_buffer.lock()
        {
            buf.push(&all_samples);
        }
    }
}

fn append_planar_stereo(
    out: &mut Vec<f32>,
    left: &[u8],
    right: &[u8],
    sample_count: usize,
) -> bool {
    let min_bytes_f32 = sample_count * 4;
    if left.len() >= min_bytes_f32 && right.len() >= min_bytes_f32 {
        let left_f32: &[f32] =
            unsafe { std::slice::from_raw_parts(left.as_ptr() as *const f32, sample_count) };
        let right_f32: &[f32] =
            unsafe { std::slice::from_raw_parts(right.as_ptr() as *const f32, sample_count) };
        for i in 0..sample_count {
            out.push(left_f32[i]);
            out.push(right_f32[i]);
        }
        return true;
    }

    let min_bytes_i16 = sample_count * 2;
    if left.len() >= min_bytes_i16 && right.len() >= min_bytes_i16 {
        let left_i16: &[i16] =
            unsafe { std::slice::from_raw_parts(left.as_ptr() as *const i16, sample_count) };
        let right_i16: &[i16] =
            unsafe { std::slice::from_raw_parts(right.as_ptr() as *const i16, sample_count) };
        for i in 0..sample_count {
            out.push(left_i16[i] as f32 * I16_TO_F32);
            out.push(right_i16[i] as f32 * I16_TO_F32);
        }
        return true;
    }

    false
}

fn append_interleaved_stereo(out: &mut Vec<f32>, data: &[u8], sample_count: usize) -> bool {
    let total_samples = sample_count * 2;
    let min_bytes_f32 = total_samples * 4;
    if data.len() >= min_bytes_f32 {
        let samples: &[f32] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f32, total_samples) };
        out.extend_from_slice(samples);
        return true;
    }

    let min_bytes_i16 = total_samples * 2;
    if data.len() >= min_bytes_i16 {
        let samples: &[i16] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const i16, total_samples) };
        out.extend(samples.iter().map(|&s| s as f32 * I16_TO_F32));
        return true;
    }

    false
}
