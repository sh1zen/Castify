use crate::assets::CAST_SERVICE_PORT;
use igd::search_gateway;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::{IpAddr, SocketAddr, SocketAddrV4};

const LOCAL_DISCOVERY_SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";
const FORWARDING_SERVICE_NAME: &'static str = "Castify";

pub(crate) fn find_caster() -> Option<SocketAddr> {
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
                println!("Resolved a new service: {:?}", ip_addr);
                addr = Option::from(SocketAddr::new(ip_addr.parse().unwrap(), info.get_port()));
                break;
            }
            _ => {}
        }
    }
    mdns.shutdown().unwrap();

    addr
}

pub(crate) fn caster_discover_service() -> ServiceDaemon {
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
    ).unwrap();

    mdns.register(my_service).expect("Failed to register our service");

    println!("Caster running and registered on mDNS");

    mdns
}

pub(crate) fn port_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = search_gateway(Default::default())?;

    let local_ip = local_ip()?;

    let IpAddr::V4(ipv4) = local_ip else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Local ipv4 not found")))
    };

    gateway.add_port(
        igd::PortMappingProtocol::TCP,
        31415,
        SocketAddrV4::new(ipv4, CAST_SERVICE_PORT),
        3600,
        FORWARDING_SERVICE_NAME,
    )?;

    println!("Port forwarding setup complete!");

    Ok(())
}