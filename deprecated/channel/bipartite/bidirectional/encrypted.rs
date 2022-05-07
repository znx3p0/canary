use crate::{
    async_snow::Snow,
    channel::{
        bipartite::encrypted::{UnformattedReceiveChannel, UnformattedSendChannel},
        encrypted::SnowWith,
    },
    serialization::formats::Format,
    Result,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    channel::bipartite::{UnformattedRawReceiveChannel, UnformattedRawSendChannel},
    serialization::formats::{ReadFormat, SendFormat},
};

pub struct UnformattedChannel {
    send_channel: UnformattedSendChannel,
    receive_channel: UnformattedReceiveChannel,
}

impl UnformattedChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        self.send_channel.send(obj, format).await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        self.receive_channel.receive(format).await
    }
}

pub struct BidirectionalChannel<F = Format> {
    channel: UnformattedChannel,
    format: F,
}

impl<F> BidirectionalChannel<F> {
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
