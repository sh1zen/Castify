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

/// Convert a YUV420p VideoFrame to RGBA pixel data (BT.709 limited range).
///
/// Uses fixed-point integer arithmetic for performance (~2-3x faster than float).
/// The capture shaders encode BGRA→NV12 using BT.709 limited range
/// (Y in [16,235], Cb/Cr in [16,240]), so decoding must match.
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

    // Fixed-point coefficients (scaled by 256, i.e. <<8):
    // Y scale: 255/219 * 256 ≈ 298
    // Cr→R:  1.5748 * (255/224) * 256 ≈ 459
    // Cb→G: -0.1873 * (255/224) * 256 ≈ -55
    // Cr→G: -0.4681 * (255/224) * 256 ≈ -136
    // Cb→B:  1.8556 * (255/224) * 256 ≈ 541

    for row in 0..height {
        let y_row_offset = row * y_stride;
        let uv_row_offset = (row / 2) * u_stride;
        let uv_row_offset_v = (row / 2) * v_stride;
        let rgba_row_offset = row * width * 4;

        for col in 0..width {
            let uv_col = col / 2;

            let y_val = y_data[y_row_offset + col] as i32;
            let u_val = u_data[uv_row_offset + uv_col] as i32;
            let v_val = v_data[uv_row_offset_v + uv_col] as i32;

            let y_scaled = (y_val - 16) * 298 + 128;
            let cb = u_val - 128;
            let cr = v_val - 128;

            let r = (y_scaled + 459 * cr) >> 8;
            let g = (y_scaled - 55 * cb - 136 * cr) >> 8;
            let b = (y_scaled + 541 * cb) >> 8;

            let idx = rgba_row_offset + col * 4;
            rgba[idx] = r.clamp(0, 255) as u8;
            rgba[idx + 1] = g.clamp(0, 255) as u8;
            rgba[idx + 2] = b.clamp(0, 255) as u8;
            // rgba[idx + 3] = 255 already set
        }
    }

    rgba
}
