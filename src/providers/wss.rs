use crate::Result;

use crate::channel::handshake::Handshake;
use crate::err;
use crate::Channel;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use crate::io::{TcpListener, ToSocketAddrs};
        use crate::io::wss;
        use backoff::ExponentialBackoff;
    } else {
        use crate::io::Wss;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(derive_more::From, derive_more::Into)]
#[into(owned, ref, ref_mut)]
/// Websocket Provider
pub struct WebSocket(TcpListener);

#[cfg(target_arch = "wasm32")]
pub struct WebSocket;

#[cfg(not(target_arch = "wasm32"))]
impl WebSocket {
    #[inline]
    /// bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(WebSocket(listener))
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
        let raw = wss::tokio::accept_async(chan)
            .await // this future doesn't suspend, hence why this await point is not delegated upwards.
            .map_err(|e| err!(e))?;
        let raw = Box::new(raw);
        Ok(Handshake::from(Channel::from_raw(
            raw,
            Default::default(),
            Default::default(),
        )))
    }

    pub async fn connect_no_backoff(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
    ) -> Result<Handshake> {
        let addrs = tokio::net::lookup_host(&addrs)
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let (raw, _) = wss::tokio::connect_async(&format!("ws://{}", &addrs))
            .await
            .map_err(err!(@other))?;
        let raw = Box::new(raw);
        Ok(Handshake::from(Channel::from_raw(
            raw,
            Default::default(),
            Default::default(),
        )))
    }
    #[inline]
    /// Connect to the following address with the given id and retry in case of failure
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
        let addrs = tokio::net::lookup_host(&addrs)
            .await
            .map_err(|e| err!(e))?
            .next()
            .ok_or(err!("no endpoint found"))?;
        let hs = backoff::future::retry(ExponentialBackoff::default(), || async {
            let (raw, _) = wss::tokio::connect_async(&format!("ws://{}", &addrs))
                .await
                .map_err(err!(@other))?;
            let raw = Box::new(raw);
            Ok(Handshake::from(Channel::from_raw(
                raw,
                Default::default(),
                Default::default(),
            )))
        })
        .await?;
        Ok(hs)
    }
}
#[cfg(target_arch = "wasm32")]
impl WebSocket {
    #[inline]
    /// connect to the following address without discovery
    pub async fn inner_connect(addrs: &str, retries: u32, time_to_retry: u64) -> Result<Wss> {
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
        Ok(stream)
    }

    #[inline]
    /// connect to the following address without discovery
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Handshake> {
        let raw = Self::inner_connect(addrs, retries, time_to_retry).await?;
        let raw = Box::new(raw);
        Ok(Handshake::from(Channel::from_raw(
            raw,
            Default::default(),
            Default::default(),
        )))
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
