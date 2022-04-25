use crate::{
    channel::Wss,
    err,
    serialization::formats::{Format, SendFormat},
    Result,
};
use derive_more::From;
use futures::{stream::SplitSink, Sink, SinkExt};
use serde::Serialize;
use tungstenite::Message;

use crate::async_snow::Snow;
#[cfg(not(target_arch = "wasm32"))]
use crate::io::{TcpStream, UnixStream, WriteHalf};

///
#[derive(From)]
pub enum RefUnformattedSendChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(&'a mut WriteHalf<TcpStream>),
    #[cfg(unix)]
    /// unix backend
    Unix(&'a mut WriteHalf<UnixStream>),
    /// wss backend
    WSS(&'a mut SplitSink<Wss, Message>),
    /// encrypted backend
    Encrypted(&'a mut Box<(Snow, UnformattedSendChannel)>),
}

impl<'a> From<&'a mut UnformattedSendChannel> for RefUnformattedSendChannel<'a> {
    #[inline]
    fn from(chan: &'a mut UnformattedSendChannel) -> Self {
        match chan {
            UnformattedSendChannel::Tcp(ref mut chan) => chan.into(),
            UnformattedSendChannel::Unix(ref mut chan) => chan.into(),
            UnformattedSendChannel::WSS(ref mut chan) => chan.into(),
            UnformattedSendChannel::Encrypted(ref mut chan) => chan.into(),
        }
    }
}

impl<'a> RefUnformattedSendChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        use crate::serialization::tx;
        match self {
            RefUnformattedSendChannel::Tcp(st) => tx(st, obj, f).await,
            RefUnformattedSendChannel::Unix(st) => tx(st, obj, f).await,
            RefUnformattedSendChannel::Encrypted(st) => {
                let snow = &st.0;
                let chan = &mut st.1;
                let buf = f.serialize(&obj).map_err(err!(@invalid_data))?;
                let obj = snow.encrypt_packets(&buf)?;
                // chan.send(obj, f).await
                todo!()
            }
            RefUnformattedSendChannel::WSS(st) => {
                let buf = f.serialize(&obj).map_err(err!(@invalid_data))?;
                let len = buf.len();
                let item = Message::Binary(buf);
                st.send(item).await.map_err(err!(@other));
                Ok(len)
            }
        }
    }
}

#[derive(From)]
pub enum UnformattedSendChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(WriteHalf<TcpStream>),
    #[cfg(unix)]
    /// unix backend
    Unix(WriteHalf<UnixStream>),
    /// wss backend
    WSS(SplitSink<Wss, Message>),
    /// encrypted backend
    Encrypted(Box<(Snow, UnformattedSendChannel)>),
}

impl UnformattedSendChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        RefUnformattedSendChannel::from(self).send(obj, f).await
    }
    pub fn to_formatted<F: SendFormat>(self, format: F) -> SendChannel<F> {
        SendChannel {
            channel: self,
            format,
        }
    }
}

#[derive(From)]
pub struct SendChannel<F: SendFormat = Format> {
    channel: UnformattedSendChannel,
    format: F,
}

impl SendChannel {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        self.channel.send(obj, &self.format).await
    }
}
