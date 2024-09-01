use crate::gui::resource::{CAST_SERVICE_PORT, MAX_PACKAGES_FAIL};
use crate::utils::net::find_caster;
use bincode::{deserialize, serialize};
use gstreamer::{Buffer, BufferFlags, ClockTime};
use serde::{Deserialize, Serialize};
use socket2::TcpKeepalive;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

pub enum SendingData {
    Transmit,
    Pause,
    Stop,
}

struct StreamEntry {
    stream: TcpStream,
    error_count: u8,
}

#[derive(Clone, Serialize, Deserialize)]
struct XGPacket {
    pts: u64,
    duration: u64,
    bytes: Vec<u8>,
    len: usize,
    flags: u32,
    offset: u64,
}

const CHUNK_SIZE: usize = 40960;


pub async fn receiver(mut socket_addr: Option<SocketAddr>, tx: tokio::sync::mpsc::Sender<Buffer>) -> bool {
    let mut stream;

    if socket_addr.is_none() {
        socket_addr = find_caster();
    }

    if let Some(socket_addr) = socket_addr {
        println!("Connecting to caster at {:?}", socket_addr);

        loop {
            match TcpStream::connect(socket_addr) {
                Ok(s) => {
                    stream = s;
                    break;
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        loop {
            let mut corrupted = false;
            let mut length_bytes = [0; 8];

            // Read the length of the image data
            if stream.read_exact(&mut length_bytes).is_err() {
                // If length can't be read, the stream has ended
                return false;
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
                println!("packet corrupted");
                continue;
            }

            match deserialize::<XGPacket>(&*buffer) {
                Ok(packet) => {
                    if packet.len != packet.bytes.len() {
                        println!("Invalid packet");
                        continue;
                    }
                    let mut buffer = Buffer::from_slice(packet.bytes);
                    {
                        let buffer_ref = buffer.get_mut().unwrap();

                        buffer_ref.set_pts(ClockTime::from_mseconds(packet.pts));
                        buffer_ref.set_dts(ClockTime::from_mseconds(packet.pts));
                        buffer_ref.set_duration(ClockTime::from_mseconds(packet.duration));
                        buffer_ref.set_flags(BufferFlags::from_bits(packet.flags).unwrap());
                        buffer_ref.set_offset(packet.offset);
                    }

                    match tx.send(buffer).await {
                        Err(e) => { println!("{}", e) }
                        _ => {}
                    }
                }
                Err(e) => {
                    println!("Failed to deserialize image data: {}", e);
                }
            }
        }
    }
    true
}

pub async fn caster(mut rx: Receiver<Buffer>, running: Arc<AtomicBool>) {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), CAST_SERVICE_PORT);
    let listener = TcpListener::bind(addr).unwrap();

    let streams: Arc<Mutex<Vec<StreamEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let streams_clone = streams.clone();

    let running_c = Arc::clone(&running);

    tokio::spawn(async move {
        while let Some(buffer) = rx.recv().await {
            if !running_c.load(Ordering::Relaxed) {
                break;
            }
            println!("Transmitting...");

            let buff_raw = buffer.map_readable().unwrap().as_slice().to_vec();

            let image_tx = XGPacket {
                pts: buffer.pts().unwrap().mseconds(),
                duration: buffer.duration().unwrap().mseconds(),
                len: buff_raw.len(),
                bytes: buff_raw,
                flags: buffer.flags().bits(),
                offset: buffer.offset(),
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

    let running_c = Arc::clone(&running);
    tokio::spawn(async move {
        loop {
            if !running_c.load(Ordering::Relaxed) {
                break;
            }

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