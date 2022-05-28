use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::channel::raw::bipartite::bidirectional::UnformattedRawBidirectionalChannel;
use crate::channel::raw::bipartite::receive_channel::UnformattedRawReceiveChannel;
use crate::channel::raw::bipartite::send_channel::UnformattedRawSendChannel;
use crate::channel::raw::unified::unformatted::{
    RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel,
};
use crate::serialization::formats::{ReadFormat, SendFormat};
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

// #[derive(From)]
// pub struct RefRawChannel<'a, F = Format> {
//     channel: RefUnformattedRawChannel<'a>,
//     format: F,
// }

// #[derive(From)]
// pub struct RawChannel<F = Format> {
//     channel: UnformattedRawChannel,
//     format: F,
// }

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
            Self::Unified(chan) => chan.receive(format).await,
            Self::Bipartite(chan) => chan.receive(format).await,
        }
    }
    // pub fn to_formatted<F>(self, format: F) -> RefRawChannel<'a, F> {
    //     RefRawChannel {
    //         channel: self,
    //         format,
    //     }
    // }
}

impl UnformattedRawChannel {
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
            Self::Unified(chan) => chan.receive(format).await,
            Self::Bipartite(chan) => chan.receive(format).await,
        }
    }
    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        match self {
            UnformattedRawChannel::Unified(chan) => chan.split(),
            UnformattedRawChannel::Bipartite(chan) => chan.split(),
        }
    }
}
