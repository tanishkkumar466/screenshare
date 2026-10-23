use anyhow::Result;
use link_core::InputEvent;
use std::{collections::HashMap, net::{IpAddr, SocketAddr}, sync::{Arc, Mutex}};
use tokio::sync::Mutex as AsyncMutex;
use crate::udp_input::UdpInputSender;

#[derive(Clone)]
pub struct ActiveSession { pub peer_name: String, pub input_send_key: [u8; 32], pub input_receive_key: [u8; 32] }

#[derive(Default)]
pub struct ActiveSessions { sessions: Mutex<HashMap<String, ActiveSession>>, senders: Mutex<HashMap<String, Arc<AsyncMutex<UdpInputSender>>>>, sequence: Mutex<u32> }

impl ActiveSessions {
    pub fn activate(&self, name: String, key: [u8; 32], client_role: bool) {
        let (send_label, receive_label) = if client_role { (b"input client to server".as_slice(), b"input server to client".as_slice()) } else { (b"input server to client".as_slice(), b"input client to server".as_slice()) };
        let session = ActiveSession {
            peer_name: name.clone(),
            input_send_key: *blake3::keyed_hash(&key, send_label).as_bytes(),
            input_receive_key: *blake3::keyed_hash(&key, receive_label).as_bytes(),
        };
        self.sessions.lock().expect("session mutex poisoned").insert(name, session);
    }
    pub fn peers(&self) -> Vec<String> { self.sessions.lock().expect("session mutex poisoned").values().map(|s| s.peer_name.clone()).collect() }
    pub fn input_send_key(&self, name: &str) -> Option<[u8; 32]> { self.sessions.lock().ok()?.get(name).map(|s| s.input_send_key) }
    pub fn input_receive_key(&self, name: &str) -> Option<[u8; 32]> { self.sessions.lock().ok()?.get(name).map(|s| s.input_receive_key) }

    pub async fn add_sender(&self, peer_name: String, ip: IpAddr, key: [u8; 32]) -> Result<()> {
        let addr = SocketAddr::new(ip, crate::discovery::INPUT_PORT);
        let sender = UdpInputSender::connect("0.0.0.0:0", addr, key, 1).await?;
        self.senders.lock().expect("sender map poisoned").insert(peer_name, Arc::new(AsyncMutex::new(sender)));
        Ok(())
    }

    pub async fn send_event(&self, peer_name: &str, event: InputEvent) -> Result<()> {
        let sender = self.senders.lock().expect("sender map poisoned").get(peer_name).cloned().ok_or_else(|| anyhow::anyhow!("no active input sender for peer"))?;
        let sequence = { let mut s = self.sequence.lock().expect("sequence mutex poisoned"); *s = s.wrapping_add(1); *s };
        let result = sender.lock().await.send_event(sequence, event).await;
        result
    }
}
