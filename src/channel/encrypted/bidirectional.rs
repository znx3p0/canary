use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    async_snow::Snow,
    channel::raw::joint::unformatted::{RefUnformattedRawChannel, UnformattedRawChannel},
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};

use super::snowwith::SnowWith;

#[derive(From)]
pub enum RefUnformattedBidirectionalChannel<'a> {
    Raw(RefUnformattedRawChannel<'a>),
    Encrypted(RefUnformattedRawChannel<'a>, &'a Snow),
}

#[derive(From)]
pub enum UnformattedBidirectionalChannel {
    Raw(UnformattedRawChannel),
    Encrypted(UnformattedRawChannel, Snow),
}

#[derive(From)]
pub struct RefBidirectionalChannel<'a, F = Format> {
    channel: RefUnformattedBidirectionalChannel<'a>,
    format: F,
}

#[derive(From)]
pub struct BidirectionalChannel<F = Format> {
    channel: UnformattedBidirectionalChannel,
    format: F,
}

impl<'a, F> RefBidirectionalChannel<'a, F> {
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

impl<F> BidirectionalChannel<F> {
    pub(crate) fn from_tcp_raw(tcp: crate::io::TcpStream, format: F) -> Self {
        Self {
            channel: UnformattedBidirectionalChannel::Raw(UnformattedRawChannel::Unified(
                From::from(tcp),
            )),
            format,
        }
    }
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
    pub fn encrypt(mut self, snow: Snow) -> Self {
        self.channel = self.channel.encrypt(snow);
        self
    }
}

impl<'a> RefUnformattedBidirectionalChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
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

impl UnformattedBidirectionalChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.receive(&with).await
            }
        }
    }
    pub fn encrypt(self, snow: Snow) -> Self {
        match self {
            Self::Raw(raw) => Self::Encrypted(raw, snow),
            encrypted => encrypted, // silently ignore
        }
    }
}
