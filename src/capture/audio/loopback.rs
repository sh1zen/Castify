//! Windows WASAPI Loopback Audio Capture
//!
//! Captures system audio (what's playing through speakers) using WASAPI loopback.
//! This allows recording application audio, browser audio, etc.

use std::thread;
use std::time::Duration;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use log::{error, info, warn};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

use ac_ffmpeg::codec::Encoder;
use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};

#[derive(Copy, Clone, Debug)]
enum InputSampleFormat {
    F32,
    I16,
}

/// WASAPI loopback audio capture for Windows system audio.
pub struct WasapiLoopbackCapture;

impl WasapiLoopbackCapture {
    /// Start capturing system audio via WASAPI loopback.
    ///
    /// Returns a channel with Opus-encoded audio packets.
    pub fn start(cancel: CancellationToken) -> Result<mpsc::Receiver<Vec<u8>>> {
        // Channels for communication
        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<Bytes>(256);
        let (async_tx, async_rx) = mpsc::channel::<Vec<u8>>(256);

        // Bridge: sync -> async
        tokio::spawn(async move {
            loop {
                match sync_rx.recv() {
                    Ok(data) => {
                        if async_tx.send(data.to_vec()).await.is_err() {
                            info!("Audio output channel closed");
                            break;
                        }
                    }
                    Err(_) => {
                        info!("Audio capture channel closed");
                        break;
                    }
                }
            }
        });

        // Capture thread - create WASAPI objects inside the thread
        thread::spawn(move || {
            if let Err(e) = Self::capture_thread(cancel, sync_tx) {
                error!("WASAPI loopback capture error: {}", e);
            }
        });

        Ok(async_rx)
    }

    fn capture_thread(
        cancel: CancellationToken,
        sender: std::sync::mpsc::SyncSender<Bytes>,
    ) -> Result<()> {
        // Initialize COM for this thread
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }

        // Create device enumerator
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("Failed to create device enumerator: {}", e))?
        };

        // Get default audio output device (speakers)
        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| anyhow!("Failed to get default audio endpoint: {}", e))?
        };

        // Activate audio client
        let audio_client: IAudioClient = unsafe {
            device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| anyhow!("Failed to activate audio client: {}", e))?
        };

        // Get mix format (the format the audio engine uses)
        let format_ptr = unsafe {
            audio_client
                .GetMixFormat()
                .map_err(|e| anyhow!("Failed to get mix format: {}", e))?
        };
        let format = unsafe { &*format_ptr };

        // Log format info
        let sample_rate = format.nSamplesPerSec;
        let input_channels = format.nChannels;
        let bits_per_sample = format.wBitsPerSample;
        info!(
            "WASAPI loopback format: {}Hz, {} channels, {} bits",
            sample_rate, input_channels, bits_per_sample
        );

        let input_sample_format = match bits_per_sample {
            32 => InputSampleFormat::F32,
            16 => InputSampleFormat::I16,
            _ => {
                return Err(anyhow!(
                    "Unsupported WASAPI mix format: {} bits per sample",
                    bits_per_sample
                ));
            }
        };

        // Initialize audio client for loopback capture
        // AUDCLNT_STREAMFLAGS_LOOPBACK = 0x00020000
        const AUDCLNT_STREAMFLAGS_LOOPBACK: u32 = 0x00020000;
        const REFTIMES_PER_SEC: i64 = 10_000_000; // 100ns units per second
        let buffer_duration = REFTIMES_PER_SEC; // 1 second buffer

        unsafe {
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    buffer_duration,
                    0,
                    format_ptr,
                    None,
                )
                .map_err(|e| anyhow!("Failed to initialize audio client: {}", e))?
        };

        // Get capture service
        let capture_client: IAudioCaptureClient = unsafe {
            audio_client
                .GetService()
                .map_err(|e| anyhow!("Failed to get capture service: {}", e))?
        };

        // Start audio stream
        unsafe {
            audio_client
                .Start()
                .map_err(|e| anyhow!("Failed to start audio stream: {}", e))?
        }

        info!("WASAPI loopback capture started");

        // Create Opus encoder
        // Force stereo encoding for compatibility with playback/saver pipeline.
        let output_channels = 2u32;
        let encoder = AudioEncoder::builder("libopus")?
            .sample_rate(sample_rate as u32)
            .channel_layout(ChannelLayout::from_channels(output_channels).unwrap())
            .sample_format(get_sample_format("flt"))
            .set_option("frame_duration", "10")
            .build()?;

        let frame_size = encoder.samples_per_frame().unwrap_or(960); // 20ms at 48kHz

        let mut capturer = AudioCapturer {
            encoder,
            sender,
            input_channels: input_channels as usize,
            output_channels: output_channels as usize,
            frame_size,
            sample_buffer: Vec::new(),
        };

        // Capture loop
        loop {
            if cancel.is_cancelled() {
                info!("WASAPI loopback capture cancelled");
                break;
            }

            // Get available packet size
            let packet_size = match unsafe { capture_client.GetNextPacketSize() } {
                Ok(size) => size,
                Err(e) => {
                    error!("Failed to get packet size: {}", e);
                    break;
                }
            };

            if packet_size == 0 {
                // No data available, sleep briefly
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            // Get buffer - GetBuffer takes output parameters
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut frames_available: u32 = 0;
            let mut flags: u32 = 0;

            let result = unsafe {
                capture_client.GetBuffer(
                    &mut data_ptr,
                    &mut frames_available,
                    &mut flags,
                    None,
                    None,
                )
            };

            if let Err(e) = result {
                error!("Failed to get buffer: {}", e);
                break;
            }

            if frames_available == 0 || data_ptr.is_null() {
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            let sample_count = frames_available as usize * capturer.input_channels;
            match input_sample_format {
                InputSampleFormat::F32 => {
                    let samples: &[f32] =
                        unsafe { std::slice::from_raw_parts(data_ptr as *const f32, sample_count) };
                    capturer.process_samples_f32(samples);
                }
                InputSampleFormat::I16 => {
                    let samples: &[i16] =
                        unsafe { std::slice::from_raw_parts(data_ptr as *const i16, sample_count) };
                    capturer.process_samples_i16(samples);
                }
            }

            // Release buffer
            unsafe {
                if let Err(e) = capture_client.ReleaseBuffer(frames_available) {
                    error!("Failed to release buffer: {}", e);
                    break;
                }
            }
        }

        // Stop audio client
        unsafe {
            let _ = audio_client.Stop();
        }

        // Flush remaining samples
        capturer.flush();

        info!("WASAPI loopback capture stopped");
        Ok(())
    }
}

struct AudioCapturer {
    encoder: AudioEncoder,
    sender: std::sync::mpsc::SyncSender<Bytes>,
    input_channels: usize,
    output_channels: usize,
    frame_size: usize,
    sample_buffer: Vec<f32>,
}

impl AudioCapturer {
    fn process_samples_f32(&mut self, samples: &[f32]) {
        self.push_downmixed_stereo(samples.iter().copied());

        // Encode complete frames
        while self.sample_buffer.len() >= self.frame_size * self.output_channels {
            self.encode_frame();
        }
    }

    fn process_samples_i16(&mut self, samples: &[i16]) {
        self.push_downmixed_stereo(samples.iter().map(|&s| s as f32 / 32768.0));

        // Encode complete frames
        while self.sample_buffer.len() >= self.frame_size * self.output_channels {
            self.encode_frame();
        }
    }

    fn push_downmixed_stereo<I>(&mut self, interleaved: I)
    where
        I: IntoIterator<Item = f32>,
    {
        let mut frame: Vec<f32> = Vec::with_capacity(self.input_channels);
        for sample in interleaved {
            frame.push(sample);
            if frame.len() == self.input_channels {
                let (l, r) = match self.input_channels {
                    0 => (0.0, 0.0),
                    1 => (frame[0], frame[0]),
                    _ => (frame[0], frame[1]),
                };
                self.sample_buffer.push(l);
                self.sample_buffer.push(r);
                frame.clear();
            }
        }
    }

    fn encode_frame(&mut self) {
        let samples_needed = self.frame_size * self.output_channels;
        if self.sample_buffer.len() < samples_needed {
            return;
        }

        // Create audio frame (silence returns AudioFrameMut directly, not Result)
        let mut frame = AudioFrameMut::silence(
            self.encoder.codec_parameters().channel_layout(),
            self.encoder.codec_parameters().sample_format(),
            self.encoder.codec_parameters().sample_rate(),
            self.frame_size,
        );

        // Write samples to frame planes
        // Use the same pattern as capture.rs: get mutable reference to plane directly
        let plane = &mut frame.planes_mut()[0];
        let data = plane.data_mut();
        let dst: &mut [f32] = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut f32, samples_needed)
        };
        dst.copy_from_slice(&self.sample_buffer[..samples_needed]);

        // Remove used samples from buffer
        self.sample_buffer.drain(..samples_needed);

        // Encode frame
        if let Err(e) = self.encoder.push(frame.freeze()) {
            warn!("Failed to push audio frame: {}", e);
            return;
        }

        // Get encoded packets
        while let Ok(Some(packet)) = self.encoder.take() {
            let _ = self.sender.send(Bytes::from(packet.data().to_vec()));
        }
    }

    fn flush(&mut self) {
        // Pad remaining samples to complete a frame
        let samples_needed = self.frame_size * self.output_channels;
        if !self.sample_buffer.is_empty() && self.sample_buffer.len() < samples_needed {
            let padding = samples_needed - self.sample_buffer.len();
            self.sample_buffer.extend(std::iter::repeat_n(0.0, padding));
            self.encode_frame();
        }

        // Flush encoder
        if let Err(e) = self.encoder.flush() {
            warn!("Failed to flush encoder: {}", e);
        }
        while let Ok(Some(packet)) = self.encoder.take() {
            let _ = self.sender.send(Bytes::from(packet.data().to_vec()));
        }
    }
}
