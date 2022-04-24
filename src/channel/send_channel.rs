use crate::{
    channel::Wss,
    serialization::formats::{Format, SendFormat},
    Result,
};
use derive_more::From;
use futures::stream::SplitSink;
use serde::Serialize;
use tungstenite::Message;

#[cfg(not(target_arch = "wasm32"))]
use crate::io::{TcpStream, UnixStream, WriteHalf};

use crate::async_snow::Snow;

// You may notice that most types are boxed. This is to avoid unnecessary padding since
// inner types can vary from 4 bytes all the way to 176 bytes.
// If types weren't boxed and you were using InsecuireTcp, you would waste 160 bytes per send channel.
#[derive(From)]
pub enum UnformattedSendChannel {
    #[cfg(not(target_arch = "wasm32"))]
    /// encrypted chan backend
    Tcp(Box<Snow<WriteHalf<TcpStream>>>),
    #[cfg(not(target_arch = "wasm32"))]
    /// unencrypted tcp backend
    InsecureTcp(WriteHalf<TcpStream>),   // doesn't need box since it's less or equal to 16 bytes

    #[cfg(unix)]
    /// encrypted unix backend
    Unix(Box<Snow<WriteHalf<UnixStream>>>),
    #[cfg(unix)]
    /// unencrypted unix backend
    InsecureUnix(WriteHalf<UnixStream>), // doesn't need box since it's less or equal to 16 bytes

    /// encrypted wss backend
    Wss(Box<Snow<SplitSink<Wss, Message>>>),
    /// unencrypted wss backend
    InsecureWSS(Box<SplitSink<Wss, Message>>),
}

impl UnformattedSendChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, f: &F) -> Result<usize> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedSendChannel::Tcp(st) => st.tx(obj, f).await,
            #[cfg(not(target_arch = "wasm32"))]
            UnformattedSendChannel::InsecureTcp(st) => crate::serialization::tx(st, obj, f).await,
            #[cfg(unix)]
            UnformattedSendChannel::Unix(st) => st.tx(obj, f).await,
            #[cfg(unix)]
            UnformattedSendChannel::InsecureUnix(st) => crate::serialization::tx(st, obj, f).await,
            UnformattedSendChannel::Wss(st) => st.wss_tx(obj, f).await,
            UnformattedSendChannel::InsecureWSS(st) => {
                crate::serialization::wss_tx(st, obj, f).await
            }
        }
    }
}


#[derive(From)]
pub struct SendChannel<F: SendFormat = Format> {
    channel: UnformattedSendChannel,
    format: F,
}

impl SendChannel {
    pub async fn send<T: Serialize>(&mut self, obj: T) -> Result<usize> {
        self.channel.send(obj, &self.format).await
    }
}
