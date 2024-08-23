use crate::gui::resource::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH};
use crate::workers;
use chrono::Local;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::{ClockTime, Element, Pipeline};
use image::RgbaImage;
use tokio::sync::mpsc::error::TrySendError;

pub fn create_view_pipeline(mut rx: tokio::sync::mpsc::Receiver<RgbaImage>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-video")
        .build().unwrap();
    src.set_property(
        "caps",
        &gst::Caps::builder("video/x-raw")
            .field("format", &"RGBA")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let video_convert = gst::ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build()
        .unwrap();

    let video_queue = gst::ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("max-size-buffers", "120")
        .property_from_str("max-size-time", "10")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("appsink")
        .name("appsink")
        .build().unwrap();

    sink.set_property(
        "caps",
        &gst::Caps::builder("video/x-raw")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("format", &"RGBA")
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let video_elements = [&src, &video_convert, &video_queue, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    let mut frame_i: u64 = 0;
    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");

    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx.blocking_recv() {
                    Some(frame) => {
                        if workers::save_stream::get_instance().lock().unwrap().is_saving {
                            workers::save_stream::get_instance().lock().unwrap().send_frame(frame.clone());
                        }

                        // Convert the image buffer into raw byte data
                        let raw_data: Vec<u8> = frame.into_raw();

                        // Create a GStreamer buffer from the raw data slice
                        let mut buffer = gst::Buffer::from_slice(raw_data);
                        {
                            let buffer_ref = buffer.get_mut().unwrap();

                            // Calculate PTS and duration based on frame rate
                            let pts = ClockTime::from_mseconds(1000 * frame_i / FRAME_RATE as u64);
                            let duration = ClockTime::from_mseconds(1000 * (1 / FRAME_RATE) as u64);

                            buffer_ref.set_pts(pts);
                            buffer_ref.set_dts(pts);
                            buffer_ref.set_duration(duration);
                        }

                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                    }
                    _ => {}
                }
                frame_i += 1;
            }).build(),
    );

    Ok(pipeline)
}

pub fn create_save_pipeline() -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-file")
        .build().unwrap();

    src.set_property(
        "caps",
        &gst::Caps::builder("video/x-raw")
            .field("format", &"RGBA")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let video_convert = gst::ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build()
        .unwrap();

    let video_encoder = gst::ElementFactory::make("x264enc")
        .name("x264enc")
        .property_from_str("pass", "quant")
        .property_from_str("tune", "zerolatency")
        .property("quantizer", 0u32)
        .property("threads", 8u32)
        .build().unwrap();

    let video_queue = gst::ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("max-size-buffers", "120")
        .property_from_str("max-size-time", "10")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .build().unwrap();

    let mp4_muxer = gstreamer::ElementFactory::make("mp4mux")
        .build().unwrap();

    let video_elements = [&src, &video_convert, &video_queue, &video_encoder, &h264parse, &mp4_muxer, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    Ok(pipeline)
}

pub fn create_stream_pipeline(mut rx_frames: tokio::sync::mpsc::Receiver<RgbaImage>, mut tx_processed: tokio::sync::mpsc::Sender<gstreamer::Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("webrtc-pipeline")
        .property("is-live", &true)
        .build().unwrap();
    src.set_property(
        "caps",
        &gst::Caps::builder("video/x-raw")
            .field("format", &"RGBA")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let video_convert = gst::ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build()
        .unwrap();

    let video_encoder = gst::ElementFactory::make("x264enc")
        .name("x264enc")
        .property_from_str("pass", "quant")
        .property_from_str("tune", "zerolatency")
        .property("quantizer", 0u32)
        .property("threads", 8u32)
        .build().unwrap();

    let video_queue = gst::ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("max-size-buffers", "120")
        .property_from_str("max-size-time", "10")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &false)
        .property("emit-signals", &true)
        .build().unwrap();
    sink.set_property(
        "caps",
        &gst::Caps::builder("video/x-h264")
            .field("stream-format", &"byte-stream")
            //.field("alignment", &"au")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let video_elements = [&src, &video_convert, &video_queue, &video_encoder, &h264parse, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    let mut frame_i: u64 = 0;
    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");
    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx_frames.blocking_recv() {
                    Some(frame) => {
                        // Convert the image buffer into raw byte data
                        let raw_data: Vec<u8> = frame.into_raw();

                        // Create a GStreamer buffer from the raw data slice
                        let mut buffer = gst::Buffer::from_slice(raw_data);
                        {
                            let buffer_ref = buffer.get_mut().unwrap();

                            // Calculate PTS and duration based on frame rate
                            let pts = ClockTime::from_mseconds(1000 * frame_i / FRAME_RATE as u64);
                            let duration = ClockTime::from_mseconds(1000 * (1 / FRAME_RATE) as u64);

                            buffer_ref.set_pts(pts);
                            buffer_ref.set_dts(pts);
                            buffer_ref.set_duration(duration);
                        }

                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                    }
                    _ => {}
                }
                frame_i += 1;
            }).build(),
    );

    let appsink = sink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("Source element is expected to be an appsink!");

    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder().
            new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?.to_owned();

                match tx_processed.try_send(buffer) {
                    Ok(_) => {}
                    Err(TrySendError::Closed(_)) => {
                        eprintln!("Receiver channel dropped: create_stream_pipeline");
                    }
                    _ => {}
                };

                Ok(gst::FlowSuccess::Ok)
            }).build(),
    );
    Ok(pipeline)
}

pub fn create_ss_save_pipeline(mut rx_processed: tokio::sync::mpsc::Receiver<gstreamer::Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-file")
        .property("is-live", &true)
        .build().unwrap();

    src.set_property(
        "caps",
        &gst::Caps::builder("video/x-h264")
            .field("stream-format", &"byte-stream")
            //.field("alignment", &"au")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
            .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
            .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
            .build(),
    );

    let sink = gstreamer::ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .build().unwrap();

    let mp4_muxer = gstreamer::ElementFactory::make("mp4mux")
        .build().unwrap();

    let video_elements = [&src, &h264parse, &mp4_muxer, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");

    let mut frame_n = 0;
    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx_processed.blocking_recv() {
                    Some(buffer) => {
                        println!("saving..........");
                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                    }
                    _ => {}
                }

                if frame_n > 80 {
                    appsrc.end_of_stream().expect("Failed to send EOS");
                }

                frame_n += 1;
            }).build(),
    );

    Ok(pipeline)
}