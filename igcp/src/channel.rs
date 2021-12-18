use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use crate::async_snow::Snow;
use crate::io::{Read, TcpStream, UnixStream, Write};

use crate::serialization::formats::{Any, Bincode, Bson, Json, Postcard, ReadFormat, SendFormat};
use crate::serialization::{rx, tx};
use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
use crate::Result;

/// channel that allows input with any serialization format. supports bincode, json, bson and postcard and deserializes in that order
pub type AnyChannel = Channel<AnyInput, Bincode>;
pub type AnyInput = Any<Bincode, Any<Json, Any<Bson, Postcard>>>;

#[derive(From)]
/// agnostic channel that can be used for local or remote communication.
///
/// do note that channels cannot send messages over u32::MAX length,
/// and they may have problems sending large messages.
/// ```norun
/// async fn send_random(mut chan: Channel) -> Result<()> {
///     chan.send(fastrand::u64(0..1000)).await?;
///     Ok(())
/// }
/// ```
pub enum Channel<ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode> {
    Tcp(Snow<TcpStream>),
    Unix(Snow<UnixStream>),
    EncryptedAny(Snow<Box<dyn ReadWrite>>),
    InsecureTcp(TcpStream),
    InsecureUnix(UnixStream),
    InsecureAny(Box<dyn ReadWrite>),
    #[allow(non_camel_case_types)] // Infallible makes sure that this value cannot be constructed
    __InternalPhantomData__((PhantomData<(ReadFmt, SendFmt)>, core::convert::Infallible)),
}

#[derive(From)]
pub enum BareChannel {
    Tcp(Snow<TcpStream>),
    Unix(Snow<UnixStream>),
    EncryptedAny(Snow<Box<dyn ReadWrite>>),
    InsecureTcp(TcpStream),
    InsecureUnix(UnixStream),
    InsecureAny(Box<dyn ReadWrite>),
}

impl<R: ReadFormat, S: SendFormat> From<BareChannel> for Channel<R, S> {
    fn from(c: BareChannel) -> Self {
        match c {
            BareChannel::Tcp(s) => s.into(),
            BareChannel::Unix(s) => s.into(),
            BareChannel::EncryptedAny(s) => s.into(),
            BareChannel::InsecureTcp(s) => s.into(),
            BareChannel::InsecureUnix(s) => s.into(),
            BareChannel::InsecureAny(s) => s.into(),
        }
    }
}

pub trait ReadWrite: Read + Write + Unpin + Send + Sync + 'static {}
impl<T: Read + Write + 'static + Unpin + Send + Sync> ReadWrite for T {}

impl<ReadFmt: ReadFormat, SendFmt: SendFormat> Channel<ReadFmt, SendFmt> {
    pub async fn new_tcp_encrypted(stream: TcpStream) -> Result<Self> {
        Ok(Snow::new(stream).await?.into())
    }
    pub async fn new_unix_encrypted(stream: UnixStream) -> Result<Self> {
        Ok(Snow::new(stream).await?.into())
    }
    /// accepts any type and uses dynamic dispatch, only use if your type is not supported
    pub async fn new_any_encrypted(stream: impl Into<Box<dyn ReadWrite>>) -> Result<Self> {
        Ok(Snow::new(stream.into()).await?.into())
    }
    pub async fn tx<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        match self {
            Channel::Tcp(st) => st.tx(obj).await,
            Channel::Unix(st) => st.tx(obj).await,
            Channel::EncryptedAny(st) => st.tx(obj).await,
            Channel::InsecureAny(st) => tx::<_, _, SendFmt>(st, obj).await,
            Channel::InsecureTcp(st) => tx::<_, _, SendFmt>(st, obj).await,
            Channel::InsecureUnix(st) => tx::<_, _, SendFmt>(st, obj).await,
            Channel::__InternalPhantomData__(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }
    pub async fn send<O: Serialize>(&mut self, obj: O) -> Result<usize> {
        self.tx(obj).await
    }
    pub async fn rx<O: DeserializeOwned>(&mut self) -> Result<O> {
        match self {
            Channel::Tcp(st) => st.rx().await,
            Channel::Unix(st) => st.rx().await,
            Channel::EncryptedAny(st) => st.rx().await,
            Channel::InsecureAny(st) => rx::<_, _, ReadFmt>(st).await,
            Channel::InsecureTcp(st) => rx::<_, _, ReadFmt>(st).await,
            Channel::InsecureUnix(st) => rx::<_, _, ReadFmt>(st).await,
            Channel::__InternalPhantomData__(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }
    pub async fn receive<O: DeserializeOwned>(&mut self) -> Result<O> {
        self.rx().await
    }
    pub fn new_main<P: Pipeline>(self) -> MainChannel<P::Pipe, ReadFmt, SendFmt> {
        MainChannel(Default::default(), self)
    }
    pub fn new_peer<P: Pipeline>(self) -> PeerChannel<P::Pipe, ReadFmt, SendFmt> {
        PeerChannel(Default::default(), self)
    }
    pub fn coerce<R: ReadFormat, S: SendFormat>(self) -> Channel<R, S> {
        match self {
            Channel::Tcp(s) => s.into(),
            Channel::Unix(s) => s.into(),
            Channel::EncryptedAny(s) => s.into(),
            Channel::InsecureTcp(s) => s.into(),
            Channel::InsecureUnix(s) => s.into(),
            Channel::InsecureAny(s) => s.into(),
            Channel::__InternalPhantomData__(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }
    pub fn bare(self) -> BareChannel {
        match self {
            Channel::Tcp(s) => s.into(),
            Channel::Unix(s) => s.into(),
            Channel::EncryptedAny(s) => s.into(),
            Channel::InsecureTcp(s) => s.into(),
            Channel::InsecureUnix(s) => s.into(),
            Channel::InsecureAny(s) => s.into(),
            Channel::__InternalPhantomData__(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}
