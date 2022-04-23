use crate::channel::WSS;
use derive_more::From;
use futures::stream::SplitStream;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{TcpStream, UnixStream, WriteHalf};

use crate::async_snow::Snow;

#[derive(From)]
pub enum SendChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted tcp backend
    Tcp(Snow<WriteHalf<TcpStream>>),
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    InsecureTcp(WriteHalf<TcpStream>),

    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted unix backend
    Unix(Snow<WriteHalf<UnixStream>>),
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted unix backend
    InsecureUnix(WriteHalf<UnixStream>),

    /// encrypted wss backend
    WSS(Snow<SplitStream<WSS>>),
    /// unencrypted wss backend
    InsecureWSS(SplitStream<WSS>),
}
