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
pub enum RefUnformattedReceiveChannel<'a> {
    Raw(RefUnformattedRawReceiveChannel<'a>),
    Encrypted(
        RefUnformattedRawReceiveChannel<'a>,
        &'a Arc<StatelessTransportState>,
        &'a mut u32,
    ),
}

#[derive(From)]
pub enum UnformattedReceiveChannel {
    Raw(UnformattedRawReceiveChannel),
    Encrypted(
        UnformattedRawReceiveChannel,
        Arc<StatelessTransportState>,
        u32,
    ),
}

#[derive(From)]
pub struct RefReceiveChannel<'a, F = Format> {
    pub channel: RefUnformattedReceiveChannel<'a>,
    pub format: F,
}

#[derive(From)]
pub struct ReceiveChannel<F = Format> {
    pub channel: UnformattedReceiveChannel,
    pub format: F,
}

impl<'a, F> RefReceiveChannel<'a, F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
}

impl<R> ReceiveChannel<R> {
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        self.channel.encrypt(transport)
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
    pub fn join<W>(self, send: SendChannel<W>) -> Channel<R, W> {
        Channel::join(send, self)
    }
}

impl<'a> RefUnformattedReceiveChannel<'a> {
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
    pub fn to_formatted<F>(self, format: F) -> ReceiveChannel<F> {
        ReceiveChannel {
            channel: self,
            format,
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

    /// Returns `true` if the unformatted receive channel is [`Encrypted`].
    ///
    /// [`Encrypted`]: UnformattedReceiveChannel::Encrypted
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(..))
    }
}
