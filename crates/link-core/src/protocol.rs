use crate::{input::InputEvent, layout::DisplayLayout};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    Hello { device_name: String, version: u16 },
    PairingProof(Vec<u8>),
    Layout(DisplayLayout),
    StartVideo { width: u32, height: u32, fps: u16, bitrate_kbps: u32 },
    StopVideo,
    Clipboard { text: String },
    Ping { sent_micros: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPacket { pub sequence: u32, pub event: InputEvent }
