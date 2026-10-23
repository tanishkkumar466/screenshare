//! Local trust store. Only peers approved through the PIN flow may use input/video.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustedPeer { pub name: String, pub address: String, pub public_key: String }

#[derive(Default, Serialize, Deserialize)]
pub struct TrustedPeers { pub peers: Vec<TrustedPeer> }

impl TrustedPeers {
    pub fn load(path: &PathBuf) -> Self { fs::read(path).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or_default() }
    pub fn approve(&mut self, peer: TrustedPeer) {
        self.peers.retain(|p| p.public_key != peer.public_key);
        self.peers.push(peer);
    }
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        fs::write(path, serde_json::to_vec_pretty(self)?)?; Ok(())
    }
}

pub fn store_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(".config");
    base.join("DeskLink").join("trusted-peers.json")
}
