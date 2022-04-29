use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::channel::bipartite::{
    RefUnformattedRawReceiveChannel, RefUnformattedRawSendChannel, UnformattedRawReceiveChannel,
    UnformattedRawSendChannel,
};

use crate::io::Wss;
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;
use futures::StreamExt;

use super::formatted::{BidirectionalRawChannel, RefBidirectionalRawChannel};

#[derive(From)]
pub struct UnformattedRawBidirectionalChannel {
    pub send_chan: UnformattedRawSendChannel,
    pub receive_chan: UnformattedRawReceiveChannel,
}

impl UnformattedRawBidirectionalChannel {
    // pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
    //     todo!()
    // }
    pub fn to_formatted<F: ReadFormat + SendFormat>(self, format: F) -> BidirectionalRawChannel<F> {
        todo!()
    }
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        self.receive_chan.receive(format).await
    }
}

pub struct RefUnformattedRawBidirectionalChannel<'a> {
    pub send_chan: RefUnformattedRawSendChannel<'a>,
    pub receive_chan: RefUnformattedRawReceiveChannel<'a>,
}

impl<'a> RefUnformattedRawBidirectionalChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        self.receive_chan.receive(format).await
    }
}
