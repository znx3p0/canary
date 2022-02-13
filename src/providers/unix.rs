#![cfg(unix)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use crate::channel::Handshake;
use crate::err;
use crate::io::UnixListener;
use crate::io::UnixStream;
use crate::Channel;
use crate::Result;

use derive_more::{From, Into};
#[derive(From, Into)]
#[into(owned, ref, ref_mut)]
/// Exposes routes over TCP
pub struct Unix(UnixListener);

impl Unix {
    #[inline]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<Self> {
        let listener = UnixListener::bind(addrs)?;
        // let listener = UnixListener::bind(addrs).await?;
        Ok(Unix(listener))
    }
    #[inline]
    /// get the next channel
    /// ```norun
    /// while let Ok(chan) = unix.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next(&self) -> Result<Handshake> {
        let (chan, _) = self.0.accept().await?;
        let chan: Channel = Channel::from(chan);
        Ok(Handshake::from(chan))
    }
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl AsRef<Path> + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
        let mut attempt = 0;
        let stream = loop {
            match UnixStream::connect(&addrs).await {
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
    pub async fn connect(addrs: impl AsRef<Path> + std::fmt::Debug) -> Result<Handshake> {
        Self::connect_retry(addrs, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl AsRef<Path> + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
        Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
    }
}
