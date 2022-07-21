use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use snow::StatelessTransportState;

use crate::{
    async_snow::RefDividedSnow,
    channel::{
        channels::{ReceiveChannel, SendChannel},
        raw::unified::unformatted::UnformattedRawUnifiedChannel,
    },
    serialization::formats::{Format, ReadFormat, SendFormat},
    Result,
};

use super::{
    receive_channel::UnformattedReceiveChannel, send_channel::UnformattedSendChannel,
    snowwith::WithCipher,
};

/// Unformmated channel that has not been split.
/// Can be encrypted or raw.
pub enum UnformattedUnifiedChannel {
    /// Unencrypted channel
    Raw(UnformattedRawUnifiedChannel),
    /// Encrypted channel with transport state and nonces
    Encrypted {
        /// Inner channel
        chan: UnformattedRawUnifiedChannel,
        /// Inner transport state
        transport: StatelessTransportState,
        /// Inner send nonce
        send_nonce: u32,
        /// Inner receive nonce
        receive_nonce: u32,
    },
}

/// Channel that has not been split with read and write formats
pub struct UnifiedChannel<R = Format, W = Format> {
    /// Inner channel
    pub channel: UnformattedUnifiedChannel,
    /// Inner receive format
    pub receive_format: R,
    /// Inner send format
    pub send_format: W,
}

impl<R, W> UnifiedChannel<R, W> {
    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted
    pub fn encrypt(
        &mut self,
        transport: StatelessTransportState,
    ) -> Result<(), StatelessTransportState> {
        self.channel.encrypt(transport)
    }
    /// Send an object through the channel
    /// ```no_run
    /// chan.send("Hello world!").await?;
    /// ```
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize>
    where
        W: SendFormat,
    {
        self.channel.send(obj, &mut self.send_format).await
    }
    /// Receive an object sent through the channel
    /// ```no_run
    /// let string: String = chan.receive().await?;
    /// ```
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T>
    where
        R: ReadFormat,
    {
        self.channel.receive(&mut self.receive_format).await
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (SendChannel<W>, ReceiveChannel<R>) {
        let (send, receive) = self.channel.split();
        let send = send.to_formatted(self.send_format);
        let receive = receive.to_formatted(self.receive_format);
        (send, receive)
    }
}

impl UnformattedUnifiedChannel {
    /// Try to encrypt channel using the provided transport.
    /// Will return an error if channel is already encrypted.
    pub fn encrypt(
        &mut self,
        transport: StatelessTransportState,
    ) -> Result<(), StatelessTransportState> {
        let mut state = Ok(());
        take_mut::take(self, |this| match this {
            UnformattedUnifiedChannel::Raw(chan) => UnformattedUnifiedChannel::Encrypted {
                chan,
                transport,
                send_nonce: 0,
                receive_nonce: 0,
            },
            UnformattedUnifiedChannel::Encrypted { .. } => {
                state = Err(transport);
                this
            }
        });
        state
    }
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted {
                chan,
                transport,
                send_nonce,
                ..
            } => {
                let ref mut snow = RefDividedSnow {
                    transport,
                    nonce: send_nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.send(obj, &mut with).await
            }
        }
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted {
                chan,
                transport,
                receive_nonce,
                ..
            } => {
                let ref mut snow = RefDividedSnow {
                    transport,
                    nonce: receive_nonce,
                };
                let mut with = WithCipher { snow, format };
                chan.receive(&mut with).await
            }
        }
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedSendChannel, UnformattedReceiveChannel) {
        match self {
            Self::Raw(chan) => {
                let (send, receive) = chan.split();
                let send = UnformattedSendChannel::Raw(send);
                let receive = UnformattedReceiveChannel::Raw(receive);
                (send, receive)
            }
            Self::Encrypted {
                chan,
                transport,
                send_nonce,
                receive_nonce,
            } => {
                let (send, receive) = chan.split();

                let transport = Arc::new(transport);
                let send = UnformattedSendChannel::Encrypted(send, transport.clone(), send_nonce);
                let receive =
                    UnformattedReceiveChannel::Encrypted(receive, transport, receive_nonce);
                (send, receive)
            }
        }
    }
}
