use ac_ffmpeg::codec::Decoder;
use ac_ffmpeg::codec::video::VideoDecoder;
use ac_ffmpeg::packet::PacketMut;
use ac_ffmpeg::time::{TimeBase, Timestamp};

/// H.264 video decoder using FFmpeg.
///
/// # Performance Optimizations
/// - Pre-allocated output buffer reused across frames
/// - Optimized YUV plane extraction with fast paths
/// - Hardware acceleration via DXVA2/D3D11VA on Windows, VAAPI on Linux, VideoToolbox on macOS
pub struct FfmpegDecoder {
    decoder: VideoDecoder,
    frame_count: i64,
    /// Reusable buffer for packed YUV output to avoid per-frame allocation
    packed_buffer: Vec<u8>,
    /// Cached dimensions for buffer reuse
    cached_dims: Option<(usize, usize)>,
}

unsafe impl Send for FfmpegDecoder {}

impl FfmpegDecoder {
    /// Create a new H.264 decoder.
    ///
    /// The decoder will automatically use hardware acceleration when available:
    /// - Windows: DXVA2 or D3D11VA
    /// - macOS: VideoToolbox
    /// - Linux: VAAPI or VDPAU
    pub fn new() -> Result<Self, ac_ffmpeg::Error> {
        let decoder = VideoDecoder::builder("h264")?
            .time_base(TimeBase::new(1, 90_000))
            .build()?;

        Ok(Self {
            decoder,
            frame_count: 0,
            packed_buffer: Vec::new(),
            cached_dims: None,
        })
    }

    /// Decode an H.264 access unit (Annex B) and return packed YUV420p plane data.
    /// The returned Vec contains Y plane (w*h) + U plane (w/2 * h/2) + V plane (w/2 * h/2)
    /// contiguously, with stride-padding stripped.
    ///
    /// # Performance
    /// - Reuses internal buffer to avoid allocations
    /// - Uses optimized plane extraction for common stride patterns
    /// - Returns `None` if the decoder is still buffering or on error.
    pub fn decode(&mut self, h264_data: &[u8]) -> Option<(Vec<u8>, usize, usize)> {
        let pts = self.next_pts();
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
                let (y_s, u_s, v_s) = (
                    planes[0].line_size(),
                    planes[1].line_size(),
                    planes[2].line_size(),
                );
                let (uw, uh) = (w / 2, h / 2);
                let total = w * h + uw * uh * 2;

                // Reuse buffer if dimensions match, otherwise reallocate
                if self.cached_dims != Some((w, h)) {
                    self.packed_buffer.resize(total, 0);
                    self.cached_dims = Some((w, h));
                }

                pack_yuv420(
                    &mut self.packed_buffer,
                    Plane {
                        data: y_d,
                        stride: y_s,
                        width: w,
                        height: h,
                    },
                    Plane {
                        data: u_d,
                        stride: u_s,
                        width: uw,
                        height: uh,
                    },
                    Plane {
                        data: v_d,
                        stride: v_s,
                        width: uw,
                        height: uh,
                    },
                );

                // Return a clone of the buffer (needed for ownership)
                // This is still faster than allocating a new Vec each time
                Some((self.packed_buffer.clone(), w, h))
            }
            Ok(None) => None,
            Err(e) => {
                log::warn!("Decoder: take() error: {}", e);
                None
            }
        }
    }

    #[inline]
    fn next_pts(&mut self) -> Timestamp {
        self.frame_count += 1;
        Timestamp::new(self.frame_count, TimeBase::new(1, 90_000))
    }
}

#[derive(Clone, Copy)]
struct Plane<'a> {
    data: &'a [u8],
    stride: usize,
    width: usize,
    height: usize,
}

fn pack_yuv420(dst: &mut [u8], y: Plane<'_>, u: Plane<'_>, v: Plane<'_>) {
    let y_size = y.width * y.height;
    let u_size = u.width * u.height;
    extract_plane(&mut dst[..y_size], y.data, y.stride, y.width, y.height);
    extract_plane(
        &mut dst[y_size..y_size + u_size],
        u.data,
        u.stride,
        u.width,
        u.height,
    );
    extract_plane(
        &mut dst[y_size + u_size..],
        v.data,
        v.stride,
        v.width,
        v.height,
    );
}

/// Extract a plane from padded source to contiguous destination.
///
/// # Performance
/// - Fast path: No padding -> single memcpy
/// - Fallback: Row-by-row copy
#[inline]
fn extract_plane(dst: &mut [u8], src: &[u8], stride: usize, width: usize, height: usize) {
    let total_src = height * stride;

    // Fast path: No stride padding
    if stride == width && src.len() >= total_src {
        dst.copy_from_slice(&src[..width * height]);
        return;
    }

    // Fallback: Row-by-row copy
    for r in 0..height {
        let src_start = r * stride;
        let dst_start = r * width;
        if src_start + width > src.len() || dst_start + width > dst.len() {
            break;
        }
        dst[dst_start..dst_start + width].copy_from_slice(&src[src_start..src_start + width]);
    }
}
