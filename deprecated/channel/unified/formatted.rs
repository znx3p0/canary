use crate::{
    async_snow::Snow,
    channel::bipartite::{
        RefUnformattedRawReceiveChannel, RefUnformattedRawSendChannel,
        UnformattedRawReceiveChannel, UnformattedRawSendChannel,
    },
    err,
    io::Wss,
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};
use derive_more::From;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{TcpStream, UnixStream, WriteHalf};

use super::{
    unformatted::{RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel},
    UnformattedUnifiedChannel,
};

#[derive(From)]
pub struct RefRawUnifiedChannel<'a, F = Format> {
    pub channel: RefUnformattedRawUnifiedChannel<'a>,
    pub format: F,
}

impl<F> RefRawUnifiedChannel<'_, F> {
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

#[derive(From)]
pub struct RawUnifiedChannel<F = Format> {
    pub channel: UnformattedRawUnifiedChannel,
    pub format: F,
}

impl<F> RawUnifiedChannel<F> {
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
