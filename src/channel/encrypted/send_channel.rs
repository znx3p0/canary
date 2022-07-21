use std::sync::Arc;

use derive_more::From;
use serde::Serialize;
use snow::StatelessTransportState;

use crate::{
    async_snow::RefDividedSnow,
    channel::{
        channels::ReceiveChannel,
        raw::bipartite::send_channel::{RefUnformattedRawSendChannel, UnformattedRawSendChannel},
    },
    serialization::formats::{Format, SendFormat},
    Channel, Result,
};

use super::snowwith::WithCipher;

#[derive(From)]
/// Reference unformatted send channel that may be encrypted
pub enum RefUnformattedSendChannel<'a> {
    /// Unencrypted channel
    Raw(RefUnformattedRawSendChannel<'a>),
    /// Encrypted channel
    Encrypted(
        RefUnformattedRawSendChannel<'a>,
        &'a Arc<StatelessTransportState>,
        &'a mut u32,
    ),
}

#[derive(From)]
/// Unformatted send channel that may be encrypted
pub enum UnformattedSendChannel {
    /// Unencrypted channel
    Raw(UnformattedRawSendChannel),
    /// Encrypted channel
    Encrypted(UnformattedRawSendChannel, Arc<StatelessTransportState>, u32),
}

/// Reference send channel with format
pub struct RefSendChannel<'a, F = Format> {
    /// Inner channel
    channel: RefUnformattedSendChannel<'a>,
    /// Inner format
    format: F,
}

impl<'a, F> RefSendChannel<'a, F> {
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
}

/// Channel that is able to send objects to the stream
pub struct SendChannel<W = Format> {
    /// Inner channel
    pub channel: UnformattedSendChannel,
    /// Inner format used to serialize objects
    pub format: W,
}

impl<W> SendChannel<W> {
    /// Returns `true` if the unformatted send channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: UnformattedSendChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self.channel, UnformattedSendChannel::Encrypted(..))
    }
    /// Join `Self` and a `SendChannel` into a bidirectional channel
    pub fn join<R>(self, receive: ReceiveChannel<R>) -> Channel<R, W> {
        Channel::join(self, receive)
    }
    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted.
    /// To turn `Arc<StatelessTransportState>` into the inner transport state
    /// use `Arc::try_unwrap(transport)`.
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        self.channel.encrypt(transport)
    }
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
}

impl<'a> RefUnformattedSendChannel<'a> {
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
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow, nonce) => {
                let ref mut snow = RefDividedSnow {
                    transport: snow,
                    nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.send(obj, &mut with).await
            }
        }
    }

    /// Returns `true` if the ref unformatted send channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: RefUnformattedSendChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}

impl UnformattedSendChannel {
    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted.
    /// To turn `Arc<StatelessTransportState>` into the inner transport state
    /// use `Arc::try_unwrap(transport)`.
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        let mut state = Ok(());
        take_mut::take(self, |this| match this {
            Self::Raw(chan) => Self::Encrypted(chan, transport, 0),
            Self::Encrypted(..) => {
                state = Err(transport);
                this
            }
        });
        state
    }
    /// Format the channel
    /// ```no_run
    /// let formatted = unformatted.to_formatted(Format::Bincode);
    /// ```
    pub fn to_formatted<F>(self, format: F) -> SendChannel<F> {
        SendChannel {
            channel: self,
            format,
        }
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
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow, nonce) => {
                let ref mut snow = RefDividedSnow {
                    transport: snow,
                    nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.send(obj, &mut with).await
            }
        }
    }

    /// Returns `true` if the unformatted send channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: UnformattedSendChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}
