use crate::assets::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, SAMPLING_RATE, TARGET_OS};
use chrono::Local;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::{Buffer, Element, ElementFactory, Fraction, Pipeline};
use gstreamer_rtp::RTPBuffer;
use std::error::Error;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;

pub fn create_stream_pipeline(monitor: &str, tx_processed: Sender<Buffer>, use_rtp: bool) -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = match TARGET_OS {
        "windows" => {
            ElementFactory::make("d3d11screencapturesrc")
                .property_from_str("monitor-handle", monitor)
                .property("show-cursor", true)
        }
        "macos" => {
            ElementFactory::make("avfvideosrc")
                .property_from_str("device-index", monitor)
                .property("capture-screen", true)
                .property("capture-screen-cursor", true)
        }
        "linux" => {
            ElementFactory::make("ximagesrc")
                .property_from_str("display-name", monitor)
                .property("use-damage", false)
                .property("show-pointer", true)
        }
        _ => {
            ElementFactory::make("appsrc")
        }
    }
        .name("src")
        .build().unwrap();

    let videobox = ElementFactory::make("videobox")
        .name("videobox")
        .build().unwrap();

    let video_convert = ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build().unwrap();

    let videoscale = ElementFactory::make("videoscale")
        .name("videoscale")
        .build().unwrap();

    let videoscale_capsfilter = ElementFactory::make("capsfilter")
        .name("videoscale-capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("width", FRAME_WITH)
                      .field("height", FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .field("framerate", Fraction::new(FRAME_RATE, 1))
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

    let sink = if use_rtp {
        ElementFactory::make("appsink")
            .name("appsink")
            .property("sync", &false)
            .property("emit-signals", &true)
            .property("caps",
                      &gst::Caps::builder("application/x-rtp")
                          .field("stream-format", &"byte-stream")
                          .field("alignment", &"au")
                          .field("media", &"video")
                          .field("clock-rate", &90000)
                          .field("encoding-name", &"H264")
                          .field("payload", &96i32)
                          .build(),
            ).build().unwrap()
    } else {
        ElementFactory::make("appsink")
            .name("appsink")
            .property("sync", &false)
            .property("emit-signals", &true)
            .property("caps",
                      &gst::Caps::builder("video/x-h264")
                          .field("stream-format", &"byte-stream")
                          .field("alignment", &"au")
                          .build(),
            ).build().unwrap()
    };

    let video_elements: Vec<&Element> = if use_rtp {
        vec![&src, &videobox, &video_convert, &videoscale, &videoscale_capsfilter, &video_encoder, &h264parse, &video_queue, &rtph264pay, &sink]
    } else {
        vec![&src, &videobox, &video_convert, &videoscale, &videoscale_capsfilter, &video_encoder, &h264parse, &video_queue, &sink]
    };

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    let appsink = sink
        .dynamic_cast::<gstreamer_app::AppSink>()
        .expect("Element is expected to be an appsink!");
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

pub fn create_view_pipeline(mut rx: Receiver<Buffer>, saver: Sender<Buffer>) -> Result<Pipeline, Box<dyn Error>> {
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
                      .build(),
        )
        .build()?;

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", true)
        .property("config-interval", -1)
        .build()?;

    let avdec_h264 = ElementFactory::make("avdec_h264")
        .build()?;
    let videoconvert = ElementFactory::make("videoconvert")
        .build()?;
    let capsfilter = ElementFactory::make("capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", "RGBA")
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .build(),
        )
        .build()?;

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
                      .build(),
        )
        .build()?;

    let video_queue1 = ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build()?;
    let video_queue2 = ElementFactory::make("queue")
        .property_from_str("leaky", "no")
        .build()?;

    let video_elements = [&src, &video_queue1, &h264parse, &avdec_h264, &video_queue2, &videoconvert, &capsfilter, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..])?;

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent()?;
    }

    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");
    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx.blocking_recv() {
                    Some(buffer) => {
                        saver.try_send(buffer.clone()).unwrap_or_default();

                        if let Err(error) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", error);
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
                      .field("pixel-aspect-ratio", &Fraction::new(1, 1))
                      .field("framerate", &Fraction::new(SAMPLING_RATE, 1))
                      .build(),
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

    let video_elements = [&src, &video_queue1, &h264parse, &mp4_muxer, &filesink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    Ok(pipeline)
}

pub fn create_rtp_view_pipeline(mut rx_processed: Receiver<Packet>, saver: Sender<Buffer>) -> Result<Pipeline, Box<dyn Error>> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("appsrc")
        .property("is-live", &true)
        .property("block", &true)
        .property("format", &gstreamer::Format::Time)
        .property("do-timestamp", &true)
        .property("caps",
                  &gst::Caps::builder("application/x-rtp")
                      .field("media", &"video")
                      .field("clock-rate", &90000)
                      .field("encoding-name", &"H264")
                      .field("payload", &102i32)
                      .build(),
        )
        .build()?;

    let rtpjitterbuffer = ElementFactory::make("rtpjitterbuffer")
        .property("latency", 500u32)
        .property("sync-interval", 500u32)
        .property("do-retransmission", false)
        .property("drop-on-latency", true)
        .build()?;

    let rtph264depay = ElementFactory::make("rtph264depay")
        .property("wait-for-keyframe", true)
        .build()?;

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", true)
        .property("config-interval", -1)
        .build()?;

    let avdec_h264 = ElementFactory::make("avdec_h264")
        .build()?;

    let videoconvert = ElementFactory::make("videoconvert")
        .build()?;

    let capsfilter = ElementFactory::make("capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("format", "RGBA")
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .build(),
        )
        .build()?;

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
                      .build(),
        )
        .build()?;

    let video_queue1 = ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("leaky", "no")
        .build()?;

    let video_elements = [&src, &rtpjitterbuffer, &rtph264depay, &video_queue1, &h264parse, &avdec_h264, &videoconvert, &capsfilter, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..])?;

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent()?;
    }

    let appsrc = src
        .dynamic_cast::<gstreamer_app::AppSrc>()
        .expect("Source element is expected to be an appsrc!");
    appsrc.set_callbacks(
        gstreamer_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                match rx_processed.blocking_recv() {
                    Some(packet) => {
                        // Convert  packet RTP in a GStreamer buffer
                        let mut buffer = Buffer::from_slice(packet.marshal().unwrap_or_default());
                        {
                            let buffer_ref = buffer.get_mut().unwrap();
                            let mut rtp_packet = RTPBuffer::from_buffer_writable(buffer_ref).unwrap();

                            rtp_packet.set_marker(packet.header.marker);
                            rtp_packet.set_seq(packet.header.sequence_number);
                            rtp_packet.set_ssrc(packet.header.ssrc);
                            rtp_packet.set_timestamp(packet.header.timestamp);
                        }

                        saver.try_send(buffer.clone()).unwrap_or_default();

                        // Invia il buffer a GStreamer
                        if let Err(e) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {:?}", e);
                        }
                    }
                    _ => {}
                }
            }).build(),
    );
    Ok(pipeline)
}

pub fn create_rtp_save_pipeline() -> Result<Pipeline, glib::Error> {
    let pipeline = Pipeline::new();

    let src = ElementFactory::make("appsrc")
        .name("appsrc")
        .property("is-live", &true)
        .property("block", &true)
        .property("format", &gstreamer::Format::Time)
        .property("do-timestamp", &true)
        .property("caps",
                  &gst::Caps::builder("application/x-rtp")
                      .field("media", &"video")
                      .field("clock-rate", &90000)
                      .field("encoding-name", &"H264")
                      .field("payload", &102i32)
                      .build(),
        )
        .build().unwrap();

    let rtpjitterbuffer = ElementFactory::make("rtpjitterbuffer")
        .property("latency", 500u32)
        .property("sync-interval", 500u32)
        .property("do-retransmission", false)
        .property("drop-on-latency", true)
        .build().unwrap();

    let rtph264depay = ElementFactory::make("rtph264depay")
        .property("wait-for-keyframe", true)
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
        .name("video-queue")
        .property_from_str("leaky", "no")
        .build().unwrap();

    let video_elements = [&src, &rtpjitterbuffer, &rtph264depay, &video_queue1, &h264parse, &mp4_muxer, &filesink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..]).unwrap();

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent().unwrap();
    }

    Ok(pipeline)
}