use ac_ffmpeg::codec::video::VideoDecoder;
use ac_ffmpeg::codec::video::VideoFrame;
use ac_ffmpeg::codec::Decoder;
use ac_ffmpeg::packet::PacketMut;
use ac_ffmpeg::time::{TimeBase, Timestamp};

pub struct FfmpegDecoder {
    decoder: VideoDecoder,
    frame_count: i64,
}

unsafe impl Send for FfmpegDecoder {}

impl FfmpegDecoder {
    pub fn new() -> Result<Self, ac_ffmpeg::Error> {
        let decoder = VideoDecoder::builder("h264")?
            .time_base(TimeBase::new(1, 90_000))
            .build()?;

        Ok(Self {
            decoder,
            frame_count: 0,
        })
    }

    /// Decode an H.264 access unit (Annex B) and return RGBA pixel data.
    /// Returns `None` if the decoder is still buffering or on error.
    pub fn decode(&mut self, h264_data: &[u8]) -> Option<(Vec<u8>, usize, usize)> {
        self.frame_count += 1;
        let pts = Timestamp::new(self.frame_count, TimeBase::new(1, 90_000));
        let packet = PacketMut::from(h264_data).with_pts(pts).freeze();

        if self.decoder.try_push(packet).is_err() {
            log::warn!("Decoder: failed to push packet {}", self.frame_count);
            return None;
        }

        match self.decoder.take() {
            Ok(Some(frame)) => {
                let w = frame.width();
                let h = frame.height();
                let rgba = yuv420p_to_rgba(&frame, w, h);
                Some((rgba, w, h))
            }
            Ok(None) => None,
            Err(e) => {
                log::warn!("Decoder: take() error: {}", e);
                None
            }
        }
    }
}

/// Convert a YUV420p VideoFrame to RGBA pixel data (BT.601).
fn yuv420p_to_rgba(frame: &VideoFrame, width: usize, height: usize) -> Vec<u8> {
    let planes = frame.planes();
    let y_plane = &planes[0];
    let u_plane = &planes[1];
    let v_plane = &planes[2];

    let y_data = y_plane.data();
    let u_data = u_plane.data();
    let v_data = v_plane.data();

    let y_stride = y_plane.line_size();
    let u_stride = u_plane.line_size();
    let v_stride = v_plane.line_size();

    let mut rgba = vec![255u8; width * height * 4];

    for row in 0..height {
        let uv_row = row / 2;
        for col in 0..width {
            let uv_col = col / 2;

            let y = y_data[row * y_stride + col] as f32;
            let u = u_data[uv_row * u_stride + uv_col] as f32;
            let v = v_data[uv_row * v_stride + uv_col] as f32;

            let r = y + 1.402 * (v - 128.0);
            let g = y - 0.344136 * (u - 128.0) - 0.714136 * (v - 128.0);
            let b = y + 1.772 * (u - 128.0);

            let idx = (row * width + col) * 4;
            rgba[idx] = r.clamp(0.0, 255.0) as u8;
            rgba[idx + 1] = g.clamp(0.0, 255.0) as u8;
            rgba[idx + 2] = b.clamp(0.0, 255.0) as u8;
            // rgba[idx + 3] = 255 already set
        }
    }

    rgba
}
