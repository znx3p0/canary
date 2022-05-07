use derive_more::From;
use serde::Serialize;

use crate::{
    async_snow::Snow,
    channel::raw::bipartite::send_channel::{
        RefUnformattedRawSendChannel, UnformattedRawSendChannel,
    },
    serialization::formats::{Format, SendFormat},
    Result,
};

use super::snowwith::SnowWith;

#[derive(From)]
pub enum RefUnformattedSendChannel<'a> {
    Raw(RefUnformattedRawSendChannel<'a>),
    Encrypted(RefUnformattedRawSendChannel<'a>, &'a Snow),
}

#[derive(From)]
pub enum UnformattedSendChannel {
    Raw(UnformattedRawSendChannel),
    Encrypted(UnformattedRawSendChannel, Snow),
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
        self.channel.send(obj, &self.format).await
    }
}

pub struct SendChannel<F = Format> {
    channel: UnformattedSendChannel,
    format: F,
}

impl<F> SendChannel<F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &self.format).await
    }
}

impl<'a> RefUnformattedSendChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
}

impl UnformattedSendChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
}
