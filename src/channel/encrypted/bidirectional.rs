use std::sync::Arc;

use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};
use snow::StatelessTransportState;

use crate::{
    async_snow::RefDividedSnow,
    channel::raw::{
        joint::unformatted::RefUnformattedRawChannel,
        unified::unformatted::UnformattedRawUnifiedChannel,
    },
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};

use super::{
    bipartite::{BipartiteChannel, UnformattedBipartiteChannel},
    receive_channel::{ReceiveChannel, UnformattedReceiveChannel},
    send_channel::{SendChannel, UnformattedSendChannel},
    snowwith::WithCipher,
    unified::{UnformattedUnifiedChannel, UnifiedChannel},
};

#[derive(From)]
/// Reference unformatted bidirectional channel, may be encrypted
pub enum RefUnformattedBidirectionalChannel<'a> {
    /// Unencrypted channel
    Raw(RefUnformattedRawChannel<'a>),
    /// Encrypted channel
    Encrypted(
        RefUnformattedRawChannel<'a>,
        &'a StatelessTransportState,
        &'a mut u32,
    ),
}

#[derive(From)]
/// Unformatted bidirectional channel which may be unified or bipartite
pub enum UnformattedBidirectionalChannel {
    /// Channel has not been split
    Unified(UnformattedUnifiedChannel),
    /// Channel has been split
    Bipartite(UnformattedBipartiteChannel),
}

#[derive(From)]
/// Reference channel with formats, similar to `&Channel`
pub struct RefChannel<'a, R = Format, W = Format> {
    /// Inner channel
    channel: RefUnformattedBidirectionalChannel<'a>,
    /// Inner receive format
    receive_format: R,
    /// Inner send format
    send_format: W,
}

#[derive(From)]
/// Channel with formats
pub enum Channel<R = Format, W = Format> {
    /// Channel has not been split
    Unified(UnifiedChannel<R, W>),
    /// Channel has been split
    Bipartite(BipartiteChannel<R, W>),
}

impl<'a, R, W> RefChannel<'a, R, W> {
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.channel.send(obj, &mut self.send_format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.channel.receive(&mut self.receive_format).await
    }
}

impl<R, W> Channel<R, W> {
    pub(crate) fn from_raw(
        raw: impl Into<UnformattedRawUnifiedChannel>,
        receive_format: R,
        send_format: W,
    ) -> Self {
        Self::Unified(UnifiedChannel {
            channel: UnformattedUnifiedChannel::Raw(raw.into()),
            receive_format,
            send_format,
        })
    }

    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted.
    /// To turn `Arc<StatelessTransportState>` into the inner transport state
    /// use `Arc::try_unwrap(transport)`.
    pub fn encrypt(
        &mut self,
        transport: StatelessTransportState,
    ) -> Result<(), Arc<StatelessTransportState>> {
        match self {
            Channel::Unified(unified) => unified.encrypt(transport).map_err(Arc::new),
            Channel::Bipartite(bipartite) => bipartite.encrypt(Arc::new(transport)),
        }
    }

    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        match self {
            Channel::Unified(chan) => chan.send(obj).await,
            Channel::Bipartite(chan) => chan.send(obj).await,
        }
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        match self {
            Channel::Unified(chan) => chan.receive().await,
            Channel::Bipartite(chan) => chan.receive().await,
        }
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (SendChannel<W>, ReceiveChannel<R>) {
        match self {
            Channel::Unified(chan) => chan.split(),
            Channel::Bipartite(chan) => chan.split(),
        }
    }
    /// Join send and receive channels into a channel
    pub fn join(send: SendChannel<W>, receive: ReceiveChannel<R>) -> Self {
        Self::Bipartite(BipartiteChannel {
            receive_channel: receive,
            send_channel: send,
        })
    }
}

impl<'a> RefUnformattedBidirectionalChannel<'a> {
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

    /// Returns `true` if the ref unformatted bidirectional channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: RefUnformattedBidirectionalChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}

impl UnformattedBidirectionalChannel {
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
            UnformattedBidirectionalChannel::Unified(chan) => chan.receive(format).await,
            UnformattedBidirectionalChannel::Bipartite(chan) => chan.receive(format).await,
        }
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedSendChannel, UnformattedReceiveChannel) {
        match self {
            UnformattedBidirectionalChannel::Unified(chan) => chan.split(),
            UnformattedBidirectionalChannel::Bipartite(chan) => chan.split(),
        }
    }
}
