use serde::{de::DeserializeOwned, Serialize};
use snow::{params::*, TransportState};

use crate::io::{ReadExt, WriteExt};
use crate::serialization::formats::{Bincode, ReadFormat, SendFormat};
use crate::serialization::{rx, tx, zc};
use crate::Result;
use crate::{err, ReadWrite};

/// Stream wrapper with encryption.
/// It uses the Noise protocol for encryption
pub struct Snow<T> {
    pub(crate) stream: T,
    transport: TransportState,
}

impl<T: ReadWrite + Unpin> Snow<T> {
    /// Starts a new snow stream using the default noise parameters
    pub async fn new(stream: T) -> Result<Self> {
        Self::new_with_params(stream, "Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap()).await
    }

    /// Starts a new snow stream using the provided parameters.
    pub async fn new_with_params(mut stream: T, noise_params: NoiseParams) -> Result<Self> {
        // To initialize the encrypted stream, we need to decide which stream
        // is the initiator and which is the responder.
        // For this we send a random number and receive it on the other side.
        // The lowest number is the responder

        let should_init = loop {
            let local_num = fastrand::u64(0..100_000);
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
        let keypair = builder.generate_keypair().or_else(|e| err!((other, e)))?;
        let builder = builder.local_private_key(&keypair.private);
        // send public key to peer
        tx::<_, _, Bincode>(&mut stream, keypair.public).await?;

        // receive peer's public key
        let peer_public_key = rx::<_, Vec<u8>, Bincode>(&mut stream).await?;
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
                tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;

                // <- e, ee, s, es
                handshake
                    .read_message(&rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
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
                    .read_message(&rx::<_, Vec<u8>, Bincode>(&mut stream).await?, &mut buf)
                    .or_else(|e| err!((other, e)))?;

                // -> e, ee, s, es
                let len = handshake
                    .write_message(&[0u8; 0], &mut buf)
                    .or_else(|e| err!((other, e)))?;
                tx::<_, _, Bincode>(&mut stream, &buf[..len]).await?;

                // Transition the state machine into transport mode now that the handshake is complete.
                let transport = handshake
                    .into_transport_mode()
                    .or_else(|e| err!((other, e)))?;

                Ok(Snow { stream, transport })
            }
        }
    }
    pub async fn rx<O: DeserializeOwned, F: ReadFormat>(&mut self) -> Result<O> {
        let size = zc::read_u32(&mut self.stream).await?;

        let mut buf = zc::try_vec(size as _)?;

        self.stream.read_exact(&mut buf).await?;

        let mut msg = vec![0u8; buf.len()];
        self.transport
            .read_message(&buf, &mut msg)
            .or_else(|e| err!((other, e)))?;

        F::deserialize(&msg)
    }
    pub async fn tx<O: Serialize, F: SendFormat>(&mut self, obj: O) -> Result<usize> {
        // serialize or return invalid data error
        let vec = F::serialize(&obj)?;

        // get length from serialized object
        let len = vec.len();

        // create message buffer
        let mut msg = vec![0u8; len + 16];
        // encrypt into message buffer
        self.transport
            .write_message(&vec, &mut msg)
            .map_err(|e| err!(invalid_data, e))?;

        zc::send_u32(&mut self.stream, msg.len() as _).await?;
        self.stream.write_all(&msg).await?;
        self.stream.flush().await?;
        Ok(msg.len())
    }
}
