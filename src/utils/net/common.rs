use crate::assets::CAST_SERVICE_PORT;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use natpmp::Natpmp;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

const LOCAL_DISCOVERY_SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";

pub fn find_caster() -> Option<SocketAddr> {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    // Browse for a service type.
    let receiver = mdns.browse(LOCAL_DISCOVERY_SERVICE_NAME).expect("Failed to browse");

    let mut addr: Option<SocketAddr> = None;

    while let Some(event) = receiver.iter().next() {
        println!("waiting for a caster");
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let ip_addr = info.get_addresses_v4().iter().next()?.to_string();
                println!("Resolved caster service at: {:?}", ip_addr);
                addr = Option::from(SocketAddr::new(ip_addr.parse().unwrap(), info.get_port()));
                break;
            }
            _ => {}
        }
    }
    mdns.shutdown().unwrap();

    addr
}

pub fn caster_discover_service() -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let ip = local_ip().expect("No internet connection");
    let host_name = String::from(ip.to_string()) + ".local.";
    let properties = [("screen_caster", CAST_SERVICE_PORT)];

    let my_service = ServiceInfo::new(
        LOCAL_DISCOVERY_SERVICE_NAME,
        "ScreenCaster",
        &*host_name,
        ip,
        CAST_SERVICE_PORT,
        &properties[..],
    )?;

    mdns.register(my_service)?;

    Ok(mdns)
}

pub fn port_forwarding() -> Result<(), Box<dyn std::error::Error>> {

    // Initialize NAT-PMP with the router's IP address
    let mut natpmp = Natpmp::new()?;

    // Send a request to create a port mapping
    natpmp.send_port_mapping_request(natpmp::Protocol::UDP, CAST_SERVICE_PORT, CAST_SERVICE_PORT + 1, 7200)?;

    sleep(Duration::from_millis(250));

    // Handle response (wait for a while for the response to come back)
    let mapping = natpmp.read_response_or_retry()?;
    println!("Port forwarding setup complete: {:?}", mapping);

    Ok(())
}