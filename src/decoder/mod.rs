mod depacketizer;
mod ffmpeg;

pub use depacketizer::H264Depacketizer;
pub use ffmpeg::FfmpegDecoder;

pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
