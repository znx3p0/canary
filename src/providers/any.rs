#[cfg(not(target_arch = "wasm32"))]
use super::Tcp;
#[cfg(unix)]
use super::Unix;
#[cfg(not(target_arch = "wasm32"))]
use crate::{channel::Handshake, Result};

use super::Wss;

use derive_more::From;

#[derive(From)]
/// abstraction over any provider
pub enum AnyProvider {
    #[cfg(not(target_arch = "wasm32"))]
    /// encapsulates the tcp provider
    Tcp(Tcp),
    #[cfg(unix)]
    /// encapsulates the unix provider
    Unix(Unix),
    /// encapsulates the websocket provider
    Wss(Wss),
}

impl AnyProvider {
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    /// get the next channel
    /// ```norun
    /// while let Ok(chan) = provider.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next(&self) -> Result<Handshake> {
        match self {
            AnyProvider::Tcp(provider) => provider.next().await,
            #[cfg(unix)]
            AnyProvider::Unix(provider) => provider.next().await,
            AnyProvider::Wss(provider) => provider.next().await,
        }
    }
}
