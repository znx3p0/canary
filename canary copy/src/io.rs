use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "tokio-net"))] {
        #[cfg(unix)]
        pub use tokio::net::{UnixListener, UnixStream};
        pub use tokio::net::{TcpListener, TcpStream};
        pub use tokio::io::AsyncRead as Read;
        pub use tokio::io::AsyncReadExt as ReadExt;
        pub use tokio::io::AsyncWrite as Write;
        pub use tokio::io::AsyncWriteExt as WriteExt;
        pub use tokio::net::ToSocketAddrs;
    }
}

cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "async-std-net"))] {
        pub use async_std::io::Read;
        pub use async_std::io::ReadExt;
        pub use async_std::io::Write;
        pub use async_std::io::WriteExt;

        #[cfg(unix)]
        pub use async_std::os::unix::net::{UnixListener, UnixStream};
        pub use async_std::net::{TcpListener, TcpStream};
        pub use async_std::net::ToSocketAddrs;
    }
}

pub use async_tungstenite as wss;
