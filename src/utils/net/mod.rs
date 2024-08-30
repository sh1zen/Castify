mod werbrtc_server;
mod webrtc_client;
mod webrtc_common;
pub mod net;
pub mod rtp;

pub use self::werbrtc_server::WebRTCServer;
pub use self::webrtc_client::WebRTCClient;


use std::net::SocketAddr;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use crate::gui::resource::CAST_SERVICE_PORT;

const SERVICE_NAME: &'static str = "_screen_caster._tcp.local.";

pub(crate) fn find_caster() -> Option<SocketAddr> {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    // Browse for a service type.
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");

    let mut addr: Option<SocketAddr> = None;

    while let Some(event) = receiver.iter().next() {
        println!("waiting for a caster");
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

pub(crate) fn caster_discover_service() {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let ip = local_ip().expect("No internet connection");
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
}
