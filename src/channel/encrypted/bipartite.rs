use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use snow::StatelessTransportState;

use crate::channel::channels::{ReceiveChannel, SendChannel};
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;

use super::{receive_channel::UnformattedReceiveChannel, send_channel::UnformattedSendChannel};

pub struct UnformattedBipartiteChannel {
    pub send_channel: UnformattedSendChannel,
    pub receive_channel: UnformattedReceiveChannel,
}

pub struct BipartiteChannel<R = Format, W = Format> {
    pub receive_channel: ReceiveChannel<R>,
    pub send_channel: SendChannel<W>,
}

impl UnformattedBipartiteChannel {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        self.receive_channel.receive(format).await
    }

    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        self.send_channel.send(obj, format).await
    }

    pub fn split(self) -> (UnformattedSendChannel, UnformattedReceiveChannel) {
        (self.send_channel, self.receive_channel)
    }
}
impl<R, W> BipartiteChannel<R, W> {
    pub fn encrypt(
        &mut self,
        transport: Arc<StatelessTransportState>,
    ) -> Result<(), Arc<StatelessTransportState>> {
        let mut state = Ok(());
        take_mut::take(self, |mut this| {
            if let Err(_) = this.receive_channel.encrypt(transport.clone()) {
                state = Err(transport);
                return this;
            }

            if let Err(_) = this.send_channel.encrypt(transport.clone()) {
                state = Err(transport);
                return this;
            }

            this
        });
        state
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.receive_channel.receive().await
    }

    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.send_channel.send(obj).await
    }

    pub fn split(self) -> (SendChannel<W>, ReceiveChannel<R>) {
        (self.send_channel, self.receive_channel)
    }
}
