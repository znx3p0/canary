use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use snow::StatelessTransportState;

use crate::channel::channels::{ReceiveChannel, SendChannel};
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::{receive_channel::UnformattedReceiveChannel, send_channel::UnformattedSendChannel};

/// Unformatted channel that unifies a send and a receive channel (bipartite)
pub struct UnformattedBipartiteChannel {
    /// Inner send channel
    pub send_channel: UnformattedSendChannel,
    /// Inner receive channel
    pub receive_channel: UnformattedReceiveChannel,
}

/// Channel that unifies a formatted send and a formatted receive channel
pub struct BipartiteChannel<R = Format, W = Format> {
    /// Inner send channel
    pub receive_channel: ReceiveChannel<R>,
    /// Inner receive channel
    pub send_channel: SendChannel<W>,
}

impl UnformattedBipartiteChannel {
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        self.receive_channel.receive(format).await
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
        self.send_channel.send(obj, format).await
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedSendChannel, UnformattedReceiveChannel) {
        (self.send_channel, self.receive_channel)
    }
}
impl<R, W> BipartiteChannel<R, W> {
    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted.
    /// To turn `Arc<StatelessTransportState>` into the inner transport state
    /// use `Arc::try_unwrap(transport)`.
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        let mut state = Ok(());
        take_mut::take(self, |mut this| {
            if let Err(_) = this.receive_channel.encrypt(transport.clone()) {
                state = Err(transport);
                return this;
            }

            if let Err(_) = this.receive_channel.encrypt(transport.clone()) {
                state = Err(transport);
                return this;
            }
            this
        });
        state
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.receive_channel.receive().await
    }

    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.send_channel.send(obj).await
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (SendChannel<W>, ReceiveChannel<R>) {
        (self.send_channel, self.receive_channel)
    }
}
