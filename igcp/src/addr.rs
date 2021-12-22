use std::net::SocketAddr;

use crate::err;
use crate::serialization::formats::ReadFormat;
use crate::serialization::formats::SendFormat;
use crate::sia::Status;
use crate::Channel;
use crate::Result;
use async_std::net::TcpStream;
use async_std::os::unix::net::UnixStream;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub enum Addr {
    Tcp(SocketAddr),
    Unix(PathBuf),
    InsecureTcp(SocketAddr),
    InsecureUnix(PathBuf),
    EncryptedAny,
    InsecureAny,
}

impl<ReadFmt: ReadFormat, SendFmt: SendFormat> Channel<ReadFmt, SendFmt> {
    pub fn addr(&self) -> Result<Addr> {
        match self {
            Channel::Tcp(s) => {
                let addr = s.stream.peer_addr()?;
                Ok(Addr::Tcp(addr))
            }
            Channel::InsecureTcp(s) => Ok(Addr::InsecureTcp(s.peer_addr()?)),
            Channel::Unix(s) => {
                let path = s
                    .stream
                    .peer_addr()?
                    .as_pathname()
                    .ok_or(err!(invalid_data, "invalid address"))?
                    .to_path_buf();
                Ok(Addr::Unix(path))
            }
            Channel::InsecureUnix(s) => {
                let path = s
                    .peer_addr()?
                    .as_pathname()
                    .ok_or(err!(invalid_data, "invalid address"))?
                    .to_path_buf();
                Ok(Addr::InsecureUnix(path))
            }
            Channel::__InternalPhantomData__(_) => unreachable!(),
            _ => err!(("dynamic channels don't support addresses")),
        }
    }
}

impl Addr {
    pub async fn raw_connect<ReadFmt: ReadFormat, SendFmt: SendFormat>(
        &self,
    ) -> Result<Channel<ReadFmt, SendFmt>> {
        match self {
            Addr::Tcp(addrs) => {
                let chan = TcpStream::connect(addrs).await?;
                let chan = Channel::new_tcp_encrypted(chan).await?;
                Ok(chan.into())
            }
            Addr::InsecureTcp(addrs) => {
                let chan = TcpStream::connect(addrs).await?;
                Ok(chan.into())
            }
            Addr::Unix(addrs) => {
                let chan = UnixStream::connect(addrs).await?;
                let chan = Channel::new_unix_encrypted(chan).await?;
                Ok(chan.into())
            }
            Addr::InsecureUnix(addrs) => {
                let chan = UnixStream::connect(addrs).await?;
                Ok(chan.into())
            }
            _ => err!((unsupported, "dynamic addresses are not supported")),
        }
    }
    /// for use in sia. should not be used outside of sia, consider using `raw_connect` for other purposes.
    pub async fn connect(&self, id: &str) -> Result<Channel> {
        let mut chan = self.raw_connect().await?;
        chan.tx(id).await?;
        match chan.rx().await? {
            Status::Found => Ok(chan),
            Status::NotFound => err!((not_found, format!("service of id `{}` not found", id))),
        }
    }
}
