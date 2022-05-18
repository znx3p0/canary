use derive_more::From;
use futures::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use tungstenite::Message;

use crate::channel::raw::bipartite::receive_channel::UnformattedRawReceiveChannel;
use crate::channel::raw::bipartite::send_channel::UnformattedRawSendChannel;
#[cfg(not(target_arch = "wasm32"))]
use crate::io::TcpStream;
#[cfg(unix)]
use crate::io::UnixStream;
use crate::{err, Result};
use crate::{
    io::Wss,
    serialization::formats::{ReadFormat, SendFormat},
};

use super::formatted::RefRawUnifiedChannel;

#[derive(From)]
pub enum RefUnformattedRawUnifiedChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(&'a mut TcpStream),
    #[cfg(unix)]
    /// unix backend
    Unix(&'a mut UnixStream),
    /// wss backend
    Wss(&'a mut Wss),
}

#[derive(From)]
pub enum UnformattedRawUnifiedChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(TcpStream),
    #[cfg(unix)]
    /// unix backend
    Unix(UnixStream),
    /// wss backend
    Wss(Box<Wss>), // boxed since it's heavy and would weigh down other variants
}

impl UnformattedRawUnifiedChannel {
    pub fn new(from: impl Into<Self>) -> Self {
        from.into()
    }
    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        match self {
            UnformattedRawUnifiedChannel::Tcp(stream) => {
                let (read, write) = stream.into_split();
                (From::from(write), From::from(read))
            }
            UnformattedRawUnifiedChannel::Unix(stream) => {
                let (read, write) = stream.into_split();
                (From::from(write), From::from(read))
            }
            UnformattedRawUnifiedChannel::Wss(stream) => {
                let (write, read) = stream.split();
                (From::from(write), From::from(read))
            }
        }
    }
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        RefUnformattedRawUnifiedChannel::from(self)
            .send(obj, format)
            .await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        RefUnformattedRawUnifiedChannel::from(self)
            .receive(format)
            .await
    }
}

impl<'a> From<&'a mut UnformattedRawUnifiedChannel> for RefUnformattedRawUnifiedChannel<'a> {
    #[inline]
    fn from(chan: &'a mut UnformattedRawUnifiedChannel) -> Self {
        match chan {
            UnformattedRawUnifiedChannel::Tcp(ref mut chan) => chan.into(),
            UnformattedRawUnifiedChannel::Unix(ref mut chan) => chan.into(),
            UnformattedRawUnifiedChannel::Wss(ref mut chan) => {
                RefUnformattedRawUnifiedChannel::Wss(chan)
            }
        }
    }
}

impl<'a> RefUnformattedRawUnifiedChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        use crate::serialization::tx;
        match self {
            Self::Tcp(st) => tx(st, obj, format).await,
            Self::Unix(st) => tx(st, obj, format).await,
            Self::Wss(st) => {
                let buf = format.serialize(&obj).map_err(err!(@invalid_data))?;
                let len = buf.len();
                let item = Message::Binary(buf);
                st.send(item).await.map_err(err!(@other))?;
                Ok(len)
            }
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(
        &mut self,
        format: &mut F,
    ) -> Result<T> {
        use crate::serialization::{rx, wss_rx};
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tcp(st) => rx(st, format).await,
            #[cfg(unix)]
            Self::Unix(st) => rx(st, format).await,
            Self::Wss(st) => wss_rx(st, format).await,
        }
    }
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawUnifiedChannel<'a, F> {
        RefRawUnifiedChannel {
            channel: self,
            format,
        }
    }
}
