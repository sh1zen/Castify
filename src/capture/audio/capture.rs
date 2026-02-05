//! Audio capture implementation
//!
//! This module provides audio capture functionality using cpal for
//! audio input and FFmpeg for Opus encoding.

use ac_ffmpeg::codec::Encoder;
use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{error, info};
use std::thread;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn convert_sample_format(format: SampleFormat) -> ac_ffmpeg::codec::audio::SampleFormat {
    get_sample_format(match format {
        SampleFormat::F32 => "flt",
        SampleFormat::I16 => "s16",
        SampleFormat::I32 => "s32",
        _ => panic!("Unsupported sample format: {:?}", format),
    })
}

pub struct AudioCapture {
    encoder: AudioEncoder,
    sender: std::sync::mpsc::SyncSender<Bytes>,
}

impl AudioCapture {
    fn write_input_data<T>(&mut self, input: &[T])
    where
        T: cpal::Sample,
    {
        let sample_size = self.encoder.samples_per_frame().unwrap();

        let mut frame = AudioFrameMut::silence(
            self.encoder.codec_parameters().channel_layout(),
            self.encoder.codec_parameters().sample_format(),
            self.encoder.codec_parameters().sample_rate(),
            sample_size,
        );

        let plane = &mut frame.planes_mut()[0];
        let data = plane.data_mut();
        let samples: &mut [T] = unsafe {
            std::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut T,
                data.len() / std::mem::size_of::<T>(),
            )
        };

        samples[..input.len()].copy_from_slice(input);

        self.encoder.push(frame.freeze()).unwrap();

        let mut ret = Vec::new();
        while let Some(packet) = self.encoder.take().unwrap() {
            ret.extend(packet.data());
        }

        let _ = self.sender.send(Bytes::from(ret));
    }

    /// Starts audio capture and returns a Tokio channel with Opus-encoded packets.
    ///
    /// The channel closes when the `CancellationToken` is cancelled.
    ///
    /// On Windows, this uses WASAPI loopback to capture system audio (what's playing through speakers).
    /// On other platforms, it captures from the default input device (microphone).
    pub fn start(cancel: CancellationToken) -> Result<mpsc::Receiver<Vec<u8>>> {
        let host = cpal::default_host();

        // On Windows, use loopback to capture system audio (speakers output)
        // On other platforms, use the default input device (microphone)
        #[cfg(target_os = "windows")]
        let (device, config) = {
            // Use OUTPUT device with loopback for system audio capture
            let device = host
                .default_output_device()
                .ok_or_else(|| anyhow!("No default output device found for loopback"))?;

            let config = device
                .default_output_config()
                .map_err(|e| anyhow!("Failed to get default output config: {}", e))?;

            info!("Audio capture (loopback) config: {:?}", config);
            (device, config)
        };

        #[cfg(not(target_os = "windows"))]
        let (device, config) = {
            let device = host
                .default_input_device()
                .ok_or_else(|| anyhow!("No default input device found"))?;

            let config = device
                .default_input_config()
                .map_err(|e| anyhow!("Failed to get default input config: {}", e))?;

            info!("Audio capture config: {:?}", config);
            (device, config)
        };

        let encoder = AudioEncoder::builder("libopus")?
            .sample_rate(config.sample_rate())
            .channel_layout(ChannelLayout::from_channels(2).unwrap())
            .sample_format(convert_sample_format(config.sample_format()))
            .set_option("frame_duration", "10")
            .build()?;

        // Synchronous channel: cpal callback → bridge thread
        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<Bytes>(256);

        // Async channel: bridge → external consumer
        // Increased from 64 to 256 to handle audio bursts without dropping
        let (async_tx, async_rx) = mpsc::channel::<Vec<u8>>(256);

        // Bridge task: synchronous → asynchronous
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

        // Audio capture thread (cpal requires a dedicated thread)
        let handle = tokio::runtime::Handle::current();
        thread::spawn(move || -> Result<()> {
            let mut capturer = AudioCapture {
                encoder,
                sender: sync_tx,
            };

            let err_fn = |err| error!("Audio stream error: {}", err);

            let stream = match config.sample_format() {
                SampleFormat::I8 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i8>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::I16 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i16>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::I32 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<i32>(data),
                    err_fn,
                    None,
                )?,
                SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data, _: &_| capturer.write_input_data::<f32>(data),
                    err_fn,
                    None,
                )?,
                _ => return Err(anyhow!("Unsupported sample format")),
            };

            stream.play()?;
            info!("Audio capture started");

            // Wait for cancellation
            tokio::task::block_in_place(move || {
                handle.block_on(async move { cancel.cancelled().await });
            });

            stream.pause()?;
            info!("Audio capture stopped");
            Ok(())
        });

        Ok(async_rx)
    }
}
