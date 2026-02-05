use ac_ffmpeg::codec::video::{PixelFormat, VideoFrame, VideoFrameMut};
use ac_ffmpeg::time::TimeBase;
use std::collections::VecDeque;

/// Frame pool for reusing video frames to avoid allocations.
///
/// # Performance
/// - Pre-allocates frames on creation to avoid runtime allocation
/// - Reuses frames via put()/take() cycle
/// - Falls back to allocation if pool is exhausted
pub(crate) struct FramePool {
    frames: VecDeque<VideoFrame>,
    w: usize,
    h: usize,
    time_base: TimeBase,
    pixel_format: PixelFormat,
}

/// Initial number of frames to pre-allocate in the pool
const INITIAL_POOL_SIZE: usize = 4;

impl FramePool {
    pub fn new(w: usize, h: usize, time_base: TimeBase, pixel_format: PixelFormat) -> Self {
        let mut frames = VecDeque::with_capacity(INITIAL_POOL_SIZE);

        // Pre-allocate initial frames to avoid runtime allocation
        for _ in 0..INITIAL_POOL_SIZE {
            let frame = VideoFrameMut::black(pixel_format, w, h)
                .with_time_base(time_base)
                .freeze();
            frames.push_back(frame);
        }

        Self {
            frames,
            w,
            h,
            time_base,
            pixel_format,
        }
    }

    /// Put a given frame back to the pool after it was used.
    #[inline]
    pub fn put(&mut self, frame: VideoFrame) {
        // Only keep frames if pool isn't too large (prevent unbounded growth)
        if self.frames.len() < INITIAL_POOL_SIZE * 2 {
            self.frames.push_back(frame);
        }
    }

    /// Take a writable frame from the pool or allocate a new one if necessary.
    #[inline]
    pub fn take(&mut self) -> VideoFrameMut {
        // Try to reuse a frame from the pool.
        // Scan each currently pooled frame at most once to avoid spinning.
        let available = self.frames.len();
        for _ in 0..available {
            let Some(frame) = self.frames.pop_front() else {
                break;
            };
            match frame.try_into_mut() {
                Ok(frame) => return frame,
                Err(frame) => {
                    // Frame is still in use, keep it in the pool.
                    self.frames.push_back(frame);
                }
            }
        }

        // Pool exhausted, allocate a new frame
        VideoFrameMut::black(self.pixel_format, self.w, self.h).with_time_base(self.time_base)
    }
}
