use gstreamer as gst;
use gstreamer::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gstreamer::Pipeline;
use gstreamer_app as gst_app;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Position in the media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    /// Position based on time.
    ///
    /// Not the most accurate format for videos.
    Time(std::time::Duration),
    /// Position based on nth frame.
    Frame(u64),
}

impl From<Position> for gst::GenericFormattedValue {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Time(t) => gst::ClockTime::from_nseconds(t.as_nanos() as _).into(),
            Position::Frame(f) => gst::format::Default::from_u64(f).into(),
        }
    }
}

impl From<std::time::Duration> for Position {
    fn from(t: std::time::Duration) -> Self {
        Position::Time(t)
    }
}

impl From<u64> for Position {
    fn from(f: u64) -> Self {
        Position::Frame(f)
    }
}

pub struct Internal {
    pub id: u64,

    pub bus: gst::Bus,
    source: Pipeline,

    pub width: i32,
    pub height: i32,
    pub framerate: f64,
    pub duration: std::time::Duration,

    pub frame: Arc<Mutex<Vec<u8>>>,
    pub upload_frame: Arc<AtomicBool>,
    pub paused: bool,
    pub muted: bool,
    pub looping: bool,
    pub is_eos: bool,
    pub restart_stream: bool,
    pub next_redraw: Instant,
}

impl Internal {
    pub fn seek(&self, position: impl Into<Position>) {
        self.source.seek_simple(
            gst::SeekFlags::FLUSH,
            gst::GenericFormattedValue::from(position.into()),
        ).expect("Cannot seek into desired position");
    }

    pub fn restart_stream(&mut self) {
        self.is_eos = false;
        self.set_paused(false);
        self.seek(0);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.source
            .set_state(if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            })
            .unwrap(/* state was changed in ctor; state errors caught there */);
        self.paused = paused;

        // Set restart_stream flag to make the stream restart on the next Message::NextFrame
        if self.is_eos && !paused {
            self.restart_stream = true;
        }
    }
}

/// A multimedia video loaded from a URI (e.g., a local file path or HTTP stream).
pub struct Video(pub RefCell<Internal>);

impl Drop for Video {
    fn drop(&mut self) {
        self.0
            .borrow()
            .source
            .set_state(gst::State::Null)
            .expect("failed to set state");
    }
}

impl Video {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        Video(RefCell::new(Internal {
            id,
            bus: Default::default(),
            source: Default::default(),
            width: 0,
            height: 0,
            framerate: 0.0,
            duration: Default::default(),
            frame: Arc::new(Mutex::new(vec![])),
            upload_frame: Arc::new(Default::default()),
            paused: false,
            muted: false,
            looping: false,
            is_eos: false,
            restart_stream: false,
            next_redraw: Instant::now(),
        }))
    }

    pub fn set_pipeline(&mut self, pipeline: Pipeline, width: i32, height: i32, framerate: gst::Fraction) {
        let live = true;

        let app_sink = pipeline
            .by_name("appsink")
            .and_then(|elem| elem.downcast::<gst_app::AppSink>().ok())
            .unwrap();

        pipeline.set_state(gst::State::Playing).unwrap();

        let duration = if !live {
            std::time::Duration::from_nanos(
                pipeline
                    .query_duration::<gst::ClockTime>()
                    .ok_or("Failed to query media duration or position").unwrap()
                    .nseconds(),
            )
        } else {
            std::time::Duration::from_secs(0)
        };

        let frame_buf = vec![0; (width * height * 4) as _];
        let frame = Arc::new(Mutex::new(frame_buf));
        let frame_ref = Arc::clone(&frame);

        let upload_frame = Arc::new(AtomicBool::new(true));
        let upload_frame_ref = Arc::clone(&upload_frame);

        app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?.to_owned();
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    frame_ref
                        .lock()
                        .map_err(|_| gst::FlowError::Error)?
                        .copy_from_slice(map.as_slice());

                    upload_frame_ref.store(true, Ordering::SeqCst);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        let mut zante = self.0.borrow_mut();

        zante.bus = pipeline.bus().unwrap();
        zante.source = pipeline;
        zante.width = width;
        zante.height = height;
        zante.framerate = framerate.numer() as f64 / framerate.denom() as f64;
        zante.duration = duration;
        zante.frame = frame;
        zante.upload_frame = upload_frame;
    }

    pub fn get_pipeline(&mut self) -> &Pipeline {
        &self.0.get_mut().source
    }

    /// Get the size/resolution of the video as `(width, height)`.
    #[inline(always)]
    pub fn size(&self) -> (i32, i32) {
        (self.0.borrow().width, self.0.borrow().height)
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    pub fn framerate(&self) -> f64 {
        self.0.borrow().framerate
    }

    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    ///
    /// This uses a linear scale, for example `0.5` is perceived as half as loud.
    pub fn set_volume(&mut self, volume: f64) {
        self.0.borrow().source.set_property("volume", volume);
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&mut self, muted: bool) {
        let mut inner = self.0.borrow_mut();
        inner.muted = muted;
        inner.source.set_property("mute", muted);
    }

    /// Get if the audio is muted or not.
    #[inline(always)]
    pub fn muted(&self) -> bool {
        self.0.borrow().muted
    }

    /// Get if the stream ended or not.
    #[inline(always)]
    pub fn eos(&self) -> bool {
        self.0.borrow().is_eos
    }

    /// Get if the media will loop or not.
    #[inline(always)]
    pub fn looping(&self) -> bool {
        self.0.borrow().looping
    }

    /// Set if the media will loop or not.
    #[inline(always)]
    pub fn set_looping(&mut self, looping: bool) {
        self.0.borrow_mut().looping = looping;
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&mut self, paused: bool) {
        let mut inner = self.0.borrow_mut();
        inner.set_paused(paused);
    }

    /// Get if the media is paused or not.
    #[inline(always)]
    pub fn paused(&self) -> bool {
        self.0.borrow().paused
    }

    /// Jumps to a specific position in the media.
    /// The seeking is not perfectly accurate.
    pub fn seek(&mut self, position: impl Into<Position>) {
        self.0.borrow_mut().seek(position)
    }

    /// Get the current playback position in time.
    pub fn position(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(
            self.0
                .borrow()
                .source
                .query_position::<gst::ClockTime>()
                .map_or(0, |pos| pos.nseconds()),
        )
    }

    /// Get the media duration.
    #[inline(always)]
    pub fn duration(&self) -> std::time::Duration {
        self.0.borrow().duration
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart_stream(&mut self) {
        self.0.borrow_mut().restart_stream()
    }
}
