use crate::{
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};
use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use super::unformatted::{RefUnformattedRawUnifiedChannel, UnformattedRawUnifiedChannel};

#[derive(From)]
/// Reference unencrypted unified channel with format
pub struct RefRawUnifiedChannel<'a, F = Format> {
    /// Inner reference to unformatted channel
    pub channel: &'a mut RefUnformattedRawUnifiedChannel<'a>,
    /// Inner format
    pub format: F,
}

impl<F> RefRawUnifiedChannel<'_, F> {
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
}

#[derive(From)]
/// Unified unencrypted channel with format
pub struct RawUnifiedChannel<F = Format> {
    /// Inner channel
    pub channel: UnformattedRawUnifiedChannel,
    /// Inner format of channel
    pub format: F,
}

impl<F> RawUnifiedChannel<F> {
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        F: SendFormat,
    {
        self.channel.send(obj, &mut self.format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        F: ReadFormat,
    {
        self.channel.receive(&mut self.format).await
    }
}
