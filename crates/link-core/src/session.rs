//! Fresh X25519 secrets for a paired connection; the PIN flow authenticates peers.
use x25519_dalek::{PublicKey, StaticSecret};

pub struct SessionOffer { secret: StaticSecret, public: PublicKey }

impl SessionOffer {
    pub fn generate() -> Self { let secret = StaticSecret::random_from_rng(rand::rngs::OsRng); let public = PublicKey::from(&secret); Self { secret, public } }
    pub fn public_bytes(&self) -> [u8; 32] { self.public.to_bytes() }
    pub fn derive_key(&self, peer_public: [u8; 32], context: &[u8]) -> [u8; 32] {
        let shared = self.secret.diffie_hellman(&PublicKey::from(peer_public));
        *blake3::keyed_hash(shared.as_bytes(), context).as_bytes()
    }

    pub fn derive_authenticated_key(&self, peer_public: [u8; 32], pake_key: &[u8; 32], transcript: &[u8]) -> [u8; 32] {
        let shared = self.secret.diffie_hellman(&PublicKey::from(peer_public));
        let mut material = Vec::with_capacity(32 + transcript.len());
        material.extend_from_slice(shared.as_bytes());
        material.extend_from_slice(transcript);
        *blake3::keyed_hash(pake_key, &material).as_bytes()
    }
}
