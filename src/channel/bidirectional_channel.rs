use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::Wss;
use super::receive_channel::UnformattedReceiveChannel;
use super::send_channel::UnformattedSendChannel;
use crate::serialization::formats::{Format, ReadFormat, SendFormat};
use crate::Result;
use futures::StreamExt;

#[derive(From)]
pub struct UnformattedBidirectionalChannel {
    pub send_chan: UnformattedSendChannel,
    pub receive_chan: UnformattedReceiveChannel,
}

impl UnformattedBidirectionalChannel {
    #[cfg(unix)]
    pub fn from_unix(stream: crate::io::UnixStream) -> Self {
        let (read, write) = crate::io::split(stream);   
        let send_chan = UnformattedSendChannel::from(write);
        let receive_chan = UnformattedReceiveChannel::from(read);
        UnformattedBidirectionalChannel {
            send_chan, receive_chan
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_tcp(stream: crate::io::TcpStream) -> Self {
        let (read, write) = crate::io::split(stream);   
        let send_chan = UnformattedSendChannel::from(write);
        let receive_chan = UnformattedReceiveChannel::from(read);
        UnformattedBidirectionalChannel {
            send_chan, receive_chan
        }
    }
    pub fn from_wss(stream: Wss) -> Self {
        let (write, read) = stream.split();
        let send_chan = UnformattedSendChannel::from(Box::new(write));
        let receive_chan = UnformattedReceiveChannel::from(read);
        UnformattedBidirectionalChannel {
            send_chan, receive_chan
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        self.receive_chan.receive(f).await
    }
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        self.send_chan.send(obj, f).await
    }
    pub async fn encrypt(&mut self) -> Result {
        // self.receive_chan;
        Ok(())
    }
}

#[derive(From)]
pub struct BidirectionalChannel<F: ReadFormat + SendFormat = Format> {
    pub chan: UnformattedBidirectionalChannel,
    pub format: F,
}

impl<F: ReadFormat + SendFormat> BidirectionalChannel<F> {
    pub fn from_unformatted_with(chan: UnformattedBidirectionalChannel, format: F) -> Self {
        Self { chan, format }
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.chan.receive(&self.format).await
    }
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        self.chan.send(obj, &self.format).await
    }
}

impl BidirectionalChannel {
    pub fn from_unformatted(chan: UnformattedBidirectionalChannel) -> Self {
        Self { chan, format: Format::Bincode }
    }
}
