//! One executable; both machines may be hosts, clients, or both at the same time.
mod udp_input;
mod discovery;
mod pairing_session;
mod control_channel;
mod trusted_peers;
mod pairing_wire;
mod active_sessions;
mod platform_input;
use anyhow::Result;
use async_trait::async_trait;
use link_core::{InputEvent, MachineId};
use link_core::protocol::ControlMessage;
use tauri::Manager;
use std::sync::atomic::{AtomicBool, Ordering};

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
fn generate_pairing_code(sessions: tauri::State<'_, pairing_session::PairingSessions>) -> String { sessions.generate_local_code() }
#[tauri::command]
async fn pair_with_peer(address: String, pin: String, app: tauri::AppHandle, peers: tauri::State<'_, std::sync::Mutex<trusted_peers::TrustedPeers>>, sessions: tauri::State<'_, active_sessions::ActiveSessions>) -> Result<String, String> {
    let paired = pairing_wire::client(&address, "DeskLink", &pin).await.map_err(|e| e.to_string())?;
    sessions.activate(paired.peer_name.clone(), paired.key, true);
    let ip = address.parse::<std::net::SocketAddr>().map_err(|e| e.to_string())?.ip();
    let send_key = sessions.input_send_key(&paired.peer_name).ok_or("missing session")?;
    sessions.add_sender(paired.peer_name.clone(), ip, send_key).await.map_err(|e| e.to_string())?;
    let receive_key = sessions.input_receive_key(&paired.peer_name).ok_or("missing session")?;
    start_input_receiver(app, receive_key);
    let mut peers = peers.lock().map_err(|e| e.to_string())?;
    peers.approve(trusted_peers::TrustedPeer { name: paired.peer_name.clone(), address, public_key: paired.peer_key.iter().map(|b| format!("{b:02x}")).collect() });
    peers.save(&trusted_peers::store_path()).map_err(|e| e.to_string())?;
    Ok("Paired and saved as trusted device".into())
}
#[tauri::command]
fn active_peers(sessions: tauri::State<'_, active_sessions::ActiveSessions>) -> Vec<String> { sessions.peers() }
#[tauri::command]
async fn send_test_input(peer_name: String, sessions: tauri::State<'_, active_sessions::ActiveSessions>) -> Result<String, String> {
    sessions.send_event(&peer_name, InputEvent::PointerMotion(link_core::PointerMotion { dx: 1, dy: 0 })).await.map_err(|e| e.to_string())?;
    Ok("Encrypted input packet sent".into())
}
#[tauri::command]
fn last_input_status(status: tauri::State<'_, std::sync::Mutex<String>>) -> String { status.lock().map(|s| s.clone()).unwrap_or_else(|_| "Input status unavailable".into()) }
#[tauri::command]
fn set_input_sharing(enabled: bool, capture: tauri::State<'_, AtomicBool>) -> String {
    capture.store(enabled, Ordering::Relaxed);
    if enabled { "Mouse sharing is on. Use only on the source computer.".into() } else { "Mouse sharing is off".into() }
}

fn captured_mouse(event: rdev::EventType, previous: &std::sync::Mutex<Option<(f64, f64)>>) -> Option<InputEvent> {
    use rdev::{Button, EventType};
    match event {
        EventType::MouseMove { x, y } => {
            let mut prev = previous.lock().ok()?;
            let delta = prev.map(|(px, py)| link_core::PointerMotion { dx: (x - px).round() as i32, dy: (y - py).round() as i32 });
            *prev = Some((x, y));
            delta.map(InputEvent::PointerMotion)
        }
        EventType::ButtonPress(button) => {
            let button = match button { Button::Left => 0, Button::Right => 1, Button::Middle => 2, _ => return None };
            Some(InputEvent::Button { button, pressed: true })
        }
        EventType::ButtonRelease(button) => {
            let button = match button { Button::Left => 0, Button::Right => 1, Button::Middle => 2, _ => return None };
            Some(InputEvent::Button { button, pressed: false })
        }
        EventType::Wheel { delta_x, delta_y } => Some(InputEvent::Scroll { horizontal: delta_x.clamp(i16::MIN as i64, i16::MAX as i64) as i16, vertical: delta_y.clamp(i16::MIN as i64, i16::MAX as i64) as i16 }),
        EventType::KeyPress(key) => Some(InputEvent::Key { scan_code: key_hid_usage(&key)?, pressed: true }),
        EventType::KeyRelease(key) => Some(InputEvent::Key { scan_code: key_hid_usage(&key)?, pressed: false }),
    }
}

fn key_hid_usage(key: &rdev::Key) -> Option<u16> {
    let name = format!("{key:?}");
    let bytes = name.as_bytes();
    if bytes.len() == 4 && bytes[0..3] == *b"Key" && bytes[3].is_ascii_uppercase() {
        return Some((bytes[3].to_ascii_lowercase() - b'a') as u16 + 0x04);
    }
    if let Some(digit) = name.strip_prefix("Num").and_then(|s| s.parse::<u8>().ok()) {
        return Some(if digit == 0 { 0x27 } else { 0x1d + digit as u16 });
    }
    Some(match name.as_str() {
        "Escape" => 0x29, "Backspace" => 0x2a, "Tab" => 0x2b, "Return" => 0x28, "Space" => 0x2c,
        "LeftArrow" => 0x50, "RightArrow" => 0x4f, "DownArrow" => 0x51, "UpArrow" => 0x52,
        "ShiftLeft" => 0xe1, "ShiftRight" => 0xe5, "ControlLeft" => 0xe0, "ControlRight" => 0xe4,
        "Alt" | "AltGr" => 0xe2, "MetaLeft" => 0xe3, "MetaRight" => 0xe7,
        _ => return None,
    })
}

fn start_mouse_capture(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let previous = std::sync::Mutex::new(None);
        if let Err(error) = rdev::listen(move |event| {
            let capture = app.state::<AtomicBool>();
            if !capture.load(Ordering::Relaxed) { return; }
            let Some(input) = captured_mouse(event.event_type, &previous) else { return; };
            let sessions = app.state::<active_sessions::ActiveSessions>();
            let Some(peer) = sessions.peers().into_iter().next() else { return; };
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = app.state::<active_sessions::ActiveSessions>().send_event(&peer, input).await { eprintln!("Could not send captured input: {error}"); }
            });
        }) { eprintln!("Global input capture stopped: {error:?}"); }
    });
}

fn start_input_receiver(app: tauri::AppHandle, key: [u8; 32]) {
    tauri::async_runtime::spawn(async move {
        let Ok(mut receiver) = udp_input::UdpInputReceiver::bind(("0.0.0.0", discovery::INPUT_PORT), key).await else {
            eprintln!("Could not bind encrypted input UDP port {}", discovery::INPUT_PORT); return;
        };
        loop {
            match receiver.receive().await {
                Ok((packet, from)) => {
                    let state = app.state::<std::sync::Mutex<String>>();
                    if let Ok(mut last) = state.lock() { *last = format!("Received encrypted input from {from}: {:?}", packet.event); }
                    eprintln!("Accepted authenticated input packet from {from}");
                    if let Err(error) = platform_input::inject(packet.event) { eprintln!("Could not inject remote input: {error}"); }
                }
                Err(error) => eprintln!("Rejected invalid input datagram: {error}"),
            }
        }
    });
}

fn main() {
    // Tauri owns the desktop event loop. The runtime services are started by the
    // setup hook once QUIC/mDNS adapters are added in the next milestone.
    let discovery = discovery::Discovery::start("DeskLink").expect("Could not start LAN discovery");
    tauri::Builder::default()
        .manage(discovery)
        .manage(pairing_session::PairingSessions::new())
        .manage(std::sync::Mutex::new(trusted_peers::TrustedPeers::load(&trusted_peers::store_path())))
        .manage(active_sessions::ActiveSessions::default())
        .manage(std::sync::Mutex::new("No input packets received".to_string()))
        .manage(AtomicBool::new(false))
        .setup(|app| { let handle = app.handle().clone(); start_mouse_capture(handle.clone()); tauri::async_runtime::spawn(async move {
            let Ok(listener) = control_channel::listen().await else { return; };
            while let Ok((mut stream, peer)) = listener.accept().await {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let Ok(ControlMessage::Hello { device_name, .. }) = control_channel::receive(&mut stream).await else { return; };
                    let Some(pin) = handle.state::<pairing_session::PairingSessions>().take_local_code() else { return; };
                    match pairing_wire::server(&mut stream, device_name.clone(), &pin).await {
                        Ok(session) => {
                            let active = handle.state::<active_sessions::ActiveSessions>();
                            active.activate(session.peer_name.clone(), session.key, false);
                            let send_key = active.input_send_key(&session.peer_name);
                            let receive_key = active.input_receive_key(&session.peer_name);
                            if let Some(key) = send_key { let _ = active.add_sender(session.peer_name.clone(), peer.ip(), key).await; }
                            if let Some(key) = receive_key { start_input_receiver(handle.clone(), key); }
                            let state = handle.state::<std::sync::Mutex<trusted_peers::TrustedPeers>>();
                            if let Ok(mut store) = state.lock() {
                                store.approve(trusted_peers::TrustedPeer { name: device_name, address: peer.to_string(), public_key: session.peer_key.iter().map(|b| format!("{b:02x}")).collect() });
                                let _ = store.save(&trusted_peers::store_path());
                            }
                            eprintln!("Trusted DeskLink peer paired: {}", session.peer_name);
                        }
                        Err(error) => eprintln!("DeskLink pairing from {peer} failed: {error}"),
                    }
                });
            }
        }); Ok(()) })
        .invoke_handler(tauri::generate_handler![runtime_status, discovered_peers, begin_pairing, generate_pairing_code, pair_with_peer, active_peers, send_test_input, last_input_status, set_input_sharing])
        .run(tauri::generate_context!())
        .expect("DeskLink desktop application failed to start");
}
