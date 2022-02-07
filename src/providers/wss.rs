use crate::Result;

use crate::channel::Handshake;
use crate::err;
use crate::Channel;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
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
    }
    #[inline]
    /// get the next channel
    /// ```norun
    /// while let Ok(chan) = wss.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next(&self) -> Result<Handshake> {
        let (chan, _) = self.0.accept().await?;
        let chan = wss::tokio::accept_async(chan)
            .await // this future doesn't suspend, hence why this await point is not delegated upwards.
            .map_err(|e| err!(e.to_string()))?;
        let chan: Channel = Channel::from(chan);
        Ok(Handshake::from(chan))
    }
    #[inline]
    #[cfg(feature = "tokio-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
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
    #[cfg(feature = "async-std-net")]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
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
#[cfg(target_arch = "wasm32")]
impl Wss {
    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
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
                    async_timer::timed(
                        std::future::pending::<()>(),
                        std::time::Duration::from_millis(time_to_retry),
                    )
                    .await
                    .ok();

                    attempt += 1;
                    if attempt == retries {
                        err!((e.to_string()))?
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
    pub async fn connect(addrs: &str) -> Result<Handshake> {
        Self::connect_retry(addrs, 3, 10).await
    }
    #[inline]
    /// connect to the following address with the given id and retry in case of failure
    pub async fn connect_retry(addrs: &str, retries: u32, time_to_retry: u64) -> Result<Handshake> {
        Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
    }
}
