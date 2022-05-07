use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{SplitSink, SplitStream};
use serde::{de::DeserializeOwned, Serialize};
use snow::{params::*, Builder, StatelessTransportState};
use tungstenite::Message;

use crate::io::{Read, ReadExt, Write, WriteExt};
use crate::serialization::formats::{Bincode, ReadFormat, SendFormat};
use crate::serialization::{rx, tx, wss_rx, wss_tx, zc};
use crate::{err, Channel};
use crate::{io::Wss, Result};

pub struct StaticSnow(StatelessTransportState, u64);
pub struct Snow(StatelessTransportState, AtomicU64);

const PACKET_LEN: u64 = 65519;

pub trait Cipher {
    fn encrypt_packets(&self, buf: Vec<u8>) -> Result<Vec<u8>>;
    // returns an error if length of buf is greater than the packet length
    fn encrypt_packet(&self, buf: &[u8]) -> Result<Vec<u8>>;
    fn encrypt_packet_raw(&self, buf: &[u8], msg: &mut [u8]) -> Result;
    fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>>;
}

impl Cipher for Snow {
    fn encrypt_packets(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        let mut total = Vec::with_capacity(buf.len() + 16);

        for buf in buf.chunks(PACKET_LEN as _) {
            let mut buf = self.encrypt_packet(buf)?;
            total.append(&mut buf);
        }
        Ok(total)
    }
    // returns an error if length of buf is greater than the packet length
    fn encrypt_packet(&self, buf: &[u8]) -> Result<Vec<u8>> {
        // create message buffer
        let mut msg = vec![0u8; buf.len() + 16];
        // encrypt into message buffer
        self.encrypt_packet_raw(buf, &mut msg)?;
        Ok(msg)
    }
    fn encrypt_packet_raw(&self, buf: &[u8], mut msg: &mut [u8]) -> Result {
        // encrypt into message buffer
        let nonce = self.1.fetch_add(1, Ordering::SeqCst);

        self.0
            .write_message(nonce, buf, &mut msg)
            .map_err(err!(@invalid_data))?;
        Ok(())
    }
    fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>> {
        let mut bytes = vec![];
        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut message = vec![0u8; buf.len()]; // move message outside the loop
            let nonce = self.1.fetch_add(1, Ordering::SeqCst);
            self.0
                .read_message(nonce, &buf, &mut message)
                .map_err(|e| err!(other, e.to_string()))?;
            bytes.append(&mut message);
        }
        Ok(bytes)
    }
}

impl Cipher for StaticSnow {
    fn encrypt_packets(&self, buf: Vec<u8>) -> Result<Vec<u8>> {
        let mut total = Vec::with_capacity(buf.len() + 16);

        for buf in buf.chunks(PACKET_LEN as _) {
            let mut buf = self.encrypt_packet(buf)?;
            total.append(&mut buf);
        }
        Ok(total)
    }
    // returns an error if length of buf is greater than the packet length
    fn encrypt_packet(&self, buf: &[u8]) -> Result<Vec<u8>> {
        // create message buffer
        let mut msg = vec![0u8; buf.len() + 16];
        // encrypt into message buffer
        self.encrypt_packet_raw(buf, &mut msg)?;
        Ok(msg)
    }
    fn encrypt_packet_raw(&self, buf: &[u8], mut msg: &mut [u8]) -> Result {
        // encrypt into message buffer
        let nonce = self.1;

        self.0
            .write_message(nonce, buf, &mut msg)
            .map_err(err!(@invalid_data))?;
        Ok(())
    }
    fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>> {
        let mut bytes = vec![];
        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut message = vec![0u8; buf.len()]; // move message outside the loop
            let nonce = self.1;
            self.0
                .read_message(nonce, &buf, &mut message)
                .map_err(|e| err!(other, e.to_string()))?;
            bytes.append(&mut message);
        }
        Ok(bytes)
    }
}

/// Starts a new snow stream using the default noise parameters
pub async fn new(stream: &mut Channel) -> Result<Snow> {
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
    new_with_params(stream, noise_params).await
}
/// starts a new snow stream using the provided parameters.
pub async fn new_with_params(chan: &mut Channel, noise_params: NoiseParams) -> Result<Snow> {
    let should_init = loop {
        let local_num = rand::random::<u64>();

        chan.send(local_num).await?;
        let peer_num: u64 = chan.receive().await?;

        if local_num == peer_num {
            continue;
        } else {
            break local_num > peer_num;
        }
    };
    if should_init {
        initialize_initiator(chan, noise_params).await
    } else {
        initialize_responder(chan, noise_params).await
    }
}

/// starts a new snow stream using the provided parameters.
pub(crate) async fn initialize_initiator(
    chan: &mut Channel,
    noise_params: NoiseParams,
) -> Result<Snow> {
    dbg!();

    let mut initiator = snow::Builder::new(noise_params).build_initiator().unwrap();
    let mut buffer_msg = vec![0u8; 128];
    let rand_payload: &[u8; 16] = &rand::random();

    let len = initiator
        .write_message(rand_payload, &mut buffer_msg)
        .unwrap(); // verified

    chan.send((&buffer_msg, len as u64)).await?;

    let (mut buffer_out, buffer_msg): (Vec<u8>, Vec<u8>) = chan.receive().await?;
    initiator
        .read_message(&buffer_msg, &mut buffer_out)
        .unwrap();

    let initiator = initiator.into_stateless_transport_mode().unwrap();
    Ok(Snow(initiator, Default::default()))
}

/// starts a new snow stream using the provided parameters.
pub(crate) async fn initialize_responder(
    chan: &mut Channel,
    noise_params: NoiseParams,
) -> Result<Snow> {
    let mut responder = snow::Builder::new(noise_params).build_responder().unwrap();
    let mut buffer_out = vec![0u8; 128];

    let (mut buffer_msg, len): (Vec<u8>, u64) = chan.receive().await?;
    responder
        .read_message(&buffer_msg[..len as usize], &mut buffer_out)
        .unwrap(); // verified

    let rand_payload: &[u8; 16] = &rand::random();

    let len = responder
        .write_message(rand_payload, &mut buffer_msg)
        .unwrap();
    chan.send((&buffer_out, &buffer_msg[..len])).await?;

    let responder = responder.into_stateless_transport_mode().unwrap();
    Ok(Snow(responder, Default::default()))
}
