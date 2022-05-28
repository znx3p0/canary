use crate::io::Message;
use crate::{
    err,
    io::Wss,
    serialization::formats::{Format, SendFormat},
    Result,
};
use derive_more::From;
use futures::{stream::SplitSink, SinkExt};
use serde::Serialize;

///
#[derive(From)]
pub enum RefUnformattedRawSendChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(&'a mut tokio::net::tcp::OwnedWriteHalf),
    #[cfg(unix)]
    /// unix backend
    Unix(&'a mut tokio::net::unix::OwnedWriteHalf),
    /// wss backend
    WSS(&'a mut SplitSink<Box<Wss>, Message>),
    #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
    /// quic backend
    Quic(&'a mut quinn::SendStream),
}

#[derive(From)]
pub enum UnformattedRawSendChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    #[cfg(unix)]
    /// unix backend
    Unix(tokio::net::unix::OwnedWriteHalf),
    /// wss backend
    WSS(SplitSink<Box<Wss>, Message>),
    #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
    /// quic backend
    Quic(quinn::SendStream),
}

#[derive(From)]
pub struct RefRawSendChannel<'a, F = Format> {
    channel: &'a mut RefUnformattedRawSendChannel<'a>,
    format: F,
}

#[derive(From)]
pub struct RawSendChannel<F = Format> {
    pub(crate) channel: UnformattedRawSendChannel,
    pub(crate) format: F,
}

impl<'a> From<&'a mut UnformattedRawSendChannel> for RefUnformattedRawSendChannel<'a> {
    #[inline]
    fn from(chan: &'a mut UnformattedRawSendChannel) -> Self {
        match chan {
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedRawSendChannel::Tcp(ref mut chan) => chan.into(),
            #[cfg(unix)]
            UnformattedRawSendChannel::Unix(ref mut chan) => chan.into(),
            UnformattedRawSendChannel::WSS(ref mut chan) => chan.into(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            UnformattedRawSendChannel::Quic(ref mut chan) => chan.into(),
        }
    }
}

impl<'a> RefUnformattedRawSendChannel<'a> {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &mut F) -> Result<usize> {
        #[allow(unused)]
        use crate::serialization::tx;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            RefUnformattedRawSendChannel::Tcp(st) => tx(st, obj, f).await,
            #[cfg(unix)]
            RefUnformattedRawSendChannel::Unix(st) => tx(st, obj, f).await,
            RefUnformattedRawSendChannel::WSS(st) => {
                let buf = f.serialize(&obj).map_err(err!(@invalid_data))?;
                let len = buf.len();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let item = Message::Binary(buf);
                    st.send(item).await.map_err(err!(@other))?;
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let item = Message::Bytes(buf);
                    st.send(item).await.map_err(|e| err!(e.to_string()))?;
                }

                Ok(len)
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            RefUnformattedRawSendChannel::Quic(st) => tx(st, obj, f).await,
        }
    }
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawSendChannel<'a, F> {
        RefRawSendChannel {
            channel: self,
            format,
        }
    }
}

impl UnformattedRawSendChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &mut F) -> Result<usize> {
        RefUnformattedRawSendChannel::from(self).send(obj, f).await
    }
    pub fn to_formatted<F: SendFormat>(self, format: F) -> RawSendChannel<F> {
        RawSendChannel {
            channel: self,
            format,
        }
    }
}

impl<F: SendFormat> RefRawSendChannel<'_, F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        self.channel.send(obj, &mut self.format).await
    }
}

impl<F: SendFormat> RawSendChannel<F> {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        self.channel.send(obj, &mut self.format).await
    }
}
