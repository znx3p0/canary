#![cfg(not(target_arch = "wasm32"))]

use crate::channel::Handshake;
use crate::err;
use crate::io::TcpListener;
use crate::io::TcpStream;
use crate::io::ToSocketAddrs;
use crate::Channel;
use crate::Result;

/// Exposes routes over TCP
pub struct Tcp(TcpListener);

impl Tcp {
    #[inline]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(Tcp(listener))
    }
    #[inline]
    pub async fn next(&self) -> Result<Handshake> {
        let (chan, _) = self.0.accept().await?;
        let chan: Channel = Channel::from(chan);
        Ok(Handshake::from(chan))
    }
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect(&addrs).await {
                Ok(s) => break s,
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    crate::io::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e))?
                    }
                    continue;
                }
            }
        };
        let chan = Channel::from(stream);
        Ok(Handshake::from(chan))
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
        Self::connect_retry(addrs, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
        Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
    }
}
