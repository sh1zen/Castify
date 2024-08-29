use crate::gui::resource::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, SAMPLING_RATE};
use chrono::Local;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::{Buffer, ClockTime, Element, ElementFactory, Fraction, Pipeline};
use image::RgbaImage;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::Receiver;
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;
use crate::workers;

pub fn create_view_pipeline(mut rx: Receiver<Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("h264-to-video")
        .property("is-live", &true)
        .property("block", &true)
        .property("format", &gstreamer::Format::Time)
        .property("do-timestamp", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-h264")
                      .field("stream-format", "byte-stream")
                      .field("alignment", "au") // Optional: set alignment if needed
                      .field("width", &FRAME_WITH)
                      .field("height", &FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
                      .field("framerate", &gst::Fraction::new(SAMPLING_RATE, 1))
                      .build()
        )
        .build().unwrap();

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", true)
        .property("config-interval", -1)
        .build().unwrap();

    let avdec_h264 = ElementFactory::make("avdec_h264")
        .build().unwrap();
    let videoconvert = ElementFactory::make("videoconvert")
        .build().unwrap();
    let capsfilter = ElementFactory::make("capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", "RGBA")
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .build(),
        )
        .build().unwrap();

    let sink = ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &false)
        .property("emit-signals", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("stream-format", "byte-stream")
                      .field("width", &FRAME_WITH)
                      .field("height", &FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
                      .field("framerate", &gst::Fraction::new(SAMPLING_RATE, 1))
                      .build()
        )
        .build().unwrap();

    let video_queue1 = ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();
    let video_queue2 = ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let video_elements = [&src, &video_queue1, &h264parse, &avdec_h264, &videoconvert, &capsfilter, &sink];

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
                        if workers::save_stream::get_instance().lock().unwrap().is_saving {
                            workers::save_stream::get_instance().lock().unwrap().send_frame(buffer.clone());
                        }
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

pub fn create_save_pipeline() -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("appsrc")
        .property("is-live", &true)
        .property("block", &true)
        .property("format", &gstreamer::Format::Time)
        .property("do-timestamp", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-h264")
                      .field("stream-format", "byte-stream")
                      .field("alignment", "au") // Optional: set alignment if needed
                      .field("width", &FRAME_WITH)
                      .field("height", &FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", &gst::Fraction::new(1, 1))
                      .field("framerate", &gst::Fraction::new(SAMPLING_RATE, 1))
                      .build()
        )
        .build().unwrap();

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", true)
        .property("config-interval", -1)
        .build().unwrap();

    let mp4_muxer = ElementFactory::make("mp4mux")
        .build().unwrap();

    let filesink = ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();

    let video_queue1 = ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let video_elements = [&src, &video_queue1, &h264parse,  &mp4_muxer, &filesink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    Ok(pipeline)
}

pub fn create_stream_pipeline(mut rx_frames: Receiver<RgbaImage>, mut tx_processed: tokio::sync::mpsc::Sender<Buffer>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("stream-pipeline")
        .property("is-live", &true)
        .property("block", &true)
        .property("format", &gst::Format::Time)
        .property("do-timestamp", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", &"RGBA")
                      .field("width", FRAME_WITH)
                      .field("height", FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .field("framerate", Fraction::new(FRAME_RATE, 1))
                      .build(),
        ).build().unwrap();

    let video_convert = ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build().unwrap();

    let caps_videoconvert = ElementFactory::make("capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", "I420")
                      .build(),
        ).build().unwrap();

    let video_encoder = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("bitrate", 1000u32)
        .property_from_str("pass", "quant")
        .property("quantizer", 20u32)
        .property_from_str("tune", "zerolatency")
        .property_from_str("speed-preset", "superfast")
        .property("key-int-max", 15u32)
        .build().unwrap();

    let video_queue = ElementFactory::make("queue")
        .property_from_str("leaky", "downstream")
        .build().unwrap();

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", &true)
        .property("config-interval", &-1) // Send SPS/PPS with every keyframe
        .build().unwrap();

    let rtph264pay = ElementFactory::make("rtph264pay")
        .property("config-interval", &-1)
        .property("pt", &96u32)
        .build().unwrap();

    let sink = ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &false)
        .property("emit-signals", &true)
        .property("caps",
                  &gst::Caps::builder("application/x-rtp")
                      .field("stream-format", &"byte-stream")
                      .field("media", &"video")
                      .field("clock-rate", &90000)
                      .field("encoding-name", &"H264")
                      .field("payload", &96i32)
                      .field("alignment", &"au")
                      .build(),
        ).build().unwrap();

    let sink = ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &false)
        .property("emit-signals", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-h264")
                      .field("stream-format", &"byte-stream")
                      .field("alignment", &"au")
                      .build(),
        ).build().unwrap();

    let video_elements = [&src, &video_convert, &video_encoder, &h264parse, &video_queue, &sink];

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
                match rx_frames.blocking_recv() {
                    Some(frame) => {
                        // Convert the image buffer into raw byte data
                        let raw_data: Vec<u8> = frame.into_raw();
                        // Create a GStreamer buffer from the raw data slice
                        let buffer = Buffer::from_slice(raw_data);

                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
                            appsrc.end_of_stream().expect("Failed to send EOS");
                        }
                    }
                    _ => {}
                }
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

                let c_buf = buffer.clone();
                match tx_processed.try_send(buffer) {
                    Ok(_) => {
                        println!("Pipeline1 Sending ... {:?}", c_buf)
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


pub fn create_ss_save_pipeline(mut rx_processed: Receiver<Packet>) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("image-to-file")
        .property("is-live", &true)
        // .property("block", &false)
        //.property("stream-type", AppStreamType::Stream)
        //.property("format", &gst::Format::Time)
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

    let rtpjitterbuffer = ElementFactory::make("rtpjitterbuffer")
        .property_from_str("latency", "100")
        .property("do-retransmission", true)
        .property("drop-on-latency", false)
        .build().unwrap();

    let rtph264depay = ElementFactory::make("rtph264depay")
        .build().unwrap();

    let h264parse = ElementFactory::make("h264parse")
        .build().unwrap();

    let sink = ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &*format!("capture-{}.mp4", Local::now().format("%Y-%m-%d_%H-%M-%S")).to_string())
        .build().unwrap();

    let video_queue = ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let mp4_muxer = ElementFactory::make("mp4mux")
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
    appsrc.set_min_latency((1000 / SAMPLING_RATE) as i64);

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