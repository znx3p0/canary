use crate::Result;
use compact_str::CompactStr;
use igcp::{err, Channel, Error};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use super::{InsecureTcp, Tcp};
#[cfg(unix)]
use super::{InsecureUnix, Unix};
use crate::runtime::JoinHandle;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub enum Addr {
    Tcp(Arc<SocketAddr>),
    #[cfg(unix)]
    Unix(Arc<PathBuf>),
    InsecureTcp(Arc<SocketAddr>),
    #[cfg(unix)]
    InsecureUnix(Arc<PathBuf>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct ServiceAddr(Addr, CompactStr);

impl ServiceAddr {
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    pub fn addr(&self) -> &Addr {
        &self.0
    }
    pub fn take_addr(self) -> Addr {
        self.0
    }
    pub async fn connect(&self) -> Result<Channel> {
        match &self.0 {
            Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref(), &self.1).await,
            Addr::InsecureTcp(addrs) => InsecureTcp::connect(addrs.as_ref(), &self.1).await,

            #[cfg(unix)]
            Addr::Unix(addrs) => Unix::connect(addrs.as_ref(), &self.1).await,
            #[cfg(unix)]
            Addr::InsecureUnix(addrs) => InsecureUnix::connect(addrs.as_ref(), &self.1).await,
        }
    }
}

impl Addr {
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    pub fn service(self, id: impl Into<CompactStr>) -> ServiceAddr {
        ServiceAddr(self, id.into())
    }
    pub async fn bind(&self) -> Result<JoinHandle<Result<()>>> {
        match self {
            Addr::Tcp(addrs) => Tcp::bind(addrs.as_ref()).await,
            Addr::InsecureTcp(addrs) => InsecureTcp::bind(addrs.as_ref()).await,
            #[cfg(unix)]
            Addr::Unix(addrs) => Unix::bind(addrs.as_ref()).await,
            #[cfg(unix)]
            Addr::InsecureUnix(addrs) => InsecureUnix::bind(addrs.as_ref()).await,
        }
    }
    pub async fn connect(&self, id: &str) -> Result<Channel> {
        match self {
            Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref(), id).await,
            Addr::InsecureTcp(addrs) => InsecureTcp::connect(addrs.as_ref(), id).await,
            #[cfg(unix)]
            Addr::Unix(addrs) => Unix::connect(addrs.as_ref(), id).await,
            #[cfg(unix)]
            Addr::InsecureUnix(addrs) => InsecureUnix::connect(addrs.as_ref(), id).await,
        }
    }
}

impl FromStr for Addr {
    type Err = Error;

    /// unix@address.sock
    /// tcp@127.0.0.1:8092
    /// cluster://tcp@127.0.0.1:8092
    /// cluster://unix@a/address.sock
    fn from_str(s: &str) -> Result<Self> {
        let mut s = s.split("@");
        let address_ty = {
            let protocol = s.next().ok_or(err!(invalid_input, "protocol not found"))?;
            match protocol {
                "tcp" => AddressType::Tcp,
                "itcp" => AddressType::InsecureTcp,
                #[cfg(unix)]
                "unix" => AddressType::Unix,
                #[cfg(unix)]
                "iunix" => AddressType::InsecureUnix,
                #[cfg(not(unix))]
                "unix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
                #[cfg(not(unix))]
                "iunix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
                protocol => err!((invalid_input, format!("unexpected protocol {:?}", protocol)))?,
            }
        };
        let addr = s.next().ok_or(err!(invalid_input, "address not found"))?;
        Ok(match address_ty {
            AddressType::Tcp => {
                let addr = addr
                    .parse::<SocketAddr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Tcp(Arc::new(addr))
            }
            #[cfg(unix)]
            AddressType::Unix => {
                let addr = addr
                    .parse::<PathBuf>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Unix(Arc::new(addr))
            }
            AddressType::InsecureTcp => {
                let addr = addr
                    .parse::<SocketAddr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureTcp(Arc::new(addr))
            }
            #[cfg(unix)]
            AddressType::InsecureUnix => {
                let addr = addr
                    .parse::<PathBuf>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureUnix(Arc::new(addr))
            }
        })
    }
}

impl FromStr for ServiceAddr {
    type Err = Error;

    /// cluster://unix@address.sock
    /// cluster://tcp@127.0.0.1:8080
    fn from_str(s: &str) -> Result<Self> {
        let mut s = s.split("://");
        let id = s
            .next()
            .ok_or(err!(invalid_input, "id of service not found"))?;
        let id = CompactStr::new_inline(id);

        let mut s = s
            .next()
            .ok_or(err!(invalid_input, "id of service not found"))?
            .split("@");
        let address_ty = {
            let protocol = s.next().ok_or(err!(invalid_input, "protocol not found"))?;
            match protocol {
                "tcp" => AddressType::Tcp,
                "itcp" => AddressType::InsecureTcp,
                #[cfg(unix)]
                "unix" => AddressType::Unix,
                #[cfg(unix)]
                "iunix" => AddressType::InsecureUnix,
                #[cfg(not(unix))]
                "unix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
                #[cfg(not(unix))]
                "iunix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
                protocol => err!((invalid_input, format!("unexpected protocol {:?}", protocol)))?,
            }
        };
        let addr = s.next().ok_or(err!(invalid_input, "address not found"))?;
        let addr = match address_ty {
            AddressType::Tcp => {
                let addr = addr
                    .parse::<SocketAddr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Tcp(Arc::new(addr))
            }
            #[cfg(unix)]
            AddressType::Unix => {
                let addr = addr
                    .parse::<PathBuf>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Unix(Arc::new(addr))
            }
            AddressType::InsecureTcp => {
                let addr = addr
                    .parse::<SocketAddr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureTcp(Arc::new(addr))
            }
            #[cfg(unix)]
            AddressType::InsecureUnix => {
                let addr = addr
                    .parse::<PathBuf>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureUnix(Arc::new(addr))
            }
        };
        Ok(ServiceAddr(addr, id))
    }
}

enum AddressType {
    Tcp,
    InsecureTcp,
    #[cfg(unix)]
    Unix,
    #[cfg(unix)]
    InsecureUnix,
}
