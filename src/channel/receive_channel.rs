use derive_more::From;
use futures::stream::SplitStream;
use serde::de::DeserializeOwned;

use crate::serialization::formats::Format;
use crate::Result;
use crate::{channel::Wss, serialization::formats::ReadFormat};

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{ReadHalf, TcpStream, UnixStream};

use crate::async_snow::Snow;

// You may notice that most types are boxed. This is to avoid unnecessary padding since
// inner types can vary from 8 bytes all the way to 128 bytes.
// If types weren't boxed and you were using InsecureTcp, you would waste 112 bytes per send channel.
#[derive(From)]
pub enum UnformattedReceiveChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    Tcp(ReadHalf<TcpStream>), // doesn't need box since it's less or equal to 16 bytes
    #[cfg(unix)]
    /// unencrypted unix backend
    Unix(ReadHalf<UnixStream>), // doesn't need box since it's less or equal to 16 bytes
    /// unencrypted wss backend
    WSS(SplitStream<Wss>), // doesn't need box since it's less or equal to 16 bytes
}

impl UnformattedReceiveChannel {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, f: &F) -> Result<T> {
        todo!()
        // match self {
        //     #[cfg(not(target_arch = "wasm32"))]
        //     UnformattedReceiveChannel::Tcp(st) => st.rx(f).await,
        //     #[cfg(not(target_arch = "wasm32"))]
        //     UnformattedReceiveChannel::InsecureTcp(st) => crate::serialization::rx(st, f).await,
        //     #[cfg(unix)]
        //     UnformattedReceiveChannel::Unix(st) => st.rx(f).await,
        //     #[cfg(unix)]
        //     UnformattedReceiveChannel::InsecureUnix(st) => crate::serialization::rx(st, f).await,
        //     UnformattedReceiveChannel::Wss(st) => st.wss_rx(f).await,
        //     UnformattedReceiveChannel::InsecureWSS(st) => crate::serialization::wss_rx(st, f).await,
        // }
    }
    pub fn to_formatted<F: ReadFormat>(self, format: F) -> ReceiveChannel<F> {
        ReceiveChannel {
            channel: self,
            format,
        }
    }
}

#[derive(From)]
pub struct ReceiveChannel<F: ReadFormat = Format> {
    channel: UnformattedReceiveChannel,
    format: F,
}

impl<F: ReadFormat> ReceiveChannel<F> {
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.channel.receive(&self.format).await
    }
}
