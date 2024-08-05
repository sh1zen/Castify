#![cfg_attr(
    not(debug_assertions),
    windows_subsystem = "windows"
)] // hide console window on Windows in release

use std::{str, thread};
use std::io::{Read, stdin, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use rdev::{grab};

use rust_st::events;

#[derive(Debug)]
enum Mode {
    Caster,
    Receiver,
}


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mode = match get_mode() {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("Error getting mode: {}", e);
            return Err(e.into());
        }
    };

    match mode {
        Mode::Caster => caster().await,
        Mode::Receiver => receiver().await,
    };


    let start = Instant::now();

    // let (tx, mut rx) = mpsc::channel(1);

    let events = events::Events::init();

    // Start grabbing events; handle errors if any occur
    if let Err(error) = grab(move |e| events.handle(e)) {
        println!("Error: {error:?}");
    }

    println!("T elapsed: {:?}", start.elapsed());

    Ok(())
}

const SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";
const SERVICE_PORT: u16 = 31413;

fn get_mode() -> Result<Mode, std::io::Error> {
    println!("Choose mode (caster/receiver):");
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let mode = input.trim().to_lowercase();
    match mode.as_str() {
        "c" => Ok(Mode::Caster),
        "r" => Ok(Mode::Receiver),
        _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid mode")),
    }
}

fn find_caster() -> Option<SocketAddr> {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    // Browse for a service type.
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");

    let mut addr: Option<SocketAddr> = None;

    while let Some(event) = receiver.iter().next() {
        println!("ciao loop");
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


async fn receiver() {
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

async fn caster() {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), SERVICE_PORT);

    let mut connection_listener = TcpListener::bind(addr).unwrap();
    let mut buffer = [0; 1024];
    let mut handles = vec![];

    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Create a service info.

    let ip = local_ip().unwrap();
    let host_name = String::from(ip.to_string()) + ".local.";
    let properties = [("property_1", "test"), ("property_2", "1234")];

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

    for stream in connection_listener.incoming() {
        println!("---- Connection established {:?}! ---- ", handles.len());
        let handle = thread::spawn(move || {
            // new thread for each connection
            let mut stream = stream.unwrap();
            stream.write("ciao\r\n".as_ref()).unwrap();
            loop {
                stdin().read(&mut buffer).unwrap();
                stream.write(&buffer).unwrap();
            }
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