use std::thread;
use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::Encoder;
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use log::{error, info};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use anyhow::{anyhow, Result};

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
    sender: std::sync::mpsc::Sender<Bytes>,
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

    /// Avvia la cattura audio e ritorna un canale Tokio con i pacchetti Opus codificati.
    ///
    /// Il canale si chiude quando viene cancellato il `CancellationToken`.
    pub fn start(cancel: CancellationToken) -> Result<mpsc::Receiver<Vec<u8>>> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No default output device found"))?;

        let config = device
            .default_output_config()
            .map_err(|e| anyhow!("Failed to get default output config: {}", e))?;

        info!("Audio config: {:?}", config);

        let encoder = AudioEncoder::builder("libopus")?
            .sample_rate(config.sample_rate())
            .channel_layout(ChannelLayout::from_channels(2).unwrap())
            .sample_format(convert_sample_format(config.sample_format()))
            .set_option("frame_duration", "10")
            .build()?;

        // Canale sincrono: cpal callback → thread bridge
        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<Bytes>();

        // Canale Tokio: bridge → consumatore esterno
        let (async_tx, async_rx) = mpsc::channel::<Vec<u8>>(64);

        // Task bridge: sincrono → asincrono
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

        // Thread di cattura audio (cpal richiede un thread dedicato)
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

            // Attendi cancellazione
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