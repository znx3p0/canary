use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::channel::raw::bipartite::bidirectional::UnformattedRawBidirectionalChannel;
use crate::channel::raw::bipartite::receive_channel::UnformattedRawReceiveChannel;
use crate::channel::raw::bipartite::send_channel::UnformattedRawSendChannel;
use crate::channel::raw::unified::unformatted::{
    RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel,
};
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::super::bipartite::bidirectional::RefUnformattedRawBidirectionalChannel;

#[derive(From)]
/// Reference unformatted unencrypted channel
pub enum RefUnformattedRawChannel<'a> {
    /// Channel has not been split into its read/write parts
    Unified(RefUnformattedRawUnifiedChannel<'a>),
    /// Channel has been split but is now joint
    Bipartite(RefUnformattedRawBidirectionalChannel<'a>),
}

#[derive(From)]
/// Unformatted unencrypted channel
pub enum UnformattedRawChannel {
    /// Channel has not been split into its read/write parts
    Unified(UnformattedRawUnifiedChannel),
    /// Channel has been split but is now joint
    Bipartite(UnformattedRawBidirectionalChannel),
}

#[derive(From)]
/// Reference unencrypted channel with format
pub struct RefRawChannel<'a, F = Format> {
    /// Inner reference unformatted channel
    channel: RefUnformattedRawChannel<'a>,
    /// Inner format
    format: F,
}

impl<'a, F> RefRawChannel<'a, F> {
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
    #[inline]
    /// Strip the format getting the inner channel
    pub fn to_unformatted(self) -> RefUnformattedRawChannel<'a> {
        self.channel
    }
}

#[derive(From)]
/// Unencrypted channel with format
pub struct RawChannel<F = Format> {
    /// Inner channel
    channel: UnformattedRawChannel,
    /// Inner format
    format: F,
}

impl<F> RawChannel<F> {
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
    #[inline]
    /// Strip the format getting the inner channel
    pub fn to_unformatted(self) -> UnformattedRawChannel {
        self.channel
    }
}

impl<'a> From<&'a mut UnformattedRawChannel> for RefUnformattedRawChannel<'a> {
    fn from(chan: &'a mut UnformattedRawChannel) -> Self {
        match chan {
            UnformattedRawChannel::Unified(chan) => {
                RefUnformattedRawChannel::Unified(RefUnformattedRawUnifiedChannel::from(chan))
            }
            UnformattedRawChannel::Bipartite(chan) => RefUnformattedRawChannel::Bipartite(
                RefUnformattedRawBidirectionalChannel::from(chan),
            ),
        }
    }
}

impl<'a> RefUnformattedRawChannel<'a> {
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        match self {
            Self::Unified(chan) => chan.send(obj, format).await,
            Self::Bipartite(chan) => chan.send(obj, format).await,
        }
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        match self {
            Self::Unified(chan) => chan.receive(format).await,
            Self::Bipartite(chan) => chan.receive(format).await,
        }
    }
    #[inline]
    /// Format the channel
    /// ```no_run
    /// let formatted = unformatted.to_formatted(Format::Bincode);
    /// ```
    pub fn to_formatted<F>(self, format: F) -> RefRawChannel<'a, F> {
        RefRawChannel {
            channel: self,
            format,
        }
    }
}

impl UnformattedRawChannel {
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        match self {
            Self::Unified(chan) => chan.send(obj, format).await,
            Self::Bipartite(chan) => chan.send(obj, format).await,
        }
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        match self {
            Self::Unified(chan) => chan.receive(format).await,
            Self::Bipartite(chan) => chan.receive(format).await,
        }
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        match self {
            UnformattedRawChannel::Unified(chan) => chan.split(),
            UnformattedRawChannel::Bipartite(chan) => chan.split(),
        }
    }
}
