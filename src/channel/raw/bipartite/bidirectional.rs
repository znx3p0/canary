use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::receive_channel::{
    RawReceiveChannel, RefUnformattedRawReceiveChannel, UnformattedRawReceiveChannel,
};
use super::send_channel::{
    RawSendChannel, RefUnformattedRawSendChannel, UnformattedRawSendChannel,
};

#[derive(From)]
/// Unformatted unencrypted bidirectional channel,
/// joins a send and a receive component
pub struct UnformattedRawBidirectionalChannel {
    /// Inner send component
    pub send_chan: UnformattedRawSendChannel,
    /// Inner receive component
    pub receive_chan: UnformattedRawReceiveChannel,
}

impl UnformattedRawBidirectionalChannel {
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        (self.send_chan, self.receive_chan)
    }
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        self.receive_chan.receive(format).await
    }
}

#[derive(From)]
/// Reference unformatted unencrypted bidirectional channel,
/// joins a send and a receive component
pub struct RefUnformattedRawBidirectionalChannel<'a> {
    /// Inner reference send component
    pub send_chan: RefUnformattedRawSendChannel<'a>,
    /// Inner reference receive component
    pub receive_chan: RefUnformattedRawReceiveChannel<'a>,
}

impl<'a> From<&'a mut UnformattedRawBidirectionalChannel>
    for RefUnformattedRawBidirectionalChannel<'a>
{
    fn from(chan: &'a mut UnformattedRawBidirectionalChannel) -> Self {
        let send_chan = RefUnformattedRawSendChannel::from(&mut chan.send_chan);
        let receive_chan = RefUnformattedRawReceiveChannel::from(&mut chan.receive_chan);
        Self {
            send_chan,
            receive_chan,
        }
    }
}

impl<'a> RefUnformattedRawBidirectionalChannel<'a> {
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        self.receive_chan.receive(format).await
    }
    /// Get a formatted channel with the specified format
    /// ```no_run
    /// unformatted.send("Hi!", &mut Format::Bincode).await?;
    /// let mut formatted = unformatted.as_formatted(Format::Bincode).await?;
    /// formatted.send("Hi!").await?;
    /// ```
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawBidirectionalChannel<'a, F> {
        RefRawBidirectionalChannel { chan: self, format }
    }
}

#[derive(From)]
/// Unencrypted bidirectional channel
pub struct RawBidirectionalChannel<F = Format> {
    /// Inner channel
    pub chan: UnformattedRawBidirectionalChannel,
    /// Inner format
    pub format: F,
}

impl<F> RawBidirectionalChannel<F> {
    /// Construct a new unencrypted bidirectional channel from an `UnformattedRawBidirectionalChannel` and a format
    pub fn from_unformatted_with(chan: UnformattedRawBidirectionalChannel, format: F) -> Self {
        Self { chan, format }
    }
    #[inline]
    /// Strip the format getting the inner channel
    pub fn to_unformatted(self) -> UnformattedRawBidirectionalChannel {
        self.chan
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.chan.receive(&mut self.format).await
    }
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.chan.send(obj, &mut self.format).await
    }
}

impl RawBidirectionalChannel {
    /// Construct a new unencrypted bidirectional channel from an `UnformattedRawBidirectionalChannel` using the default format (bincode)
    pub fn from_unformatted(chan: UnformattedRawBidirectionalChannel) -> Self {
        Self {
            chan,
            format: Format::default(),
        }
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (RawSendChannel, RawReceiveChannel) {
        let (send, receive) = self.chan.split();
        let send = send.to_formatted(self.format.clone());
        let receive = receive.to_formatted(self.format);
        (send, receive)
    }
}

#[derive(From)]
/// Reference unencrypted bidirectional channel
pub struct RefRawBidirectionalChannel<'a, F = Format> {
    /// Inner reference channel
    pub chan: &'a mut RefUnformattedRawBidirectionalChannel<'a>,
    /// Inner format
    pub format: F,
}

impl<'a, F> RefRawBidirectionalChannel<'a, F> {
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.chan.receive(&mut self.format).await
    }
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.chan.send(obj, &mut self.format).await
    }
}
