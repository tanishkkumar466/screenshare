//! High-priority encrypted UDP path for mouse and keyboard events.
//!
//! Session keys are supplied only after a future authenticated control-plane
//! pairing flow. This module intentionally has no fallback plaintext mode.
use anyhow::{bail, Result};
use link_core::{InputEvent, InputOpener, InputSealer};
use link_core::protocol::InputPacket;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::UdpSocket;

const MAX_INPUT_DATAGRAM: usize = 1200;

pub struct UdpInputSender {
    socket: UdpSocket,
    peer: SocketAddr,
    sealer: InputSealer,
}

pub struct UdpInputReceiver {
    socket: UdpSocket,
    opener: InputOpener,
}

impl UdpInputSender {
    pub async fn connect(bind: impl ToSocketAddrs, peer: SocketAddr, session_key: [u8; 32], initial_sequence: u64) -> Result<Self> {
        let bind = bind.to_socket_addrs()?.next().ok_or_else(|| anyhow::anyhow!("no bind address"))?;
        Ok(Self { socket: UdpSocket::bind(bind).await?, peer, sealer: InputSealer::new(session_key, initial_sequence) })
    }

    pub async fn send(&mut self, packet: &InputPacket) -> Result<()> {
        let datagram = self.sealer.seal(packet)?;
        if datagram.len() > MAX_INPUT_DATAGRAM { bail!("input event exceeds UDP safety budget"); }
        self.socket.send_to(&datagram, self.peer).await?;
        Ok(())
    }

    pub async fn send_event(&mut self, sequence: u32, event: InputEvent) -> Result<()> {
        self.send(&InputPacket { sequence, event }).await
    }
}

impl UdpInputReceiver {
    pub async fn bind(bind: impl ToSocketAddrs, session_key: [u8; 32]) -> Result<Self> {
        let bind = bind.to_socket_addrs()?.next().ok_or_else(|| anyhow::anyhow!("no bind address"))?;
        Ok(Self { socket: UdpSocket::bind(bind).await?, opener: InputOpener::new(session_key) })
    }

    /// Returns the sender address only for observability. Authentication comes
    /// from the packet AEAD, not from the IP address.
    pub async fn receive(&mut self) -> Result<(InputPacket, SocketAddr)> {
        let mut datagram = [0u8; MAX_INPUT_DATAGRAM];
        let (length, sender) = self.socket.recv_from(&mut datagram).await?;
        Ok((self.opener.open(&datagram[..length])?, sender))
    }
}
