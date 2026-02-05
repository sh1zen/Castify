use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{video, Encoder};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use bytes::Bytes;
use itertools::enumerate;
use crate::capture::YUVFrame;
use crate::encoder::frame_pool::FramePool;

pub struct FfmpegEncoder {
    encoder: VideoEncoder,
    frame_pool: FramePool,
    pixel_format: String,
    w: usize,
    h: usize,
    pub force_idr: Arc<AtomicBool>,
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

        let pixel_format = video::frame::get_pixel_format("yuv420p");

        let encoder = VideoEncoder::builder("libx264")
            .unwrap()
            .pixel_format(pixel_format)
            .width(w)
            .height(h)
            .time_base(time_base)
            .set_option("profile", "baseline")
            .set_option("preset", "ultrafast")
            .set_option("tune", "zerolatency")
            .build()
            .unwrap();

        Self {
            encoder,
            pixel_format: String::from("yuv420p"),
            frame_pool: FramePool::new(w, h, time_base, pixel_format),
            force_idr: Arc::new(AtomicBool::new(false)),
            w,
            h,
        }
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
                // Piano Y (luminance): dimensione piena
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

                // Piano UV (chrominance): metà altezza per NV12/YUV420
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