use cfg_if::cfg_if;
use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use crate::serialization::formats::Format;

// use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
use crate::Result;

use super::unified::{UnformattedUnifiedChannel, UnifiedChannel};

/// `Channel` abstracts network communications as object streams.
///
/// ```norun
/// async fn send_random(mut chan: Channel) -> Result {
///     chan.send(fastrand::u64(0..1000)).await?;
///     Ok(())
/// }
/// ```
#[derive(From)]
pub enum Channel {
    Unified(crate::channel::unified::UnifiedChannel),
    Bipartite(crate::channel::bipartite::BidirectionalChannel),
}

impl Channel {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        match self {
            Channel::Unified(c) => c.send(obj).await,
            Channel::Bipartite(c) => c.send(obj).await,
        }
    }
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        match self {
            Channel::Unified(c) => c.receive().await,
            Channel::Bipartite(c) => c.receive().await,
        }
    }
}

// impl Channel {
// /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `PeerChannel`
// pub fn new_main<P: Pipeline>(self) -> MainChannel<P::Pipe> {
//     MainChannel(Default::default(), self)
// }
// /// construct a typed wrapper for a channel using pipelines, its asymmetric peer is `MainChannel`
// pub fn new_peer<P: Pipeline>(self) -> PeerChannel<P::Pipe> {
//     PeerChannel(Default::default(), self)
// }
// /// set the format of the channel
// pub fn set_format(&mut self, format: Format) {
//     self.format = format
// }
// }

impl Channel {
    pub fn new_tcp_raw(stream: crate::io::TcpStream) -> Self {
        let chan = UnifiedChannel {
            channel: UnformattedUnifiedChannel::Raw(From::from(stream)),
            format: Default::default(),
        };

        Channel::Unified(chan)
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
        // match self.0.inner {
        //     #[cfg(not(target_arch = "wasm32"))]
        //     InnerChannel::InsecureTcp(tcp) => Channel::new_tcp_encrypted(tcp).await,
        //     #[cfg(unix)]
        //     InnerChannel::InsecureUnix(unix) => Channel::new_unix_encrypted(unix).await,
        //     InnerChannel::InsecureWSS(wss) => Channel::new_wss_encrypted(wss).await,
        //     encrypted => Ok(encrypted.into()), // double encryption is not supported
        // }
        todo!()
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
