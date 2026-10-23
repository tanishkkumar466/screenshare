//! Datagram protection for latency-sensitive input events.
//!
//! A Noise/QUIC control session supplies an independent 32-byte directional
//! session key. UDP data is never accepted until that authenticated step ends.
use crate::protocol::InputPacket;
use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Key, Nonce};
use thiserror::Error;

const HEADER_BYTES: usize = 8;

#[derive(Debug, Error)]
pub enum SecureInputError {
    #[error("input datagram is too short")]
    TooShort,
    #[error("input datagram was rejected: authentication failed or it was tampered with")]
    Authentication,
    #[error("replayed or out-of-order input datagram")]
    Replay,
    #[error("input payload could not be decoded")]
    Decode,
}

pub struct InputSealer { cipher: ChaCha20Poly1305, next_sequence: u64 }
pub struct InputOpener { cipher: ChaCha20Poly1305, last_sequence: Option<u64> }

impl InputSealer {
    /// `key` must be a fresh, directional secret derived from the paired session.
    pub fn new(key: [u8; 32], initial_sequence: u64) -> Self {
        Self { cipher: ChaCha20Poly1305::new(Key::from_slice(&key)), next_sequence: initial_sequence }
    }

    pub fn seal(&mut self, packet: &InputPacket) -> Result<Vec<u8>, SecureInputError> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let plaintext = serde_json::to_vec(packet).map_err(|_| SecureInputError::Decode)?;
        let ciphertext = self.cipher.encrypt(&nonce(sequence), plaintext.as_ref()).map_err(|_| SecureInputError::Authentication)?;
        let mut datagram = Vec::with_capacity(HEADER_BYTES + ciphertext.len());
        datagram.extend_from_slice(&sequence.to_be_bytes());
        datagram.extend_from_slice(&ciphertext);
        Ok(datagram)
    }
}

impl InputOpener {
    pub fn new(key: [u8; 32]) -> Self {
        Self { cipher: ChaCha20Poly1305::new(Key::from_slice(&key)), last_sequence: None }
    }

    pub fn open(&mut self, datagram: &[u8]) -> Result<InputPacket, SecureInputError> {
        if datagram.len() <= HEADER_BYTES { return Err(SecureInputError::TooShort); }
        let sequence = u64::from_be_bytes(datagram[..HEADER_BYTES].try_into().expect("header length checked"));
        if self.last_sequence.is_some_and(|last| sequence <= last) { return Err(SecureInputError::Replay); }
        let plaintext = self.cipher.decrypt(&nonce(sequence), &datagram[HEADER_BYTES..]).map_err(|_| SecureInputError::Authentication)?;
        let packet = serde_json::from_slice(&plaintext).map_err(|_| SecureInputError::Decode)?;
        self.last_sequence = Some(sequence);
        Ok(packet)
    }
}

fn nonce(sequence: u64) -> Nonce {
    let mut bytes = [0u8; 12];
    bytes[4..].copy_from_slice(&sequence.to_be_bytes());
    *Nonce::from_slice(&bytes)
}
