use crate::Result;
use cfg_if::cfg_if;
use compact_str::CompactStr;
use igcp::{err, Channel, Error};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Display, net::SocketAddr};

use super::{InsecureWss, Wss};

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use super::{InsecureTcp, Tcp};
        use crate::runtime::JoinHandle;

        #[cfg(unix)]
        use super::{InsecureUnix, Unix};
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
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

impl Into<String> for &Addr {
    #[inline]
    fn into(self) -> String {
        match self {
            Addr::Tcp(addr) => {
                format!("tcp@{}", addr)
            }
            Addr::Unix(addr) => {
                format!("unix@{}", addr.to_string_lossy())
            }
            Addr::InsecureTcp(addr) => {
                format!("itcp@{}", addr)
            }
            Addr::InsecureUnix(addr) => {
                format!("iunix@{}", addr.to_string_lossy())
            }
            Addr::Wss(addr) => {
                format!("wss@{}", addr)
            }
            Addr::InsecureWss(addr) => {
                format!("ws@{}", addr)
            }
        }
    }
}

impl Display for Addr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: String = self.into();
        f.write_str(&string)
    }
}

impl Debug for Addr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: String = self.into();
        f.write_str(&string)
    }
}

impl Serialize for Addr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string: String = self.into();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Addr {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = CompactStr::deserialize(deserializer)?;
        Addr::from_str(&string).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
/// Represents the full address of a service.
/// ```norun
/// let service = "tcp@127.0.0.1:8080://my_service".parse::<ServiceAddr>()?;
///
/// let chan = service.connect().await?;
/// ```
pub struct ServiceAddr(Addr, CompactStr);

impl Serialize for ServiceAddr {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = format!("{}://{}", self.0, self.1);
        string.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ServiceAddr {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = CompactStr::deserialize(deserializer)?;
        ServiceAddr::from_str(&string).map_err(serde::de::Error::custom)
    }
}

impl Into<String> for &ServiceAddr {
    #[inline]
    fn into(self) -> String {
        format!("{}://{}", self.0, self.1)
    }
}

impl Display for ServiceAddr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string: String = self.into();
        f.write_str(&string)
    }
}

impl ServiceAddr {
    #[inline]
    /// create a new service address from a string
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    #[inline]
    /// get the underlying address from the service
    pub fn addr(&self) -> &Addr {
        &self.0
    }
    #[inline]
    /// take the underlying address from the service
    pub fn take_addr(self) -> Addr {
        self.0
    }

    #[inline]
    /// connect to the service
    pub async fn connect(&self) -> Result<Channel> {
        self.0.connect(&self.1).await
    }
}

impl Addr {
    #[inline]
    /// create a new address from a string
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    #[inline]
    /// create a service address by tying the address to an id
    pub fn service(self, id: impl Into<CompactStr>) -> ServiceAddr {
        ServiceAddr(self, id.into())
    }
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    /// bind the address to the global route
    pub async fn bind(&self) -> Result<JoinHandle<Result<()>>> {
        cfg_if! {
            if #[cfg(unix)] {
                match self {
                    Addr::Tcp(addrs) => Tcp::bind(addrs.as_ref()).await,
                    Addr::InsecureTcp(addrs) => InsecureTcp::bind(addrs.as_ref()).await,
                    Addr::Unix(addrs) => Unix::bind(addrs.as_ref()).await,
                    Addr::InsecureUnix(addrs) => InsecureUnix::bind(addrs.as_ref()).await,
                    Addr::Wss(addrs) => Wss::bind(addrs.as_str()).await,
                    Addr::InsecureWss(addrs) => InsecureWss::bind(addrs.as_str()).await,
                }
            } else {
                match self {
                    Addr::Tcp(addrs) => Tcp::bind(addrs.as_ref()).await,
                    Addr::InsecureTcp(addrs) => InsecureTcp::bind(addrs.as_ref()).await,
                    Addr::Wss(addrs) => Wss::bind(addrs.as_str()).await,
                    Addr::InsecureWss(addrs) => InsecureWss::bind(addrs.as_str()).await,
                    Addr::Unix(_) => err!((
                        unsupported,
                        "binding an unix provider is not supported on non-unix platforms"
                    )),
                    Addr::InsecureUnix(_) => err!((
                        unsupported,
                        "binding an unix provider is not supported on non-unix platforms"
                    )),
                }
            }
        }
    }

    #[inline]
    /// connect to the address with the provided id
    pub async fn connect(&self, id: &str) -> Result<Channel> {
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                match self {
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str(), id).await,
                    Addr::InsecureWss(addrs) => InsecureWss::connect(addrs.as_str(), id).await,
                    Addr::Tcp(_) => err!((
                        unsupported,
                        "connecting to tcp providers is not supported on wasm"
                    )),
                    Addr::InsecureTcp(_) => err!((
                        unsupported,
                        "connecting to tcp providers is not supported on wasm"
                    )),
                    Addr::InsecureUnix(_) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on wasm"
                    )),
                    Addr::Unix(_) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on wasm"
                    )),
                }
            } else if #[cfg(unix)] {
                match self {
                    Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref(), id).await,
                    Addr::InsecureTcp(addrs) => InsecureTcp::connect(addrs.as_ref(), id).await,
                    Addr::Unix(addrs) => Unix::connect(addrs.as_ref(), id).await,
                    Addr::InsecureUnix(addrs) => InsecureUnix::connect(addrs.as_ref(), id).await,

                    Addr::Wss(addrs) => Wss::connect(addrs.as_str(), id).await,
                    Addr::InsecureWss(addrs) => InsecureWss::connect(addrs.as_str(), id).await,
                }
            } else {
                match self {
                    Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref(), id).await,
                    Addr::InsecureTcp(addrs) => InsecureTcp::connect(addrs.as_ref(), id).await,
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str(), id).await,
                    Addr::InsecureWss(addrs) => InsecureWss::connect(addrs.as_str(), id).await,
                    Addr::Unix(_) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on non-unix platforms"
                    )),
                    Addr::InsecureUnix(_) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on non-unix platforms"
                    )),
                }
            }
        }
    }
}

impl FromStr for Addr {
    type Err = Error;

    #[inline]
    /// unix@address.sock://cluster
    /// tcp@127.0.0.1:8092://cluster
    /// tcp@127.0.0.1:8092://cluster
    /// unix@folder/address.sock://cluster
    fn from_str(s: &str) -> Result<Self> {
        let (protocol, addr) = s
            .rsplit_once("@")
            .ok_or(err!(invalid_input, "malformed address"))?;
        let address_ty = protocol.parse::<AddressType>()?;
        Ok(match address_ty {
            AddressType::Tcp => {
                let addr = addr
                    .parse::<SocketAddr>()
                    .map_err(|e| err!(invalid_input, e))?;
                Addr::Tcp(Arc::new(addr))
            }
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

    #[inline]
    /// unix@address.sock://service
    /// tcp@127.0.0.1:8080://service
    fn from_str(s: &str) -> Result<Self> {
        let (addr, id) = s
            .rsplit_once("://")
            .ok_or(err!(invalid_input, "malformed service address"))?;
        let id = CompactStr::new_inline(id);
        let addr = addr.parse()?;
        Ok(ServiceAddr(addr, id))
    }
}

#[derive(Serialize, Deserialize)]
enum AddressType {
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "itcp")]
    InsecureTcp,
    #[serde(rename = "unix")]
    Unix,
    #[serde(rename = "iunix")]
    InsecureUnix,
    #[serde(rename = "wss")]
    Wss,
    #[serde(rename = "ws")]
    InsecureWss,
}

impl FromStr for AddressType {
    type Err = Error;

    #[inline]
    fn from_str(protocol: &str) -> Result<Self> {
        let protocol = match protocol {
            "tcp" => AddressType::Tcp,
            "itcp" => AddressType::InsecureTcp,
            "wss" => AddressType::Wss,
            "ws" => AddressType::InsecureWss,
            "unix" => AddressType::Unix,
            "iunix" => AddressType::InsecureUnix,
            protocol => err!((invalid_input, format!("unexpected protocol {:?}", protocol)))?,
        };
        Ok(protocol)
    }
}

impl AsRef<str> for AddressType {
    #[inline]
    fn as_ref(&self) -> &str {
        match self {
            AddressType::Tcp => "tcp",
            AddressType::InsecureTcp => "itcp",
            AddressType::Unix => "unix",
            AddressType::InsecureUnix => "iunix",
            AddressType::Wss => "wss",
            AddressType::InsecureWss => "ws",
        }
    }
}
