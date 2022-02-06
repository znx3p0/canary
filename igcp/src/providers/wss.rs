use crate::discovery::Status;
use crate::Result;

use cfg_if::cfg_if;
use igcp::err;
use igcp::Channel;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use crate::runtime;
        use igcp::BareChannel;
        use crate::io::{TcpListener, ToSocketAddrs};
        use crate::io::wss;
    }
}
/// Exposes routes over WebSockets
pub struct Wss(#[cfg(not(target_arch = "wasm32"))] TcpListener);
// pub struct WebsocketHandshake();

#[cfg(not(target_arch = "wasm32"))]
impl Wss {
    #[inline]
    #[cfg(feature = "tokio-net")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(Wss(listener))
        // Ok(runtime::spawn(async move {
        //     loop {
        //         let (stream, _) = listener.accept().await?;
        //         runtime::spawn(async move {
        //             let stream = wss::tokio::accept_async(stream)
        //                 .await
        //                 .map_err(|e| err!(e.to_string()))?;
        //             let chan: Channel = Channel::new_wss_encrypted(stream).await?;
        //             let chan: BareChannel = chan.bare();
        //             GLOBAL_ROUTE.introduce(chan).await?;
        //             Ok::<_, igcp::Error>(())
        //         });
        //     }
        // }))
    }
    // #[inline]
    // #[cfg(feature = "async-std-net")]
    // /// bind the global route on the given address
    // pub async fn bind(addrs: impl ToSocketAddrs) -> Result<JoinHandle<Result<()>>> {
    //     let listener = TcpListener::bind(addrs).await?;
    //     Ok(runtime::spawn(async move {
    //         loop {
    //             let (stream, _) = listener.accept().await?;
    //             runtime::spawn(async move {
    //                 let stream = wss::accept_async(stream)
    //                     .await
    //                     .map_err(|e| err!(e.to_string()))?;
    //                 let chan: Channel = Channel::new_wss_encrypted(stream).await?;
    //                 let chan: BareChannel = chan.bare();
    //                 GLOBAL_ROUTE.introduce(chan).await?;
    //                 Ok::<_, igcp::Error>(())
    //             });
    //         }
    //     }))
    // }
    #[inline]
    #[cfg(feature = "tokio-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let addrs = tokio::net::lookup_host(&addrs)
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let stream = loop {
            match wss::tokio::connect_async(&format!("ws://{}", &addrs)).await {
                Ok((client, _)) => {
                    break client;
                }
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs.to_string(),
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
        let chan = Channel::new_wss_encrypted(stream).await?;
        Ok(chan)
    }
    #[inline]
    #[cfg(feature = "async-std-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let addrs = addrs
            .to_socket_addrs()
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let stream = loop {
            match wss::async_std::connect_async(&format!("ws://{}", &addrs)).await {
                Ok((client, _)) => {
                    break client;
                }
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs.to_string(),
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
        let chan = Channel::new_wss_encrypted(stream).await?;
        Ok(chan)
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
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

#[cfg(target_arch = "wasm32")]
impl Wss {
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match reqwasm::websocket::futures::WebSocket::open(&format!("ws://{}", addrs)) {
                Ok(s) => break s,
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{}` failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e.to_string()))?
                    }
                    continue;
                }
            }
        };
        let chan = Channel::new_wss_encrypted(stream).await?;
        Ok(chan)
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: &str, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: &str,
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
pub struct InsecureWss(#[cfg(not(target_arch = "wasm32"))] TcpListener);

#[cfg(not(target_arch = "wasm32"))]
impl InsecureWss {
    #[inline]
    #[cfg(feature = "async-std-net")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let stream = wss::accept_async(stream)
                        .await
                        .map_err(|e| err!(e.to_string()))?;
                    let chan: Channel = Channel::new_wss_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    #[cfg(feature = "tokio-net")]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let stream = wss::tokio::accept_async(stream)
                        .await
                        .map_err(|e| err!(e))?;
                    let chan: Channel = Channel::new_wss_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce(chan).await?;
                    Ok::<_, igcp::Error>(())
                });
            }
        }))
    }
    #[inline]
    #[cfg(feature = "async-std-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let addrs = addrs
            .to_socket_addrs()
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let stream = loop {
            match wss::async_std::connect_async(&format!("ws://{}", &addrs)).await {
                Ok((client, _)) => {
                    break client;
                }
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs.to_string(),
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
        let chan = Channel::from(stream);
        Ok(chan)
    }
    #[inline]
    #[cfg(feature = "tokio-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let addrs = tokio::net::lookup_host(&addrs)
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let stream = loop {
            match wss::tokio::connect_async(&format!("ws://{}", &addrs)).await {
                Ok((client, _)) => {
                    break client;
                }
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs.to_string(),
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
        let chan = Channel::from(stream);
        Ok(chan)
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
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

#[cfg(target_arch = "wasm32")]
impl InsecureWss {
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match reqwasm::websocket::futures::WebSocket::open(addrs) {
                Ok(s) => break s,
                Err(e) => {
                    tracing::error!(
                        "connecting to address `{:?}` failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e.to_string()))?
                    }
                    continue;
                }
            }
        };
        let chan = Channel::from(stream);
        Ok(chan)
    }
    #[inline]
    /// connect to the following address with the following id. Defaults to 3 retries.
    pub async fn connect(addrs: &str, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(
        addrs: &str,
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
