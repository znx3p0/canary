use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::channel::raw::bipartite::bidirectional::UnformattedRawBidirectionalChannel;
use crate::channel::raw::unified::unformatted::{
    RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel,
};
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::super::bipartite::bidirectional::RefUnformattedRawBidirectionalChannel;

#[derive(From)]
pub enum RefUnformattedRawChannel<'a> {
    Unified(RefUnformattedRawUnifiedChannel<'a>),
    Bipartite(RefUnformattedRawBidirectionalChannel<'a>),
}

#[derive(From)]
pub enum UnformattedRawChannel {
    Unified(UnformattedRawUnifiedChannel),
    Bipartite(UnformattedRawBidirectionalChannel),
}
#[derive(From)]
pub struct RefRawChannel<'a, F = Format> {
    channel: RefUnformattedRawChannel<'a>,
    format: F,
}

#[derive(From)]
pub struct RawChannel<F = Format> {
    channel: UnformattedRawChannel,
    format: F,
}

impl<'a> From<&'a mut UnformattedRawChannel> for RefUnformattedRawChannel<'a> {
    fn from(chan: &'a mut UnformattedRawChannel) -> Self {
        match chan {
            UnformattedRawChannel::Unified(chan) => {
                RefUnformattedRawChannel::Unified(RefUnformattedRawUnifiedChannel::from(chan))
            }
            UnformattedRawChannel::Bipartite(chan) => RefUnformattedRawChannel::Bipartite(
                RefUnformattedRawBidirectionalChannel::from(chan),
            ),
        }
    }
}

impl<'a> RefUnformattedRawChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        match self {
            Self::Unified(chan) => chan.send(obj, f).await,
            Self::Bipartite(chan) => chan.send(obj, f).await,
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        match self {
            Self::Unified(chan) => chan.receive(f).await,
            Self::Bipartite(chan) => chan.receive(f).await,
        }
    }
    pub fn to_formatted<F>(self, format: F) -> RefRawChannel<'a, F> {
        RefRawChannel {
            channel: self,
            format,
        }
    }
}

impl UnformattedRawChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        match self {
            Self::Unified(chan) => chan.send(obj, f).await,
            Self::Bipartite(chan) => chan.send(obj, f).await,
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        match self {
            Self::Unified(chan) => chan.receive(f).await,
            Self::Bipartite(chan) => chan.receive(f).await,
        }
    }
}
