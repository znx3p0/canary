#![allow(unused_imports)]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        #[cfg(unix)]
        pub(crate) use tokio::net::{UnixListener, UnixStream};
        pub(crate) use tokio::net::{TcpListener, TcpStream, UdpSocket};
        pub(crate) use tokio::io::AsyncRead as Read;
        pub(crate) use tokio::io::AsyncReadExt as ReadExt;
        pub(crate) use tokio::io::AsyncWrite as Write;
        pub(crate) use tokio::io::AsyncWriteExt as WriteExt;
        pub(crate) use tokio::io::WriteHalf;
        pub(crate) use tokio::io::ReadHalf;
        pub(crate) use tokio::io::split;

        pub(crate) use tokio::net::ToSocketAddrs;

        pub(crate) use tokio::time::sleep;
        pub(crate) use async_tungstenite as wss;

        pub(crate) type Wss = crate::io::wss::WebSocketStream<
            async_tungstenite::tokio::TokioAdapter<TcpStream>
        >;
        pub(crate) type Message = tungstenite::Message;
    } else if #[cfg(target_arch = "wasm32")] {
        pub(crate) use futures::io::AsyncRead as Read;
        pub(crate) use futures::io::AsyncReadExt as ReadExt;
        pub(crate) use futures::io::AsyncWrite as Write;
        pub(crate) use futures::io::AsyncWriteExt as WriteExt;
        pub(crate) type Wss = reqwasm::websocket::futures::WebSocket;
        pub(crate) type Message = reqwasm::websocket::Message;
    }
}
