use anyhow::{bail, Result};
use link_core::{pake::PakeOffer, protocol::ControlMessage, session::SessionOffer};
use subtle::ConstantTimeEq;
use tokio::net::TcpStream;

pub struct PairedSession { pub peer_name: String, pub key: [u8; 32], pub peer_key: [u8; 32] }

fn transcript(session_id: &[u8], a: &[u8; 32], b: &[u8; 32]) -> Vec<u8> {
    let mut t = b"DeskLink pairing v1".to_vec(); t.extend_from_slice(session_id); t.extend_from_slice(a); t.extend_from_slice(b); t
}
fn confirm(key: &[u8; 32], label: &[u8], t: &[u8]) -> [u8; 32] {
    let mut data = label.to_vec(); data.extend_from_slice(t); *blake3::keyed_hash(key, &data).as_bytes()
}

pub async fn client(address: &str, local_name: &str, pin: &str) -> Result<PairedSession> {
    if pin.len() != 6 || !pin.bytes().all(|b| b.is_ascii_digit()) { bail!("pairing code must be six digits"); }
    let mut stream = TcpStream::connect(address).await?;
    let session_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let local = SessionOffer::generate();
    let (pake, pake_message) = PakeOffer::start(pin, &session_id);
    control_channel::send(&mut stream, &ControlMessage::Hello { device_name: local_name.into(), version: 1 }).await?;
    control_channel::send(&mut stream, &ControlMessage::PairingInit { session_id: session_id.clone(), pake_message, ephemeral_key: local.public_bytes() }).await?;
    let (peer_name, peer_pake, peer_key, proof) = match control_channel::receive(&mut stream).await? {
        ControlMessage::PairingReply { peer_name, pake_message, ephemeral_key, proof } => (peer_name, pake_message, ephemeral_key, proof),
        _ => bail!("peer rejected or sent an invalid pairing response"),
    };
    let pake_key = pake.finish(&peer_pake).map_err(anyhow::Error::msg)?;
    let t = transcript(&session_id, &local.public_bytes(), &peer_key);
    let key = local.derive_authenticated_key(peer_key, &pake_key, &t);
    if !bool::from(confirm(&key, b"server confirm", &t).ct_eq(&proof)) { bail!("pairing code did not match"); }
    control_channel::send(&mut stream, &ControlMessage::PairingFinish { proof: confirm(&key, b"client confirm", &t) }).await?;
    match control_channel::receive(&mut stream).await? {
        ControlMessage::PairingAccepted { .. } => Ok(PairedSession { peer_name, key, peer_key }),
        _ => bail!("peer did not confirm pairing"),
    }
}

pub async fn server(stream: &mut TcpStream, peer_name: String, pin: &str) -> Result<PairedSession> {
    let (session_id, remote_pake, remote_key) = match control_channel::receive(stream).await? {
        ControlMessage::PairingInit { session_id, pake_message, ephemeral_key } if session_id.len() == 16 => (session_id, pake_message, ephemeral_key),
        _ => bail!("invalid pairing initiation"),
    };
    let local = SessionOffer::generate();
    let (pake, pake_message) = PakeOffer::start(pin, &session_id);
    let pake_key = pake.finish(&remote_pake).map_err(anyhow::Error::msg)?;
    let t = transcript(&session_id, &remote_key, &local.public_bytes());
    let key = local.derive_authenticated_key(remote_key, &pake_key, &t);
    control_channel::send(stream, &ControlMessage::PairingReply { peer_name: "DeskLink".into(), pake_message, ephemeral_key: local.public_bytes(), proof: confirm(&key, b"server confirm", &t) }).await?;
    match control_channel::receive(stream).await? {
        ControlMessage::PairingFinish { proof } if bool::from(confirm(&key, b"client confirm", &t).ct_eq(&proof)) => {
            control_channel::send(stream, &ControlMessage::PairingAccepted { peer_name: peer_name.clone() }).await?;
            Ok(PairedSession { peer_name, key, peer_key: remote_key })
        }
        _ => bail!("pairing confirmation failed"),
    }
}

use crate::control_channel;
