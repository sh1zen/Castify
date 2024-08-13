use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use socket2::TcpKeepalive;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use std::str;
use tokio::sync::Mutex;
use crate::gui::resource::MAX_PACKAGES_FAIL;

const SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";
const SERVICE_PORT: u16 = 31413;

pub enum SendingData {
    Transmit,
    Pause,
    Stop,
}

struct StreamEntry {
    stream: TcpStream,
    error_count: u8,
}

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

    return addr;
}

pub async fn receiver() {
    let mut stream;
    let mut buffer = [0; 1024];

    loop {
        let socket_addr = find_caster().unwrap();

        println!("Caster found at: {:?}", socket_addr);

        match TcpStream::connect(socket_addr) {
            Ok(s) => {
                stream = s;
                break;
            }
            Err(_) => {
                println!("Connection error, waiting 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        let received_data = &buffer[..bytes_read];

        match str::from_utf8(received_data) {
            Ok(message) => println!("{:?}", message.trim()),
            Err(e) => println!("Failed to convert to string: {:?}", e),
        }
    }
}

pub async fn caster(rx: Option<tokio::sync::mpsc::Receiver<String>>) {

    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), SERVICE_PORT);
    let listener = TcpListener::bind(addr).unwrap();

    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let ip = local_ip().unwrap();
    let host_name = String::from(ip.to_string()) + ".local.";
    let properties = [("screen_caster", SERVICE_PORT)];

    let my_service = ServiceInfo::new(
        SERVICE_NAME,
        "ScreenCaster",
        &*host_name,
        ip,
        SERVICE_PORT,
        &properties[..],
    ).unwrap();

    mdns.register(my_service).expect("Failed to register our service");

    println!("Caster running and registered on mDNS");

    let streams: Arc<Mutex<Vec<StreamEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let mut urx = rx.unwrap();

    let streams_clone = streams.clone();
    tokio::spawn(async move {
        while let Some(buf) = urx.recv().await {

            println!("Transmitting {:?}", &buf);

            let mut streams = streams_clone.lock().await;
            let mut i = 0;

            while i < streams.len() {
                let entry = &mut streams[i];
                match entry.stream.write_all((&buf).as_ref()) {
                    Ok(_) => {
                        entry.error_count = 0;
                        i += 1;
                    }
                    Err(_) => {
                        entry.error_count += 1;
                        println!("Receiver {} has shutdown connection.", i);

                        if entry.error_count >= MAX_PACKAGES_FAIL {
                            streams.remove(i);
                        } else {
                            i += 1;
                        }
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            let (mut stream, _addr) = listener.accept().unwrap();
            println!("---- Connection established! N° {:?} ----", streams.lock().await.len() + 1);

            set_keep_alive(&stream);
            stream.write_all(b"Hello Receiver\r\n").expect("Error while sending data.");

            streams.lock().await.push(StreamEntry {stream :stream, error_count: 0});
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