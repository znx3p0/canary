use derive_more::From;
use futures::stream::SplitSink;
use tungstenite::Message;

use crate::channel::WSS;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{ReadHalf, TcpStream, UnixStream};

use crate::async_snow::Snow;

#[derive(From)]
pub enum ReceiveChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted tcp backend
    Tcp(Snow<ReadHalf<TcpStream>>),
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    InsecureTcp(ReadHalf<TcpStream>),

    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted unix backend
    Unix(Snow<ReadHalf<UnixStream>>),
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted unix backend
    InsecureUnix(ReadHalf<UnixStream>),

    /// encrypted wss backend
    WSS(Snow<SplitSink<WSS, Message>>),
    /// unencrypted wss backend
    InsecureWSS(SplitSink<WSS, Message>),
}
