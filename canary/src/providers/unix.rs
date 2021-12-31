#![cfg(unix)]

use std::fmt::Debug;

use crate::routes::Status;
use crate::routes::GLOBAL_ROUTE;
use crate::runtime;
use crate::runtime::JoinHandle;
use crate::Result;
use async_std::os::unix::net::UnixListener;
use async_std::os::unix::net::UnixStream;
use async_std::path::Path;
use igcp::err;
use igcp::BareChannel;
use igcp::Channel;

pub struct Unix(UnixListener);

impl Unix {
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let chan: Channel = Channel::new_unix_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce_static_unspawn(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
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
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
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
    pub async fn connect(addrs: impl AsRef<Path> + Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
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
                format!("id `{id}` not found at address {:?}", addrs)
            )),
        }
    }
}

pub struct InsecureUnix(UnixListener);

impl InsecureUnix {
    pub async fn bind(addrs: impl AsRef<Path>) -> Result<JoinHandle<Result<()>>> {
        let listener = UnixListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let chan = BareChannel::InsecureUnix(stream);
                GLOBAL_ROUTE.introduce_static(chan);
            }
        }))
    }
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
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
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
    pub async fn connect(addrs: impl AsRef<Path> + Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
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
                format!("id `{id}` not found at address {:?}", addrs)
            )),
        }
    }
}
