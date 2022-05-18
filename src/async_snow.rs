use crate::Result;
use crate::{err, Channel};
use snow::{params::*, StatelessTransportState};

const PACKET_LEN: u64 = 65519;

pub struct RefDividedSnow<'a> {
    pub transport: &'a StatelessTransportState,
    pub nonce: &'a mut u32,
}
pub trait Encrypt {
    fn encrypt_packets(&mut self, buf: Vec<u8>) -> Result<Vec<u8>>;
}

// // returns an error if length of buf is greater than the packet length
// fn encrypt_packet(&mut self, buf: &[u8]) -> Result<Vec<u8>>;
// fn encrypt_packet_raw(&mut self, buf: &[u8], msg: &mut [u8]) -> Result;

pub trait Decrypt {
    fn decrypt(&mut self, buf: &[u8]) -> Result<Vec<u8>>;
}

impl RefDividedSnow<'_> {
    // returns an error if length of buf is greater than the packet length
    fn encrypt_packet(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        // create message buffer
        let mut msg = vec![0u8; buf.len() + 16];
        // encrypt into message buffer
        self.encrypt_packet_raw(buf, &mut msg)?;
        Ok(msg)
    }
    fn encrypt_packet_raw(&mut self, buf: &[u8], mut msg: &mut [u8]) -> Result {
        // encrypt into message buffer
        let nonce = self.nonce.wrapping_add(1) as _;
        self.transport
            .write_message(nonce, buf, &mut msg)
            .map_err(err!(@invalid_data))?;
        Ok(())
    }
}

impl Encrypt for RefDividedSnow<'_> {
    fn encrypt_packets(&mut self, buf: Vec<u8>) -> Result<Vec<u8>> {
        let mut total = Vec::with_capacity(buf.len() + 16);
        for buf in buf.chunks(PACKET_LEN as _) {
            let mut buf = self.encrypt_packet(buf)?;
            total.append(&mut buf);
        }
        Ok(total)
    }
}

impl Decrypt for RefDividedSnow<'_> {
    fn decrypt(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        let mut bytes = vec![];
        for buf in buf.chunks(PACKET_LEN as usize + 16) {
            let mut message = vec![0u8; buf.len()]; // move message outside the loop

            let nonce = self.nonce.wrapping_add(1) as _;

            self.transport
                .read_message(nonce, &buf, &mut message)
                .map_err(|e| err!(other, e.to_string()))?;
            bytes.append(&mut message);
        }
        Ok(bytes)
    }
}

/// Starts a new snow stream using the default noise parameters
pub async fn new(stream: &mut Channel) -> Result<StatelessTransportState> {
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
pub async fn new_with_params(
    chan: &mut Channel,
    noise_params: NoiseParams,
) -> Result<StatelessTransportState> {
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
) -> Result<StatelessTransportState> {
    dbg!();

    let mut initiator = snow::Builder::new(noise_params)
        .build_initiator()
        .map_err(err!(@other))?;
    let mut buffer_msg = vec![0u8; 128];
    let rand_payload: &[u8; 16] = &rand::random();

    let len = initiator
        .write_message(rand_payload, &mut buffer_msg)
        .map_err(err!(@other))?; // verified

    chan.send((&buffer_msg, len as u64)).await?;

    let (mut buffer_out, buffer_msg): (Vec<u8>, Vec<u8>) = chan.receive().await?;
    initiator
        .read_message(&buffer_msg, &mut buffer_out)
        .map_err(err!(@other))?;

    initiator
        .into_stateless_transport_mode()
        .map_err(err!(@other))
}

/// starts a new snow stream using the provided parameters.
pub(crate) async fn initialize_responder(
    chan: &mut Channel,
    noise_params: NoiseParams,
) -> Result<StatelessTransportState> {
    let mut responder = snow::Builder::new(noise_params)
        .build_responder()
        .map_err(err!(@other))?;
    let mut buffer_out = vec![0u8; 128];

    let (mut buffer_msg, len): (Vec<u8>, u64) = chan.receive().await?;
    responder
        .read_message(&buffer_msg[..len as usize], &mut buffer_out)
        .map_err(err!(@other))?;

    let rand_payload: &[u8; 16] = &rand::random();

    let len = responder
        .write_message(rand_payload, &mut buffer_msg)
        .map_err(err!(@other))?;
    chan.send((&buffer_out, &buffer_msg[..len])).await?;

    responder
        .into_stateless_transport_mode()
        .map_err(err!(@other))
}
