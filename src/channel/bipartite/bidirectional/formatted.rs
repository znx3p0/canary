use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::unformatted::{
    RefUnformattedRawBidirectionalChannel, UnformattedRawBidirectionalChannel,
};

#[derive(From)]
pub struct BidirectionalRawChannel<F = Format> {
    pub chan: UnformattedRawBidirectionalChannel,
    pub format: F,
}

impl<F> BidirectionalRawChannel<F> {
    pub fn from_unformatted_with(chan: UnformattedRawBidirectionalChannel, format: F) -> Self {
        Self { chan, format }
    }
    pub fn to_unformatted(self) -> UnformattedRawBidirectionalChannel {
        self.chan
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.chan.receive(&self.format).await
    }
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.chan.send(obj, &self.format).await
    }
}

impl BidirectionalRawChannel {
    pub fn from_unformatted(chan: UnformattedRawBidirectionalChannel) -> Self {
        Self {
            chan,
            format: Format::Bincode,
        }
    }
    // pub fn split(self) -> (SendChannel, ReceiveChannel) {
    //     let (send, receive) = self.chan.split();
    //     let send = send.to_formatted(self.format.clone());
    //     let receive = receive.to_formatted(self.format);
    //     (send, receive)
    // }
}

#[derive(From)]
pub struct RefBidirectionalRawChannel<'a, F = Format> {
    pub chan: RefUnformattedRawBidirectionalChannel<'a>,
    pub format: F,
}

impl<'a, F> RefBidirectionalRawChannel<'a, F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.chan.receive(&self.format).await
    }
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.chan.send(obj, &self.format).await
    }
}
