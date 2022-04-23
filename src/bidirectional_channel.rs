use derive_more::From;

use futures::{
    io::WriteHalf,
    stream::{SplitSink, SplitStream},
};
use tungstenite::Message;

use crate::{channel::WSS, send_channel::SendChannel, receive_channel::ReceiveChannel, Channel};

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{ReadHalf, TcpStream, UnixStream};

use crate::async_snow::Snow;

#[derive(From)]
pub enum BidirectionalChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted tcp backend
    Tcp(Snow<ReadHalf<TcpStream>>, Snow<WriteHalf<TcpStream>>),
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    InsecureTcp(ReadHalf<TcpStream>, WriteHalf<TcpStream>),

    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted unix backend
    Unix(Snow<ReadHalf<UnixStream>>, Snow<WriteHalf<UnixStream>>),
    #[cfg(unix)]
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted unix backend
    InsecureUnix(ReadHalf<UnixStream>, WriteHalf<UnixStream>),

    /// encrypted wss backend
    WSS(Snow<SplitStream<WSS>>, Snow<SplitSink<WSS, Message>>),
    /// unencrypted wss backend
    InsecureWSS(SplitStream<WSS>, SplitSink<WSS, Message>),

    Any(SendChannel, ReceiveChannel)
}

#[test]
fn p() {
    println!("{}", std::mem::size_of::<Channel>());
}
