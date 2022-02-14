#![feature(prelude_import)]
#![forbid(unsafe_code)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
//! # Canary
//! Canary is a library for making communication through the network easy.
//! It abstracts over network primitives such as streams and provides
//! constructs that are easier to use such as `Channel`.
//!
//! The main constructs offered by Canary are:
//! - Channels
//! - Providers
//!
//! Channels help communicate through the network,
//! and providers help expose services through the network.
//!
//! The crate is well-documented, but if you need any examples
//! you should use [the book](https://znx3p0.github.io/canary-book/),
//! and any questions should be asked in [the discord](https://discord.gg/QaWxMzAZs8)
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
/// contains encrypted stream
pub mod async_snow {
    use serde::{de::DeserializeOwned, Serialize};
    use snow::{params::*, TransportState};
    use crate::io::{ReadExt, WriteExt};
    use crate::serialization::formats::{Bincode, ReadFormat, SendFormat};
    use crate::serialization::{rx, tx, wss_rx, wss_tx, zc};
    use crate::{channel::ReadWrite, err};
    use crate::{channel::WSS, Result};
    /// Stream wrapper with encryption.
    /// It uses the Noise protocol for encryption
    pub struct Snow<T> {
        pub(crate) stream: T,
        transport: TransportState,
    }
    const PACKET_LEN: u64 = 65519;
    impl<T> Snow<T> {
        fn encrypt_packets(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
            let mut total = ::alloc::vec::Vec::new();
            for buf in buf.chunks(PACKET_LEN as _) {
                let mut buf = self.encrypt_packet(buf)?;
                total.append(&mut buf);
            }
            Ok(total)
        }
        fn encrypt_packet(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
            let mut msg = ::alloc::vec::from_elem(0u8, buf.len() + 16);
            self.encrypt_packet_raw(buf, &mut msg)?;
            Ok(msg)
        }
        fn encrypt_packet_raw(&mut self, buf: &[u8], mut msg: &mut [u8]) -> Result<()> {
            self.transport.write_message(buf, &mut msg).map_err(|e| {
                crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })?;
            Ok(())
        }
    }
    impl<T: ReadWrite + Unpin> Snow<T> {
        /// Starts a new snow stream using the default noise parameters
        #[inline]
        pub async fn new(stream: T) -> Result<Self> {
            let noise_params = NoiseParams::new(
                "".into(),
                BaseChoice::Noise,
                HandshakeChoice {
                    pattern: HandshakePattern::NN,
                    modifiers: HandshakeModifierList {
                        list: ::alloc::vec::Vec::new(),
                    },
                },
                DHChoice::Curve25519,
                CipherChoice::ChaChaPoly,
                HashChoice::Blake2s,
            );
            Self::new_with_params(stream, noise_params).await
        }
        /// starts a new snow stream using the provided parameters.
        #[inline]
        pub async fn new_with_params(mut stream: T, noise_params: NoiseParams) -> Result<Self> {
            let should_init = loop {
                let local_num = rand::random::<u64>();
                tx::<_, _, Bincode>(&mut stream, local_num).await?;
                let peer_num = rx::<_, _, Bincode>(&mut stream).await?;
                if local_num == peer_num {
                    continue;
                }
                if local_num > peer_num {
                    break false;
                }
                break true;
            };
            let builder = snow::Builder::new(noise_params);
            let keypair = builder.generate_keypair().or_else(|e| {
                Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })?;
            let builder = builder.local_private_key(&keypair.private);
            tx::<_, _, Bincode>(&mut stream, keypair.public).await?;
            let peer_public_key = rx::<_, Vec<u8>, Bincode>(&mut stream).await?;
            let builder = builder.remote_public_key(&peer_public_key);
            let mut buf = ::alloc::vec::from_elem(0u8, 256);
            match should_init {
                true => {
                    let mut handshake = builder.build_initiator().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    let len = handshake.write_message(&[], &mut buf).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;
                    handshake
                        .read_message(&rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
                        .or_else(|e| {
                            Err(crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        })?;
                    let transport = handshake.into_transport_mode().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    Ok(Snow { stream, transport })
                }
                false => {
                    let mut handshake = builder.build_responder().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    handshake
                        .read_message(&rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
                        .or_else(|e| {
                            Err(crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        })?;
                    let len = handshake.write_message(&[0u8; 0], &mut buf).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;
                    let transport = handshake.into_transport_mode().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    Ok(Snow { stream, transport })
                }
            }
        }
        /// receive message from stream
        /// ```norun
        /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
        ///     let num: u64 = peer.rx().await?;
        ///     Ok(())
        /// }
        /// ```
        #[inline]
        pub async fn rx<O: DeserializeOwned, F: ReadFormat>(&mut self) -> Result<O> {
            let size = zc::read_u64(&mut self.stream).await?;
            let mut buf = zc::try_vec(size as _)?;
            self.stream.read_exact(&mut buf).await?;
            let mut msg = ::alloc::vec::Vec::new();
            for buf in buf.chunks(PACKET_LEN as usize + 16) {
                let mut inner = ::alloc::vec::from_elem(0u8, buf.len());
                self.transport.read_message(&buf, &mut inner).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    )))
                })?;
                msg.append(&mut inner);
            }
            F::deserialize(&msg)
        }
        /// send message to stream
        /// ```norun
        /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
        ///     peer.tx(123).await?;
        ///     Ok(())
        /// }
        /// ```
        #[inline]
        pub async fn tx<O: Serialize, F: SendFormat>(&mut self, obj: O) -> Result<usize> {
            let vec = F::serialize(&obj)?;
            let msg = self.encrypt_packets(&vec)?;
            zc::send_u64(&mut self.stream, msg.len() as _).await?;
            self.stream.write_all(&msg).await?;
            self.stream.flush().await?;
            Ok(msg.len())
        }
    }
    impl Snow<WSS> {
        #[inline]
        /// Starts a new snow stream using the default noise parameters
        pub async fn new_wss(stream: WSS) -> Result<Self> {
            Self::new_with_params_wss(stream, "Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
                .await
        }
        #[inline]
        /// starts a new snow stream using the provided parameters.
        pub async fn new_with_params_wss(
            mut stream: WSS,
            noise_params: NoiseParams,
        ) -> Result<Self> {
            let should_init = loop {
                let local_num = rand::random::<u64>();
                wss_tx::<_, _, Bincode>(&mut stream, local_num).await?;
                let peer_num = wss_rx::<_, _, Bincode>(&mut stream).await?;
                if local_num == peer_num {
                    continue;
                }
                if local_num > peer_num {
                    break false;
                }
                break true;
            };
            let builder = snow::Builder::new(noise_params);
            let keypair = builder.generate_keypair().or_else(|e| {
                Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })?;
            let builder = builder.local_private_key(&keypair.private);
            wss_tx::<_, _, Bincode>(&mut stream, keypair.public).await?;
            let peer_public_key = wss_rx::<_, Vec<u8>, Bincode>(&mut stream).await?;
            let builder = builder.remote_public_key(&peer_public_key);
            let mut buf = ::alloc::vec::from_elem(0u8, 256);
            match should_init {
                true => {
                    let mut handshake = builder.build_initiator().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    let len = handshake.write_message(&[], &mut buf).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    wss_tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;
                    handshake
                        .read_message(&wss_rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
                        .or_else(|e| {
                            Err(crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        })?;
                    let transport = handshake.into_transport_mode().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    Ok(Snow { stream, transport })
                }
                false => {
                    let mut handshake = builder.build_responder().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    handshake
                        .read_message(&wss_rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
                        .or_else(|e| {
                            Err(crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        })?;
                    let len = handshake.write_message(&[0u8; 0], &mut buf).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    wss_tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;
                    let transport = handshake.into_transport_mode().or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    })?;
                    Ok(Snow { stream, transport })
                }
            }
        }
        #[inline]
        /// receive message from stream
        /// ```norun
        /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
        ///     let num: u64 = peer.rx().await?;
        ///     Ok(())
        /// }
        /// ```
        pub async fn wss_rx<O: DeserializeOwned, F: ReadFormat>(&mut self) -> Result<O> {
            let buf: Vec<u8> = wss_rx::<_, _, F>(&mut self.stream).await?;
            let mut msg = ::alloc::vec::Vec::new();
            for buf in buf.chunks(PACKET_LEN as usize + 16) {
                let mut inner = ::alloc::vec::from_elem(0u8, buf.len());
                self.transport.read_message(&buf, &mut inner).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    )))
                })?;
                msg.append(&mut inner);
            }
            F::deserialize(&msg)
        }
        #[inline]
        /// send message to stream
        /// ```norun
        /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
        ///     peer.tx(123).await?;
        ///     Ok(())
        /// }
        /// ```
        pub async fn wss_tx<O: Serialize, F: SendFormat>(&mut self, obj: O) -> Result<usize> {
            let vec = F::serialize(&obj)?;
            let msg = self.encrypt_packets(&vec)?;
            let len = msg.len();
            wss_tx::<_, _, F>(&mut self.stream, msg).await?;
            Ok(len)
        }
    }
}
/// contains channels and constructs associated with them
pub mod channel {
    use cfg_if::cfg_if;
    use derive_more::From;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::marker::PhantomData;
    use crate::async_snow::Snow;
    use crate::io::{Read, Write};
    #[cfg(not(target_arch = "wasm32"))]
    use crate::io::TcpStream;
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    use crate::io::UnixStream;
    use crate::serialization::formats::{Any, Bincode, Bson, Json, Postcard, ReadFormat, SendFormat};
    use crate::serialization::{rx, tx, wss_rx, wss_tx};
    use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
    use crate::Result;
    /// channel that allows input with any serialization format. supports bincode, json, bson and postcard and deserializes in that order
    pub type AnyChannel = Channel<AnyInput, Bincode>;
    /// read format that allows input with any serialization format. supports bincode, json, bson and postcard and deserializes in that order
    pub type AnyInput = Any<Bincode, Any<Json, Any<Bson, Postcard>>>;
    /// inner websocket type
    pub type WSS =
        crate::io::wss::WebSocketStream<async_tungstenite::tokio::TokioAdapter<TcpStream>>;
    /// `Channel` abstracts network communications as object streams.
    ///
    /// ```norun
    /// async fn send_random(mut chan: Channel) -> Result<()> {
    ///     chan.send(fastrand::u64(0..1000)).await?;
    ///     Ok(())
    /// }
    /// ```
    pub enum Channel<ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode> {
        #[cfg(not(target_arch = "wasm32"))]
        /// encrypted tcp backend
        Tcp(Snow<TcpStream>),
        #[cfg(not(target_arch = "wasm32"))]
        /// unencrypted tcp backend
        InsecureTcp(TcpStream),
        /// encrypted backend for any type that implements Read + Write
        EncryptedAny(Snow<Box<dyn ReadWrite>>),
        /// unencrypted backend for any type that implements Read + Write
        InsecureAny(Box<dyn ReadWrite>),
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
        #[allow(non_camel_case_types)]
        /// used to hold the generic types, `Infallible` makes sure that this variant cannot
        /// be constructed without unsafe
        __InternalPhantomData__((PhantomData<(ReadFmt, SendFmt)>, core::convert::Infallible)),
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(Snow<WSS>)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (Snow<WSS>)) -> Channel<ReadFmt, SendFmt> {
            Channel::WSS(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(TcpStream)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (TcpStream)) -> Channel<ReadFmt, SendFmt> {
            Channel::InsecureTcp(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(Snow<TcpStream>)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (Snow<TcpStream>)) -> Channel<ReadFmt, SendFmt> {
            Channel::Tcp(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(Snow<UnixStream>)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (Snow<UnixStream>)) -> Channel<ReadFmt, SendFmt> {
            Channel::Unix(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(Snow<Box<dyn ReadWrite>>)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (Snow<Box<dyn ReadWrite>>)) -> Channel<ReadFmt, SendFmt> {
            Channel::EncryptedAny(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat>
        ::core::convert::From<((PhantomData<(ReadFmt, SendFmt)>, core::convert::Infallible))>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(
            original: ((PhantomData<(ReadFmt, SendFmt)>, core::convert::Infallible)),
        ) -> Channel<ReadFmt, SendFmt> {
            Channel::__InternalPhantomData__(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(Box<dyn ReadWrite>)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (Box<dyn ReadWrite>)) -> Channel<ReadFmt, SendFmt> {
            Channel::InsecureAny(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(WSS)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (WSS)) -> Channel<ReadFmt, SendFmt> {
            Channel::InsecureWSS(original)
        }
    }
    #[automatically_derived]
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> ::core::convert::From<(UnixStream)>
        for Channel<ReadFmt, SendFmt>
    {
        #[inline]
        fn from(original: (UnixStream)) -> Channel<ReadFmt, SendFmt> {
            Channel::InsecureUnix(original)
        }
    }
    /// `BareChannel` is a non-generic version of `Channel` used to make conversion between channels types easier
    pub enum BareChannel {
        /// encrypted tcp backend
        #[cfg(not(target_arch = "wasm32"))]
        Tcp(Snow<TcpStream>),
        /// encrypted backend for any type that implements Read + Write
        EncryptedAny(Snow<Box<dyn ReadWrite>>),
        /// unencrypted tcp backend
        #[cfg(not(target_arch = "wasm32"))]
        InsecureTcp(TcpStream),
        /// unencrypted backend for any type that implements Read + Write
        InsecureAny(Box<dyn ReadWrite>),
        #[cfg(unix)]
        #[cfg(not(target_arch = "wasm32"))]
        /// encrypted unix backend
        Unix(Snow<UnixStream>),
        /// unencrypted unix backend
        #[cfg(unix)]
        #[cfg(not(target_arch = "wasm32"))]
        InsecureUnix(UnixStream),
        /// encrypted wss backend
        WSS(Snow<WSS>),
        /// unencrypted wss backend
        InsecureWSS(WSS),
    }
    #[automatically_derived]
    impl ::core::convert::From<(Snow<WSS>)> for BareChannel {
        #[inline]
        fn from(original: (Snow<WSS>)) -> BareChannel {
            BareChannel::WSS(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(TcpStream)> for BareChannel {
        #[inline]
        fn from(original: (TcpStream)) -> BareChannel {
            BareChannel::InsecureTcp(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(Snow<TcpStream>)> for BareChannel {
        #[inline]
        fn from(original: (Snow<TcpStream>)) -> BareChannel {
            BareChannel::Tcp(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(Snow<UnixStream>)> for BareChannel {
        #[inline]
        fn from(original: (Snow<UnixStream>)) -> BareChannel {
            BareChannel::Unix(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(Snow<Box<dyn ReadWrite>>)> for BareChannel {
        #[inline]
        fn from(original: (Snow<Box<dyn ReadWrite>>)) -> BareChannel {
            BareChannel::EncryptedAny(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(Box<dyn ReadWrite>)> for BareChannel {
        #[inline]
        fn from(original: (Box<dyn ReadWrite>)) -> BareChannel {
            BareChannel::InsecureAny(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(WSS)> for BareChannel {
        #[inline]
        fn from(original: (WSS)) -> BareChannel {
            BareChannel::InsecureWSS(original)
        }
    }
    #[automatically_derived]
    impl ::core::convert::From<(UnixStream)> for BareChannel {
        #[inline]
        fn from(original: (UnixStream)) -> BareChannel {
            BareChannel::InsecureUnix(original)
        }
    }
    impl<R: ReadFormat, S: SendFormat> From<BareChannel> for Channel<R, S> {
        #[inline]
        fn from(c: BareChannel) -> Self {
            match c {
                #[cfg(not(target_arch = "wasm32"))]
                BareChannel::Tcp(s) => s.into(),
                BareChannel::EncryptedAny(s) => s.into(),
                #[cfg(not(target_arch = "wasm32"))]
                BareChannel::InsecureTcp(s) => s.into(),
                BareChannel::InsecureAny(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                BareChannel::Unix(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                BareChannel::InsecureUnix(s) => s.into(),
                BareChannel::WSS(s) => s.into(),
                BareChannel::InsecureWSS(s) => s.into(),
            }
        }
    }
    /// wrapper trait to allow any type that implements `Read`, `Write`, `Send`, `Sync` and `'static`
    /// to use `Channel`
    pub trait ReadWrite: Read + Write + Unpin + Send + Sync + 'static {}
    impl<T: Read + Write + 'static + Unpin + Send + Sync> ReadWrite for T {}
    impl<ReadFmt: ReadFormat, SendFmt: SendFormat> Channel<ReadFmt, SendFmt> {
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
            match self {
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Tcp(st) => st.tx::<_, SendFmt>(obj).await,
                Channel::EncryptedAny(st) => st.tx::<_, SendFmt>(obj).await,
                Channel::InsecureAny(st) => tx::<_, _, SendFmt>(st, obj).await,
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureTcp(st) => tx::<_, _, SendFmt>(st, obj).await,
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Unix(st) => st.tx::<_, SendFmt>(obj).await,
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureUnix(st) => tx::<_, _, SendFmt>(st, obj).await,
                Channel::WSS(st) => st.wss_tx::<_, SendFmt>(obj).await,
                Channel::InsecureWSS(st) => wss_tx::<_, _, SendFmt>(st, obj).await,
                Channel::__InternalPhantomData__((_, unreachable)) => match *unreachable {},
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
            match self {
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Tcp(st) => st.rx::<_, ReadFmt>().await,
                Channel::EncryptedAny(st) => st.rx::<_, ReadFmt>().await,
                Channel::InsecureAny(st) => rx::<_, _, ReadFmt>(st).await,
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureTcp(st) => rx::<_, _, ReadFmt>(st).await,
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Unix(st) => st.rx::<_, ReadFmt>().await,
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureUnix(st) => rx::<_, _, ReadFmt>(st).await,
                Channel::WSS(st) => st.wss_rx::<_, ReadFmt>().await,
                Channel::InsecureWSS(st) => wss_rx::<_, _, ReadFmt>(st).await,
                Channel::__InternalPhantomData__((_, unreachable)) => match *unreachable {},
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
        pub fn new_main<P: Pipeline>(self) -> MainChannel<P::Pipe, ReadFmt, SendFmt> {
            MainChannel(Default::default(), self)
        }
        /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `MainChannel`
        pub fn new_peer<P: Pipeline>(self) -> PeerChannel<P::Pipe, ReadFmt, SendFmt> {
            PeerChannel(Default::default(), self)
        }
        /// coerce a channel into another kind of channel:
        /// `Channel` -> `Channel<Json, Bincode>` -> `AnyChannel`
        pub fn coerce<R: ReadFormat, S: SendFormat>(self) -> Channel<R, S> {
            match self {
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Tcp(s) => s.into(),
                Channel::EncryptedAny(s) => s.into(),
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureTcp(s) => s.into(),
                Channel::InsecureAny(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Unix(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureUnix(s) => s.into(),
                Channel::WSS(s) => s.into(),
                Channel::InsecureWSS(s) => s.into(),
                Channel::__InternalPhantomData__((_, unreachable)) => match unreachable {},
            }
        }
        #[inline]
        /// make the channel bare, stripping it from its generics
        pub fn bare(self) -> BareChannel {
            match self {
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Tcp(s) => s.into(),
                Channel::EncryptedAny(s) => s.into(),
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureTcp(s) => s.into(),
                Channel::InsecureAny(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::Unix(s) => s.into(),
                #[cfg(unix)]
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureUnix(s) => s.into(),
                Channel::WSS(s) => s.into(),
                Channel::InsecureWSS(s) => s.into(),
                Channel::__InternalPhantomData__((_, unreachable)) => match unreachable {},
            }
        }
    }
    /// a channel handshake that determines if the channel will have encryption
    pub struct Handshake(Channel);
    #[automatically_derived]
    impl ::core::convert::From<(Channel)> for Handshake {
        #[inline]
        fn from(original: (Channel)) -> Handshake {
            Handshake(original)
        }
    }
    impl Handshake {
        /// encrypt the channel
        /// ```norun
        /// while let Ok(chan) = provider.next().await {
        ///     let mut chan = chan.encrypted().await?;
        ///     chan.send("hello!").await?;
        /// }
        /// ```
        pub async fn encrypted(self) -> Result<Channel> {
            match self.0 {
                #[cfg(not(target_arch = "wasm32"))]
                Channel::InsecureTcp(tcp) => Channel::new_tcp_encrypted(tcp).await,
                Channel::InsecureAny(any) => Channel::new_any_encrypted(any).await,
                #[cfg(not(target_arch = "wasm32"))]
                #[cfg(unix)]
                Channel::InsecureUnix(unix) => Channel::new_unix_encrypted(unix).await,
                Channel::InsecureWSS(wss) => Channel::new_wss_encrypted(wss).await,
                encrypted => Ok(encrypted),
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
}
/// contains custom error types and result
pub mod err {
    use serde::{ser::SerializeTuple, Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};
    use std::{
        fmt::{Debug, Display},
        io::ErrorKind,
    };
    /// a result type equivalent to std::io::Result, but implements `Serialize` and `Deserialize`
    pub type Result<T> = ::std::result::Result<T, Error>;
    /// a result type equivalent to std::io::Error, but implements `Serialize` and `Deserialize`
    pub struct Error(std::io::Error);
    impl Error {
        /// construct a new Error type from a std::io::Error
        pub fn new(e: std::io::Error) -> Self {
            Error(e)
        }
    }
    impl From<std::io::Error> for Error {
        fn from(error: std::io::Error) -> Self {
            Error(error)
        }
    }
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            <std::io::Error as Display>::fmt(&self.0, f)
        }
    }
    impl Debug for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            <std::io::Error as Debug>::fmt(&self.0, f)
        }
    }
    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.0.source()
        }
        fn description(&self) -> &str {
            #[allow(deprecated)]
            self.0.description()
        }
        fn cause(&self) -> Option<&dyn std::error::Error> {
            #[allow(deprecated)]
            self.0.cause()
        }
    }
    impl Serialize for Error {
        fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let string = self.0.to_string();
            let mut tuple = serializer.serialize_tuple(2)?;
            tuple.serialize_element(&string)?;
            let kind: ErrorKindSer = self.0.kind().into();
            tuple.serialize_element(&kind)?;
            tuple.end()
        }
    }
    impl<'de> Deserialize<'de> for Error {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let (error, kind) = <(String, ErrorKindSer)>::deserialize(deserializer)?;
            Ok(Error(std::io::Error::new(kind.into(), error)))
        }
    }
    #[repr(u8)]
    /// Serializable version of `std::io::ErrorKind`
    pub enum ErrorKindSer {
        /// An entity was not found, often a file.
        NotFound,
        /// The operation lacked the necessary privileges to complete.
        PermissionDenied,
        /// The connection was refused by the remote server.
        ConnectionRefused,
        /// The connection was reset by the remote server.
        ConnectionReset,
        /// The remote host is not reachable.
        HostUnreachable,
        /// The network containing the remote host is not reachable.
        NetworkUnreachable,
        /// The connection was aborted (terminated) by the remote server.
        ConnectionAborted,
        /// The network operation failed because it was not connected yet.
        NotConnected,
        /// A socket address could not be bound because the address is already in
        /// use elsewhere.
        AddrInUse,
        /// A nonexistent interface was requested or the requested address was not
        /// local.
        AddrNotAvailable,
        /// The system's networking is down.
        NetworkDown,
        /// The operation failed because a pipe was closed.
        BrokenPipe,
        /// An entity already exists, often a file.
        AlreadyExists,
        /// The operation needs to block to complete, but the blocking operation was
        /// requested to not occur.
        WouldBlock,
        /// A filesystem object is, unexpectedly, not a directory.
        ///
        /// For example, a filesystem path was specified where one of the intermediate directory
        /// components was, in fact, a plain file.
        NotADirectory,
        /// The filesystem object is, unexpectedly, a directory.
        ///
        /// A directory was specified when a non-directory was expected.
        IsADirectory,
        /// A non-empty directory was specified where an empty directory was expected.
        DirectoryNotEmpty,
        /// The filesystem or storage medium is read-only, but a write operation was attempted.
        ReadOnlyFilesystem,
        /// Loop in the filesystem or IO subsystem; often, too many levels of symbolic links.
        ///
        /// There was a loop (or excessively long chain) resolving a filesystem object
        /// or file IO object.
        ///
        /// On Unix this is usually the result of a symbolic link loop; or, of exceeding the
        /// system-specific limit on the depth of symlink traversal.
        FilesystemLoop,
        /// Stale network file handle.
        ///
        /// With some network filesystems, notably NFS, an open file (or directory) can be invalidated
        /// by problems with the network or server.
        StaleNetworkFileHandle,
        /// A parameter was incorrect.
        InvalidInput,
        /// Data not valid for the operation were encountered.
        ///
        /// Unlike [`InvalidInput`], this typically means that the operation
        /// parameters were valid, however the error was caused by malformed
        /// input data.
        ///
        /// For example, a function that reads a file into a string will error with
        /// `InvalidData` if the file's contents are not valid UTF-8.
        ///
        /// [`InvalidInput`]: ErrorKind::InvalidInput
        InvalidData,
        /// The I/O operation's timeout expired, causing it to be canceled.
        TimedOut,
        /// An error returned when an operation could not be completed because a
        /// call to [`write`] returned [`Ok(0)`].
        ///
        /// This typically means that an operation could only succeed if it wrote a
        /// particular number of bytes but only a smaller number of bytes could be
        /// written.
        ///
        /// [`write`]: crate::io::Write::write
        /// [`Ok(0)`]: Ok
        WriteZero,
        /// The underlying storage (typically, a filesystem) is full.
        ///
        /// This does not include out of quota errors.
        StorageFull,
        /// Seek on unseekable file.
        ///
        /// Seeking was attempted on an open file handle which is not suitable for seeking - for
        /// example, on Unix, a named pipe opened with `File::open`.
        NotSeekable,
        /// Filesystem quota was exceeded.
        FilesystemQuotaExceeded,
        /// File larger than allowed or supported.
        ///
        /// This might arise from a hard limit of the underlying filesystem or file access API, or from
        /// an administratively imposed resource limitation.  Simple disk full, and out of quota, have
        /// their own errors.
        FileTooLarge,
        /// Resource is busy.
        ResourceBusy,
        /// Executable file is busy.
        ///
        /// An attempt was made to write to a file which is also in use as a running program.  (Not all
        /// operating systems detect this situation.)
        ExecutableFileBusy,
        /// Deadlock (avoided).
        ///
        /// A file locking operation would result in deadlock.  This situation is typically detected, if
        /// at all, on a best-effort basis.
        Deadlock,
        /// Cross-device or cross-filesystem (hard) link or rename.
        CrossesDevices,
        /// Too many (hard) links to the same filesystem object.
        ///
        /// The filesystem does not support making so many hardlinks to the same file.
        TooManyLinks,
        /// Filename too long.
        ///
        /// The limit might be from the underlying filesystem or API, or an administratively imposed
        /// resource limit.
        FilenameTooLong,
        /// Program argument list too long.
        ///
        /// When trying to run an external program, a system or process limit on the size of the
        /// arguments would have been exceeded.
        ArgumentListTooLong,
        /// This operation was interrupted.
        ///
        /// Interrupted operations can typically be retried.
        Interrupted,
        /// This operation is unsupported on this platform.
        ///
        /// This means that the operation can never succeed.
        Unsupported,
        /// An error returned when an operation could not be completed because an
        /// "end of file" was reached prematurely.
        ///
        /// This typically means that an operation could only succeed if it read a
        /// particular number of bytes but only a smaller number of bytes could be
        /// read.
        UnexpectedEof,
        /// An operation could not be completed, because it failed
        /// to allocate enough memory.
        OutOfMemory,
        /// A custom error that does not fall under any other I/O error kind.
        ///
        /// This can be used to construct your own [`Error`]s that do not match any
        /// [`ErrorKind`].
        ///
        /// This [`ErrorKind`] is not used by the standard library.
        ///
        /// Errors from the standard library that do not fall under any of the I/O
        /// error kinds cannot be `match`ed on, and will only match a wildcard (`_`) pattern.
        /// New [`ErrorKind`]s might be added in the future for some of those.
        Other,
        /// Any I/O error from the standard library that's not part of this list.
        ///
        /// Errors that are `Uncategorized` now may move to a different or a new
        /// [`ErrorKind`] variant in the future. It is not recommended to match
        /// an error against `Uncategorized`; use a wildcard match (`_`) instead.
        Uncategorized,
    }
    impl serde::Serialize for ErrorKindSer {
        #[allow(clippy::use_self)]
        fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let value: u8 = match *self {
                ErrorKindSer::NotFound => ErrorKindSer::NotFound as u8,
                ErrorKindSer::PermissionDenied => ErrorKindSer::PermissionDenied as u8,
                ErrorKindSer::ConnectionRefused => ErrorKindSer::ConnectionRefused as u8,
                ErrorKindSer::ConnectionReset => ErrorKindSer::ConnectionReset as u8,
                ErrorKindSer::HostUnreachable => ErrorKindSer::HostUnreachable as u8,
                ErrorKindSer::NetworkUnreachable => ErrorKindSer::NetworkUnreachable as u8,
                ErrorKindSer::ConnectionAborted => ErrorKindSer::ConnectionAborted as u8,
                ErrorKindSer::NotConnected => ErrorKindSer::NotConnected as u8,
                ErrorKindSer::AddrInUse => ErrorKindSer::AddrInUse as u8,
                ErrorKindSer::AddrNotAvailable => ErrorKindSer::AddrNotAvailable as u8,
                ErrorKindSer::NetworkDown => ErrorKindSer::NetworkDown as u8,
                ErrorKindSer::BrokenPipe => ErrorKindSer::BrokenPipe as u8,
                ErrorKindSer::AlreadyExists => ErrorKindSer::AlreadyExists as u8,
                ErrorKindSer::WouldBlock => ErrorKindSer::WouldBlock as u8,
                ErrorKindSer::NotADirectory => ErrorKindSer::NotADirectory as u8,
                ErrorKindSer::IsADirectory => ErrorKindSer::IsADirectory as u8,
                ErrorKindSer::DirectoryNotEmpty => ErrorKindSer::DirectoryNotEmpty as u8,
                ErrorKindSer::ReadOnlyFilesystem => ErrorKindSer::ReadOnlyFilesystem as u8,
                ErrorKindSer::FilesystemLoop => ErrorKindSer::FilesystemLoop as u8,
                ErrorKindSer::StaleNetworkFileHandle => ErrorKindSer::StaleNetworkFileHandle as u8,
                ErrorKindSer::InvalidInput => ErrorKindSer::InvalidInput as u8,
                ErrorKindSer::InvalidData => ErrorKindSer::InvalidData as u8,
                ErrorKindSer::TimedOut => ErrorKindSer::TimedOut as u8,
                ErrorKindSer::WriteZero => ErrorKindSer::WriteZero as u8,
                ErrorKindSer::StorageFull => ErrorKindSer::StorageFull as u8,
                ErrorKindSer::NotSeekable => ErrorKindSer::NotSeekable as u8,
                ErrorKindSer::FilesystemQuotaExceeded => {
                    ErrorKindSer::FilesystemQuotaExceeded as u8
                }
                ErrorKindSer::FileTooLarge => ErrorKindSer::FileTooLarge as u8,
                ErrorKindSer::ResourceBusy => ErrorKindSer::ResourceBusy as u8,
                ErrorKindSer::ExecutableFileBusy => ErrorKindSer::ExecutableFileBusy as u8,
                ErrorKindSer::Deadlock => ErrorKindSer::Deadlock as u8,
                ErrorKindSer::CrossesDevices => ErrorKindSer::CrossesDevices as u8,
                ErrorKindSer::TooManyLinks => ErrorKindSer::TooManyLinks as u8,
                ErrorKindSer::FilenameTooLong => ErrorKindSer::FilenameTooLong as u8,
                ErrorKindSer::ArgumentListTooLong => ErrorKindSer::ArgumentListTooLong as u8,
                ErrorKindSer::Interrupted => ErrorKindSer::Interrupted as u8,
                ErrorKindSer::Unsupported => ErrorKindSer::Unsupported as u8,
                ErrorKindSer::UnexpectedEof => ErrorKindSer::UnexpectedEof as u8,
                ErrorKindSer::OutOfMemory => ErrorKindSer::OutOfMemory as u8,
                ErrorKindSer::Other => ErrorKindSer::Other as u8,
                ErrorKindSer::Uncategorized => ErrorKindSer::Uncategorized as u8,
            };
            serde::Serialize::serialize(&value, serializer)
        }
    }
    impl<'de> serde::Deserialize<'de> for ErrorKindSer {
        #[allow(clippy::use_self)]
        fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct discriminant;
            #[allow(non_upper_case_globals)]
            impl discriminant {
                const NotFound: u8 = ErrorKindSer::NotFound as u8;
                const PermissionDenied: u8 = ErrorKindSer::PermissionDenied as u8;
                const ConnectionRefused: u8 = ErrorKindSer::ConnectionRefused as u8;
                const ConnectionReset: u8 = ErrorKindSer::ConnectionReset as u8;
                const HostUnreachable: u8 = ErrorKindSer::HostUnreachable as u8;
                const NetworkUnreachable: u8 = ErrorKindSer::NetworkUnreachable as u8;
                const ConnectionAborted: u8 = ErrorKindSer::ConnectionAborted as u8;
                const NotConnected: u8 = ErrorKindSer::NotConnected as u8;
                const AddrInUse: u8 = ErrorKindSer::AddrInUse as u8;
                const AddrNotAvailable: u8 = ErrorKindSer::AddrNotAvailable as u8;
                const NetworkDown: u8 = ErrorKindSer::NetworkDown as u8;
                const BrokenPipe: u8 = ErrorKindSer::BrokenPipe as u8;
                const AlreadyExists: u8 = ErrorKindSer::AlreadyExists as u8;
                const WouldBlock: u8 = ErrorKindSer::WouldBlock as u8;
                const NotADirectory: u8 = ErrorKindSer::NotADirectory as u8;
                const IsADirectory: u8 = ErrorKindSer::IsADirectory as u8;
                const DirectoryNotEmpty: u8 = ErrorKindSer::DirectoryNotEmpty as u8;
                const ReadOnlyFilesystem: u8 = ErrorKindSer::ReadOnlyFilesystem as u8;
                const FilesystemLoop: u8 = ErrorKindSer::FilesystemLoop as u8;
                const StaleNetworkFileHandle: u8 = ErrorKindSer::StaleNetworkFileHandle as u8;
                const InvalidInput: u8 = ErrorKindSer::InvalidInput as u8;
                const InvalidData: u8 = ErrorKindSer::InvalidData as u8;
                const TimedOut: u8 = ErrorKindSer::TimedOut as u8;
                const WriteZero: u8 = ErrorKindSer::WriteZero as u8;
                const StorageFull: u8 = ErrorKindSer::StorageFull as u8;
                const NotSeekable: u8 = ErrorKindSer::NotSeekable as u8;
                const FilesystemQuotaExceeded: u8 = ErrorKindSer::FilesystemQuotaExceeded as u8;
                const FileTooLarge: u8 = ErrorKindSer::FileTooLarge as u8;
                const ResourceBusy: u8 = ErrorKindSer::ResourceBusy as u8;
                const ExecutableFileBusy: u8 = ErrorKindSer::ExecutableFileBusy as u8;
                const Deadlock: u8 = ErrorKindSer::Deadlock as u8;
                const CrossesDevices: u8 = ErrorKindSer::CrossesDevices as u8;
                const TooManyLinks: u8 = ErrorKindSer::TooManyLinks as u8;
                const FilenameTooLong: u8 = ErrorKindSer::FilenameTooLong as u8;
                const ArgumentListTooLong: u8 = ErrorKindSer::ArgumentListTooLong as u8;
                const Interrupted: u8 = ErrorKindSer::Interrupted as u8;
                const Unsupported: u8 = ErrorKindSer::Unsupported as u8;
                const UnexpectedEof: u8 = ErrorKindSer::UnexpectedEof as u8;
                const OutOfMemory: u8 = ErrorKindSer::OutOfMemory as u8;
                const Other: u8 = ErrorKindSer::Other as u8;
                const Uncategorized: u8 = ErrorKindSer::Uncategorized as u8;
            }
            match <u8 as serde::Deserialize>::deserialize(deserializer)? {
                discriminant::NotFound => core::result::Result::Ok(ErrorKindSer::NotFound),
                discriminant::PermissionDenied => {
                    core::result::Result::Ok(ErrorKindSer::PermissionDenied)
                }
                discriminant::ConnectionRefused => {
                    core::result::Result::Ok(ErrorKindSer::ConnectionRefused)
                }
                discriminant::ConnectionReset => {
                    core::result::Result::Ok(ErrorKindSer::ConnectionReset)
                }
                discriminant::HostUnreachable => {
                    core::result::Result::Ok(ErrorKindSer::HostUnreachable)
                }
                discriminant::NetworkUnreachable => {
                    core::result::Result::Ok(ErrorKindSer::NetworkUnreachable)
                }
                discriminant::ConnectionAborted => {
                    core::result::Result::Ok(ErrorKindSer::ConnectionAborted)
                }
                discriminant::NotConnected => core::result::Result::Ok(ErrorKindSer::NotConnected),
                discriminant::AddrInUse => core::result::Result::Ok(ErrorKindSer::AddrInUse),
                discriminant::AddrNotAvailable => {
                    core::result::Result::Ok(ErrorKindSer::AddrNotAvailable)
                }
                discriminant::NetworkDown => core::result::Result::Ok(ErrorKindSer::NetworkDown),
                discriminant::BrokenPipe => core::result::Result::Ok(ErrorKindSer::BrokenPipe),
                discriminant::AlreadyExists => {
                    core::result::Result::Ok(ErrorKindSer::AlreadyExists)
                }
                discriminant::WouldBlock => core::result::Result::Ok(ErrorKindSer::WouldBlock),
                discriminant::NotADirectory => {
                    core::result::Result::Ok(ErrorKindSer::NotADirectory)
                }
                discriminant::IsADirectory => core::result::Result::Ok(ErrorKindSer::IsADirectory),
                discriminant::DirectoryNotEmpty => {
                    core::result::Result::Ok(ErrorKindSer::DirectoryNotEmpty)
                }
                discriminant::ReadOnlyFilesystem => {
                    core::result::Result::Ok(ErrorKindSer::ReadOnlyFilesystem)
                }
                discriminant::FilesystemLoop => {
                    core::result::Result::Ok(ErrorKindSer::FilesystemLoop)
                }
                discriminant::StaleNetworkFileHandle => {
                    core::result::Result::Ok(ErrorKindSer::StaleNetworkFileHandle)
                }
                discriminant::InvalidInput => core::result::Result::Ok(ErrorKindSer::InvalidInput),
                discriminant::InvalidData => core::result::Result::Ok(ErrorKindSer::InvalidData),
                discriminant::TimedOut => core::result::Result::Ok(ErrorKindSer::TimedOut),
                discriminant::WriteZero => core::result::Result::Ok(ErrorKindSer::WriteZero),
                discriminant::StorageFull => core::result::Result::Ok(ErrorKindSer::StorageFull),
                discriminant::NotSeekable => core::result::Result::Ok(ErrorKindSer::NotSeekable),
                discriminant::FilesystemQuotaExceeded => {
                    core::result::Result::Ok(ErrorKindSer::FilesystemQuotaExceeded)
                }
                discriminant::FileTooLarge => core::result::Result::Ok(ErrorKindSer::FileTooLarge),
                discriminant::ResourceBusy => core::result::Result::Ok(ErrorKindSer::ResourceBusy),
                discriminant::ExecutableFileBusy => {
                    core::result::Result::Ok(ErrorKindSer::ExecutableFileBusy)
                }
                discriminant::Deadlock => core::result::Result::Ok(ErrorKindSer::Deadlock),
                discriminant::CrossesDevices => {
                    core::result::Result::Ok(ErrorKindSer::CrossesDevices)
                }
                discriminant::TooManyLinks => core::result::Result::Ok(ErrorKindSer::TooManyLinks),
                discriminant::FilenameTooLong => {
                    core::result::Result::Ok(ErrorKindSer::FilenameTooLong)
                }
                discriminant::ArgumentListTooLong => {
                    core::result::Result::Ok(ErrorKindSer::ArgumentListTooLong)
                }
                discriminant::Interrupted => core::result::Result::Ok(ErrorKindSer::Interrupted),
                discriminant::Unsupported => core::result::Result::Ok(ErrorKindSer::Unsupported),
                discriminant::UnexpectedEof => {
                    core::result::Result::Ok(ErrorKindSer::UnexpectedEof)
                }
                discriminant::OutOfMemory => core::result::Result::Ok(ErrorKindSer::OutOfMemory),
                discriminant::Other => core::result::Result::Ok(ErrorKindSer::Other),
                discriminant::Uncategorized => {
                    core::result::Result::Ok(ErrorKindSer::Uncategorized)
                }
                other => core::result::Result::Err(serde::de::Error::custom(
                    ::core::fmt::Arguments::new_v1(
                        &[
                            "invalid value: ",
                            ", expected one of: ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                            ", ",
                        ],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&other),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NotFound),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::PermissionDenied),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ConnectionRefused),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ConnectionReset),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::HostUnreachable),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NetworkUnreachable),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ConnectionAborted),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NotConnected),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::AddrInUse),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::AddrNotAvailable),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NetworkDown),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::BrokenPipe),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::AlreadyExists),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::WouldBlock),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NotADirectory),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::IsADirectory),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::DirectoryNotEmpty),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ReadOnlyFilesystem),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::FilesystemLoop),
                            ::core::fmt::ArgumentV1::new_display(
                                &discriminant::StaleNetworkFileHandle,
                            ),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::InvalidInput),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::InvalidData),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::TimedOut),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::WriteZero),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::StorageFull),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::NotSeekable),
                            ::core::fmt::ArgumentV1::new_display(
                                &discriminant::FilesystemQuotaExceeded,
                            ),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::FileTooLarge),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ResourceBusy),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::ExecutableFileBusy),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::Deadlock),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::CrossesDevices),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::TooManyLinks),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::FilenameTooLong),
                            ::core::fmt::ArgumentV1::new_display(
                                &discriminant::ArgumentListTooLong,
                            ),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::Interrupted),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::Unsupported),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::UnexpectedEof),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::OutOfMemory),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::Other),
                            ::core::fmt::ArgumentV1::new_display(&discriminant::Uncategorized),
                        ],
                    ),
                )),
            }
        }
    }
    impl From<ErrorKindSer> for ErrorKind {
        fn from(kind: ErrorKindSer) -> Self {
            match kind {
                ErrorKindSer::NotFound => ErrorKind::NotFound,
                ErrorKindSer::PermissionDenied => ErrorKind::PermissionDenied,
                ErrorKindSer::ConnectionRefused => ErrorKind::ConnectionRefused,
                ErrorKindSer::ConnectionReset => ErrorKind::ConnectionReset,
                ErrorKindSer::ConnectionAborted => ErrorKind::ConnectionAborted,
                ErrorKindSer::NotConnected => ErrorKind::NotConnected,
                ErrorKindSer::AddrInUse => ErrorKind::AddrInUse,
                ErrorKindSer::AddrNotAvailable => ErrorKind::AddrNotAvailable,
                ErrorKindSer::BrokenPipe => ErrorKind::BrokenPipe,
                ErrorKindSer::AlreadyExists => ErrorKind::AlreadyExists,
                ErrorKindSer::WouldBlock => ErrorKind::WouldBlock,
                ErrorKindSer::InvalidInput => ErrorKind::InvalidInput,
                ErrorKindSer::InvalidData => ErrorKind::InvalidData,
                ErrorKindSer::TimedOut => ErrorKind::TimedOut,
                ErrorKindSer::WriteZero => ErrorKind::WriteZero,
                ErrorKindSer::Interrupted => ErrorKind::Interrupted,
                ErrorKindSer::Unsupported => ErrorKind::Unsupported,
                ErrorKindSer::UnexpectedEof => ErrorKind::UnexpectedEof,
                ErrorKindSer::OutOfMemory => ErrorKind::OutOfMemory,
                ErrorKindSer::Other => ErrorKind::Other,
                _ => ErrorKind::Other,
            }
        }
    }
    impl From<ErrorKind> for ErrorKindSer {
        fn from(kind: ErrorKind) -> Self {
            match kind {
                ErrorKind::NotFound => ErrorKindSer::NotFound,
                ErrorKind::PermissionDenied => ErrorKindSer::PermissionDenied,
                ErrorKind::ConnectionRefused => ErrorKindSer::ConnectionRefused,
                ErrorKind::ConnectionReset => ErrorKindSer::ConnectionReset,
                ErrorKind::ConnectionAborted => ErrorKindSer::ConnectionAborted,
                ErrorKind::NotConnected => ErrorKindSer::NotConnected,
                ErrorKind::AddrInUse => ErrorKindSer::AddrInUse,
                ErrorKind::AddrNotAvailable => ErrorKindSer::AddrNotAvailable,
                ErrorKind::BrokenPipe => ErrorKindSer::BrokenPipe,
                ErrorKind::AlreadyExists => ErrorKindSer::AlreadyExists,
                ErrorKind::WouldBlock => ErrorKindSer::WouldBlock,
                ErrorKind::InvalidInput => ErrorKindSer::InvalidInput,
                ErrorKind::InvalidData => ErrorKindSer::InvalidData,
                ErrorKind::TimedOut => ErrorKindSer::TimedOut,
                ErrorKind::WriteZero => ErrorKindSer::WriteZero,
                ErrorKind::Interrupted => ErrorKindSer::Interrupted,
                ErrorKind::Unsupported => ErrorKindSer::Unsupported,
                ErrorKind::UnexpectedEof => ErrorKindSer::UnexpectedEof,
                ErrorKind::OutOfMemory => ErrorKindSer::OutOfMemory,
                ErrorKind::Other => ErrorKindSer::Other,
                _ => ErrorKindSer::Other,
            }
        }
    }
}
mod io {
    use cfg_if::cfg_if;
    #[cfg(unix)]
    pub use tokio::net::{UnixListener, UnixStream};
    pub use tokio::net::{TcpListener, TcpStream};
    pub use tokio::io::AsyncRead as Read;
    pub use tokio::io::AsyncReadExt as ReadExt;
    pub use tokio::io::AsyncWrite as Write;
    pub use tokio::io::AsyncWriteExt as WriteExt;
    pub use tokio::net::ToSocketAddrs;
    pub(crate) use tokio::time::sleep;
    pub use async_tungstenite as wss;
}
/// contains common imports
pub mod prelude {
    pub use crate::err;
    pub use crate::Channel;
    pub use crate::Result;
}
/// contains providers and address
pub mod providers {
    mod addr {
        use crate::{err, Error};
        use crate::{Channel, Result};
        use cfg_if::cfg_if;
        use compact_str::CompactStr;
        use serde::ser::SerializeSeq;
        use serde::{Deserialize, Serialize};
        use std::fmt::Debug;
        use std::fmt::Display;
        use std::net::SocketAddr;
        use std::path::PathBuf;
        use std::str::FromStr;
        use std::sync::Arc;
        use crate::providers::Wss;
        use crate::providers::Tcp;
        #[cfg(unix)]
        use crate::providers::Unix;
        /// Represents the address of a provider.
        /// ```norun
        /// let tcp = "tcp@127.0.0.1:8080".parse::<Addr>()?;
        /// let unix = "unix@mysocket.sock".parse::<Addr>()?;
        /// let insecure_tcp = "itcp@127.0.0.1:8080".parse::<Addr>()?;
        /// let insecure_unix = "iunix@mysocket.sock".parse::<Addr>()?;
        ///
        /// tcp.bind().await?; // bind all addresses to the global route
        /// unix.bind().await?;
        /// insecure_tcp.bind().await?;
        /// insecure_unix.bind().await?;
        /// ```
        pub enum Addr {
            /// Tcp provider
            Tcp(Arc<SocketAddr>),
            /// Unix provider
            Unix(Arc<PathBuf>),
            /// Unencrypted tcp provider
            InsecureTcp(Arc<SocketAddr>),
            /// Unencrypted unix provider
            InsecureUnix(Arc<PathBuf>),
            /// Websocket provider
            Wss(Arc<CompactStr>),
            /// Unencrypted websocket provider
            InsecureWss(Arc<CompactStr>),
        }
        impl ::core::marker::StructuralPartialEq for Addr {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Addr {
            #[inline]
            fn eq(&self, other: &Addr) -> bool {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (&Addr::Tcp(ref __self_0), &Addr::Tcp(ref __arg_1_0)) => {
                                (*__self_0) == (*__arg_1_0)
                            }
                            (&Addr::Unix(ref __self_0), &Addr::Unix(ref __arg_1_0)) => {
                                (*__self_0) == (*__arg_1_0)
                            }
                            (
                                &Addr::InsecureTcp(ref __self_0),
                                &Addr::InsecureTcp(ref __arg_1_0),
                            ) => (*__self_0) == (*__arg_1_0),
                            (
                                &Addr::InsecureUnix(ref __self_0),
                                &Addr::InsecureUnix(ref __arg_1_0),
                            ) => (*__self_0) == (*__arg_1_0),
                            (&Addr::Wss(ref __self_0), &Addr::Wss(ref __arg_1_0)) => {
                                (*__self_0) == (*__arg_1_0)
                            }
                            (
                                &Addr::InsecureWss(ref __self_0),
                                &Addr::InsecureWss(ref __arg_1_0),
                            ) => (*__self_0) == (*__arg_1_0),
                            _ => unsafe { ::core::intrinsics::unreachable() },
                        }
                    } else {
                        false
                    }
                }
            }
            #[inline]
            fn ne(&self, other: &Addr) -> bool {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (&Addr::Tcp(ref __self_0), &Addr::Tcp(ref __arg_1_0)) => {
                                (*__self_0) != (*__arg_1_0)
                            }
                            (&Addr::Unix(ref __self_0), &Addr::Unix(ref __arg_1_0)) => {
                                (*__self_0) != (*__arg_1_0)
                            }
                            (
                                &Addr::InsecureTcp(ref __self_0),
                                &Addr::InsecureTcp(ref __arg_1_0),
                            ) => (*__self_0) != (*__arg_1_0),
                            (
                                &Addr::InsecureUnix(ref __self_0),
                                &Addr::InsecureUnix(ref __arg_1_0),
                            ) => (*__self_0) != (*__arg_1_0),
                            (&Addr::Wss(ref __self_0), &Addr::Wss(ref __arg_1_0)) => {
                                (*__self_0) != (*__arg_1_0)
                            }
                            (
                                &Addr::InsecureWss(ref __self_0),
                                &Addr::InsecureWss(ref __arg_1_0),
                            ) => (*__self_0) != (*__arg_1_0),
                            _ => unsafe { ::core::intrinsics::unreachable() },
                        }
                    } else {
                        true
                    }
                }
            }
        }
        impl ::core::marker::StructuralEq for Addr {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Addr {
            #[inline]
            #[doc(hidden)]
            #[no_coverage]
            fn assert_receiver_is_total_eq(&self) -> () {
                {
                    let _: ::core::cmp::AssertParamIsEq<Arc<SocketAddr>>;
                    let _: ::core::cmp::AssertParamIsEq<Arc<PathBuf>>;
                    let _: ::core::cmp::AssertParamIsEq<Arc<SocketAddr>>;
                    let _: ::core::cmp::AssertParamIsEq<Arc<PathBuf>>;
                    let _: ::core::cmp::AssertParamIsEq<Arc<CompactStr>>;
                    let _: ::core::cmp::AssertParamIsEq<Arc<CompactStr>>;
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::hash::Hash for Addr {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                match (&*self,) {
                    (&Addr::Tcp(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                    (&Addr::Unix(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                    (&Addr::InsecureTcp(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                    (&Addr::InsecureUnix(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                    (&Addr::Wss(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                    (&Addr::InsecureWss(ref __self_0),) => {
                        ::core::hash::Hash::hash(
                            &::core::intrinsics::discriminant_value(self),
                            state,
                        );
                        ::core::hash::Hash::hash(&(*__self_0), state)
                    }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialOrd for Addr {
            #[inline]
            fn partial_cmp(&self, other: &Addr) -> ::core::option::Option<::core::cmp::Ordering> {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (&Addr::Tcp(ref __self_0), &Addr::Tcp(ref __arg_1_0)) => {
                                match ::core::cmp::PartialOrd::partial_cmp(
                                    &(*__self_0),
                                    &(*__arg_1_0),
                                ) {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                    }
                                    cmp => cmp,
                                }
                            }
                            (&Addr::Unix(ref __self_0), &Addr::Unix(ref __arg_1_0)) => {
                                match ::core::cmp::PartialOrd::partial_cmp(
                                    &(*__self_0),
                                    &(*__arg_1_0),
                                ) {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                    }
                                    cmp => cmp,
                                }
                            }
                            (
                                &Addr::InsecureTcp(ref __self_0),
                                &Addr::InsecureTcp(ref __arg_1_0),
                            ) => match ::core::cmp::PartialOrd::partial_cmp(
                                &(*__self_0),
                                &(*__arg_1_0),
                            ) {
                                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                }
                                cmp => cmp,
                            },
                            (
                                &Addr::InsecureUnix(ref __self_0),
                                &Addr::InsecureUnix(ref __arg_1_0),
                            ) => match ::core::cmp::PartialOrd::partial_cmp(
                                &(*__self_0),
                                &(*__arg_1_0),
                            ) {
                                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                }
                                cmp => cmp,
                            },
                            (&Addr::Wss(ref __self_0), &Addr::Wss(ref __arg_1_0)) => {
                                match ::core::cmp::PartialOrd::partial_cmp(
                                    &(*__self_0),
                                    &(*__arg_1_0),
                                ) {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                    }
                                    cmp => cmp,
                                }
                            }
                            (
                                &Addr::InsecureWss(ref __self_0),
                                &Addr::InsecureWss(ref __arg_1_0),
                            ) => match ::core::cmp::PartialOrd::partial_cmp(
                                &(*__self_0),
                                &(*__arg_1_0),
                            ) {
                                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                                    ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                                }
                                cmp => cmp,
                            },
                            _ => unsafe { ::core::intrinsics::unreachable() },
                        }
                    } else {
                        ::core::cmp::PartialOrd::partial_cmp(&__self_vi, &__arg_1_vi)
                    }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Ord for Addr {
            #[inline]
            fn cmp(&self, other: &Addr) -> ::core::cmp::Ordering {
                {
                    let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) {
                            (&Addr::Tcp(ref __self_0), &Addr::Tcp(ref __arg_1_0)) => {
                                match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                    ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                    cmp => cmp,
                                }
                            }
                            (&Addr::Unix(ref __self_0), &Addr::Unix(ref __arg_1_0)) => {
                                match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                    ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                    cmp => cmp,
                                }
                            }
                            (
                                &Addr::InsecureTcp(ref __self_0),
                                &Addr::InsecureTcp(ref __arg_1_0),
                            ) => match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                cmp => cmp,
                            },
                            (
                                &Addr::InsecureUnix(ref __self_0),
                                &Addr::InsecureUnix(ref __arg_1_0),
                            ) => match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                cmp => cmp,
                            },
                            (&Addr::Wss(ref __self_0), &Addr::Wss(ref __arg_1_0)) => {
                                match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                    ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                    cmp => cmp,
                                }
                            }
                            (
                                &Addr::InsecureWss(ref __self_0),
                                &Addr::InsecureWss(ref __arg_1_0),
                            ) => match ::core::cmp::Ord::cmp(&(*__self_0), &(*__arg_1_0)) {
                                ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                                cmp => cmp,
                            },
                            _ => unsafe { ::core::intrinsics::unreachable() },
                        }
                    } else {
                        ::core::cmp::Ord::cmp(&__self_vi, &__arg_1_vi)
                    }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Addr {
            #[inline]
            fn clone(&self) -> Addr {
                match (&*self,) {
                    (&Addr::Tcp(ref __self_0),) => {
                        Addr::Tcp(::core::clone::Clone::clone(&(*__self_0)))
                    }
                    (&Addr::Unix(ref __self_0),) => {
                        Addr::Unix(::core::clone::Clone::clone(&(*__self_0)))
                    }
                    (&Addr::InsecureTcp(ref __self_0),) => {
                        Addr::InsecureTcp(::core::clone::Clone::clone(&(*__self_0)))
                    }
                    (&Addr::InsecureUnix(ref __self_0),) => {
                        Addr::InsecureUnix(::core::clone::Clone::clone(&(*__self_0)))
                    }
                    (&Addr::Wss(ref __self_0),) => {
                        Addr::Wss(::core::clone::Clone::clone(&(*__self_0)))
                    }
                    (&Addr::InsecureWss(ref __self_0),) => {
                        Addr::InsecureWss(::core::clone::Clone::clone(&(*__self_0)))
                    }
                }
            }
        }
        impl Into<String> for &Addr {
            #[inline]
            fn into(self) -> String {
                {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &[""],
                        &[::core::fmt::ArgumentV1::new_display(&self)],
                    ));
                    res
                }
            }
        }
        impl Display for Addr {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Addr::Tcp(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["tcp@"],
                        &[::core::fmt::ArgumentV1::new_display(&addr)],
                    )),
                    Addr::Unix(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["unix@"],
                        &[::core::fmt::ArgumentV1::new_display(
                            &addr.to_string_lossy(),
                        )],
                    )),
                    Addr::InsecureTcp(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["itcp@"],
                        &[::core::fmt::ArgumentV1::new_display(&addr)],
                    )),
                    Addr::InsecureUnix(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["iunix@"],
                        &[::core::fmt::ArgumentV1::new_display(
                            &addr.to_string_lossy(),
                        )],
                    )),
                    Addr::Wss(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["wss@"],
                        &[::core::fmt::ArgumentV1::new_display(&addr)],
                    )),
                    Addr::InsecureWss(addr) => f.write_fmt(::core::fmt::Arguments::new_v1(
                        &["ws@"],
                        &[::core::fmt::ArgumentV1::new_display(&addr)],
                    )),
                }
            }
        }
        impl Debug for Addr {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self, f)
            }
        }
        impl Serialize for Addr {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    Addr::Tcp(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("tcp@")?;
                        seq.serialize_element(&addr.to_string())?;
                        seq.end()
                    }
                    Addr::Unix(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("unix@")?;
                        seq.serialize_element(&addr.to_string_lossy())?;
                        seq.end()
                    }
                    Addr::InsecureTcp(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("itcp@")?;
                        seq.serialize_element(&addr.to_string())?;
                        seq.end()
                    }
                    Addr::InsecureUnix(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("iunix@")?;
                        seq.serialize_element(&addr.to_string_lossy())?;
                        seq.end()
                    }
                    Addr::Wss(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("wss@")?;
                        seq.serialize_element(addr.as_str())?;
                        seq.end()
                    }
                    Addr::InsecureWss(addr) => {
                        let mut seq = serializer.serialize_seq(Some(2))?;
                        seq.serialize_element("ws@")?;
                        seq.serialize_element(addr.as_str())?;
                        seq.end()
                    }
                }
            }
        }
        impl<'de> Deserialize<'de> for Addr {
            #[inline]
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let string: CompactStr = CompactStr::deserialize(deserializer)?;
                Addr::from_str(&string).map_err(serde::de::Error::custom)
            }
        }
        impl Addr {
            #[inline]
            /// create a new address from a string
            pub fn new(addr: &str) -> Result<Self> {
                addr.parse()
            }
            #[inline]
            /// connect to the address
            pub async fn connect(&self) -> Result<Channel> {
                match self {
                    Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref()).await?.encrypted().await,
                    Addr::InsecureTcp(addrs) => Ok(Tcp::connect(addrs.as_ref()).await?.raw()),
                    Addr::Unix(addrs) => Unix::connect(addrs.as_ref()).await?.encrypted().await,
                    Addr::InsecureUnix(addrs) => Ok(Unix::connect(addrs.as_ref()).await?.raw()),
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str()).await?.encrypted().await,
                    Addr::InsecureWss(addrs) => Ok(Wss::connect(addrs.as_str()).await?.raw()),
                }
            }
        }
        impl FromStr for Addr {
            type Err = Error;
            #[inline]
            /// unix@address.sock
            /// tcp@127.0.0.1:8092
            /// tcp@127.0.0.1:8092
            /// unix@folder/address.sock
            fn from_str(addr: &str) -> Result<Self> {
                let (protocol, addr) =
                    addr.rsplit_once("@")
                        .ok_or(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "malformed address",
                        )))?;
                let address_ty = protocol.parse::<AddressType>()?;
                Ok(match address_ty {
                    AddressType::Tcp => {
                        let addr = addr.parse::<SocketAddr>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::Tcp(Arc::new(addr))
                    }
                    AddressType::Unix => {
                        let addr = addr.parse::<PathBuf>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::Unix(Arc::new(addr))
                    }
                    AddressType::InsecureTcp => {
                        let addr = addr.parse::<SocketAddr>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::InsecureTcp(Arc::new(addr))
                    }
                    AddressType::InsecureUnix => {
                        let addr = addr.parse::<PathBuf>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::InsecureUnix(Arc::new(addr))
                    }
                    AddressType::Wss => {
                        let addr = addr.parse::<CompactStr>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::Wss(Arc::new(addr))
                    }
                    AddressType::InsecureWss => {
                        let addr = addr.parse::<CompactStr>().map_err(|e| {
                            crate::err::Error::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                e,
                            ))
                        })?;
                        Addr::InsecureWss(Arc::new(addr))
                    }
                })
            }
        }
        enum AddressType {
            #[serde(rename = "tcp")]
            Tcp,
            #[serde(rename = "itcp")]
            InsecureTcp,
            #[serde(rename = "unix")]
            Unix,
            #[serde(rename = "iunix")]
            InsecureUnix,
            #[serde(rename = "wss")]
            Wss,
            #[serde(rename = "ws")]
            InsecureWss,
        }
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for AddressType {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    match *self {
                        AddressType::Tcp => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            0u32,
                            "tcp",
                        ),
                        AddressType::InsecureTcp => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            1u32,
                            "itcp",
                        ),
                        AddressType::Unix => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            2u32,
                            "unix",
                        ),
                        AddressType::InsecureUnix => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            3u32,
                            "iunix",
                        ),
                        AddressType::Wss => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            4u32,
                            "wss",
                        ),
                        AddressType::InsecureWss => _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "AddressType",
                            5u32,
                            "ws",
                        ),
                    }
                }
            }
        };
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for AddressType {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    enum __Field {
                        __field0,
                        __field1,
                        __field2,
                        __field3,
                        __field4,
                        __field5,
                    }
                    struct __FieldVisitor;
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "variant identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                1u64 => _serde::__private::Ok(__Field::__field1),
                                2u64 => _serde::__private::Ok(__Field::__field2),
                                3u64 => _serde::__private::Ok(__Field::__field3),
                                4u64 => _serde::__private::Ok(__Field::__field4),
                                5u64 => _serde::__private::Ok(__Field::__field5),
                                _ => _serde::__private::Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"variant index 0 <= i < 6",
                                )),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "tcp" => _serde::__private::Ok(__Field::__field0),
                                "itcp" => _serde::__private::Ok(__Field::__field1),
                                "unix" => _serde::__private::Ok(__Field::__field2),
                                "iunix" => _serde::__private::Ok(__Field::__field3),
                                "wss" => _serde::__private::Ok(__Field::__field4),
                                "ws" => _serde::__private::Ok(__Field::__field5),
                                _ => _serde::__private::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                )),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"tcp" => _serde::__private::Ok(__Field::__field0),
                                b"itcp" => _serde::__private::Ok(__Field::__field1),
                                b"unix" => _serde::__private::Ok(__Field::__field2),
                                b"iunix" => _serde::__private::Ok(__Field::__field3),
                                b"wss" => _serde::__private::Ok(__Field::__field4),
                                b"ws" => _serde::__private::Ok(__Field::__field5),
                                _ => {
                                    let __value = &_serde::__private::from_utf8_lossy(__value);
                                    _serde::__private::Err(_serde::de::Error::unknown_variant(
                                        __value, VARIANTS,
                                    ))
                                }
                            }
                        }
                    }
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<AddressType>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = AddressType;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(__formatter, "enum AddressType")
                        }
                        fn visit_enum<__A>(
                            self,
                            __data: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::EnumAccess<'de>,
                        {
                            match match _serde::de::EnumAccess::variant(__data) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            } {
                                (__Field::__field0, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::Tcp)
                                }
                                (__Field::__field1, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::InsecureTcp)
                                }
                                (__Field::__field2, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::Unix)
                                }
                                (__Field::__field3, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::InsecureUnix)
                                }
                                (__Field::__field4, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::Wss)
                                }
                                (__Field::__field5, __variant) => {
                                    match _serde::de::VariantAccess::unit_variant(__variant) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    };
                                    _serde::__private::Ok(AddressType::InsecureWss)
                                }
                            }
                        }
                    }
                    const VARIANTS: &'static [&'static str] =
                        &["tcp", "itcp", "unix", "iunix", "wss", "ws"];
                    _serde::Deserializer::deserialize_enum(
                        __deserializer,
                        "AddressType",
                        VARIANTS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<AddressType>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        impl FromStr for AddressType {
            type Err = Error;
            #[inline]
            fn from_str(protocol: &str) -> Result<Self> {
                let protocol = match protocol {
                    "tcp" => AddressType::Tcp,
                    "itcp" => AddressType::InsecureTcp,
                    "wss" => AddressType::Wss,
                    "ws" => AddressType::InsecureWss,
                    "unix" => AddressType::Unix,
                    "iunix" => AddressType::InsecureUnix,
                    protocol => Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        {
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["unexpected protocol "],
                                &[::core::fmt::ArgumentV1::new_debug(&protocol)],
                            ));
                            res
                        },
                    )))?,
                };
                Ok(protocol)
            }
        }
        impl AsRef<str> for AddressType {
            #[inline]
            fn as_ref(&self) -> &str {
                match self {
                    AddressType::Tcp => "tcp",
                    AddressType::InsecureTcp => "itcp",
                    AddressType::Unix => "unix",
                    AddressType::InsecureUnix => "iunix",
                    AddressType::Wss => "wss",
                    AddressType::InsecureWss => "ws",
                }
            }
        }
    }
    mod any {
        #[cfg(not(target_arch = "wasm32"))]
        use super::Tcp;
        #[cfg(unix)]
        use super::Unix;
        #[cfg(not(target_arch = "wasm32"))]
        use crate::{channel::Handshake, Result};
        use super::Wss;
        use derive_more::From;
        /// abstraction over any provider
        pub enum AnyProvider {
            #[cfg(not(target_arch = "wasm32"))]
            /// encapsulates the tcp provider
            Tcp(Tcp),
            #[cfg(unix)]
            /// encapsulates the unix provider
            Unix(Unix),
            /// encapsulates the websocket provider
            Wss(Wss),
        }
        #[automatically_derived]
        impl ::core::convert::From<(Wss)> for AnyProvider {
            #[inline]
            fn from(original: (Wss)) -> AnyProvider {
                AnyProvider::Wss(original)
            }
        }
        #[automatically_derived]
        impl ::core::convert::From<(Unix)> for AnyProvider {
            #[inline]
            fn from(original: (Unix)) -> AnyProvider {
                AnyProvider::Unix(original)
            }
        }
        #[automatically_derived]
        impl ::core::convert::From<(Tcp)> for AnyProvider {
            #[inline]
            fn from(original: (Tcp)) -> AnyProvider {
                AnyProvider::Tcp(original)
            }
        }
        impl AnyProvider {
            #[inline]
            #[cfg(not(target_arch = "wasm32"))]
            /// get the next channel
            /// ```norun
            /// while let Ok(chan) = provider.next().await {
            ///     let mut chan = chan.encrypted().await?;
            ///     chan.send("hello!").await?;
            /// }
            /// ```
            pub async fn next(&self) -> Result<Handshake> {
                match self {
                    AnyProvider::Tcp(provider) => provider.next().await,
                    #[cfg(unix)]
                    AnyProvider::Unix(provider) => provider.next().await,
                    AnyProvider::Wss(provider) => provider.next().await,
                }
            }
        }
    }
    mod tcp {
        #![cfg(not(target_arch = "wasm32"))]
        use crate::channel::Handshake;
        use crate::err;
        use crate::io::TcpListener;
        use crate::io::TcpStream;
        use crate::io::ToSocketAddrs;
        use crate::Channel;
        use crate::Result;
        use derive_more::{From, Into};
        #[into(owned, ref, ref_mut)]
        /// Exposes routes over TCP
        pub struct Tcp(TcpListener);
        #[automatically_derived]
        impl ::core::convert::From<(TcpListener)> for Tcp {
            #[inline]
            fn from(original: (TcpListener)) -> Tcp {
                Tcp(original)
            }
        }
        #[automatically_derived]
        impl ::core::convert::From<Tcp> for (TcpListener) {
            #[inline]
            fn from(original: Tcp) -> Self {
                (original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime Tcp>
            for (&'__deriveMoreLifetime TcpListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime Tcp) -> Self {
                (&original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime mut Tcp>
            for (&'__deriveMoreLifetime mut TcpListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime mut Tcp) -> Self {
                (&mut original.0)
            }
        }
        impl Tcp {
            #[inline]
            /// Bind the global route on the given address
            pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
                let listener = TcpListener::bind(addrs).await?;
                Ok(Tcp(listener))
            }
            #[inline]
            /// Get the next channel
            /// ```norun
            /// while let Ok(chan) = tcp.next().await {
            ///     let mut chan = chan.encrypted().await?;
            ///     chan.send("hello!").await?;
            /// }
            /// ```
            pub async fn next(&self) -> Result<Handshake> {
                let (chan, _) = self.0.accept().await?;
                let chan: Channel = Channel::from(chan);
                Ok(Handshake::from(chan))
            }
            #[inline]
            /// Connect to the following address without discovery
            pub async fn raw_connect_with_retries(
                addrs: impl ToSocketAddrs + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                let mut attempt = 0;
                let stream = loop {
                    match TcpStream::connect(&addrs).await {
                        Ok(s) => break s,
                        Err(e) => {
                            {
                                use ::tracing::__macro_support::Callsite as _;
                                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                                    use ::tracing::__macro_support::MacroCallsite;
                                    static META: ::tracing::Metadata<'static> = {
                                        ::tracing_core::metadata::Metadata::new(
                                            "event src/providers/tcp.rs:50",
                                            "canary::providers::tcp",
                                            ::tracing::Level::ERROR,
                                            Some("src/providers/tcp.rs"),
                                            Some(50u32),
                                            Some("canary::providers::tcp"),
                                            ::tracing_core::field::FieldSet::new(
                                                &["message"],
                                                ::tracing_core::callsite::Identifier(&CALLSITE),
                                            ),
                                            ::tracing::metadata::Kind::EVENT,
                                        )
                                    };
                                    MacroCallsite::new(&META)
                                };
                                let enabled = ::tracing::Level::ERROR
                                    <= ::tracing::level_filters::STATIC_MAX_LEVEL
                                    && ::tracing::Level::ERROR
                                        <= ::tracing::level_filters::LevelFilter::current()
                                    && {
                                        let interest = CALLSITE.interest();
                                        !interest.is_never() && CALLSITE.is_enabled(interest)
                                    };
                                if enabled {
                                    (|value_set: ::tracing::field::ValueSet| {
                                        let meta = CALLSITE.metadata();
                                        ::tracing::Event::dispatch(meta, &value_set);
                                    })({
                                        #[allow(unused_imports)]
                                        use ::tracing::field::{debug, display, Value};
                                        let mut iter = CALLSITE.metadata().fields().iter();
                                        CALLSITE.metadata().fields().value_set(&[(
                                            &iter
                                                .next()
                                                .expect("FieldSet corrupted (this is a bug)"),
                                            Some(&::core::fmt::Arguments::new_v1(
                                                &[
                                                    "connecting to address `",
                                                    "` failed, attempt ",
                                                    " starting",
                                                ],
                                                &[
                                                    ::core::fmt::ArgumentV1::new_debug(&addrs),
                                                    ::core::fmt::ArgumentV1::new_display(&attempt),
                                                ],
                                            )
                                                as &Value),
                                        )])
                                    });
                                } else {
                                }
                            };
                            crate::io::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                            attempt += 1;
                            if attempt == retries {
                                Err(crate::err::Error::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                )))?
                            }
                            continue;
                        }
                    }
                };
                let chan = Channel::from(stream);
                Ok(Handshake::from(chan))
            }
            #[inline]
            /// Connect to the following address with the following id. Defaults to 3 retries.
            pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
                Self::connect_retry(addrs, 3, 10).await
            }
            #[inline]
            /// Connect to the following address with the given id and retry in case of failure
            pub async fn connect_retry(
                addrs: impl ToSocketAddrs + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
            }
        }
    }
    mod unix {
        #![cfg(unix)]
        #![cfg(not(target_arch = "wasm32"))]
        use std::path::Path;
        use crate::channel::Handshake;
        use crate::err;
        use crate::io::UnixListener;
        use crate::io::UnixStream;
        use crate::Channel;
        use crate::Result;
        use derive_more::{From, Into};
        #[into(owned, ref, ref_mut)]
        /// Exposes routes over TCP
        pub struct Unix(UnixListener);
        #[automatically_derived]
        impl ::core::convert::From<(UnixListener)> for Unix {
            #[inline]
            fn from(original: (UnixListener)) -> Unix {
                Unix(original)
            }
        }
        #[automatically_derived]
        impl ::core::convert::From<Unix> for (UnixListener) {
            #[inline]
            fn from(original: Unix) -> Self {
                (original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime Unix>
            for (&'__deriveMoreLifetime UnixListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime Unix) -> Self {
                (&original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime mut Unix>
            for (&'__deriveMoreLifetime mut UnixListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime mut Unix) -> Self {
                (&mut original.0)
            }
        }
        impl Unix {
            #[inline]
            /// bind the global route on the given address
            pub async fn bind(addrs: impl AsRef<Path>) -> Result<Self> {
                let listener = UnixListener::bind(addrs)?;
                Ok(Unix(listener))
            }
            #[inline]
            /// get the next channel
            /// ```norun
            /// while let Ok(chan) = unix.next().await {
            ///     let mut chan = chan.encrypted().await?;
            ///     chan.send("hello!").await?;
            /// }
            /// ```
            pub async fn next(&self) -> Result<Handshake> {
                let (chan, _) = self.0.accept().await?;
                let chan: Channel = Channel::from(chan);
                Ok(Handshake::from(chan))
            }
            #[inline]
            /// connect to the following address without discovery
            pub async fn raw_connect_with_retries(
                addrs: impl AsRef<Path> + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                let mut attempt = 0;
                let stream = loop {
                    match UnixStream::connect(&addrs).await {
                        Ok(s) => break s,
                        Err(e) => {
                            {
                                use ::tracing::__macro_support::Callsite as _;
                                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                                    use ::tracing::__macro_support::MacroCallsite;
                                    static META: ::tracing::Metadata<'static> = {
                                        ::tracing_core::metadata::Metadata::new(
                                            "event src/providers/unix.rs:52",
                                            "canary::providers::unix",
                                            ::tracing::Level::ERROR,
                                            Some("src/providers/unix.rs"),
                                            Some(52u32),
                                            Some("canary::providers::unix"),
                                            ::tracing_core::field::FieldSet::new(
                                                &["message"],
                                                ::tracing_core::callsite::Identifier(&CALLSITE),
                                            ),
                                            ::tracing::metadata::Kind::EVENT,
                                        )
                                    };
                                    MacroCallsite::new(&META)
                                };
                                let enabled = ::tracing::Level::ERROR
                                    <= ::tracing::level_filters::STATIC_MAX_LEVEL
                                    && ::tracing::Level::ERROR
                                        <= ::tracing::level_filters::LevelFilter::current()
                                    && {
                                        let interest = CALLSITE.interest();
                                        !interest.is_never() && CALLSITE.is_enabled(interest)
                                    };
                                if enabled {
                                    (|value_set: ::tracing::field::ValueSet| {
                                        let meta = CALLSITE.metadata();
                                        ::tracing::Event::dispatch(meta, &value_set);
                                    })({
                                        #[allow(unused_imports)]
                                        use ::tracing::field::{debug, display, Value};
                                        let mut iter = CALLSITE.metadata().fields().iter();
                                        CALLSITE.metadata().fields().value_set(&[(
                                            &iter
                                                .next()
                                                .expect("FieldSet corrupted (this is a bug)"),
                                            Some(&::core::fmt::Arguments::new_v1(
                                                &[
                                                    "connecting to address `",
                                                    "` failed, attempt ",
                                                    " starting",
                                                ],
                                                &[
                                                    ::core::fmt::ArgumentV1::new_debug(&addrs),
                                                    ::core::fmt::ArgumentV1::new_display(&attempt),
                                                ],
                                            )
                                                as &Value),
                                        )])
                                    });
                                } else {
                                }
                            };
                            crate::io::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                            attempt += 1;
                            if attempt == retries {
                                Err(crate::err::Error::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                )))?
                            }
                            continue;
                        }
                    }
                };
                let chan = Channel::from(stream);
                Ok(Handshake::from(chan))
            }
            #[inline]
            /// connect to the following address with the following id. Defaults to 3 retries.
            pub async fn connect(addrs: impl AsRef<Path> + std::fmt::Debug) -> Result<Handshake> {
                Self::connect_retry(addrs, 3, 10).await
            }
            #[inline]
            /// connect to the following address with the given id and retry in case of failure
            pub async fn connect_retry(
                addrs: impl AsRef<Path> + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
            }
        }
    }
    mod wss {
        use crate::Result;
        use crate::channel::Handshake;
        use crate::err;
        use crate::Channel;
        use cfg_if::cfg_if;
        use crate::channel::WSS;
        use crate::io::{TcpListener, ToSocketAddrs};
        use crate::io::wss;
        use derive_more::{From, Into};
        #[into(owned, ref, ref_mut)]
        /// Exposes routes over WebSockets
        pub struct Wss(#[cfg(not(target_arch = "wasm32"))] TcpListener);
        #[automatically_derived]
        impl ::core::convert::From<(TcpListener)> for Wss {
            #[inline]
            fn from(original: (TcpListener)) -> Wss {
                Wss(original)
            }
        }
        #[automatically_derived]
        impl ::core::convert::From<Wss> for (TcpListener) {
            #[inline]
            fn from(original: Wss) -> Self {
                (original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime Wss>
            for (&'__deriveMoreLifetime TcpListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime Wss) -> Self {
                (&original.0)
            }
        }
        #[automatically_derived]
        impl<'__deriveMoreLifetime> ::core::convert::From<&'__deriveMoreLifetime mut Wss>
            for (&'__deriveMoreLifetime mut TcpListener)
        {
            #[inline]
            fn from(original: &'__deriveMoreLifetime mut Wss) -> Self {
                (&mut original.0)
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        impl Wss {
            #[inline]
            #[cfg(feature = "tokio-net")]
            /// bind the global route on the given address
            pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
                let listener = TcpListener::bind(addrs).await?;
                Ok(Wss(listener))
            }
            #[inline]
            /// get the next channel
            /// ```norun
            /// while let Ok(chan) = wss.next().await {
            ///     let mut chan = chan.encrypted().await?;
            ///     chan.send("hello!").await?;
            /// }
            /// ```
            pub async fn next(&self) -> Result<Handshake> {
                let (chan, _) = self.0.accept().await?;
                let chan = wss::tokio::accept_async(chan).await.map_err(|e| {
                    crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
                let chan: Channel = Channel::from(chan);
                Ok(Handshake::from(chan))
            }
            #[inline]
            #[cfg(feature = "tokio-net")]
            /// connect to the following address without discovery
            pub async fn inner_connect(
                addrs: impl ToSocketAddrs + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<WSS> {
                let mut attempt = 0;
                let addrs = tokio::net::lookup_host(&addrs)
                    .await
                    .map_err(|e| {
                        crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                    })?
                    .next()
                    .ok_or(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "no endpoint found",
                    )))?;
                let stream = loop {
                    match wss::tokio::connect_async(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["ws://"],
                            &[::core::fmt::ArgumentV1::new_display(&&addrs)],
                        ));
                        res
                    })
                    .await
                    {
                        Ok((client, _)) => {
                            break client;
                        }
                        Err(e) => {
                            {
                                use ::tracing::__macro_support::Callsite as _;
                                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                                    use ::tracing::__macro_support::MacroCallsite;
                                    static META: ::tracing::Metadata<'static> = {
                                        ::tracing_core::metadata::Metadata::new(
                                            "event src/providers/wss.rs:69",
                                            "canary::providers::wss",
                                            ::tracing::Level::ERROR,
                                            Some("src/providers/wss.rs"),
                                            Some(69u32),
                                            Some("canary::providers::wss"),
                                            ::tracing_core::field::FieldSet::new(
                                                &["message"],
                                                ::tracing_core::callsite::Identifier(&CALLSITE),
                                            ),
                                            ::tracing::metadata::Kind::EVENT,
                                        )
                                    };
                                    MacroCallsite::new(&META)
                                };
                                let enabled = ::tracing::Level::ERROR
                                    <= ::tracing::level_filters::STATIC_MAX_LEVEL
                                    && ::tracing::Level::ERROR
                                        <= ::tracing::level_filters::LevelFilter::current()
                                    && {
                                        let interest = CALLSITE.interest();
                                        !interest.is_never() && CALLSITE.is_enabled(interest)
                                    };
                                if enabled {
                                    (|value_set: ::tracing::field::ValueSet| {
                                        let meta = CALLSITE.metadata();
                                        ::tracing::Event::dispatch(meta, &value_set);
                                    })({
                                        #[allow(unused_imports)]
                                        use ::tracing::field::{debug, display, Value};
                                        let mut iter = CALLSITE.metadata().fields().iter();
                                        CALLSITE.metadata().fields().value_set(&[(
                                            &iter
                                                .next()
                                                .expect("FieldSet corrupted (this is a bug)"),
                                            Some(&::core::fmt::Arguments::new_v1(
                                                &[
                                                    "connecting to address `",
                                                    "` failed, attempt ",
                                                    " starting",
                                                ],
                                                &[
                                                    ::core::fmt::ArgumentV1::new_debug(
                                                        &addrs.to_string(),
                                                    ),
                                                    ::core::fmt::ArgumentV1::new_display(&attempt),
                                                ],
                                            )
                                                as &Value),
                                        )])
                                    });
                                } else {
                                }
                            };
                            crate::io::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                            attempt += 1;
                            if attempt == retries {
                                Err(crate::err::Error::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                )))?
                            }
                            continue;
                        }
                    }
                };
                Ok(stream)
            }
            #[inline]
            #[cfg(feature = "tokio-net")]
            /// connect to the following address without discovery
            pub async fn raw_connect_with_retries(
                addrs: impl ToSocketAddrs + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                let stream = Self::inner_connect(addrs, retries, time_to_retry).await?;
                let chan = Channel::from(stream);
                Ok(Handshake::from(chan))
            }
            #[inline]
            /// connect to the following address with the following id. Defaults to 3 retries.
            pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
                Self::connect_retry(addrs, 3, 10).await
            }
            #[inline]
            /// connect to the following address with the given id and retry in case of failure
            pub async fn connect_retry(
                addrs: impl ToSocketAddrs + std::fmt::Debug,
                retries: u32,
                time_to_retry: u64,
            ) -> Result<Handshake> {
                Self::raw_connect_with_retries(&addrs, retries, time_to_retry).await
            }
        }
    }
    pub use addr::*;
    pub use wss::*;
    pub use any::*;
    #[cfg(not(target_arch = "wasm32"))]
    pub use tcp::*;
    #[cfg(unix)]
    pub use unix::*;
}
/// contains the serialization methods for channels
/// and formats
pub mod serialization {
    mod comms {
        use crate::io::{Read, ReadExt, Write, WriteExt};
        use crate::{err, Result};
        use futures::SinkExt;
        use futures_lite::StreamExt;
        use serde::{de::DeserializeOwned, Serialize};
        #[cfg(not(target_arch = "wasm32"))]
        use crate::io::wss::tungstenite::Message;
        use super::formats::{ReadFormat, SendFormat};
        use super::zc;
        /// send an item through the stream
        pub async fn tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
        where
            T: Write + Unpin,
            O: Serialize,
        {
            let serialized = F::serialize(&obj)?;
            zc::send_u64(st, serialized.len() as _).await?;
            st.write_all(&serialized).await?;
            st.flush().await?;
            Ok(serialized.len())
        }
        /// receive an item from the stream
        pub async fn rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
        where
            T: Read + Unpin,
            O: DeserializeOwned,
        {
            let size = zc::read_u64(st).await?;
            let mut buf = zc::try_vec(size as usize)?;
            st.read_exact(&mut buf).await?;
            F::deserialize(&buf)
        }
        #[cfg(not(target_arch = "wasm32"))]
        /// send a message from a websocket stream
        pub async fn wss_tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
        where
            T: futures::prelude::Sink<Message> + Unpin,
            O: Serialize,
            <T as futures::prelude::Sink<Message>>::Error: ToString,
        {
            let serialized = F::serialize(&obj)?;
            let len = serialized.len();
            let msg = Message::Binary(serialized);
            st.feed(msg).await.map_err(|e| {
                crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
            st.flush().await.map_err(|e| {
                crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
            Ok(len)
        }
        #[cfg(not(target_arch = "wasm32"))]
        /// receive a message from a websocket stream
        pub async fn wss_rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
        where
            T: futures::prelude::Stream<
                    Item = std::result::Result<Message, crate::io::wss::tungstenite::error::Error>,
                > + Unpin,
            O: DeserializeOwned,
        {
            let msg = st
                .next()
                .await
                .ok_or(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "websocket connection broke",
                )))?
                .map_err(|e| {
                    crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
                })?;
            match msg {
                Message::Binary(vec) => F::deserialize(&vec),
                Message::Text(_) => Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected binary message, found text message",
                ))),
                Message::Ping(_) => Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected binary message, found ping message",
                ))),
                Message::Pong(_) => Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected binary message, found pong message",
                ))),
                Message::Close(_) => Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected binary message, found close message",
                ))),
            }
        }
    }
    /// contains serialization formats
    pub mod formats {
        use std::marker::PhantomData;
        use bincode::Options;
        use serde::Serialize;
        use crate::err;
        /// bincode serialization format
        pub struct Bincode;
        /// JSON serialization format
        pub struct Json;
        /// BSON serialization format
        pub struct Bson;
        /// Postcard serialization format
        pub struct Postcard;
        /// trait that represents the serialize side of a format
        pub trait SendFormat {
            /// serialize object in this format
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>>;
        }
        /// trait that represents the deserialize side of a format
        pub trait ReadFormat {
            /// deserialize object in this format
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>;
        }
        /// trait that represents a format that can serialize and deserialize
        pub trait Format: SendFormat + ReadFormat {}
        impl SendFormat for Bincode {
            #[inline]
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
                let obj = bincode::DefaultOptions::new()
                    .allow_trailing_bytes()
                    .serialize(obj)
                    .or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e,
                        )))
                    })?;
                Ok(obj)
            }
        }
        impl ReadFormat for Bincode {
            #[inline]
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>,
            {
                bincode::DefaultOptions::new()
                    .allow_trailing_bytes()
                    .deserialize(bytes)
                    .or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e,
                        )))
                    })
            }
        }
        impl SendFormat for Json {
            #[inline]
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
                serde_json::to_vec(obj).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        impl ReadFormat for Json {
            #[inline]
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>,
            {
                serde_json::from_slice(bytes).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        impl SendFormat for Bson {
            #[inline]
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
                bson::ser::to_vec(obj).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        impl ReadFormat for Bson {
            #[inline]
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>,
            {
                bson::de::from_slice(bytes).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        impl SendFormat for Postcard {
            #[inline]
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
                postcard::to_allocvec(obj).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        impl ReadFormat for Postcard {
            #[inline]
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>,
            {
                postcard::from_bytes(bytes).or_else(|e| {
                    Err(crate::err::Error::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })
            }
        }
        /// combinator that allows input from any two formats:
        /// ```norun
        /// type BincodeOrJson = Any<Bincode, Json>
        /// ```
        pub struct Any<T, X>(PhantomData<(T, X)>);
        impl<T: SendFormat, X: SendFormat> SendFormat for Any<T, X> {
            #[inline]
            fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
                match T::serialize(obj) {
                    Ok(obj) => Ok(obj),
                    Err(_) => X::serialize(obj).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e,
                        )))
                    }),
                }
            }
        }
        impl<F: ReadFormat, X: ReadFormat> ReadFormat for Any<F, X> {
            #[inline]
            fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
            where
                T: serde::de::Deserialize<'a>,
            {
                match F::deserialize(bytes) {
                    Ok(s) => Ok(s),
                    Err(_) => X::deserialize(bytes).or_else(|e| {
                        Err(crate::err::Error::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e,
                        )))
                    }),
                }
            }
        }
    }
    /// contains zero-cost stream operations and more
    /// ```norun
    /// zc::send_u64(&mut stream, 42).await?;
    /// ```
    pub mod zc {
        #![allow(unused)]
        //! complete zero cost wrappers over network communications
        use crate::io::{Read, ReadExt, Write, WriteExt};
        use crate::{err, Result};
        pub(crate) fn try_vec<T: Default + Clone>(size: usize) -> Result<Vec<T>> {
            let mut buf = Vec::new();
            buf.try_reserve(size as usize).or_else(|e| {
                Err(crate::err::Error::new(std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["failed to reserve ", " bytes, error: "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&size),
                                ::core::fmt::ArgumentV1::new_debug(&e),
                            ],
                        ));
                        res
                    },
                )))
            })?;
            buf.resize(size as usize, T::default());
            Ok(buf)
        }
        pub(crate) async fn send_u8<T: Write + Unpin>(st: &mut T, obj: u8) -> Result<()> {
            st.write_all(&u8::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u16<T: Write + Unpin>(st: &mut T, obj: u16) -> Result<()> {
            st.write_all(&u16::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u32<T: Write + Unpin>(st: &mut T, obj: u32) -> Result<()> {
            st.write_all(&u32::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u64<T: Write + Unpin>(st: &mut T, obj: u64) -> Result<()> {
            st.write_all(&u64::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn read_u8<T: Read + Unpin>(st: &mut T) -> Result<u8> {
            let mut buf = [0u8; 1];
            st.read_exact(&mut buf).await?;
            Ok(u8::from_be_bytes(buf))
        }
        pub(crate) async fn read_u16<T: Read + Unpin>(st: &mut T) -> Result<u16> {
            let mut buf = [0u8; 2];
            st.read_exact(&mut buf).await?;
            Ok(u16::from_be_bytes(buf))
        }
        pub(crate) async fn read_u32<T: Read + Unpin>(st: &mut T) -> Result<u32> {
            let mut buf = [0u8; 4];
            st.read_exact(&mut buf).await?;
            Ok(u32::from_be_bytes(buf))
        }
        pub(crate) async fn read_u64<T: Read + Unpin>(st: &mut T) -> Result<u64> {
            let mut buf = [0u8; 8];
            st.read_exact(&mut buf).await?;
            Ok(u64::from_be_bytes(buf))
        }
    }
    pub use comms::*;
}
pub mod type_iter {
    //! Type juggling. Do not enter unless you want a headache, or if you want to understand how this works.
    //! I won't bother documenting this.
    use std::marker::PhantomData;
    use serde::{de::DeserializeOwned, Serialize};
    use crate::{
        channel::BareChannel,
        serialization::formats::{Bincode, ReadFormat, SendFormat},
        Channel,
    };
    /// used for iterating over types
    pub trait TypeIterT {
        /// next type iterator
        type Next;
        /// current value of node
        type Type;
    }
    impl TypeIterT for () {
        type Next = ();
        type Type = ();
    }
    /// type iterator which allows compile-time magic
    pub struct TypeIter<T, L = ()>(PhantomData<T>, PhantomData<L>);
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<T: ::core::default::Default, L: ::core::default::Default> ::core::default::Default
        for TypeIter<T, L>
    {
        #[inline]
        fn default() -> TypeIter<T, L> {
            TypeIter(
                ::core::default::Default::default(),
                ::core::default::Default::default(),
            )
        }
    }
    impl<T, L: TypeIterT> TypeIterT for TypeIter<T, L> {
        type Next = L;
        type Type = T;
    }
    /// trait that represents send or tx in pipelines
    pub trait Transmit {
        /// type that can be transmitted
        type Type;
    }
    /// trait that represents receive or rx in pipelines
    pub trait Receive {
        /// type that can be received
        type Type;
    }
    impl<T> Transmit for Tx<T> {
        type Type = T;
    }
    impl<T> Receive for Rx<T> {
        type Type = T;
    }
    /// type iterator that represents a type to be sent
    pub struct Tx<T>(T);
    /// type iterator that represents a type to be received
    pub struct Rx<T>(T);
    /// used for constructing pipelines
    pub trait Pipeline {
        /// inner pipeline
        type Pipe: TypeIterT;
    }
    impl Pipeline for () {
        type Pipe = ();
    }
    /// optimization to allow &str to be sent whenever a String needs to be received
    pub trait Str {}
    impl Str for Tx<String> {}
    impl Str for Tx<&str> {}
    /// optimization to allow &str to be sent whenever a Vec needs to be received
    pub trait Slice<T> {}
    impl<T> Slice<T> for Tx<&[T]> {}
    impl<T> Slice<T> for Tx<Vec<T>> {}
    /// Used for writing services, peer services should use PeerChannel.
    pub struct MainChannel<T: TypeIterT, ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode>(
        pub(crate) PhantomData<T>,
        pub(crate) Channel<ReadFmt, SendFmt>,
    );
    impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat> MainChannel<T, ReadFmt, SendFmt> {
        /// construct a new main channel
        pub fn new<P: Pipeline>(
            chan: Channel<ReadFmt, SendFmt>,
        ) -> MainChannel<P::Pipe, ReadFmt, SendFmt> {
            MainChannel(PhantomData, chan)
        }
        /// send an object through the stream and iterate to the next type
        pub async fn tx(
            mut self,
            obj: <T::Type as Transmit>::Type,
        ) -> crate::Result<MainChannel<T::Next, ReadFmt, SendFmt>>
        where
            T::Type: Transmit,
            <T as TypeIterT>::Next: TypeIterT,
            <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
        {
            self.1.tx(obj).await?;
            Ok(MainChannel(PhantomData, self.1))
        }
        /// receive an object from the stream and iterate to the next type
        pub async fn rx(
            mut self,
        ) -> crate::Result<(
            <T::Type as Receive>::Type,
            MainChannel<T::Next, ReadFmt, SendFmt>,
        )>
        where
            T::Type: Receive,
            <T as TypeIterT>::Next: TypeIterT,
            <T::Type as Receive>::Type: DeserializeOwned + 'static,
        {
            let res = self.1.rx::<<T::Type as Receive>::Type>().await?;
            let chan = MainChannel(PhantomData, self.1);
            Ok((res, chan))
        }
        /// coerce into a different kind of channel:
        pub fn coerce(self) -> Channel<ReadFmt, SendFmt> {
            self.1
        }
        /// make the channel bare, stripping it from its generics
        pub fn bare(self) -> BareChannel {
            self.1.bare()
        }
        /// send a str through the stream, this is an optimization done for pipelines receiving String
        /// to make sure an unnecessary allocation is not made
        pub async fn tx_str(
            mut self,
            obj: &str,
        ) -> crate::Result<MainChannel<T::Next, ReadFmt, SendFmt>>
        where
            T::Type: Transmit + Str,
            <T as TypeIterT>::Next: TypeIterT,
            <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
        {
            self.1.tx(obj).await?;
            Ok(MainChannel(PhantomData, self.1))
        }
        /// send a str through the stream, this is an optimization done for pipelines receiving String
        /// to make sure an unnecessary allocation is not made
        pub async fn tx_slice(
            mut self,
            obj: &[T::Type],
        ) -> crate::Result<MainChannel<T::Next, ReadFmt, SendFmt>>
        where
            T::Type: Transmit + Slice<T::Type> + Serialize,
            <T as TypeIterT>::Next: TypeIterT,
            <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
        {
            self.1.tx(obj).await?;
            Ok(MainChannel(PhantomData, self.1))
        }
    }
    impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat>
        From<MainChannel<T, ReadFmt, SendFmt>> for Channel<ReadFmt, SendFmt>
    {
        fn from(s: MainChannel<T, ReadFmt, SendFmt>) -> Self {
            s.coerce()
        }
    }
    impl<T: TypeIterT> From<PeerChannel<T>> for Channel {
        fn from(s: PeerChannel<T>) -> Self {
            s.coerce()
        }
    }
    /// Used for consuming services. Services should use MainChannel.
    pub struct PeerChannel<T: TypeIterT, ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode>(
        pub(crate) PhantomData<T>,
        pub(crate) Channel<ReadFmt, SendFmt>,
    );
    impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat> PeerChannel<T, ReadFmt, SendFmt> {
        /// construct a new peer channel
        pub fn new<P: Pipeline>(
            chan: Channel<ReadFmt, SendFmt>,
        ) -> PeerChannel<P::Pipe, ReadFmt, SendFmt>
        where
            <P as Pipeline>::Pipe: TypeIterT,
        {
            PeerChannel(PhantomData, chan)
        }
        /// send an object through the stream and iterate to the next type
        pub async fn tx(
            mut self,
            obj: <T::Type as Receive>::Type,
        ) -> crate::Result<PeerChannel<T::Next, ReadFmt, SendFmt>>
        where
            T::Type: Receive,
            <T as TypeIterT>::Next: TypeIterT,
            <<T as TypeIterT>::Type as Receive>::Type: Serialize + Send + 'static,
        {
            self.1.tx(obj).await?;
            Ok(PeerChannel(PhantomData, self.1))
        }
        /// receive an object from the stream and iterate to the next type
        pub async fn rx(
            mut self,
        ) -> crate::Result<(
            <T::Type as Transmit>::Type,
            PeerChannel<T::Next, ReadFmt, SendFmt>,
        )>
        where
            T::Type: Transmit,
            <T as TypeIterT>::Next: TypeIterT,
            <T::Type as Transmit>::Type: DeserializeOwned + 'static,
        {
            let res = self.1.rx::<<T::Type as Transmit>::Type>().await?;
            let chan = PeerChannel(PhantomData, self.1);
            Ok((res, chan))
        }
        /// coerce into a different kind of channel:
        pub fn channel(self) -> Channel<ReadFmt, SendFmt> {
            self.1
        }
        /// make the channel bare, stripping it from its generics
        pub fn bare(self) -> BareChannel {
            self.1.bare()
        }
        /// coerce into a different kind of channel:
        pub fn coerce<R: ReadFormat, S: SendFormat>(self) -> Channel<R, S> {
            self.1.coerce()
        }
        /// send a str through the stream, this is an optimization done for pipelines receiving String
        /// to make sure an unnecessary allocation is not made
        pub async fn tx_str(
            mut self,
            obj: &str,
        ) -> crate::Result<PeerChannel<T::Next, ReadFmt, SendFmt>>
        where
            T::Type: Transmit + Str,
            <T as TypeIterT>::Next: TypeIterT,
            <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
        {
            self.1.tx(obj).await?;
            Ok(PeerChannel(PhantomData, self.1))
        }
    }
}
#[cfg(feature = "nightly")]
/// offers nightly features such as zero-cost communications
pub mod nightly {
    use crate::err;
    use crate::io::{Read, ReadExt};
    use crate::io::{Write, WriteExt};
    use async_t::async_trait;
    use impl_trait_for_tuples::impl_for_tuples;
    pub trait AsyncPull: Sized {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static;
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>: core::future::Future<Output = crate::Result<Self>>
            + 'future
            + Send
        where
            R: 'static;
    }
    pub trait AsyncSend: Sized {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W>;
        #[allow(non_camel_case_types)]
        type impl_trait_send_0 < 'future , W : Write + Unpin + Send + 'static + 'future > : core :: future :: Future < Output = crate :: Result < () > > + 'future + Send where Self : 'future ;
    }
    impl AsyncPull for i8 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for i8 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for i16 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for i16 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for i32 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for i32 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for i64 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for i64 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for i128 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for i128 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for u8 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for u8 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for u16 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for u16 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for u32 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for u32 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for u64 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for u64 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for u128 {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for u128 {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncPull for bool {
        fn pull<'future, R: Read + Unpin + Send + 'static + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R> {
            async move {
                {
                    let mut bytes: [u8; 1] = [0u8; 1];
                    io.read_exact(&mut bytes).await?;
                    Ok(bytes[0] == 1)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'static + 'future> =
            impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for bool {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let bytes = [*self as u8];
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl<T: Send + AsyncPull + 'static> AsyncPull for Vec<T> {
        fn pull<'future, R: Read + Unpin + Send + 'static + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R> {
            async move {
                {
                    let len = u64::pull(io).await?;
                    let mut v = ::alloc::vec::Vec::new();
                    for _ in 0..len {
                        let val = T::pull(io).await?;
                        v.push(val)
                    }
                    Ok(v)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'static + 'future> =
            impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl<T: Send + Sync + AsyncSend + 'static> AsyncSend for &[T] {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    let len = self.len() as u64;
                    len.send(io).await?;
                    for val in self.iter() {
                        val.send(io).await?;
                    }
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl<T: Send + Sync + AsyncSend + 'static, const N: usize> AsyncSend for [T; N] {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    for val in self {
                        val.send(io).await?;
                    }
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl<T: Send + AsyncPull + 'static + Default + Copy, const N: usize> AsyncPull for [T; N] {
        fn pull<'future, R: Read + Unpin + Send + 'static + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R> {
            async move {
                {
                    let mut v = [T::default(); N];
                    for ptr in v.iter_mut() {
                        let val = T::pull(io).await?;
                        *ptr = val;
                    }
                    Ok(v)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'static + 'future> =
            impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncPull for String {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let vec = Vec::pull(io).await?;
                    String::from_utf8(vec).map_err(|e| {
                        crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                    })
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    impl AsyncSend for String {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.as_str().send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    impl AsyncSend for &str {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.as_bytes().send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<TupleElement0: AsyncSend, TupleElement1: AsyncSend> AsyncSend
        for (TupleElement0, TupleElement1)
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<TupleElement0: AsyncSend, TupleElement1: AsyncSend, TupleElement2: AsyncSend> AsyncSend
        for (TupleElement0, TupleElement1, TupleElement2)
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
        > AsyncSend for (TupleElement0, TupleElement1, TupleElement2, TupleElement3)
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
            TupleElement11: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
        TupleElement11: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    self.11.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
            TupleElement11: AsyncSend,
            TupleElement12: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
        TupleElement11: Send + Sync + 'static,
        TupleElement12: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    self.11.send(io).await?;
                    self.12.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
            TupleElement11: AsyncSend,
            TupleElement12: AsyncSend,
            TupleElement13: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
        TupleElement11: Send + Sync + 'static,
        TupleElement12: Send + Sync + 'static,
        TupleElement13: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    self.11.send(io).await?;
                    self.12.send(io).await?;
                    self.13.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
            TupleElement11: AsyncSend,
            TupleElement12: AsyncSend,
            TupleElement13: AsyncSend,
            TupleElement14: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
            TupleElement14,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
        TupleElement11: Send + Sync + 'static,
        TupleElement12: Send + Sync + 'static,
        TupleElement13: Send + Sync + 'static,
        TupleElement14: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    self.11.send(io).await?;
                    self.12.send(io).await?;
                    self.13.send(io).await?;
                    self.14.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncSend,
            TupleElement1: AsyncSend,
            TupleElement2: AsyncSend,
            TupleElement3: AsyncSend,
            TupleElement4: AsyncSend,
            TupleElement5: AsyncSend,
            TupleElement6: AsyncSend,
            TupleElement7: AsyncSend,
            TupleElement8: AsyncSend,
            TupleElement9: AsyncSend,
            TupleElement10: AsyncSend,
            TupleElement11: AsyncSend,
            TupleElement12: AsyncSend,
            TupleElement13: AsyncSend,
            TupleElement14: AsyncSend,
            TupleElement15: AsyncSend,
        > AsyncSend
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
            TupleElement14,
            TupleElement15,
        )
    where
        TupleElement0: Send + Sync + 'static,
        TupleElement1: Send + Sync + 'static,
        TupleElement2: Send + Sync + 'static,
        TupleElement3: Send + Sync + 'static,
        TupleElement4: Send + Sync + 'static,
        TupleElement5: Send + Sync + 'static,
        TupleElement6: Send + Sync + 'static,
        TupleElement7: Send + Sync + 'static,
        TupleElement8: Send + Sync + 'static,
        TupleElement9: Send + Sync + 'static,
        TupleElement10: Send + Sync + 'static,
        TupleElement11: Send + Sync + 'static,
        TupleElement12: Send + Sync + 'static,
        TupleElement13: Send + Sync + 'static,
        TupleElement14: Send + Sync + 'static,
        TupleElement15: Send + Sync + 'static,
    {
        fn send<'future, W: Write + Unpin + Send + 'static + 'future>(
            &'future self,
            io: &'future mut W,
        ) -> Self::impl_trait_send_0<'future, W> {
            async move {
                {
                    self.0.send(io).await?;
                    self.1.send(io).await?;
                    self.2.send(io).await?;
                    self.3.send(io).await?;
                    self.4.send(io).await?;
                    self.5.send(io).await?;
                    self.6.send(io).await?;
                    self.7.send(io).await?;
                    self.8.send(io).await?;
                    self.9.send(io).await?;
                    self.10.send(io).await?;
                    self.11.send(io).await?;
                    self.12.send(io).await?;
                    self.13.send(io).await?;
                    self.14.send(io).await?;
                    self.15.send(io).await?;
                    Ok(())
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_send_0<'future, W: Write + Unpin + Send + 'static + 'future>
        where
            Self: 'future,
        = impl core::future::Future<Output = crate::Result<()>> + 'future + Send;
    }
    #[allow(unused)]
    impl<TupleElement0: AsyncPull, TupleElement1: AsyncPull> AsyncPull
        for (TupleElement0, TupleElement1)
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<TupleElement0: AsyncPull, TupleElement1: AsyncPull, TupleElement2: AsyncPull> AsyncPull
        for (TupleElement0, TupleElement1, TupleElement2)
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
        > AsyncPull for (TupleElement0, TupleElement1, TupleElement2, TupleElement3)
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
            TupleElement11: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
        TupleElement11: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                        TupleElement11::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
            TupleElement11: AsyncPull,
            TupleElement12: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
        TupleElement11: Send + Sync + 'static + AsyncPull,
        TupleElement12: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                        TupleElement11::pull(io).await?,
                        TupleElement12::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
            TupleElement11: AsyncPull,
            TupleElement12: AsyncPull,
            TupleElement13: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
        TupleElement11: Send + Sync + 'static + AsyncPull,
        TupleElement12: Send + Sync + 'static + AsyncPull,
        TupleElement13: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                        TupleElement11::pull(io).await?,
                        TupleElement12::pull(io).await?,
                        TupleElement13::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
            TupleElement11: AsyncPull,
            TupleElement12: AsyncPull,
            TupleElement13: AsyncPull,
            TupleElement14: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
            TupleElement14,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
        TupleElement11: Send + Sync + 'static + AsyncPull,
        TupleElement12: Send + Sync + 'static + AsyncPull,
        TupleElement13: Send + Sync + 'static + AsyncPull,
        TupleElement14: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                        TupleElement11::pull(io).await?,
                        TupleElement12::pull(io).await?,
                        TupleElement13::pull(io).await?,
                        TupleElement14::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
    #[allow(unused)]
    impl<
            TupleElement0: AsyncPull,
            TupleElement1: AsyncPull,
            TupleElement2: AsyncPull,
            TupleElement3: AsyncPull,
            TupleElement4: AsyncPull,
            TupleElement5: AsyncPull,
            TupleElement6: AsyncPull,
            TupleElement7: AsyncPull,
            TupleElement8: AsyncPull,
            TupleElement9: AsyncPull,
            TupleElement10: AsyncPull,
            TupleElement11: AsyncPull,
            TupleElement12: AsyncPull,
            TupleElement13: AsyncPull,
            TupleElement14: AsyncPull,
            TupleElement15: AsyncPull,
        > AsyncPull
        for (
            TupleElement0,
            TupleElement1,
            TupleElement2,
            TupleElement3,
            TupleElement4,
            TupleElement5,
            TupleElement6,
            TupleElement7,
            TupleElement8,
            TupleElement9,
            TupleElement10,
            TupleElement11,
            TupleElement12,
            TupleElement13,
            TupleElement14,
            TupleElement15,
        )
    where
        TupleElement0: Send + Sync + 'static + AsyncPull,
        TupleElement1: Send + Sync + 'static + AsyncPull,
        TupleElement2: Send + Sync + 'static + AsyncPull,
        TupleElement3: Send + Sync + 'static + AsyncPull,
        TupleElement4: Send + Sync + 'static + AsyncPull,
        TupleElement5: Send + Sync + 'static + AsyncPull,
        TupleElement6: Send + Sync + 'static + AsyncPull,
        TupleElement7: Send + Sync + 'static + AsyncPull,
        TupleElement8: Send + Sync + 'static + AsyncPull,
        TupleElement9: Send + Sync + 'static + AsyncPull,
        TupleElement10: Send + Sync + 'static + AsyncPull,
        TupleElement11: Send + Sync + 'static + AsyncPull,
        TupleElement12: Send + Sync + 'static + AsyncPull,
        TupleElement13: Send + Sync + 'static + AsyncPull,
        TupleElement14: Send + Sync + 'static + AsyncPull,
        TupleElement15: Send + Sync + 'static + AsyncPull,
    {
        fn pull<'future, R: Read + Unpin + Send + 'future>(
            io: &'future mut R,
        ) -> Self::impl_trait_pull_0<'future, R>
        where
            R: 'static,
        {
            async move {
                {
                    let tpl = (
                        TupleElement0::pull(io).await?,
                        TupleElement1::pull(io).await?,
                        TupleElement2::pull(io).await?,
                        TupleElement3::pull(io).await?,
                        TupleElement4::pull(io).await?,
                        TupleElement5::pull(io).await?,
                        TupleElement6::pull(io).await?,
                        TupleElement7::pull(io).await?,
                        TupleElement8::pull(io).await?,
                        TupleElement9::pull(io).await?,
                        TupleElement10::pull(io).await?,
                        TupleElement11::pull(io).await?,
                        TupleElement12::pull(io).await?,
                        TupleElement13::pull(io).await?,
                        TupleElement14::pull(io).await?,
                        TupleElement15::pull(io).await?,
                    );
                    Ok(tpl)
                }
            }
        }
        #[allow(non_camel_case_types)]
        type impl_trait_pull_0<'future, R: Read + Unpin + Send + 'future>
        where
            R: 'static,
        = impl core::future::Future<Output = crate::Result<Self>> + 'future + Send;
    }
}
pub use channel::Channel;
pub use err::Error;
pub use err::Result;
