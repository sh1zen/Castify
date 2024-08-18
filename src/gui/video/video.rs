use crate::gui::video::Error;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer::Pipeline;
use gstreamer_app as gst_app;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
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
    pub(crate) id: u64,

    pub(crate) bus: gst::Bus,
    pub(crate) source: Pipeline,

    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) framerate: f64,
    pub(crate) duration: std::time::Duration,

    pub(crate) frame: Arc<Mutex<Vec<u8>>>,
    pub(crate) upload_frame: Arc<AtomicBool>,
    pub(crate) wait: mpsc::Receiver<()>,
    pub(crate) notify: mpsc::Sender<()>,
    pub(crate) paused: bool,
    pub(crate) muted: bool,
    pub(crate) looping: bool,
    pub(crate) is_eos: bool,
    pub(crate) restart_stream: bool,
    pub(crate) next_redraw: Instant,
}

impl Internal {
    pub(crate) fn seek(&self, position: impl Into<Position>) -> Result<(), Error> {
        self.source.seek_simple(
            gst::SeekFlags::FLUSH,
            gst::GenericFormattedValue::from(position.into()),
        )?;
        Ok(())
    }

    pub(crate) fn restart_stream(&mut self) -> Result<(), Error> {
        self.is_eos = false;
        self.set_paused(false);
        self.seek(0)?;
        Ok(())
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
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
pub struct Video(pub(crate) RefCell<Internal>);

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
    // let pipeline = format!("uridecodebin uri=\"{}\" ! videoconvert ! videoscale ! appsink name=iced_video caps=video/x-raw,format=RGBA,pixel-aspect-ratio=1/1", uri.as_str());

    pub fn new() -> Result<Self, Error> {
        let (notify, wait) = mpsc::channel();
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        Ok(Video(RefCell::new(Internal {
            id,
            bus: Default::default(),
            source: Default::default(),
            width: 0,
            height: 0,
            framerate: 0.0,
            duration: Default::default(),
            frame: Arc::new(Mutex::new(vec![])),
            upload_frame: Arc::new(Default::default()),
            wait,
            notify,
            paused: false,
            muted: false,
            looping: false,
            is_eos: false,
            restart_stream: false,
            next_redraw: Instant::now(),
        })))
    }

    pub fn set_pipeline(&mut self, pipeline: Pipeline) {
        let live = true;

        let app_sink_name = "appsink";
        let app_sink = pipeline
            .by_name(app_sink_name)
            .and_then(|elem| elem.downcast::<gst_app::AppSink>().ok())
            .unwrap();

        let pad = app_sink.pads().first().cloned().unwrap();

        pipeline.set_state(gst::State::Playing).unwrap();

        // wait for up to 5 seconds until the decoder gets the source capabilities
        pipeline.state(gst::ClockTime::from_seconds(5));

        // extract resolution and framerate
        let caps = pad.current_caps().ok_or(Error::Caps).unwrap();
        let s = caps.structure(0).ok_or(Error::Caps).unwrap();
        let width = s.get::<i32>("width").map_err(|_| Error::Caps).unwrap();
        let height = s.get::<i32>("height").map_err(|_| Error::Caps).unwrap();
        let framerate = s
            .get::<gst::Fraction>("framerate")
            .map_err(|_| Error::Caps).unwrap();

        let duration = if !live {
            std::time::Duration::from_nanos(
                pipeline
                    .query_duration::<gst::ClockTime>()
                    .ok_or(Error::Duration).unwrap()
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

        let (notify, wait) = mpsc::channel();

        app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    frame_ref
                        .lock()
                        .map_err(|_| gst::FlowError::Error)?
                        .copy_from_slice(map.as_slice());

                    upload_frame_ref.store(true, Ordering::SeqCst);

                    notify.send(()).map_err(|_| gst::FlowError::Error)?;

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        let mut zante = self.0.borrow_mut();

        zante.bus = pipeline.bus().unwrap();
        zante.source = pipeline;
        zante.wait = wait;
        zante.width = width;
        zante.height = height;
        zante.framerate = framerate.numer() as f64 / framerate.denom() as f64;
        zante.duration = duration;
        zante.frame = frame;
        zante.upload_frame = upload_frame;
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
    pub fn seek(&mut self, position: impl Into<Position>) -> Result<(), Error> {
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
    pub fn restart_stream(&mut self) -> Result<(), Error> {
        self.0.borrow_mut().restart_stream()
    }
}
