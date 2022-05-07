use crate::async_snow::Snow;
use crate::channel::encrypted::SnowWith;
use crate::serialization::formats::Format;
use crate::serialization::formats::ReadFormat;
use crate::serialization::formats::SendFormat;
use crate::Result;
use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::RefUnformattedRawUnifiedChannel;
use super::UnformattedRawUnifiedChannel;

// encrypted
#[derive(From)]
pub enum RefUnformattedUnifiedChannel<'a> {
    Raw(RefUnformattedRawUnifiedChannel<'a>),
    Encrypted(RefUnformattedRawUnifiedChannel<'a>, &'a Snow),
}

// encrypted
#[derive(From)]
pub enum UnformattedUnifiedChannel {
    Raw(UnformattedRawUnifiedChannel),
    Encrypted(UnformattedRawUnifiedChannel, Snow),
}

impl From<&mut UnformattedUnifiedChannel> for RefUnformattedUnifiedChannel<'_> {
    fn from(chan: &mut UnformattedUnifiedChannel) -> Self {
        match chan {
            UnformattedUnifiedChannel::Raw(c) => RefUnformattedUnifiedChannel::Raw(c.into()),
            UnformattedUnifiedChannel::Encrypted(c, s) => {
                RefUnformattedUnifiedChannel::Encrypted(c.into(), s)
            }
        }
    }
}

impl UnformattedUnifiedChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            UnformattedUnifiedChannel::Raw(chan) => chan.send(obj, format).await,
            UnformattedUnifiedChannel::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        match self {
            UnformattedUnifiedChannel::Raw(chan) => chan.receive(format).await,
            UnformattedUnifiedChannel::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.receive(&with).await
            }
        }
    }
}

#[derive(From)]
pub struct UnifiedChannel<F = Format> {
    pub channel: UnformattedUnifiedChannel,
    pub format: F,
}

impl<F> UnifiedChannel<F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &self.format).await
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&self.format).await
    }
}
