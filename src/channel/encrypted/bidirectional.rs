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
pub enum RefUnformattedBidirectionalChannel<'a> {
    Raw(RefUnformattedRawChannel<'a>),
    Encrypted(
        RefUnformattedRawChannel<'a>,
        &'a StatelessTransportState,
        &'a mut u32,
    ),
}

#[derive(From)]
pub enum UnformattedBidirectionalChannel {
    Unified(UnformattedUnifiedChannel),
    Bipartite(UnformattedBipartiteChannel),
}

#[derive(From)]
pub struct RefChannel<'a, R = Format, W = Format> {
    channel: RefUnformattedBidirectionalChannel<'a>,
    receive_format: R,
    send_format: W,
}

#[derive(From)]
pub enum Channel<R = Format, W = Format> {
    Unified(UnifiedChannel<R, W>),
    Bipartite(BipartiteChannel<R, W>),
}

impl<'a, R, W> RefChannel<'a, R, W> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.channel.send(obj, &mut self.send_format).await
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.channel.receive(&mut self.receive_format).await
    }
}

impl<R, W> Channel<R, W> {
    pub(crate) fn from_tcp_raw(
        tcp: crate::io::TcpStream,
        receive_format: R,
        send_format: W,
    ) -> Self {
        Self::Unified(UnifiedChannel {
            channel: UnformattedUnifiedChannel::Raw(From::from(tcp)),
            receive_format,
            send_format,
        })
    }
    pub(crate) fn encrypt(
        &mut self,
        transport: StatelessTransportState,
    ) -> Result<(), StatelessTransportState> {
        match self {
            Channel::Unified(unified) => unified.encrypt(transport),
            Channel::Bipartite(bipartite) => todo!(),
        }
    }

    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        match self {
            Channel::Unified(chan) => chan.send(obj).await,
            Channel::Bipartite(chan) => chan.send(obj).await,
        }
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        match self {
            Channel::Unified(chan) => chan.receive().await,
            Channel::Bipartite(chan) => chan.receive().await,
        }
    }
    pub fn split(self) -> (SendChannel<W>, ReceiveChannel<R>) {
        match self {
            Channel::Unified(chan) => chan.split(),
            Channel::Bipartite(chan) => chan.split(),
        }
    }
    pub fn join(send: SendChannel<W>, receive: ReceiveChannel<R>) -> Self {
        Self::Bipartite(BipartiteChannel {
            receive_channel: receive,
            send_channel: send,
        })
    }
}

impl<'a> RefUnformattedBidirectionalChannel<'a> {
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
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        match self {
            UnformattedBidirectionalChannel::Unified(chan) => chan.receive(format).await,
            UnformattedBidirectionalChannel::Bipartite(chan) => chan.receive(format).await,
        }
    }
    pub fn split(self) -> (UnformattedSendChannel, UnformattedReceiveChannel) {
        match self {
            UnformattedBidirectionalChannel::Unified(chan) => chan.split(),
            UnformattedBidirectionalChannel::Bipartite(chan) => chan.split(),
        }
    }
    // pub(crate) fn encrypt(self, snow: Snow) -> Self {
    //     match self {
    //         Self::Raw(raw) => Self::Encrypted(raw, snow),
    //         encrypted => encrypted, // silently ignore
    //     }
    // }

    // /// Returns `true` if the unformatted bidirectional channel is [`Encrypted`].
    // ///
    // /// [`Encrypted`]: UnformattedBidirectionalChannel::Encrypted
    // #[must_use]
    // pub fn is_encrypted(&self) -> bool {
    //     matches!(self, Self::Encrypted(..))
    // }
}
