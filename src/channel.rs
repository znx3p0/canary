use crate::async_snow::Snow;
use crate::io::{Read, Write};
use cfg_if::cfg_if;
use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::TcpStream;

#[cfg(unix)]
#[cfg(not(target_arch = "wasm32"))]
use crate::io::UnixStream;

use crate::serialization::formats::Format;
use crate::serialization::{rx, tx, wss_rx, wss_tx};
use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
use crate::Result;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// inner websocket type
        pub type WSS = reqwasm::websocket::futures::WebSocket;
    } else {
        /// inner websocket type
        pub type WSS = crate::io::wss::WebSocketStream<
            async_tungstenite::tokio::TokioAdapter<TcpStream>
        >;
    }
}

/// `Channel` abstracts network communications as object streams.
///
/// ```norun
/// async fn send_random(mut chan: Channel) -> Result<()> {
///     chan.send(fastrand::u64(0..1000)).await?;
///     Ok(())
/// }
/// ```
pub struct Channel {
    inner: InnerChannel,
    format: Format,
}

impl<T: Into<InnerChannel>> From<T> for Channel {
    #[inline]
    fn from(inner: T) -> Self {
        let inner: InnerChannel = inner.into();
        Channel {
            inner,
            format: Format::Bincode,
        }
    }
}

#[derive(From)]
enum InnerChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted tcp backend
    Tcp(Snow<TcpStream>),
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    InsecureTcp(TcpStream),

    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted unix backend
    Unix(Snow<UnixStream>),
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted unix backend
    InsecureUnix(UnixStream),

    /// encrypted wss backend
    WSS(Snow<WSS>),
    /// unencrypted wss backend
    InsecureWSS(WSS),

    /// encrypted backend for any type that implements Read + Write
    EncryptedAny(Snow<Box<dyn ReadWrite>>),
    /// unencrypted backend for any type that implements Read + Write
    InsecureAny(Box<dyn ReadWrite>),
}

/// wrapper trait to allow any type that implements `Read`, `Write`, `Send` and `Sync`
/// to use `Channel`
pub trait ReadWrite: Read + Write + Unpin + Send + Sync {}

impl<T: Read + Write + Unpin + Send + Sync> ReadWrite for T {}

impl Channel {
    #[cfg(not(target_arch = "wasm32"))]
    /// create a new channel from a tcp stream
    pub async fn new_tcp_encrypted(stream: TcpStream) -> Result<Self> {
        Ok(Snow::new(stream).await?.into())
    }
    /// create a new channel from a tcp stream
    pub async fn new_wss_encrypted(stream: WSS) -> Result<Self> {
        Ok(Snow::new_wss(stream).await?.into())
    }
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// create a new channel from a unix stream
    pub async fn new_unix_encrypted(stream: UnixStream) -> Result<Self> {
        Ok(Snow::new(stream).await?.into())
    }
    /// create a new channel from an unsupported type
    ///
    /// accepts any type and uses dynamic dispatch, only use if your type is not supported
    pub async fn new_any_encrypted(stream: impl Into<Box<dyn ReadWrite>>) -> Result<Self> {
        Ok(Snow::new(stream.into()).await?.into())
    }

    /// send message to stream
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     peer.send(123).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn send<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        let f = &self.format;
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::Tcp(st) => st.tx(obj, f).await,
            InnerChannel::EncryptedAny(st) => st.tx(obj, f).await,
            InnerChannel::InsecureAny(st) => tx(st, obj, f).await,
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::InsecureTcp(st) => tx(st, obj, f).await,

            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::Unix(st) => st.tx(obj, f).await,
            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::InsecureUnix(st) => tx(st, obj, f).await,

            InnerChannel::WSS(st) => st.wss_tx(obj, f).await,
            InnerChannel::InsecureWSS(st) => wss_tx(st, obj, f).await,
        }
    }

    /// alias to the `.send` method
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     peer.tx(123).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn tx<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        self.send(obj).await
    }
    /// receive message from stream
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     let num: u64 = peer.receive().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive<O: DeserializeOwned>(&mut self) -> Result<O> {
        let f = &self.format;
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::Tcp(st) => st.rx(f).await,
            InnerChannel::EncryptedAny(st) => st.rx(f).await,
            InnerChannel::InsecureAny(st) => rx(st, f).await,
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::InsecureTcp(st) => rx(st, f).await,

            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::Unix(st) => st.rx(f).await,
            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::InsecureUnix(st) => rx(st, f).await,

            InnerChannel::WSS(st) => st.wss_rx(f).await,
            InnerChannel::InsecureWSS(st) => wss_rx(st, f).await,
        }
    }

    /// alias of the `.receive` method.
    /// Receive message from stream
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     let num: u64 = peer.rx().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn rx<O: DeserializeOwned>(&mut self) -> Result<O> {
        self.receive().await
    }
    /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `PeerChannel`
    pub fn new_main<P: Pipeline>(self) -> MainChannel<P::Pipe> {
        MainChannel(Default::default(), self)
    }
    /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `MainChannel`
    pub fn new_peer<P: Pipeline>(self) -> PeerChannel<P::Pipe> {
        PeerChannel(Default::default(), self)
    }
    /// set the format of the channel
    pub fn set_format(&mut self, format: Format) {
        self.format = format
    }
}

#[derive(From)]
/// a channel handshake that determines if the channel will have encryption
pub struct Handshake(Channel);
impl Handshake {
    /// encrypt the channel
    /// ```norun
    /// while let Ok(chan) = provider.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn encrypted(self) -> Result<Channel> {
        match self.0.inner {
            #[cfg(not(target_arch = "wasm32"))]
            InnerChannel::InsecureTcp(tcp) => Channel::new_tcp_encrypted(tcp).await,
            InnerChannel::InsecureAny(any) => Channel::new_any_encrypted(any).await,
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(unix)]
            InnerChannel::InsecureUnix(unix) => Channel::new_unix_encrypted(unix).await,
            InnerChannel::InsecureWSS(wss) => Channel::new_wss_encrypted(wss).await,
            encrypted => Ok(encrypted.into()), // double encryption is not supported
        }
    }
    #[inline]
    /// get an unencrypted channel
    /// ```norun
    /// while let Ok(chan) = provider.next().await {
    ///     let mut chan = chan.raw();
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub fn raw(self) -> Channel {
        self.0
    }
}
