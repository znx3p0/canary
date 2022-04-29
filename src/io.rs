use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        #[cfg(unix)]
        pub use tokio::net::{UnixListener, UnixStream};
        pub use tokio::net::{TcpListener, TcpStream, UdpSocket};
        pub use tokio::io::AsyncRead as Read;
        pub use tokio::io::AsyncReadExt as ReadExt;
        pub use tokio::io::AsyncWrite as Write;
        pub use tokio::io::AsyncWriteExt as WriteExt;
        pub use tokio::io::WriteHalf;
        pub use tokio::io::ReadHalf;
        pub use tokio::io::split;

        pub use tokio::net::ToSocketAddrs;

        pub(crate) use tokio::time::sleep;
        pub use async_tungstenite as wss;

        pub type Wss = crate::io::wss::WebSocketStream<
            async_tungstenite::tokio::TokioAdapter<TcpStream>
        >;
    } else if #[cfg(target_arch = "wasm32")] {
        pub use futures::io::AsyncRead as Read;
        pub use futures::io::AsyncReadExt as ReadExt;
        pub use futures::io::AsyncWrite as Write;
        pub use futures::io::AsyncWriteExt as WriteExt;
        pub type Wss = reqwasm::websocket::futures::WebSocket;

    }
}
