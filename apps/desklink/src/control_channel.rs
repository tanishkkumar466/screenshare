//! Framed reliable LAN control channel. mDNS advertises the listener port.
use anyhow::{bail, Result};
use link_core::protocol::ControlMessage;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};

const MAX_CONTROL_FRAME: usize = 64 * 1024;

pub async fn listen() -> Result<TcpListener> { Ok(TcpListener::bind(("0.0.0.0", crate::discovery::CONTROL_PORT)).await?) }

pub async fn send(stream: &mut TcpStream, message: &ControlMessage) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    if data.len() > MAX_CONTROL_FRAME { bail!("control message too large"); }
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn receive(stream: &mut TcpStream) -> Result<ControlMessage> {
    let length = stream.read_u32().await? as usize;
    if length > MAX_CONTROL_FRAME { bail!("invalid control message size"); }
    let mut data = vec![0; length];
    stream.read_exact(&mut data).await?;
    Ok(serde_json::from_slice(&data)?)
}
