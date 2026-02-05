use crate::capture::{NV12FrameRef, YUVFrame};
use crate::encoder::frame_pool::FramePool;
use ac_ffmpeg::codec::video::VideoEncoder;
use ac_ffmpeg::codec::{Encoder, video};
use ac_ffmpeg::time::{TimeBase, Timestamp};
use bytes::Bytes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Encoder fallback chain: try hardware encoders first, then software.
/// Optimized for low latency streaming with hardware acceleration.
///
/// PERFORMANCE NOTES:
/// - NVENC: Use p1 (fastest preset), ull tune for ultra-low latency
/// - QSV: Use veryfast preset with low_power mode for efficiency
/// - AMF: Use ultralowlatency usage with speed quality preset
/// - libx264: Use ultrafast + zerolatency for CPU fallback
const ENCODER_CHAIN: &[(&str, &[(&str, &str)])] = &[
    // NVIDIA NVENC - Best performance for NVIDIA GPUs
    (
        "h264_nvenc",
        &[
            ("preset", "p2"),       // Faster preset with better quality than p1
            ("tune", "ll"),         // Low latency tuning (better quality than ull)
            ("zerolatency", "1"),   // Zero latency operation
            ("rc", "vbr"),          // Variable bitrate for quality
            ("b", "3500000"),       // Target bitrate: 3.5 Mbps
            ("maxrate", "5500000"), // Max bitrate: 5.5 Mbps
            ("bufsize", "7000000"), // Buffer size: ~2x target
            ("g", "60"),            // GOP size: 2 seconds at 30fps
            ("gpu", "0"),           // Use first GPU
            ("delay", "0"),         // Zero delay
            ("forced-idr", "1"),    // Allow forced IDR frames
        ],
    ),
    // Intel Quick Sync Video - Good for Intel iGPUs
    (
        "h264_qsv",
        &[
            ("preset", "fast"),     // Faster preset with better quality
            ("g", "60"),            // GOP size
            ("b", "3000000"),       // Target bitrate
            ("maxrate", "4500000"), // Max bitrate
            ("bufsize", "6000000"), // Buffer size
            ("low_power", "0"),     // Disable low power mode for better quality
            ("async_depth", "4"),   // Increased async depth for better performance
            ("look_ahead", "1"),    // Enable look-ahead for better quality
        ],
    ),
    // AMD AMF - For AMD GPUs
    (
        "h264_amf",
        &[
            ("usage", "lowlatency"), // Low latency mode (better quality than ultralowlatency)
            ("quality", "balanced"), // Balanced quality and speed
            ("b", "3000000"),        // Target bitrate
            ("maxrate", "4500000"),  // Max bitrate
            ("bufsize", "6000000"),  // Buffer size
            ("rc", "vbr_peak"),      // VBR with peak constraint
            ("g", "60"),             // GOP size
            ("preanalysis", "1"),    // Enable pre-analysis for better quality
            ("frame_skipping", "0"), // No frame skipping
        ],
    ),
    // libx264 - CPU fallback (always available)
    (
        "libx264",
        &[
            ("profile", "main"),     // Main profile for better quality
            ("preset", "fast"),      // Faster preset with better quality than ultrafast
            ("tune", "zerolatency"), // Zero latency tuning
            ("crf", "21"),           // Higher quality baseline
            ("maxrate", "5000000"),  // Max 5 Mbps
            ("bufsize", "8000000"),  // Buffer size
            ("keyint", "60"),        // Keyframe every 2 seconds
            ("min-keyint", "30"),    // Minimum 1 second between keyframes
            ("scenecut", "40"),      // Scene change detection
            ("threads", "0"),        // Auto-detect thread count
            ("sliced-threads", "1"), // Better for low latency
            ("sync-lookahead", "0"), // Disable lookahead
            ("bframes", "0"),        // No B-frames for low latency
        ],
    ),
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

pub enum FrameData<'a> {
    NV12(&'a YUVFrame),
    NV12Ref(NV12FrameRef<'a>),
    BGR0(&'a [u8]),
}

impl FfmpegEncoder {
    pub fn new(w: u32, h: u32) -> Self {
        let w = if w.is_multiple_of(2) { w } else { w + 1 } as usize;
        let h = if h.is_multiple_of(2) { h } else { h + 1 } as usize;
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

    /// Encode a frame to H.264 Annex B format.
    ///
    /// # Performance Optimizations
    /// - Uses frame pooling to avoid allocations
    /// - Optimized NV12 copy with fast path for matching strides
    /// - Pre-allocated output buffer with capacity hint
    pub fn encode(
        &mut self,
        frame_data: FrameData,
        frame_time: i64,
    ) -> Result<Bytes, anyhow::Error> {
        let mut frame = self.frame_pool.take();
        let time_base = frame.time_base();
        frame = frame
            .with_pts(self.pts_from_frame_time(frame_time, time_base))
            .with_picture_type(self.next_picture_type());

        match frame_data {
            FrameData::NV12(nv12) => self.write_nv12_planes(&mut frame, nv12),
            FrameData::NV12Ref(nv12) => self.write_nv12_ref_planes(&mut frame, nv12),
            FrameData::BGR0(bgr0) => match self.pixel_format.as_str() {
                "bgra" => {
                    let mut planes = frame.planes_mut();
                    planes[0].data_mut().copy_from_slice(bgr0);
                }
                _ => unimplemented!(),
            },
        }

        // Freeze the frame and push to encoder
        // Note: We don't clone here - the encoder takes ownership temporarily
        // and we get it back via take() for reuse in the frame pool
        let frame = frame.freeze();
        self.encoder.push(frame.clone())?;
        self.frame_pool.put(frame);

        // Pre-allocate output buffer with capacity hint
        // Typical encoded frame size: ~10-50KB for 800Kbps @ 30fps
        let mut ret = Vec::with_capacity(32 * 1024);
        while let Some(packet) = self.encoder.take()? {
            ret.extend_from_slice(packet.data());
        }
        Ok(Bytes::from(ret))
    }

    #[inline]
    fn pts_from_frame_time(&self, frame_time: i64, time_base: TimeBase) -> Timestamp {
        let pts = if cfg!(target_os = "windows") {
            (frame_time as f64 * 9.0 / 1000.0) as i64
        } else if cfg!(target_os = "macos") {
            (frame_time as f64 * 9.0 / 1e5) as i64
        } else {
            panic!("Unsupported OS")
        };
        Timestamp::new(pts, time_base)
    }

    #[inline]
    fn next_picture_type(&self) -> video::frame::PictureType {
        if self.force_idr.swap(false, Ordering::Relaxed) {
            video::frame::PictureType::I
        } else {
            video::frame::PictureType::None
        }
    }

    fn write_nv12_planes(&self, frame: &mut video::VideoFrameMut, nv12: &YUVFrame) {
        // Y plane (luminance): full size
        {
            let mut planes = frame.planes_mut();
            let y_plane = planes[0].data_mut();
            let y_line_size = y_plane.len() / self.h;
            self.copy_nv12_optimized(
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
            self.copy_nv12_optimized(
                &nv12.chrominance_bytes,
                nv12.chrominance_stride as usize,
                uv_line_size,
                uv_h,
                uv_plane,
            );
        }
    }

    fn write_nv12_ref_planes(&self, frame: &mut video::VideoFrameMut, nv12: NV12FrameRef<'_>) {
        // Y plane (luminance): full size
        {
            let mut planes = frame.planes_mut();
            let y_plane = planes[0].data_mut();
            let y_line_size = y_plane.len() / self.h;
            self.copy_nv12_optimized(
                nv12.luminance_bytes,
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
            self.copy_nv12_optimized(
                nv12.chrominance_bytes,
                nv12.chrominance_stride as usize,
                uv_line_size,
                uv_h,
                uv_plane,
            );
        }
    }

    /// Optimized NV12 plane copy with fast paths for common cases.
    ///
    /// # Performance
    /// - Fast path 1: Exact stride match -> single memcpy
    /// - Fast path 2: Contiguous source -> single memcpy with offset
    /// - Fallback: Row-by-row copy with bounds checking
    #[inline]
    fn copy_nv12_optimized(
        &self,
        source: &[u8],
        stride: usize,
        encoder_line_size: usize,
        rows: usize,
        destination: &mut [u8],
    ) {
        let copy_width = self.w.min(stride).min(encoder_line_size);
        let total_src = rows * stride;
        let total_dst = rows * encoder_line_size;

        // Fast path 1: Exact stride match and correct sizes
        if stride == encoder_line_size
            && source.len() >= total_src
            && destination.len() >= total_dst
        {
            destination[..total_dst].copy_from_slice(&source[..total_dst]);
            return;
        }

        // Fast path 2: No stride padding (contiguous data)
        if stride == copy_width && encoder_line_size == copy_width {
            let copy_len = copy_width * rows;
            if source.len() >= copy_len && destination.len() >= copy_len {
                destination[..copy_len].copy_from_slice(&source[..copy_len]);
                return;
            }
        }

        // Fallback: Row-by-row copy with pre-computed bounds
        let src_end = source.len().saturating_sub(copy_width);
        let dst_end = destination.len().saturating_sub(copy_width);

        for r in 0..rows {
            let src_start = r * stride;
            let dst_start = r * encoder_line_size;

            // Bounds check with early termination
            if src_start > src_end || dst_start > dst_end {
                break;
            }

            destination[dst_start..dst_start + copy_width]
                .copy_from_slice(&source[src_start..src_start + copy_width]);
        }
    }
}
