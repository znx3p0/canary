
#![cfg(not(target_arch = "wasm32"))]

use crate::routes::Status;
use crate::routes::GLOBAL_ROUTE;
use crate::runtime;
use crate::runtime::JoinHandle;
use crate::Result;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_tungstenite::tungstenite::protocol::Role;
use igcp::err;
use igcp::BareChannel;
use igcp::Channel;

/// Exposes routes over WebSockets
pub struct Wss(TcpListener);

impl Wss {
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let stream = async_tungstenite::WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
                    let chan: Channel = Channel::new_wss_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce_static_unspawn(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect(&addrs).await {
                Ok(s) => {
                    let ws = async_tungstenite::WebSocketStream::from_raw_socket(s, Role::Client, None)
                        .await;
                    break ws
                },
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
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
        let chan = Channel::new_wss_encrypted(stream).await?;
        Ok(chan)
    }
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
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


/// Exposes routes over WebSockets
pub struct InsecureWss(TcpListener);

impl InsecureWss {
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let stream = async_tungstenite::WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
                    let chan: Channel = Channel::from(stream);
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce_static_unspawn(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect(&addrs).await {
                Ok(s) => {
                    let ws = async_tungstenite::WebSocketStream::from_raw_socket(s, Role::Client, None)
                        .await;
                    break ws
                },
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
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
        let chan = Channel::from(stream);
        Ok(chan)
    }
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
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
