//! SPAKE2 password-authenticated key exchange for short pairing codes.
use spake2::{Ed25519Group, Identity, Password, Spake2};

pub struct PakeOffer(Spake2<Ed25519Group>);

impl PakeOffer {
    pub fn start(pin: &str, session_id: &[u8]) -> (Self, Vec<u8>) {
        let (state, message) = Spake2::<Ed25519Group>::start_symmetric(&Password::new(pin.as_bytes()), &Identity::new(session_id));
        (Self(state), message)
    }
    pub fn finish(self, message: &[u8]) -> Result<[u8; 32], String> {
        let secret = self.0.finish(message).map_err(|_| "pairing PIN proof rejected".to_string())?;
        let mut key = [0; 32];
        if secret.len() < 32 { return Err("PAKE returned an invalid secret".into()); }
        key.copy_from_slice(&secret[..32]);
        Ok(key)
    }
}
