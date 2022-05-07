use derive_more::From;
use futures::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use tungstenite::Message;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::TcpStream;
#[cfg(unix)]
use crate::io::UnixStream;
use crate::{async_snow::Snow, channel::encrypted::SnowWith};
use crate::{
    channel::bipartite::{UnformattedRawReceiveChannel, UnformattedRawSendChannel},
    io::Wss,
    serialization::formats::{ReadFormat, SendFormat},
};
use crate::{err, Result};

use super::formatted::RawUnifiedChannel;

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
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        RefUnformattedRawUnifiedChannel::from(self)
            .send(obj, f)
            .await
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        RefUnformattedRawUnifiedChannel::from(self).receive(f).await
    }

    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        match self {
            Self::Tcp(t) => {
                let (r, w) = t.into_split();
                let r = UnformattedRawReceiveChannel::from(r);
                let w = UnformattedRawSendChannel::from(w);
                (w, r)
            }
            Self::Unix(t) => {
                let (r, w) = t.into_split();
                let r = UnformattedRawReceiveChannel::from(r);
                let w = UnformattedRawSendChannel::from(w);
                (w, r)
            }
            Self::Wss(t) => {
                let (w, r) = t.split();
                let r = UnformattedRawReceiveChannel::from(r);
                let w = UnformattedRawSendChannel::from(w);
                (w, r)
            }
        }
    }
    pub fn to_formatted<F: SendFormat>(self, format: F) -> RawUnifiedChannel<F> {
        RawUnifiedChannel {
            channel: self,
            format,
        }
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
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        use crate::serialization::tx;
        match self {
            Self::Tcp(st) => tx(st, obj, f).await,
            Self::Unix(st) => tx(st, obj, f).await,
            Self::Wss(st) => {
                let buf = f.serialize(&obj).map_err(err!(@invalid_data))?;
                let len = buf.len();
                let item = Message::Binary(buf);
                st.send(item).await.map_err(err!(@other))?;
                Ok(len)
            }
        }
    }
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        use crate::serialization::{rx, wss_rx};
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tcp(st) => rx(st, f).await,
            #[cfg(unix)]
            Self::Unix(st) => rx(st, f).await,
            Self::Wss(st) => wss_rx(st, f).await,
        }
    }
}
