#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use crate::channel::Handshake;
use crate::err;
use crate::io::ToSocketAddrs;
use crate::Channel;
use crate::Result;

use derive_more::{From, Into};
use futures::StreamExt;
use quinn::Endpoint;
use quinn::EndpointConfig;
use quinn::Incoming;
use crate::io::UdpSocket;

#[derive(From, Into)]
#[into(owned, ref, ref_mut)]
/// Quic provider
pub struct Quic(pub Endpoint, pub Incoming);

impl Quic {
    #[inline]
    /// Bind a listener to the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let socket = UdpSocket::bind(addrs).await?;
        
        let config = EndpointConfig::default();
        let (e, i) = quinn::Endpoint::new(config, None, socket.into_std()?)?;
        Ok(Quic(e, i))
    }
    /// Get the next channel
    pub async fn next(&mut self) -> Result<Channel> {
        let connecting = self.1.next().await.ok_or(err!("quic socket closed"))?;
        let chan = connecting.await.map_err(|e| err!(e))?;
        let (send, recv) = chan.connection.open_bi().await.map_err(|e| err!(e))?;
        todo!()
    }
}


