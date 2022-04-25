use cfg_if::cfg_if;
use derive_more::From;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::TcpStream;

use crate::serialization::formats::Format;

use crate::type_iter::{MainChannel, PeerChannel, Pipeline};
use crate::Result;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// inner websocket type
        pub type Wss = reqwasm::websocket::futures::WebSocket;
    } else {
        /// inner websocket type
        pub type Wss = crate::io::wss::WebSocketStream<
            async_tungstenite::tokio::TokioAdapter<TcpStream>
        >;
    }
}

/// `Channel` abstracts network communications as object streams.
///
/// ```norun
/// async fn send_random(mut chan: Channel) -> Result {
///     chan.send(fastrand::u64(0..1000)).await?;
///     Ok(())
/// }
/// ```
pub type Channel = super::BidirectionalChannel;

impl Channel {
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
