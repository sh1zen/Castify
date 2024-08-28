use crate::gui::resource::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH};
use crate::workers;
use chrono::Local;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::{Buffer, ClockTime, Element, Pipeline};
use image::RgbaImage;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::Receiver;
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;

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
        //.property("is-live", &true)
        .property(
            "caps",
            &gst::Caps::builder("video/x-raw")
                .field("format", &"RGBA")
                .field("width", &FRAME_WITH)
                .field("height", &FRAME_HEIGHT)
                .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
                .field("framerate", &gst::Fraction::new(FRAME_RATE, 1))
                .build(),
        )
        .build().unwrap();

    let video_convert = gst::ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build().unwrap();

    let vc_caps_filter = gst::ElementFactory::make("capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", &"I420")
                      .build(),
        ).build().unwrap();

    let video_encoder = gst::ElementFactory::make("x264enc")
        .name("x264enc")
        .property_from_str("pass", "quant")
        .property_from_str("tune", "zerolatency")
        .property_from_str("speed-preset", "ultrafast")
        .property("quantizer", 10u32)
        .property("threads", 8u32)
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .property("disable-passthrough", &true)
        .build().unwrap();

    let rtph264pay = gstreamer::ElementFactory::make("rtph264pay")
        .property("config-interval", &1)
        .property("pt", &96u32)
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &true)
        .property("emit-signals", &false)
        .property("caps",
                  &gst::Caps::builder("application/x-rtp")
                      //.field("stream-format", &"byte-stream")
                      .field("media", &"video")
                      .field("clock-rate", &90000)
                      .field("encoding-name", &"H264")
                      .field("payload", &96i32)
                      //.field("a-framerate", &gst::Fraction::new(FRAME_RATE, 1))
                      .build(),
        )
        .build().unwrap();

    let video_queue1 = gst::ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();
    let video_queue2 = gst::ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();
    let video_queue3 = gst::ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();


    let video_elements = [&src, &video_convert, &vc_caps_filter, &video_encoder, &h264parse, &video_queue1, &rtph264pay, &video_queue2, &sink];

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
                            let pts = ClockTime::from_mseconds(1_000 * frame_i / FRAME_RATE as u64);
                            let duration = ClockTime::from_mseconds((1_000f32 / FRAME_RATE as f32) as u64);

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
        .expect("Element is expected to be an appsink!");
    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder().
            new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?.to_owned();

                match tx_processed.try_send(buffer) {
                    Ok(_) => {
                        println!("server pipeline pushed frame");
                    }
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


pub fn create_ss_save_pipeline(mut rx_processed: tokio::sync::mpsc::Receiver<Packet>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-file")
        .property("is-live", &true)
        // .property("block", &false)
        //.property("stream-type", AppStreamType::Stream)
        //.property("format", &gstreamer::Format::Time)
        .build().expect("INVALID APPSRC");
    src.set_property(
        "caps",
        &gst::Caps::builder("application/x-rtp")
            .field("media", &"video")
            .field("clock-rate", &90000)
            .field("encoding-name", &"H264")
            .field("payload", &102i32)
            .build(),
    );

    let rtpjitterbuffer = gstreamer::ElementFactory::make("rtpjitterbuffer")
        .property_from_str("latency", "100")
        .property("do-retransmission", true)
        .property("drop-on-latency", false)
        .build().unwrap();

    let rtph264depay = gstreamer::ElementFactory::make("rtph264depay")
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();

    let video_queue = gst::ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("max-size-buffers", "260")
        .property_from_str("max-size-time", "10")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let mp4_muxer = gstreamer::ElementFactory::make("mp4mux")
        .build().unwrap();

    let video_elements = [&src, &rtpjitterbuffer, &rtph264depay, &h264parse, &video_queue, &mp4_muxer, &sink];

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
    appsrc.set_min_latency((1000 / FRAME_RATE) as i64);

    let mut frame_i = 0;
    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx_processed.blocking_recv() {
                    Some(packet) => {
                        // Converti il pacchetto RTP in un buffer GStreamer
                        let payload = packet.marshal().unwrap();

                        // Create a GStreamer buffer from the raw data slice
                        let mut buffer = gst::Buffer::from_slice(payload);
                        {
                            let buffer_ref = buffer.get_mut().unwrap();
                            // Imposta il timestamp corretto per il buffer 1_000_000_000 / 90_000
                            let pts = ClockTime::from_mseconds(packet.header.timestamp as u64 * (100 / 9));
                            buffer_ref.set_pts(pts);
                            buffer_ref.set_dts(pts);
                        }


                        // Invia il buffer a GStreamer
                        if let Err(e) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", e);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }

                        if frame_i > 1000 {
                            println!("Setting eos");
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                        frame_i += 1;
                    }
                    _ => {}
                }
            }).build(),
    );
    Ok(pipeline)
}

pub fn create_stream_view_pipeline(mut rx: tokio::sync::mpsc::Receiver<gstreamer::Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-video")
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
            .field("stream-format", &"byte-stream")
            //.field("format", &"RGBA")
            .field("width", &FRAME_WITH)
            .field("height", &FRAME_HEIGHT)
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

    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");

    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx.blocking_recv() {
                    Some(buffer) => {
                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                    }
                    _ => {}
                }
            }).build(),
    );

    Ok(pipeline)
}


pub fn create_test_save_pipeline(mut rx_processed: Receiver<Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = gst::ElementFactory::make("appsrc")
        .name("image-to-file")
        .property("is-live", &true)
        .property("caps",
                  &gst::Caps::builder("application/x-rtp")
                      //.field("stream-format", &"byte-stream")
                      .field("media", &"video")
                      .field("clock-rate", &90000)
                      .field("encoding-name", &"H264")
                      .field("payload", &96i32)
                      //.field("a-framerate", &gst::Fraction::new(FRAME_RATE, 1))
                      .build(),
        )
        .build().unwrap();

    let sink = gstreamer::ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();


    let rtpjitterbuffer = gstreamer::ElementFactory::make("rtpjitterbuffer")
        .property_from_str("latency", "100")
        .property("do-retransmission", false)
        .property("drop-on-latency", false)
        .build().unwrap();

    let rtph264depay = gstreamer::ElementFactory::make("rtph264depay")
        .build().unwrap();

    let h264parse = gstreamer::ElementFactory::make("h264parse")
        .property("disable-passthrough", &true)
        .build().unwrap();

    let mp4_muxer = gstreamer::ElementFactory::make("mp4mux")
        .build().unwrap();

    let video_queue1 = gst::ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();
    let video_queue2 = gst::ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let video_elements = [&src, &rtph264depay, &video_queue1, &h264parse, &mp4_muxer, &sink];

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
    appsrc.set_format(gstreamer::Format::Time);

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

