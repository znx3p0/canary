use serde::{de::DeserializeOwned, Serialize};
use snow::{params::*, TransportState};

use crate::io::{ReadExt, WriteExt};
use crate::serialization::formats::{Bincode, ReadFormat, SendFormat};
use crate::serialization::{rx, tx, wss_rx, wss_tx, zc};
use crate::{channel::ReadWrite, err};
use crate::{channel::WSS, Result};

#[cfg(feature = "static_ser")]
use crate::serialization::formats::Format;
#[cfg(feature = "static_ser")]
use bytes::Buf;
#[cfg(feature = "static_ser")]
use smallvec::SmallVec;

#[cfg(feature = "static_ser")]
use crate::static_ser::{StaticDeserialize, StaticSerialize};

/// Stream wrapper with encryption.
/// It uses the Noise protocol for encryption
pub struct Snow<T> {
    pub(crate) stream: T,
    transport: TransportState,
}

const PACKET_LEN: u64 = 65519;

impl<T> Snow<T> {
    fn encrypt_packets(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        let mut total = Vec::with_capacity(buf.len() + 16);

        for buf in buf.chunks(PACKET_LEN as _) {
            let mut buf = self.encrypt_packet(buf)?;
            total.append(&mut buf);
        }
        Ok(total)
    }

    // returns an error if length of buf is greater than the packet length
    fn encrypt_packet(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        // create message buffer
        let mut msg = vec![0u8; buf.len() + 16];
        // encrypt into message buffer
        self.encrypt_packet_raw(buf, &mut msg)?;
        Ok(msg)
    }
    fn encrypt_packet_raw(&mut self, buf: &[u8], mut msg: &mut [u8]) -> Result<()> {
        // encrypt into message buffer
        self.transport
            .write_message(buf, &mut msg)
            .map_err(|e| err!(invalid_data, e))?;
        Ok(())
    }
}

impl<T: ReadWrite + Unpin> Snow<T> {
    /// Starts a new snow stream using the default noise parameters
    #[inline]
    pub async fn new(stream: T) -> Result<Self> {
        // let noise_params = "Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
        let noise_params = NoiseParams::new(
            "".into(),
            BaseChoice::Noise,
            HandshakeChoice {
                pattern: HandshakePattern::NN,
                modifiers: HandshakeModifierList { list: vec![] },
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
        // To initialize the encrypted stream, we need to decide which stream
        // is the initiator and which is the responder.
        // For this we send a random number and receive it on the other side.
        // The lowest number is the responder

        let should_init = loop {
            let local_num = rand::random::<u64>();
            tx(&mut stream, local_num, &Bincode).await?;
            let peer_num = rx(&mut stream, &Bincode).await?;
            if local_num == peer_num {
                continue;
            }
            if local_num > peer_num {
                break false;
            }
            break true;
        };

        let builder = snow::Builder::new(noise_params);
        let keypair = builder.generate_keypair().or_else(|e| err!((other, e)))?;
        let builder = builder.local_private_key(&keypair.private);
        // send public key to peer
        tx(&mut stream, keypair.public, &Bincode).await?;

        // receive peer's public key
        let peer_public_key = rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?;
        // set peer's public key
        let builder = builder.remote_public_key(&peer_public_key);

        let mut buf = vec![0u8; 256];
        // initialize the encrypted stream
        match should_init {
            true => {
                let mut handshake = builder.build_initiator().or_else(|e| err!((other, e)))?;

                let len = handshake
                    .write_message(&[], &mut buf)
                    .or_else(|e| err!((other, e)))?;
                tx(&mut stream, &buf[..len], &Bincode).await?;

                // <- e, ee, s, es
                handshake
                    .read_message(&rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?, &mut buf)
                    .or_else(|e| err!((other, e)))?;

                let transport = handshake
                    .into_transport_mode()
                    .or_else(|e| err!((other, e)))?;

                Ok(Snow { stream, transport })
            }
            false => {
                let mut handshake = builder.build_responder().or_else(|e| err!((other, e)))?;

                // <- e
                handshake
                    .read_message(&rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?, &mut buf)
                    .or_else(|e| err!((other, e)))?;

                // -> e, ee, s, es
                let len = handshake
                    .write_message(&[0u8; 0], &mut buf)
                    .or_else(|e| err!((other, e)))?;
                tx(&mut stream, &buf[..len], &Bincode).await?;

                // Transition the state machine into transport mode now that the handshake is complete.
                let transport = handshake
                    .into_transport_mode()
                    .or_else(|e| err!((other, e)))?;

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
    pub async fn rx<O: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<O> {
        let size = zc::read_u64(&mut self.stream).await?;
        // receive message
        let mut buf = zc::try_vec(size as _)?;
        self.stream.read_exact(&mut buf).await?;

        let mut msg = vec![];

        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut inner = vec![0u8; buf.len()];
            self.transport
                .read_message(&buf, &mut inner)
                .or_else(|e| err!((other, e)))?;
            msg.append(&mut inner);
        }

        f.deserialize(&msg)
    }

    /// receive message from stream using a custom deserialization strategy
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     let num: u64 = peer.rx().await?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    #[cfg(feature = "static_ser")]
    pub async fn static_rx<O: StaticDeserialize>(&mut self) -> Result<O> {
        let size = zc::read_u64(&mut self.stream).await?;
        // receive message
        let mut buf = zc::try_vec(size as _)?;
        self.stream.read_exact(&mut buf).await?;

        let mut msg = vec![];

        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut inner = vec![0u8; buf.len()];
            self.transport
                .read_message(&buf, &mut inner)
                .or_else(|e| err!((other, e)))?;
            msg.append(&mut inner);
        }

        let mut msg = msg.reader();
        O::deserialize(&mut msg)
    }

    /// send message to stream
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     peer.tx(123).await?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub async fn tx<O: Serialize, F: SendFormat>(&mut self, obj: O, f: &F) -> Result<usize> {
        // serialize or return invalid data error
        let vec = f.serialize(&obj)?;

        let msg = self.encrypt_packets(&vec)?;

        zc::send_u64(&mut self.stream, msg.len() as _).await?;
        self.stream.write_all(&msg).await?;
        self.stream.flush().await?;
        Ok(msg.len())
    }

    /// send message to stream using a custom serialization strategy
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     peer.static_tx(123u32).await?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    #[cfg(feature = "static_ser")]
    pub async fn static_tx<O: StaticSerialize>(&mut self, obj: O) -> Result<usize> {
        // serialize or return invalid data error
        let mut vec = vec![];
        obj.serialize(&mut vec)?;
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
    pub async fn new_with_params_wss(mut stream: WSS, noise_params: NoiseParams) -> Result<Self> {
        // To initialize the encrypted stream, we need to decide which stream
        // is the initiator and which is the responder.
        // For this we send a random number and receive it on the other side.
        // The lowest number is the responder

        let should_init = loop {
            let local_num = rand::random::<u64>();

            wss_tx(&mut stream, local_num, &Bincode).await?;

            let peer_num = wss_rx(&mut stream, &Bincode).await?;
            if local_num == peer_num {
                continue;
            }
            if local_num > peer_num {
                break false;
            }
            break true;
        };

        let builder = snow::Builder::new(noise_params);
        let keypair = builder.generate_keypair().or_else(|e| err!((other, e)))?;
        let builder = builder.local_private_key(&keypair.private);
        // send public key to peer
        wss_tx(&mut stream, keypair.public, &Bincode).await?;

        // receive peer's public key
        let peer_public_key = wss_rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?;
        // set peer's public key
        let builder = builder.remote_public_key(&peer_public_key);

        let mut buf = vec![0u8; 256];
        // initialize the encrypted stream
        match should_init {
            true => {
                let mut handshake = builder.build_initiator().or_else(|e| err!((other, e)))?;

                let len = handshake
                    .write_message(&[], &mut buf)
                    .or_else(|e| err!((other, e)))?;
                wss_tx(&mut stream, &buf[..len], &Bincode).await?;

                // <- e, ee, s, es
                handshake
                    .read_message(
                        &wss_rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?,
                        &mut buf,
                    )
                    .or_else(|e| err!((other, e)))?;

                let transport = handshake
                    .into_transport_mode()
                    .or_else(|e| err!((other, e)))?;

                Ok(Snow { stream, transport })
            }
            false => {
                let mut handshake = builder.build_responder().or_else(|e| err!((other, e)))?;

                // <- e
                handshake
                    .read_message(
                        &wss_rx::<_, Vec<u8>, _>(&mut stream, &Bincode).await?,
                        &mut buf,
                    )
                    .or_else(|e| err!((other, e)))?;

                // -> e, ee, s, es
                let len = handshake
                    .write_message(&[0u8; 0], &mut buf)
                    .or_else(|e| err!((other, e)))?;
                wss_tx(&mut stream, &buf[..len], &Bincode).await?;

                // Transition the state machine into transport mode now that the handshake is complete.
                let transport = handshake
                    .into_transport_mode()
                    .or_else(|e| err!((other, e)))?;

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
    pub async fn wss_rx<O: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<O> {
        let buf: Vec<u8> = wss_rx(&mut self.stream, f).await?;

        let mut msg = vec![];

        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut inner = vec![0u8; buf.len()];
            self.transport
                .read_message(&buf, &mut inner)
                .or_else(|e| err!((other, e)))?;
            msg.append(&mut inner);
        }

        f.deserialize(&msg)
    }

    #[inline]
    #[cfg(feature = "static_ser")]
    /// receive message from stream
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     let num: u64 = peer.rx().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn static_wss_rx<O: StaticDeserialize>(&mut self) -> Result<O> {
        let buf: Vec<u8> = wss_rx(&mut self.stream, &Format::Bincode).await?;

        let mut msg = vec![];

        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut inner = vec![0u8; buf.len()];
            self.transport
                .read_message(&buf, &mut inner)
                .or_else(|e| err!((other, e)))?;
            msg.append(&mut inner);
        }
        O::deserialize(&mut msg.reader())
    }

    #[inline]
    /// send message to stream
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     peer.tx(123).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn wss_tx<O: Serialize, F: SendFormat>(&mut self, obj: O, f: &F) -> Result<usize> {
        // serialize or return invalid data error
        let vec = f.serialize(&obj)?;

        let msg = self.encrypt_packets(&vec)?;
        let len = msg.len();

        wss_tx(&mut self.stream, msg, f).await?;
        Ok(len)
    }

    #[inline]
    #[cfg(feature = "static_ser")]
    /// send message to stream
    /// ```norun
    /// async fn service(mut peer: Snow<TcpStream>) -> Result<()> {
    ///     peer.tx(123).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn static_wss_tx<O: StaticSerialize>(&mut self, obj: O) -> Result<usize> {
        // serialize or return invalid data error
        let mut vec = SmallVec::<[u8; 8]>::with_capacity(O::LEN);
        obj.serialize(&mut vec)?;

        let msg = self.encrypt_packets(&vec)?;
        let len = msg.len();

        wss_tx(&mut self.stream, msg, &Format::Bincode).await?;
        Ok(len)
    }
}
