use derive_more::From;
use serde::de::DeserializeOwned;

use crate::{
    async_snow::Snow,
    channel::raw::{
        bipartite::receive_channel::RefUnformattedRawReceiveChannel,
        joint::unformatted::UnformattedRawChannel,
    },
    serialization::formats::{Format, ReadFormat},
    Result,
};

use super::snowwith::SnowWith;

#[derive(From)]
pub enum RefUnformattedReceiveChannel<'a> {
    Raw(RefUnformattedRawReceiveChannel<'a>),
    Encrypted(RefUnformattedRawReceiveChannel<'a>, &'a Snow),
}

#[derive(From)]
pub enum UnformattedReceiveChannel {
    Raw(UnformattedRawChannel),
    Encrypted(UnformattedRawChannel, Snow),
}

#[derive(From)]
pub struct RefReceiveChannel<'a, F = Format> {
    channel: RefUnformattedReceiveChannel<'a>,
    format: F,
}

#[derive(From)]
pub struct ReceiveChannel<F = Format> {
    channel: UnformattedReceiveChannel,
    format: F,
}

impl<'a, F> RefReceiveChannel<'a, F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&self.format).await
    }
}

impl<F> ReceiveChannel<F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&self.format).await
    }
}

impl<'a> RefUnformattedReceiveChannel<'a> {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.receive(&with).await
            }
        }
    }
}

impl UnformattedReceiveChannel {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.receive(&with).await
            }
        }
    }
}
