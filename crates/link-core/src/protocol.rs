use crate::{input::InputEvent, layout::DisplayLayout};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    Hello { device_name: String, version: u16 },
    PairingInit { session_id: Vec<u8>, pake_message: Vec<u8>, ephemeral_key: [u8; 32] },
    PairingReply { peer_name: String, pake_message: Vec<u8>, ephemeral_key: [u8; 32], proof: [u8; 32] },
    PairingFinish { proof: [u8; 32] },
    PairingAccepted { peer_name: String },
    Layout(DisplayLayout),
    StartVideo { width: u32, height: u32, fps: u16, bitrate_kbps: u32 },
    StopVideo,
    Clipboard { text: String },
    Ping { sent_micros: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPacket { pub sequence: u32, pub event: InputEvent }
