use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::io::{stdin, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration};
use std::{io, str, thread};
use std::sync::Arc;
use socket2::TcpKeepalive;
use tokio::sync::broadcast::{channel, Sender, Receiver};
use tokio::sync::Mutex;

const SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";
const SERVICE_PORT: u16 = 31413;

pub enum SendingData {
    Transmit,
    Pause,
    Stop
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

pub async fn caster(mut rx: Option<Receiver<String>>) {

    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), SERVICE_PORT);

    let mut connection_listener = TcpListener::bind(addr).unwrap();
    let mut handles = vec![];

    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Create a service info.
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

    // Register with the daemon, which publishes the service.
    mdns.register(my_service).expect("Failed to register our service");

    println!("Caster running and registered on mDNS");

    let urx = rx.unwrap();

    for stream in connection_listener.incoming() {

        println!("---- Connection established! N° {:?} ---- ", handles.len());

        let mut urx = urx.resubscribe();

        // new thread for each connection
        let handle = thread::spawn(|| async move {

            let mut stream = stream.unwrap();

            set_keep_alive(&stream);

            stream.write("Hello Receiver\r\n".as_ref()).unwrap();

            while let buf = urx.recv().await {

                println!("{:?}", buf);
                // println!("{:?}", buf.unwrap());
                stream.write(buf.unwrap().as_ref()).unwrap();
                //thread::sleep(Duration::from_secs(1))
            }

            // continue running until forced to stop
        });
        handles.push(handle);
    }

    // Wait for all spawned threads to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Unregister the service when done (in this case never, but added for completeness)
    mdns.shutdown().unwrap();
}

fn set_keep_alive<'a>(stream: &'a TcpStream) {
    let sock_ref = socket2::SockRef::from(stream);

    let mut keepAlive = TcpKeepalive::new();
    keepAlive = keepAlive.with_time(Duration::from_secs(20));
    keepAlive = keepAlive.with_interval(Duration::from_secs(20));

    sock_ref.set_tcp_keepalive(&keepAlive).unwrap();
}