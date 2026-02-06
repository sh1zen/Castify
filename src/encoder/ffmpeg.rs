use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use bytes::Bytes;
use crate::capture::YUVFrame;
use crate::encoder::frame_pool::FramePool;

/// Encoder fallback chain: try hardware encoders first, then software.
const ENCODER_CHAIN: &[(&str, &[(&str, &str)])] = &[
    ("h264_nvenc", &[
        ("preset", "p1"),
        ("tune", "ull"),
        ("zerolatency", "1"),
        ("rc", "vbr"),
        ("b", "2000000"),
        ("maxrate", "3000000"),
        ("bufsize", "4000000"),
        ("g", "30"),
        ("gpu", "0"),
    ]),
    ("h264_qsv", &[
        ("preset", "veryfast"),
        ("g", "30"),
        ("b", "2000000"),
        ("maxrate", "3000000"),
        ("bufsize", "4000000"),
        ("low_power", "1"),
    ]),
    ("h264_amf", &[
        ("usage", "ultralowlatency"),
        ("quality", "speed"),
        ("b", "2000000"),
        ("maxrate", "3000000"),
        ("rc", "vbr_peak"),
        ("g", "30"),
    ]),
    ("libx264", &[
        ("profile", "baseline"),
        ("preset", "ultrafast"),
        ("tune", "zerolatency"),
        ("crf", "28"),
        ("maxrate", "2000000"),
        ("bufsize", "4000000"),
        ("keyint", "30"),
        ("min-keyint", "15"),
        ("scenecut", "40"),
        ("threads", "0"),
        ("sliced-threads", "1"),
    ]),
];

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    pixel_format: String,
    w: usize,
    h: usize,
    pub force_idr: Arc<AtomicBool>,
    pub codec_name: String,
}

unsafe impl Send for FfmpegEncoder {}

#[allow(dead_code)]
pub enum FrameData<'a> {
    NV12(&'a YUVFrame),
    BGR0(&'a [u8]),
}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32) -> Self {
        let w = if w % 2 == 0 { w } else { w + 1 } as usize;
        let h = if h % 2 == 0 { h } else { h + 1 } as usize;
        let time_base = TimeBase::new(1, 90_000);

        let pixel_format = video::frame::get_pixel_format("nv12");

        let (encoder, codec_name) = Self::try_create_encoder(w, h, time_base, pixel_format);
        log::info!("Using encoder: {}", codec_name);

        Self {
            encoder,
            pixel_format: String::from("nv12"),
            frame_pool: FramePool::new(w, h, time_base, pixel_format),
            force_idr: Arc::new(AtomicBool::new(false)),
            codec_name,
            w,
            h,
        }
    }

    fn try_create_encoder(
        w: usize,
        h: usize,
        time_base: TimeBase,
        pixel_format: video::frame::PixelFormat,
    ) -> (VideoEncoder, String) {
        for (codec, options) in ENCODER_CHAIN {
            let mut builder = match VideoEncoder::builder(codec) {
                Ok(b) => b,
                Err(e) => {
                    log::debug!("Encoder {} not available, skipping: {}", codec, e);
                    continue;
                }
            };
            builder = builder
                .pixel_format(pixel_format)
                .width(w)
                .height(h)
                .time_base(time_base);
            for (k, v) in *options {
                builder = builder.set_option(k, v);
            }
            match builder.build() {
                Ok(enc) => return (enc, codec.to_string()),
                Err(e) => {
                    log::debug!("Encoder {} failed to initialize: {}", codec, e);
                    continue;
                }
            }
        }
        panic!("No H.264 encoder available — install FFmpeg with at least libx264 support");
    }

    pub fn encode(
        &mut self,
        frame_data: FrameData,
        frame_time: i64,
    ) -> Result<Bytes, anyhow::Error> {
        let mut frame = self.frame_pool.take();
        let time_base = frame.time_base();
        frame = frame
            .with_pts(Timestamp::new(
                if cfg!(target_os = "windows") {
                    (frame_time as f64 * 9. / 1000.) as i64
                } else if cfg!(target_os = "macos") {
                    (frame_time as f64 * 9. / 1e5) as i64
                } else {
                    panic!("Unsupported OS")
                },
                time_base,
            ))
            .with_picture_type(
                if self
                    .force_idr
                    .swap(false, std::sync::atomic::Ordering::Relaxed)
                {
                    video::frame::PictureType::I
                } else {
                    video::frame::PictureType::None
                },
            );

        match frame_data {
            FrameData::NV12(nv12) => {
                // Y plane (luminance): full size
                {
                    let mut planes = frame.planes_mut();
                    let y_plane = planes[0].data_mut();
                    let y_line_size = y_plane.len() / self.h;

                    self.copy_nv12(
                        &nv12.luminance_bytes,
                        nv12.luminance_stride as usize,
                        y_line_size,
                        self.h,
                        y_plane,
                    );
                }

                // UV plane (chrominance): half height for NV12/YUV420
                {
                    let mut planes = frame.planes_mut();
                    let uv_plane = planes[1].data_mut();
                    let uv_h = self.h / 2;
                    let uv_line_size = uv_plane.len() / uv_h;

                    self.copy_nv12(
                        &nv12.chrominance_bytes,
                        nv12.chrominance_stride as usize,
                        uv_line_size,
                        uv_h,
                        uv_plane,
                    );
                }
            }
            FrameData::BGR0(bgr0) => match self.pixel_format.as_str() {
                "bgra" => {
                    let mut planes = frame.planes_mut();
                    planes[0].data_mut().copy_from_slice(bgr0);
                }
                _ => unimplemented!(),
            },
        }
        let frame = frame.freeze();
        self.encoder.push(frame.clone())?;
        self.frame_pool.put(frame);
        let mut ret = Vec::new();
        while let Some(packet) = self.encoder.take()? {
            ret.extend(packet.data());
        }
        Ok(Bytes::from(ret))
    }

    fn copy_nv12(
        &self,
        source: &[u8],
        stride: usize,
        encoder_line_size: usize,
        rows: usize,
        destination: &mut [u8],
    ) {
        // fast path: strides match exactly
        if stride == encoder_line_size && source.len() == destination.len() {
            destination.copy_from_slice(source);
            return;
        }

        let copy_width = self.w.min(stride).min(encoder_line_size);
        for r in 0..rows {
            let src_start = r * stride;
            let dst_start = r * encoder_line_size;
            if src_start + copy_width > source.len() || dst_start + copy_width > destination.len() {
                break;
            }
            destination[dst_start..dst_start + copy_width]
                .copy_from_slice(&source[src_start..src_start + copy_width]);
        }
    }
}
