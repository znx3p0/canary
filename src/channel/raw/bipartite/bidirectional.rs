use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::receive_channel::{RefUnformattedRawReceiveChannel, UnformattedRawReceiveChannel};
use super::send_channel::{RefUnformattedRawSendChannel, UnformattedRawSendChannel};

#[derive(From)]
pub struct UnformattedRawBidirectionalChannel {
    pub send_chan: UnformattedRawSendChannel,
    pub receive_chan: UnformattedRawReceiveChannel,
}

impl UnformattedRawBidirectionalChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        self.receive_chan.receive(format).await
    }
}

#[derive(From)]
pub struct RefUnformattedRawBidirectionalChannel<'a> {
    pub send_chan: RefUnformattedRawSendChannel<'a>,
    pub receive_chan: RefUnformattedRawReceiveChannel<'a>,
}

impl<'a> From<&'a mut UnformattedRawBidirectionalChannel>
    for RefUnformattedRawBidirectionalChannel<'a>
{
    fn from(chan: &'a mut UnformattedRawBidirectionalChannel) -> Self {
        let send_chan = RefUnformattedRawSendChannel::from(&mut chan.send_chan);
        let receive_chan = RefUnformattedRawReceiveChannel::from(&mut chan.receive_chan);
        Self {
            send_chan,
            receive_chan,
        }
    }
}

impl<'a> RefUnformattedRawBidirectionalChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        self.send_chan.send(obj, format).await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        self.receive_chan.receive(format).await
    }
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawBidirectionalChannel<'a, F> {
        RefRawBidirectionalChannel { chan: self, format }
    }
}

#[derive(From)]
pub struct RawBidirectionalChannel<F = Format> {
    pub chan: UnformattedRawBidirectionalChannel,
    pub format: F,
}

impl<F> RawBidirectionalChannel<F> {
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

impl RawBidirectionalChannel {
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
pub struct RefRawBidirectionalChannel<'a, F = Format> {
    pub chan: &'a mut RefUnformattedRawBidirectionalChannel<'a>,
    pub format: F,
}

impl<'a, F> RefRawBidirectionalChannel<'a, F> {
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
