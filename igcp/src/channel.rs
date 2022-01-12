use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::async_snow::Snow;
use crate::io::{Read, TcpStream, Write};

#[cfg(unix)]
use crate::io::UnixStream;

use crate::serialization::formats::{Any, Bincode, Bson, Json, Postcard, ReadFormat, SendFormat};
use crate::serialization::{rx, tx, wss_tx, wss_rx};
use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
use crate::Result;

/// channel that allows input with any serialization format. supports bincode, json, bson and postcard and deserializes in that order
pub type AnyChannel = Channel<AnyInput, Bincode>;
/// read format that allows input with any serialization format. supports bincode, json, bson and postcard and deserializes in that order
pub type AnyInput = Any<Bincode, Any<Json, Any<Bson, Postcard>>>;

pub(crate) type WSS = async_tungstenite::WebSocketStream<TcpStream>;

#[derive(From)]
/// `Channel` abstracts network communications as object streams.
///
/// ```norun
/// async fn send_random(mut chan: Channel) -> Result<()> {
///     chan.send(fastrand::u64(0..1000)).await?;
///     Ok(())
/// }
/// ```
pub enum Channel<ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode> {
    /// encrypted tcp backend
    Tcp(Snow<TcpStream>),
    /// encrypted backend for any type that implements Read + Write
    EncryptedAny(Snow<Box<dyn ReadWrite>>),
    /// unencrypted tcp backend
    InsecureTcp(TcpStream),
    /// unencrypted backend for any type that implements Read + Write
    InsecureAny(Box<dyn ReadWrite>),

    #[cfg(unix)]
    /// encrypted unix backend
    Unix(Snow<UnixStream>),
    #[cfg(unix)]
    /// unencrypted unix backend
    InsecureUnix(UnixStream),

    /// encrypted wss backend
    WSS(Snow<WSS>),
    /// unencrypted wss backend
    InsecureWSS(WSS),

    #[allow(non_camel_case_types)]
    /// used to hold the generic types, `Infallible` makes sure that this variant cannot
    /// be constructed without unsafe
    __InternalPhantomData__((PhantomData<(ReadFmt, SendFmt)>, core::convert::Infallible)),
}


#[derive(From)]
/// `BareChannel` is a non-generic version of `Channel` used to make conversion between channels types easier
pub enum BareChannel {
    /// encrypted tcp backend
    Tcp(Snow<TcpStream>),
    /// encrypted backend for any type that implements Read + Write
    EncryptedAny(Snow<Box<dyn ReadWrite>>),
    /// unencrypted tcp backend
    InsecureTcp(TcpStream),
    /// unencrypted backend for any type that implements Read + Write
    InsecureAny(Box<dyn ReadWrite>),

    #[cfg(unix)]
    /// encrypted unix backend
    Unix(Snow<UnixStream>),
    /// unencrypted unix backend
    #[cfg(unix)]
    InsecureUnix(UnixStream),
    /// encrypted wss backend
    WSS(Snow<WSS>),
    /// unencrypted wss backend
    InsecureWSS(WSS),
}

impl<R: ReadFormat, S: SendFormat> From<BareChannel> for Channel<R, S> {
    fn from(c: BareChannel) -> Self {
        match c {
            BareChannel::Tcp(s) => s.into(),
            BareChannel::EncryptedAny(s) => s.into(),
            BareChannel::InsecureTcp(s) => s.into(),
            BareChannel::InsecureAny(s) => s.into(),

            #[cfg(unix)]
            BareChannel::Unix(s) => s.into(),
            #[cfg(unix)]
            BareChannel::InsecureUnix(s) => s.into(),
            BareChannel::WSS(s) => s.into(),
            BareChannel::InsecureWSS(s) => s.into(),
        }
    }
}

/// wrapper trait to allow any type that implements `Read`, `Write`, `Send`, `Sync` and `'static`
/// to use `Channel`
pub trait ReadWrite: Read + Write + Unpin + Send + Sync + 'static {}

/// wrapper trait to allow any type that implements `Read`, `Write`, `Send`, `Sync` and `'static`
/// to use `Channel`, uses `futures` instead of `futures_lite`
pub trait FuturesReadWrite: futures::prelude::AsyncRead + futures::prelude::AsyncWrite + Unpin + Send + Sync + 'static {}

impl<T: Read + Write + 'static + Unpin + Send + Sync> ReadWrite for T {}
impl<T: futures::prelude::AsyncRead + futures::prelude::AsyncWrite + Unpin + Send + Sync + 'static> FuturesReadWrite for T {}

impl<ReadFmt: ReadFormat, SendFmt: SendFormat> Channel<ReadFmt, SendFmt> {
    /// create a new channel from a tcp stream
    pub async fn new_tcp_encrypted(stream: TcpStream) -> Result<Self> {
        Ok(Snow::new(stream).await?.into())
    }
    /// create a new channel from a tcp stream
    pub async fn new_wss_encrypted(stream: WSS) -> Result<Self> {
        Ok(Snow::new_wss(stream).await?.into())
    }
    #[cfg(unix)]
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
    ///     peer.tx(123).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn tx<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        match self {
            Channel::Tcp(st) => st.tx::<_, SendFmt>(obj).await,
            Channel::EncryptedAny(st) => st.tx::<_, SendFmt>(obj).await,
            Channel::InsecureAny(st) => tx::<_, _, SendFmt>(st, obj).await,
            Channel::InsecureTcp(st) => tx::<_, _, SendFmt>(st, obj).await,

            #[cfg(unix)]
            Channel::Unix(st) => st.tx::<_, SendFmt>(obj).await,
            #[cfg(unix)]
            Channel::InsecureUnix(st) => tx::<_, _, SendFmt>(st, obj).await,

            Channel::WSS(st) => st.wss_tx::<_, SendFmt>(obj).await,
            Channel::InsecureWSS(st) => wss_tx::<_, _, SendFmt>(st, obj).await,

            Channel::__InternalPhantomData__(_) => unreachable!(),
        }
    }
    /// send an object through the wire and return the size of the object sent
    pub async fn send<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        self.tx(obj).await
    }
    /// receive message from stream
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     let num: u64 = peer.rx().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn rx<O: DeserializeOwned>(&mut self) -> Result<O> {
        match self {
            Channel::Tcp(st) => st.rx::<_, ReadFmt>().await,
            Channel::EncryptedAny(st) => st.rx::<_, ReadFmt>().await,
            Channel::InsecureAny(st) => rx::<_, _, ReadFmt>(st).await,
            Channel::InsecureTcp(st) => rx::<_, _, ReadFmt>(st).await,

            #[cfg(unix)]
            Channel::Unix(st) => st.rx::<_, ReadFmt>().await,
            #[cfg(unix)]
            Channel::InsecureUnix(st) => rx::<_, _, ReadFmt>(st).await,

            Channel::WSS(st) => st.wss_rx::<_, ReadFmt>().await,
            Channel::InsecureWSS(st) => wss_rx::<_, _, ReadFmt>(st).await,

            Channel::__InternalPhantomData__(_) => unreachable!(),
        }
    }
    /// receive message from stream
    /// ```norun
    /// async fn service(mut peer: Channel) -> Result<()> {
    ///     let num: u64 = peer.receive().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive<O: DeserializeOwned>(&mut self) -> Result<O> {
        self.rx().await
    }
    /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `PeerChannel`
    pub fn new_main<P: Pipeline>(self) -> MainChannel<P::Pipe, ReadFmt, SendFmt> {
        MainChannel(Default::default(), self)
    }
    /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `MainChannel`
    pub fn new_peer<P: Pipeline>(self) -> PeerChannel<P::Pipe, ReadFmt, SendFmt> {
        PeerChannel(Default::default(), self)
    }
    /// coerce a channel into another kind of channel:
    /// Channel -> Channel<Json, Bincode> -> AnyChannel
    pub fn coerce<R: ReadFormat, S: SendFormat>(self) -> Channel<R, S> {
        match self {
            Channel::Tcp(s) => s.into(),
            Channel::EncryptedAny(s) => s.into(),
            Channel::InsecureTcp(s) => s.into(),
            Channel::InsecureAny(s) => s.into(),

            #[cfg(unix)]
            Channel::Unix(s) => s.into(),
            #[cfg(unix)]
            Channel::InsecureUnix(s) => s.into(),

            Channel::WSS(s) => s.into(),
            Channel::InsecureWSS(s) => s.into(),
            Channel::__InternalPhantomData__(_) => unreachable!(),
        }
    }
    /// make the channel bare, stripping it from its generics
    pub fn bare(self) -> BareChannel {
        match self {
            Channel::Tcp(s) => s.into(),
            Channel::EncryptedAny(s) => s.into(),
            Channel::InsecureTcp(s) => s.into(),
            Channel::InsecureAny(s) => s.into(),

            #[cfg(unix)]
            Channel::Unix(s) => s.into(),
            #[cfg(unix)]
            Channel::InsecureUnix(s) => s.into(),

            Channel::WSS(s) => s.into(),
            Channel::InsecureWSS(s) => s.into(),
            Channel::__InternalPhantomData__(_) => unreachable!(),
        }
    }
}
