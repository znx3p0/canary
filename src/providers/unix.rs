#![cfg(unix)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use crate::channel::handshake::Handshake;
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
    /// Bind to this address
    /// ```no_run
    /// let unix = Unix::bind("127.0.0.1:8080").await?;
    /// while let Ok(chan) = unix.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<Self> {
        let listener = UnixListener::bind(addrs)?;
        Ok(Unix(listener))
    }
    #[inline]
    /// get the next channel
    /// ```no_run
    /// while let Ok(chan) = unix.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next(&self) -> Result<Handshake> {
        let (raw, _) = self.0.accept().await?;
        Ok(Handshake::from(Channel::from_raw(
            raw,
            Default::default(),
            Default::default(),
        )))
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
        let addrs = &addrs;
        let mut attempt = 0;
        let raw = loop {
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
        Ok(Handshake::from(Channel::from_raw(
            raw,
            Default::default(),
            Default::default(),
        )))
    }
}
