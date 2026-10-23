use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingChallenge { pub pin: String, pub nonce: [u8; 16] }

impl PairingChallenge {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        Self { pin: format!("{:06}", rng.gen_range(0..1_000_000)), nonce: rng.gen() }
    }
    /// The control-plane implementation verifies this proof before persisting a peer key.
    pub fn proof(&self, public_key: &[u8]) -> [u8; 32] {
        *blake3::keyed_hash(&blake3::derive_key("desklink pairing v1", self.pin.as_bytes()), &[self.nonce.as_slice(), public_key].concat()).as_bytes()
    }

    pub fn verifies(&self, public_key: &[u8], proof: &[u8]) -> bool {
        proof.len() == 32 && self.proof(public_key).as_slice() == proof
    }
}
