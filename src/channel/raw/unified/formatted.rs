use crate::{
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};
use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use super::unformatted::{RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel};

#[derive(From)]
pub struct RefRawUnifiedChannel<'a, F = Format> {
    pub channel: &'a mut RefUnformattedRawUnifiedChannel<'a>,
    pub format: F,
}

impl<F> RefRawUnifiedChannel<'_, F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
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
        self.channel.send(obj, &mut self.format).await
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
}
