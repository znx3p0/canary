use derive_more::From;
use futures::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};

use crate::channel::raw::bipartite::receive_channel::UnformattedRawReceiveChannel;
use crate::channel::raw::bipartite::send_channel::UnformattedRawSendChannel;
use crate::io::Message;
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
/// Reference unformatted unencrypted channel that has not been separated
/// into its read/write components
pub enum RefUnformattedRawUnifiedChannel<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    /// tcp backend
    Tcp(&'a mut TcpStream),
    #[cfg(unix)]
    /// unix backend
    Unix(&'a mut UnixStream),
    /// wss backend
    Wss(&'a mut Wss),
    #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
    /// quic backend
    Quic(&'a mut quinn::SendStream, &'a mut quinn::RecvStream),
}

#[derive(From)]
/// Unformatted unencrypted channel that has not been separated
/// into its read/write components
pub enum UnformattedRawUnifiedChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// Tcp backend
    Tcp(TcpStream),
    #[cfg(unix)]
    /// Unix backend
    Unix(UnixStream),
    /// WebSocket backend
    Wss(Box<Wss>), // boxed since it's heavy and would weigh down other variants
    #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
    /// Quic backend
    Quic(quinn::SendStream, quinn::RecvStream),
}

impl UnformattedRawUnifiedChannel {
    /// Construct an unformatted raw unified channel from any backend
    pub fn new(from: impl Into<Self>) -> Self {
        from.into()
    }
    #[must_use]
    /// Split channel into its send and receive components
    pub fn split(self) -> (UnformattedRawSendChannel, UnformattedRawReceiveChannel) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedRawUnifiedChannel::Tcp(stream) => {
                let (read, write) = stream.into_split();
                (From::from(write), From::from(read))
            }
            #[cfg(unix)]
            UnformattedRawUnifiedChannel::Unix(stream) => {
                let (read, write) = stream.into_split();
                (From::from(write), From::from(read))
            }
            UnformattedRawUnifiedChannel::Wss(stream) => {
                let (write, read) = stream.split();
                (From::from(write), From::from(read))
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            UnformattedRawUnifiedChannel::Quic(write, read) => {
                (From::from(write), From::from(read))
            }
        }
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
        RefUnformattedRawUnifiedChannel::from(self)
            .send(obj, format)
            .await
    }
    /// Receive an object sent through the channel with format
    /// ```no_run
    /// let string: String = chan.receive(&mut Format::Bincode).await?;
    /// ```
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
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedRawUnifiedChannel::Tcp(ref mut chan) => chan.into(),
            #[cfg(unix)]
            UnformattedRawUnifiedChannel::Unix(ref mut chan) => chan.into(),
            UnformattedRawUnifiedChannel::Wss(ref mut chan) => {
                RefUnformattedRawUnifiedChannel::Wss(chan)
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            UnformattedRawUnifiedChannel::Quic(ref mut tx, ref mut rx) => From::from((tx, rx)),
        }
    }
}

impl<'a> RefUnformattedRawUnifiedChannel<'a> {
    /// Send an object through the channel serialized with format
    /// ```no_run
    /// chan.send("Hello world!", &mut Format::Bincode).await?;
    /// ```
    pub async fn send<T: Serialize, F: SendFormat>(
        &mut self,
        obj: T,
        format: &mut F,
    ) -> Result<usize> {
        #[allow(unused)]
        use crate::serialization::tx;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tcp(st) => tx(st, obj, format).await,
            #[cfg(unix)]
            Self::Unix(st) => tx(st, obj, format).await,
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            Self::Quic(st, _) => tx(st, obj, format).await,
            Self::Wss(st) => {
                let buf = format.serialize(&obj).map_err(err!(@invalid_data))?;
                let len = buf.len();

                // use tungstenite for native
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let item = Message::Binary(buf);
                    st.send(item).await.map_err(err!(@other))?;
                };

                // use reqwasm for wasm
                #[cfg(target_arch = "wasm32")]
                {
                    let item = Message::Bytes(buf);
                    st.send(item).await.map_err(|e| err!(e.to_string()))?;
                };
                Ok(len)
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
        #[allow(unused)]
        use crate::serialization::{rx, wss_rx};
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tcp(st) => rx(st, format).await,
            #[cfg(unix)]
            Self::Unix(st) => rx(st, format).await,
            Self::Wss(st) => wss_rx(st, format).await,
            #[cfg(all(not(target_arch = "wasm32"), feature = "quic"))]
            Self::Quic(_, st) => rx(st, format).await,
        }
    }
    /// Get a formatted channel with the specified format
    /// ```no_run
    /// unformatted.send("Hi!", &mut Format::Bincode).await?;
    /// let mut formatted = unformatted.as_formatted(Format::Bincode).await?;
    /// formatted.send("Hi!").await?;
    /// ```
    pub fn as_formatted<F>(&'a mut self, format: F) -> RefRawUnifiedChannel<'a, F> {
        RefRawUnifiedChannel {
            channel: self,
            format,
        }
    }
}
