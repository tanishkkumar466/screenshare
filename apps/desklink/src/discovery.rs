use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use std::{sync::{Arc, Mutex}, thread};

pub const SERVICE_TYPE: &str = "_desklink._udp.local.";
pub const CONTROL_PORT: u16 = 49152;

/// Advertises this DeskLink endpoint and continuously listens for peers on the LAN.
/// mDNS stays local to the current network; it does not expose a public service.
#[derive(Clone, Debug, Serialize)]
pub struct DiscoveredPeer { pub name: String, pub addresses: Vec<String>, pub port: u16 }

pub struct Discovery { _daemon: ServiceDaemon, peers: Arc<Mutex<Vec<DiscoveredPeer>>> }

impl Discovery {
    pub fn start(device_name: &str) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let instance = format!("{}-{}", device_name, &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let properties = [("version", "1"), ("control_port", "49152")];
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, "desklink.local.", "", CONTROL_PORT, &properties[..])?.enable_addr_auto();
        daemon.register(info)?;
        let receiver = daemon.browse(SERVICE_TYPE)?;
        let peers = Arc::new(Mutex::new(Vec::<DiscoveredPeer>::new()));
        let found = Arc::clone(&peers);
        thread::spawn(move || while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(peer) = event {
                eprintln!("DeskLink peer discovered: {} at {:?}:{}", peer.get_fullname(), peer.get_addresses(), peer.get_port());
                let item = DiscoveredPeer { name: peer.get_fullname().to_string(), addresses: peer.get_addresses().iter().map(ToString::to_string).collect(), port: peer.get_port() };
                let mut list = found.lock().expect("peer list mutex poisoned");
                if let Some(old) = list.iter_mut().find(|p| p.name == item.name) { *old = item; } else { list.push(item); }
            }
        });
        Ok(Self { _daemon: daemon, peers })
    }
    pub fn peers(&self) -> Vec<DiscoveredPeer> { self.peers.lock().expect("peer list mutex poisoned").clone() }
}
