use derive_more::From;
use futures::stream::SplitStream;
use serde::de::DeserializeOwned;

use crate::serialization::formats::Format;
use crate::Result;
use crate::{io::Wss, serialization::formats::ReadFormat};

#[derive(From)]
/// Reference unformatted raw receive channel
pub enum RefUnformattedRawReceiveChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    Tcp(&'a mut tokio::net::tcp::OwnedReadHalf),
    #[cfg(unix)]
    /// unencrypted unix backend
    Unix(&'a mut tokio::net::unix::OwnedReadHalf),
    /// unencrypted wss backend
    WSS(&'a mut SplitStream<Box<Wss>>),
    #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
    /// unencrypted quic backend
    Quic(&'a mut quinn::RecvStream),
}

#[derive(From)]
/// Owned unformatted unencrypted receive channel
pub enum UnformattedRawReceiveChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// Unencrypted tcp backend
    Tcp(tokio::net::tcp::OwnedReadHalf),
    #[cfg(unix)]
    /// Unencrypted unix backend
    Unix(tokio::net::unix::OwnedReadHalf),
    /// Unencrypted wss backend
    WSS(SplitStream<Box<Wss>>),

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "quic")]
    /// Unencrypted quic backend
    Quic(quinn::RecvStream),
}

#[derive(From)]
/// Reference unencrypted receive channel
pub struct RefRawReceiveChannel<'a, F = Format> {
    /// Inner reference unformatted channel
    channel: &'a mut RefUnformattedRawReceiveChannel<'a>,
    /// Inner format
    format: F,
}

#[derive(From)]
/// Unencrypted receive channel with format
pub struct RawReceiveChannel<F = Format> {
    /// Inner channel
    channel: UnformattedRawReceiveChannel,
    /// Inner format
    format: F,
}

impl<'a> RefUnformattedRawReceiveChannel<'a> {
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        #[allow(unused)]
        use crate::serialization::{rx, wss_rx};
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            RefUnformattedRawReceiveChannel::Tcp(st) => rx(st, format).await,
            #[cfg(unix)]
            RefUnformattedRawReceiveChannel::Unix(st) => rx(st, format).await,
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            RefUnformattedRawReceiveChannel::Quic(st) => rx(st, format).await,
            RefUnformattedRawReceiveChannel::WSS(st) => wss_rx(st, format).await,
        }
    }
    /// Get a formatted channel with the specified format
    /// ```no_run
    /// let string: String = unformatted.receive(&mut Format::Bincode).await?;
    /// let mut formatted = unformatted.as_formatted(Format::Bincode).await?;
    /// let string: String = formatted.receive().await?;
    /// ```
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawReceiveChannel<'a, F> {
        RefRawReceiveChannel {
            channel: self,
            format,
        }
    }
}

impl<'a> From<&'a mut UnformattedRawReceiveChannel> for RefUnformattedRawReceiveChannel<'a> {
    #[inline]
    fn from(chan: &'a mut UnformattedRawReceiveChannel) -> Self {
        match chan {
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedRawReceiveChannel::Tcp(ref mut chan) => chan.into(),
            #[cfg(unix)]
            UnformattedRawReceiveChannel::Unix(ref mut chan) => chan.into(),
            UnformattedRawReceiveChannel::WSS(ref mut chan) => chan.into(),
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "quic")]
            UnformattedRawReceiveChannel::Quic(ref mut chan) => chan.into(),
        }
    }
}

impl UnformattedRawReceiveChannel {
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        RefUnformattedRawReceiveChannel::from(self)
            .receive(format)
            .await
    }
    #[inline]
    /// Format the channel
    /// ```no_run
    /// let formatted = unformatted.to_formatted(Format::Bincode);
    /// ```
    pub fn to_formatted<F: ReadFormat>(self, format: F) -> RawReceiveChannel<F> {
        RawReceiveChannel {
            channel: self,
            format,
        }
    }
}

impl<F: ReadFormat> RefRawReceiveChannel<'_, F> {
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.channel.receive(&mut self.format).await
    }
}

impl<F: ReadFormat> RawReceiveChannel<F> {
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.channel.receive(&mut self.format).await
    }
}
