use ac_ffmpeg::codec::video::VideoDecoder;
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

    /// Decode an H.264 access unit (Annex B) and return packed YUV420p plane data.
    /// The returned Vec contains Y plane (w*h) + U plane (w/2 * h/2) + V plane (w/2 * h/2)
    /// contiguously, with stride-padding stripped.
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
                let planes = frame.planes();
                let (y_d, u_d, v_d) = (planes[0].data(), planes[1].data(), planes[2].data());
                let (y_s, u_s, v_s) = (planes[0].line_size(), planes[1].line_size(), planes[2].line_size());
                let (uw, uh) = (w / 2, h / 2);
                let total = w * h + uw * uh * 2;
                let mut packed = Vec::with_capacity(total);
                for r in 0..h  { packed.extend_from_slice(&y_d[r * y_s .. r * y_s + w]); }
                for r in 0..uh { packed.extend_from_slice(&u_d[r * u_s .. r * u_s + uw]); }
                for r in 0..uh { packed.extend_from_slice(&v_d[r * v_s .. r * v_s + uw]); }
                Some((packed, w, h))
            }
            Ok(None) => None,
            Err(e) => {
                log::warn!("Decoder: take() error: {}", e);
                None
            }
        }
    }
}
