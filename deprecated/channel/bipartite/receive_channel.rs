use derive_more::From;
use futures::stream::SplitStream;
use serde::de::DeserializeOwned;

use crate::serialization::formats::Format;
use crate::Result;
use crate::{io::Wss, serialization::formats::ReadFormat};

#[derive(From)]
pub enum RefUnformattedRawReceiveChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    Tcp(&'a mut tokio::net::tcp::OwnedReadHalf),
    #[cfg(unix)]
    /// unencrypted unix backend
    Unix(&'a mut tokio::net::unix::OwnedReadHalf),
    /// unencrypted wss backend
    WSS(&'a mut SplitStream<Box<Wss>>),
}

impl RefUnformattedRawReceiveChannel<'_> {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        use crate::serialization::{rx, wss_rx};
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            RefUnformattedRawReceiveChannel::Tcp(st) => rx(st, f).await,
            #[cfg(unix)]
            RefUnformattedRawReceiveChannel::Unix(st) => rx(st, f).await,
            RefUnformattedRawReceiveChannel::WSS(st) => wss_rx(st, f).await,
        }
    }
}

#[derive(From)]
// owned ref unformatted raw receive channel
pub enum UnformattedRawReceiveChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    Tcp(tokio::net::tcp::OwnedReadHalf),
    #[cfg(unix)]
    /// unencrypted unix backend
    Unix(tokio::net::unix::OwnedReadHalf),
    /// unencrypted wss backend
    WSS(SplitStream<Box<Wss>>),
}

impl<'a> From<&'a mut UnformattedRawReceiveChannel> for RefUnformattedRawReceiveChannel<'a> {
    #[inline]
    fn from(chan: &'a mut UnformattedRawReceiveChannel) -> Self {
        match chan {
            UnformattedRawReceiveChannel::Tcp(ref mut chan) => chan.into(),
            UnformattedRawReceiveChannel::Unix(ref mut chan) => chan.into(),
            UnformattedRawReceiveChannel::WSS(ref mut chan) => chan.into(),
        }
    }
}

impl UnformattedRawReceiveChannel {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        RefUnformattedRawReceiveChannel::from(self).receive(f).await
    }
}
#[derive(From)]
pub struct RefRawReceiveChannel<'a, F = Format> {
    channel: RefUnformattedRawReceiveChannel<'a>,
    format: F,
}

impl<F: ReadFormat> RefRawReceiveChannel<'_, F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.channel.receive(&self.format).await
    }
}

#[derive(From)]
pub struct RawReceiveChannel<F = Format> {
    channel: UnformattedRawReceiveChannel,
    format: F,
}

impl<F: ReadFormat> RawReceiveChannel<F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.channel.receive(&self.format).await
    }
}
