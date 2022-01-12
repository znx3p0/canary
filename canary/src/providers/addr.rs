use crate::Result;
use compact_str::CompactStr;
use igcp::{err, Channel, Error};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::{InsecureTcp, Tcp};

use super::{InsecureWss, Wss};

#[cfg(unix)]
#[cfg(not(target_arch = "wasm32"))]
use super::{InsecureUnix, Unix};

#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::JoinHandle;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
/// Represents the address of a provider.
/// ```norun
/// let tcp = "tcp@127.0.0.1:8080".parse::<Addr>()?;
/// let unix = "unix@mysocket.sock".parse::<Addr>()?;
/// let insecure_tcp = "itcp@127.0.0.1:8080".parse::<Addr>()?;
/// let insecure_unix = "iunix@mysocket.sock".parse::<Addr>()?;
///
/// tcp.bind().await?; // bind all addresses to the global route
/// unix.bind().await?;
/// insecure_tcp.bind().await?;
/// insecure_unix.bind().await?;
/// ```
pub enum Addr {
    /// tcp provider
    Tcp(Arc<SocketAddr>),
    /// unix provider
    Unix(Arc<PathBuf>),
    /// unencrypted tcp provider
    InsecureTcp(Arc<SocketAddr>),
    /// unencrypted unix provider
    InsecureUnix(Arc<PathBuf>),
    /// websocket provider
    Wss(Arc<CompactStr>),
    /// unencrypted websocket provider
    InsecureWss(Arc<CompactStr>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
/// Represents the full address of a service.
/// ```norun
/// let service = "my_service://tcp@127.0.0.1:8080".parse::<ServiceAddr>()?;
///
/// let chan = service.connect().await?;
/// ```
pub struct ServiceAddr(Addr, CompactStr);

impl ServiceAddr {
    /// create a new service address from a string
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    /// get the underlying address from the service
    pub fn addr(&self) -> &Addr {
        &self.0
    }
    /// take the underlying address from the service
    pub fn take_addr(self) -> Addr {
        self.0
    }

    /// connect to the service
    pub async fn connect(&self) -> Result<Channel> {
        self.0.connect(&self.1).await
    }
}

impl Addr {
    /// create a new address from a string
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    /// create a service address by tying the address to an id
    pub fn service(self, id: impl Into<CompactStr>) -> ServiceAddr {
        ServiceAddr(self, id.into())
    }
    /// bind the address to the global route
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn bind(&self) -> Result<JoinHandle<Result<()>>> {
        match self {
            Addr::Tcp(addrs) => Tcp::bind(addrs.as_ref()).await,
            Addr::InsecureTcp(addrs) => InsecureTcp::bind(addrs.as_ref()).await,
            #[cfg(unix)]
            Addr::Unix(addrs) => Unix::bind(addrs.as_ref()).await,
            #[cfg(unix)]
            Addr::InsecureUnix(addrs) => InsecureUnix::bind(addrs.as_ref()).await,
            Addr::Wss(addrs) => Wss::bind(addrs.as_str()).await,
            Addr::InsecureWss(addrs) => InsecureWss::bind(addrs.as_str()).await,
        }
    }

    /// connect to the address with the provided id
    pub async fn connect(&self, id: &str) -> Result<Channel> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref(), id).await,
            #[cfg(not(target_arch = "wasm32"))]
            Addr::InsecureTcp(addrs) => InsecureTcp::connect(addrs.as_ref(), id).await,
            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            Addr::Unix(addrs) => Unix::connect(addrs.as_ref(), id).await,
            #[cfg(unix)]
            #[cfg(not(target_arch = "wasm32"))]
            Addr::InsecureUnix(addrs) => InsecureUnix::connect(addrs.as_ref(), id).await,

            Addr::Wss(addrs) => Wss::connect(addrs.as_str(), id).await,
            Addr::InsecureWss(addrs) => InsecureWss::connect(addrs.as_str(), id).await,

            #[cfg(target_arch = "wasm32")]
            Addr::Tcp(_) => err!((
                unsupported,
                "connecting to tcp providers is not supported on wasm"
            )),
            #[cfg(target_arch = "wasm32")]
            Addr::InsecureTcp(_) => err!((
                unsupported,
                "connecting to tcp providers is not supported on wasm"
            )),

            #[cfg(target_arch = "wasm32")]
            Addr::InsecureUnix(_) => err!((
                unsupported,
                "connecting to unix providers is not supported on wasm"
            )),
            #[cfg(target_arch = "wasm32")]
            Addr::Unix(_) => err!((
                unsupported,
                "connecting to unix providers is not supported on wasm"
            )),
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
            protocol.parse::<AddressType>()?
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
            AddressType::Wss => {
                let addr = addr
                    .parse::<CompactStr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Wss(Arc::new(addr))
            }
            AddressType::InsecureWss => {
                let addr = addr
                    .parse::<CompactStr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureWss(Arc::new(addr))
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
            protocol.parse::<AddressType>()?
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
            AddressType::Wss => {
                let addr = addr
                    .parse::<CompactStr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Wss(Arc::new(addr))
            }
            AddressType::InsecureWss => {
                let addr = addr
                    .parse::<CompactStr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::InsecureWss(Arc::new(addr))
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
    Wss,
    InsecureWss,
}

impl FromStr for AddressType {
    type Err = Error;

    fn from_str(protocol: &str) -> Result<Self> {
        let protocol = match protocol {
            "tcp" => AddressType::Tcp,
            "itcp" => AddressType::InsecureTcp,
            "wss" => AddressType::Wss,
            "ws" => AddressType::InsecureWss,
            #[cfg(unix)]
            "unix" => AddressType::Unix,
            #[cfg(unix)]
            "iunix" => AddressType::InsecureUnix,
            #[cfg(not(unix))]
            "unix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
            #[cfg(not(unix))]
            "iunix" => err!((unsupported, "Unix is not supported on non-unix targets"))?,
            protocol => err!((invalid_input, format!("unexpected protocol {:?}", protocol)))?,
        };
        Ok(protocol)
    }
}
