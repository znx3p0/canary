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
pub enum RefUnformattedSendChannel<'a> {
    Raw(RefUnformattedRawSendChannel<'a>),
    Encrypted(
        RefUnformattedRawSendChannel<'a>,
        &'a Arc<StatelessTransportState>,
        &'a mut u32,
    ),
}

#[derive(From)]
pub enum UnformattedSendChannel {
    Raw(UnformattedRawSendChannel),
    Encrypted(UnformattedRawSendChannel, Arc<StatelessTransportState>, u32),
}

pub struct RefSendChannel<'a, F = Format> {
    channel: RefUnformattedSendChannel<'a>,
    format: F,
}

impl<'a, F> RefSendChannel<'a, F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
}

pub struct SendChannel<W = Format> {
    pub channel: UnformattedSendChannel,
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
    pub fn join<R>(self, receive: ReceiveChannel<R>) -> Channel<R, W> {
        Channel::join(self, receive)
    }
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        self.channel.encrypt(transport)
    }
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
}

impl<'a> RefUnformattedSendChannel<'a> {
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
    pub fn to_formatted<F>(self, format: F) -> SendChannel<F> {
        SendChannel {
            channel: self,
            format,
        }
    }
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
