use std::collections::VecDeque;
use image::{Rgba, RgbaImage};

pub(crate) struct FramePool {
    frames: VecDeque<RgbaImage>,
    w: usize,
    h: usize,
}

impl FramePool {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            frames: VecDeque::new(),
            w,
            h,
        }
    }

    /// Put a given frame back to the pool after it was used.
    pub fn put(&mut self, frame: RgbaImage) {
        self.frames.push_back(frame);
    }

    /// Take a writable frame from the pool or allocate a new one if necessary.
    pub fn take(&mut self) -> RgbaImage {
        if let Some(mut frame) = self.frames.pop_front() {
            return frame;
        }

        let mut frame = RgbaImage::new(self.w as u32, self.h as u32);
        for pixel in frame.pixels_mut() {
            *pixel = Rgba([128, 128, 128, 255]);
        }
        frame
    }
}