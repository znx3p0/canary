use std::sync::Arc;

use derive_more::From;
use serde::de::DeserializeOwned;
use snow::StatelessTransportState;

use crate::{
    async_snow::RefDividedSnow,
    channel::{
        channels::SendChannel,
        raw::bipartite::receive_channel::{
            RefUnformattedRawReceiveChannel, UnformattedRawReceiveChannel,
        },
    },
    serialization::formats::{Format, ReadFormat},
    Channel, Result,
};

use super::snowwith::WithCipher;

#[derive(From)]
/// Reference unformatted receive channel, may be encrypted
pub enum RefUnformattedReceiveChannel<'a> {
    /// Unencrypted channel
    Raw(RefUnformattedRawReceiveChannel<'a>),
    /// Encrypted channel
    Encrypted(
        RefUnformattedRawReceiveChannel<'a>,
        &'a Arc<StatelessTransportState>,
        &'a mut u32,
    ),
}

#[derive(From)]
/// Unformatted receive channel, may be encrypted
pub enum UnformattedReceiveChannel {
    /// Unencrypted channel
    Raw(UnformattedRawReceiveChannel),
    /// Encrypted channel
    Encrypted(
        UnformattedRawReceiveChannel,
        Arc<StatelessTransportState>,
        u32,
    ),
}

#[derive(From)]
/// Reference receive channel with format
pub struct RefReceiveChannel<'a, F = Format> {
    /// Inner channel
    pub channel: RefUnformattedReceiveChannel<'a>,
    /// Inner format
    pub format: F,
}

#[derive(From)]
/// Receive channel with format
pub struct ReceiveChannel<F = Format> {
    /// Inner channel
    pub channel: UnformattedReceiveChannel,
    /// Inner format
    pub format: F,
}

impl<'a, F> RefReceiveChannel<'a, F> {
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
}

impl<R> ReceiveChannel<R> {
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
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
    /// Join `Self` and a `SendChannel` into a bidirectional channel
    pub fn join<W>(self, send: SendChannel<W>) -> Channel<R, W> {
        Channel::join(send, self)
    }
}

impl<'a> RefUnformattedReceiveChannel<'a> {
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow, nonce) => {
                let ref mut snow = RefDividedSnow {
                    transport: snow,
                    nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.receive(&mut with).await
            }
        }
    }

    /// Returns `true` if the ref unformatted receive channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: RefUnformattedReceiveChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}

impl UnformattedReceiveChannel {
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
    #[inline]
    /// Format the channel
    /// ```no_run
    /// let formatted = unformatted.to_formatted(Format::Bincode);
    /// ```
    pub fn to_formatted<F>(self, format: F) -> ReceiveChannel<F> {
        ReceiveChannel {
            channel: self,
            format,
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
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow, nonce) => {
                let ref mut snow = RefDividedSnow {
                    transport: snow,
                    nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.receive(&mut with).await
            }
        }
    }

    /// Returns `true` if the unformatted receive channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: UnformattedReceiveChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}
