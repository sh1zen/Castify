use crate::gui::resource::{CAST_SERVICE_PORT, MAX_PACKAGES_FAIL};
use crate::gui::types::messages::Message;
use bincode::{deserialize, serialize};
use image::RgbaImage;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use socket2::TcpKeepalive;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";

pub enum SendingData {
    Transmit,
    Pause,
    Stop,
}

struct StreamEntry {
    stream: TcpStream,
    error_count: u8,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct ImageData {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

const CHUNK_SIZE: usize = 10240;

fn find_caster() -> Option<SocketAddr> {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    // Browse for a service type.
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");

    let mut addr: Option<SocketAddr> = None;

    while let Some(event) = receiver.iter().next() {
        println!("waiting for caster");
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let ip_addr = info.get_addresses_v4().iter().next().unwrap().to_string();
                println!("Resolved a new service: {:?}", ip_addr);
                addr = Option::from(SocketAddr::new(ip_addr.parse().unwrap(), info.get_port()));
                break;
            }
            _ => {
                // skipping event
            }
        }
    }
    mdns.shutdown().unwrap();

    addr
}

pub async fn receiver(mut socket_addr: Option<SocketAddr>, tx: tokio::sync::mpsc::Sender<RgbaImage>) -> Message {
    let mut stream;

    if socket_addr.is_none() {
        socket_addr = find_caster();
    }

    if !socket_addr.is_none() {
        let socket_addr = socket_addr.unwrap();
        println!("Connecting to caster at {:?}", socket_addr);

        loop {
            match TcpStream::connect(socket_addr) {
                Ok(s) => {
                    stream = s;
                    break;
                }
                Err(_) => {
                    //return Message::ConnectionError;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        loop {
            let mut corrupted = false;
            let mut length_bytes = [0; 8];

            // Read the length of the image data
            if stream.read_exact(&mut length_bytes).is_err() {
                // If length can't be read, the stream has ended
                return Message::ConnectionError;
            }
            let packet_len = usize::from_le_bytes(length_bytes);
            let mut received: usize = 0;
            let mut buffer = vec![0; packet_len];

            // Read the image data in chunks
            while received < packet_len {
                let end = std::cmp::min(received + CHUNK_SIZE, packet_len);
                if stream.read_exact(&mut buffer[received..end]).is_err() {
                    corrupted = true;
                    break;
                }
                received += end - received;
            }

            if corrupted {
                continue;
            }

            match deserialize::<ImageData>(&*buffer) {
                Ok(image_data) => {
                    // Create an RgbaImage from the deserialized data
                    match RgbaImage::from_raw(image_data.width, image_data.height, image_data.bytes) {
                        Some(rgba_image) => {
                            match tx.send(rgba_image).await {
                                Err(e) => { println!("{}", e) }
                                _ => {}
                            }
                        }
                        None => {
                            println!("Failed to reconstruct image from received data.");
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to deserialize image data: {}", e);
                }
            }
        }
    }

    Message::Ignore
}

pub async fn caster(mut rx: tokio::sync::mpsc::Receiver<RgbaImage>) {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), CAST_SERVICE_PORT);
    let listener = TcpListener::bind(addr).unwrap();

    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let ip = local_ip().unwrap();
    let host_name = String::from(ip.to_string()) + ".local.";
    let properties = [("screen_caster", CAST_SERVICE_PORT)];

    let my_service = ServiceInfo::new(
        SERVICE_NAME,
        "ScreenCaster",
        &*host_name,
        ip,
        CAST_SERVICE_PORT,
        &properties[..],
    ).unwrap();

    mdns.register(my_service).expect("Failed to register our service");

    println!("Caster running and registered on mDNS");

    let streams: Arc<Mutex<Vec<StreamEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let streams_clone = streams.clone();

    tokio::spawn(async move {
        while let Some(image) = rx.recv().await {
            println!("Transmitting...");

            let image_tx = ImageData {
                width: image.width(),
                height: image.height(),
                bytes: image.into_raw(),
            };

            let serialized_tx: Vec<u8> = serialize(&image_tx).unwrap();
            let mut streams = streams_clone.lock().await;
            let mut index_stream = 0;
            let mut offset: usize;

            // Get the length of the serialized data
            let packet_len = serialized_tx.len();

            while index_stream < streams.len() {
                let entry = &mut streams[index_stream];

                if entry.error_count >= MAX_PACKAGES_FAIL {
                    streams.remove(index_stream);
                    println!("Receiver {} dropped connection..", index_stream);
                    continue;
                }

                // Send the length of the data first
                if entry.stream.write_all(&packet_len.to_le_bytes().as_ref()).is_err() {
                    entry.error_count += 1;
                    continue;
                }
                entry.stream.flush().unwrap();

                offset = 0;
                while offset < packet_len {
                    let end = std::cmp::min(offset + CHUNK_SIZE, packet_len);
                    if entry.stream.write_all(&(serialized_tx[offset..end]).as_ref()).is_err() {
                        entry.error_count += 1;
                        break;
                    }
                    offset += CHUNK_SIZE;
                }
                entry.stream.flush().unwrap();
                entry.error_count = 0;

                index_stream += 1;
            }
        }
    });

    tokio::spawn(async move {
        loop {
            let (stream, _addr) = listener.accept().unwrap();
            println!("---- Connection established! N° {:?} ----", streams.lock().await.len() + 1);

            set_keep_alive(&stream);

            streams.lock().await.push(StreamEntry { stream, error_count: 0 });
        }
    });
}

fn set_keep_alive(stream: &TcpStream) {
    let sock_ref = socket2::SockRef::from(stream);

    let mut keep_alive = TcpKeepalive::new();
    keep_alive = keep_alive.with_time(Duration::from_secs(20));
    keep_alive = keep_alive.with_interval(Duration::from_secs(20));

    sock_ref.set_tcp_keepalive(&keep_alive).unwrap();
}