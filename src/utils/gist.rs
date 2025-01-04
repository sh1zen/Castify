use crate::assets::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, TARGET_OS};
use crate::utils::monitors::XMonitor;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::{Buffer, Element, ElementFactory, Fraction, Pipeline};
use gstreamer_rtp::RTPBuffer;
use std::error::Error;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;

pub fn create_stream_pipeline(monitor: &XMonitor, tx_processed: Sender<Buffer>) -> Result<Pipeline, Box<dyn Error>> {
    let pipeline = Pipeline::new();

    let src = match TARGET_OS {
        "windows" => {
            ElementFactory::make("d3d11screencapturesrc")
                .property_from_str("monitor-handle", &*monitor.dev_id)
                .property("show-cursor", true)
        }
        "macos" => {
            ElementFactory::make("avfvideosrc")
                //.property_from_str("device-index", &*monitor.dev_id)
                .property("capture-screen", true)
                .property("capture-screen-cursor", true)
        }
        "linux" => {
            ElementFactory::make("ximagesrc")
                .property("startx", monitor.x as u32)
                .property("starty", monitor.y as u32)
                .property("endx", monitor.width + monitor.x as u32 - 1)
                .property("endy", monitor.height + monitor.y as u32 - 1)
                .property("use-damage", false)
                .property("show-pointer", true)
        }
        _ => { unreachable!("TargetOS not supported") }
    }
        .name("src")
        .build()?;

    let videobox = ElementFactory::make("videobox")
        .name("videobox")
        .build()?;

    let videofilter = ElementFactory::make("videobalance")
        .name("videofilter")
        .build()?;

    let video_convert = ElementFactory::make("videoconvert")
        .name("videoconvert")
        .build()?;

    let videoscale = ElementFactory::make("videoscale")
        .name("videoscale")
        .build()?;

    let videoscale_capsfilter = ElementFactory::make("capsfilter")
        .name("videoscale-capsfilter")
        .property("caps",
                  &gst::Caps::builder("video/x-raw")
                      .field("width", FRAME_WITH)
                      .field("height", FRAME_HEIGHT)
                      .field("pixel-aspect-ratio", Fraction::new(1, 1))
                      .field("framerate", Fraction::new(FRAME_RATE, 1))
                      .field("format", "I420")
                      .build(),
        ).build()?;

    let video_encoder = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("bitrate", 1000u32)
        .property_from_str("pass", "quant")
        .property("quantizer", 20u32)
        .property_from_str("tune", "zerolatency")
        .property_from_str("speed-preset", "superfast")
        .property("key-int-max", 15u32)
        .build()?;

    let video_queue = ElementFactory::make("queue")
        .property_from_str("leaky", "downstream")
        .build()?;

    let h264parse = ElementFactory::make("h264parse")
        .property("disable-passthrough", true)
        .property("config-interval", -1) // Send SPS/PPS with every keyframe
        .build()?;

    let rtph264pay = ElementFactory::make("rtph264pay")
        .property("config-interval", -1)
        .property("pt", 96u32)
        .build()?;

    let sink = ElementFactory::make("appsink")
        .name("appsink")
        .property("sync", &false)
        .property("emit-signals", &true)
        .property("caps",
                  &gst::Caps::builder("video/x-h264")
                      .field("stream-format", "byte-stream")
                      .field("alignment", "au")
                      .build(),
        ).build()?;

    let video_elements: Vec<&Element> =
        vec![&src, &videofilter, &videobox, &video_convert, &videoscale, &videoscale_capsfilter, &video_encoder, &h264parse, &video_queue, &sink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..])?;

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent()?;
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

pub fn create_view_pipeline(mut rx_processed: Receiver<Packet>, saver: Sender<Buffer>) -> Result<Pipeline, Box<dyn Error>> {
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
                      .field("pixel-aspect-ratio", &Fraction::new(1, 1))
                      .field("framerate", &Fraction::new(FRAME_RATE, 1))
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
                            let Some(buffer_ref) = buffer.get_mut() else {
                                return;
                            };

                            if let Ok(mut rtp_packet) = RTPBuffer::from_buffer_writable(buffer_ref) {
                                rtp_packet.set_marker(packet.header.marker);
                                rtp_packet.set_seq(packet.header.sequence_number);
                                rtp_packet.set_ssrc(packet.header.ssrc);
                                rtp_packet.set_timestamp(packet.header.timestamp);
                            }
                        }

                        let _ = saver.try_send(buffer.clone());

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

pub fn create_save_pipeline(mut filepath: String) -> Result<Pipeline, Box<dyn Error>> {
    let pipeline = Pipeline::new();

    if filepath.len() <= 2 {
        filepath = String::from("");
    }

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
                      .field("framerate", &Fraction::new(FRAME_RATE, 1))
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
        .property("config-interval", 0)
        .build()?;

    let mp4_muxer = ElementFactory::make("mp4mux")
        .property("faststart", true)
        .build()?;

    let filesink = ElementFactory::make("filesink")
        .name("filesink")
        .property_from_str("location", &filepath)
        .build()?;

    let video_queue1 = ElementFactory::make("queue")
        .name("video-queue")
        .property_from_str("leaky", "no")
        .build()?;

    let video_elements = [&src, &rtpjitterbuffer, &rtph264depay, &video_queue1, &h264parse, &mp4_muxer, &filesink];

    // Add elements to pipeline
    pipeline.add_many(&video_elements[..])?;

    // Link elements
    Element::link_many(&video_elements[..]).expect("Failed to link elements");

    for e in video_elements {
        e.sync_state_with_parent()?;
    }

    Ok(pipeline)
}