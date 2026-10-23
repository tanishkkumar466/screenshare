use link_core::pairing::PairingChallenge;
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};

const PIN_LIFETIME: Duration = Duration::from_secs(120);

pub struct PairingSessions { pending: Mutex<HashMap<String, (PairingChallenge, Instant)>>, local_code: Mutex<Option<(String, Instant)>> }

impl PairingSessions {
    pub fn new() -> Self { Self { pending: Mutex::new(HashMap::new()), local_code: Mutex::new(None) } }
    pub fn generate_local_code(&self) -> String {
        let challenge = PairingChallenge::generate();
        let pin = challenge.pin;
        *self.local_code.lock().expect("pairing mutex poisoned") = Some((pin.clone(), Instant::now()));
        pin
    }
    pub fn take_local_code(&self) -> Option<String> {
        let mut code = self.local_code.lock().expect("pairing mutex poisoned");
        let (pin, created) = code.take()?;
        (created.elapsed() <= PIN_LIFETIME).then_some(pin)
    }
    pub fn begin(&self, peer_name: String) -> String {
        let challenge = PairingChallenge::generate();
        let pin = challenge.pin.clone();
        self.pending.lock().expect("pairing mutex poisoned").insert(peer_name, (challenge, Instant::now()));
        pin
    }
    pub fn verify(&self, peer_name: &str, public_key: &[u8], proof: &[u8]) -> bool {
        let mut sessions = self.pending.lock().expect("pairing mutex poisoned");
        let Some((challenge, created)) = sessions.remove(peer_name) else { return false; };
        created.elapsed() <= PIN_LIFETIME && challenge.verifies(public_key, proof)
    }
}
