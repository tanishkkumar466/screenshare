//! One executable; both machines may be hosts, clients, or both at the same time.
mod udp_input;
mod discovery;
mod pairing_session;
mod control_channel;
use anyhow::Result;
use async_trait::async_trait;
use link_core::{InputEvent, MachineId};
use link_core::protocol::ControlMessage;
use tokio::net::TcpStream;

#[async_trait]
pub trait InputInjector: Send + Sync {
    async fn inject(&self, event: InputEvent) -> Result<()>;
}

#[async_trait]
pub trait ScreenCapture: Send + Sync {
    async fn next_frame(&mut self) -> Result<Frame>;
}

pub struct Frame { pub width: u32, pub height: u32, pub bytes: Vec<u8> }

pub struct Runtime { pub local_machine: MachineId }

impl Runtime {
    pub async fn run(self) -> Result<()> {
        // Next milestone: QUIC control session + mDNS discovery, then platform adapters.
        println!("DeskLink {} ready: host and client roles are available in one process.", self.local_machine);
        Ok(())
    }
}

#[tauri::command]
fn runtime_status() -> String {
    "DeskLink is ready to pair a trusted LAN device.".to_owned()
}
#[tauri::command]
fn discovered_peers(discovery: tauri::State<'_, discovery::Discovery>) -> Vec<discovery::DiscoveredPeer> { discovery.peers() }
#[tauri::command]
fn begin_pairing(peer_name: String, sessions: tauri::State<'_, pairing_session::PairingSessions>) -> String { sessions.begin(peer_name) }
#[tauri::command]
async fn connect_peer(address: String) -> Result<(), String> {
    let mut stream = TcpStream::connect(address).await.map_err(|e| e.to_string())?;
    control_channel::send(&mut stream, &ControlMessage::Hello { device_name: "DeskLink".into(), version: 1 }).await.map_err(|e| e.to_string())
}

fn main() {
    // Tauri owns the desktop event loop. The runtime services are started by the
    // setup hook once QUIC/mDNS adapters are added in the next milestone.
    let discovery = discovery::Discovery::start("DeskLink").expect("Could not start LAN discovery");
    tauri::Builder::default()
        .manage(discovery)
        .manage(pairing_session::PairingSessions::new())
        .setup(|_| { tauri::async_runtime::spawn(async {
            let Ok(listener) = control_channel::listen().await else { return; };
            while let Ok((mut stream, peer)) = listener.accept().await {
                tauri::async_runtime::spawn(async move {
                    if let Ok(ControlMessage::Hello { device_name, .. }) = control_channel::receive(&mut stream).await {
                        eprintln!("DeskLink control connection from {device_name} ({peer})");
                    }
                });
            }
        }); Ok(()) })
        .invoke_handler(tauri::generate_handler![runtime_status, discovered_peers, begin_pairing, connect_peer])
        .run(tauri::generate_context!())
        .expect("DeskLink desktop application failed to start");
}
