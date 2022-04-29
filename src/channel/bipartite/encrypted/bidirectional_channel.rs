use crate::{
    async_snow::Snow,
    channel::{
        bipartite::encrypted::{UnformattedReceiveChannel, UnformattedSendChannel},
        encrypted::SnowWith,
    },
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
