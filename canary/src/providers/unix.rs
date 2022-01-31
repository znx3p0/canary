#![cfg(unix)]
#![cfg(not(target_arch = "wasm32"))]

use std::fmt::Debug;

use crate::discovery::Status;
use crate::io::UnixListener;
use crate::io::UnixStream;
use crate::routes::GLOBAL_ROUTE;
use crate::runtime;
use crate::runtime::JoinHandle;
use crate::Result;
use igcp::err;
use igcp::BareChannel;
use igcp::Channel;

#[cfg(feature = "async-std-rt")]
use async_std::path::Path;
#[cfg(feature = "tokio-rt")]
use std::path::Path;

/// Exposes routes over a Unix socket
pub struct Unix(UnixListener);

impl Unix {
    #[inline]
    #[cfg(feature = "async-std-rt")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl AsRef<async_std::path::Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs).await?;

        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let chan: Channel = Channel::new_unix_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    #[inline]
    #[cfg(feature = "tokio-rt")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs)?;

        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let chan: Channel = Channel::new_unix_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl AsRef<Path> + Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match UnixStream::connect(&addrs).await {
                Ok(s) => break s,
                Err(e) => {
                    tracing::error!(
                        "connecting to address {:?} failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    crate::runtime::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e))?
                    }
                    continue;
                }
            }
        };
        let chan = Channel::new_unix_encrypted(stream).await?;
        Ok(chan)
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl AsRef<Path> + Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl AsRef<Path> + Debug,
        id: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut c = Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await?;
        c.tx(id).await?;
        match c.rx().await? {
            Status::Found => Ok(c),
            Status::NotFound => err!((
                not_found,
                format!("id `{}` not found at address {:?}", id, addrs)
            )),
        }
    }
}

/// Exposes routes over a Unix socket without any encryption
pub struct InsecureUnix(UnixListener);

impl InsecureUnix {
    #[inline]
    #[cfg(feature = "async-std-rt")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let chan = BareChannel::InsecureUnix(stream);
                GLOBAL_ROUTE.introduce(chan).await?;
            }
        }))
    }
    #[inline]
    #[cfg(feature = "tokio-rt")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs)?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let chan = BareChannel::InsecureUnix(stream);
                GLOBAL_ROUTE.introduce(chan).await?;
            }
        }))
    }
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl AsRef<Path> + Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match UnixStream::connect(&addrs).await {
                Ok(s) => break s,
                Err(e) => {
                    tracing::error!(
                        "connecting to address {:?} failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    crate::runtime::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e))?
                    }
                    continue;
                }
            }
        };
        Ok(Channel::InsecureUnix(stream))
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl AsRef<Path> + Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl AsRef<Path> + Debug,
        id: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut c = Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await?;
        c.tx(id).await?;
        match c.rx().await? {
            Status::Found => Ok(c),
            Status::NotFound => err!((
                not_found,
                format!("id `{}` not found at address {:?}", id, addrs)
            )),
        }
    }
}
