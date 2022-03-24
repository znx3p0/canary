use std::pin::Pin;

use futures::{pin_mut, select, stream::FuturesUnordered, FutureExt};
use futures_lite::StreamExt;

#[cfg(not(target_arch = "wasm32"))]
use super::Tcp;
#[cfg(unix)]
use super::Unix;
use crate::channel::Handshake;
use crate::Channel;
use crate::Result;

use super::Wss;

/// abstraction over any provider
pub enum AnyProvider {
    #[cfg(not(target_arch = "wasm32"))]
    /// encapsulates the tcp provider
    Tcp(Tcp),
    #[cfg(not(target_arch = "wasm32"))]
    /// encapsulates the tcp provider without any encryption
    InsecureTcp(Tcp),
    #[cfg(unix)]
    /// encapsulates the unix provider
    Unix(Unix),
    #[cfg(unix)]
    /// encapsulates the unix provider without any encryption
    InsecureUnix(Unix),
    /// encapsulates the websocket provider
    Wss(Wss),
    /// encapsulates the websocket provider without any encryption
    InsecureWss(Wss),
}

impl AnyProvider {
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    /// get the next handshake
    ///
    /// CANCEL SAFETY: this method is cancel-safe, feel free to use it in select statements.
    /// ```norun
    /// while let Ok(chan) = provider.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next_handshake(&self) -> Result<Handshake> {
        match self {
            AnyProvider::Tcp(provider) => provider.next().await,
            AnyProvider::InsecureTcp(provider) => provider.next().await,
            #[cfg(unix)]
            AnyProvider::Unix(provider) => provider.next().await,
            #[cfg(unix)]
            AnyProvider::InsecureUnix(provider) => provider.next().await,
            AnyProvider::Wss(provider) => provider.next().await,
            AnyProvider::InsecureWss(provider) => provider.next().await,
        }
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    /// get the encryption of the provider
    pub fn encrypted(&self) -> bool {
        match self {
            AnyProvider::Tcp(_) => true,
            AnyProvider::InsecureTcp(_) => false,
            #[cfg(unix)]
            AnyProvider::Unix(_) => true,
            #[cfg(unix)]
            AnyProvider::InsecureUnix(_) => false,
            AnyProvider::Wss(_) => true,
            AnyProvider::InsecureWss(_) => false,
        }
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    /// get the next channel
    /// ! NOTE: You should only use this method as the example shows, since
    /// it uses internal future tooling to avoid using another runtime.
    /// ```norun
    /// while let Ok(chan) = provider.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub fn channels(self) -> ChannelIter {
        ChannelIter {
            listener: self,
            futures: FuturesUnordered::new(),
        }
    }
}

/// iterator over channels. NOTE: not completely zero-cost
pub struct ChannelIter {
    listener: AnyProvider,
    futures: FuturesUnordered<Pin<Box<dyn std::future::Future<Output = Result<Channel>>>>>,
}

impl ChannelIter {
    /// get the next channel from the provider
    pub async fn next(&mut self) -> Result<Channel> {
        let hs = self.listener.next_handshake().fuse();
        pin_mut!(hs);

        loop {
            let chan = select! {
                chan = self.futures.next().fuse() => {
                    match chan {
                        Some(chan) => chan,
                        None => continue,
                    }
                },
                res = hs => {
                    let hs: Handshake = res?;
                    if self.listener.encrypted() {
                        let fut = hs.encrypted();
                        self.futures.push(Box::pin(fut));
                        continue;
                    } else {
                        return Ok(hs.raw());
                    }
                },
            };
            break chan;
        }
    }
}
