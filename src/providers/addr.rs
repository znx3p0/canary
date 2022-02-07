use crate::{err, Error};
use crate::{Channel, Result};
use cfg_if::cfg_if;
use compact_str::CompactStr;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Display, net::SocketAddr};

use crate::providers::Wss;

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use crate::providers::Tcp;
        #[cfg(unix)]
        use crate::providers::Unix;
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
        let string: CompactStr = CompactStr::deserialize(deserializer)?;
        Addr::from_str(&string).map_err(serde::de::Error::custom)
    }
}

impl Addr {
    #[inline]
    /// create a new address from a string
    pub fn new(addr: &str) -> Result<Self> {
        addr.parse()
    }
    #[inline]
    /// connect to the address
    pub async fn connect(&self) -> Result<Channel> {
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                match self {
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str()).await?.encrypted().await,
                    Addr::InsecureWss(addrs) => Ok(Wss::connect(addrs.as_str()).await?.raw()),
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
                    Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref()).await?.encrypted().await,
                    Addr::InsecureTcp(addrs) => Ok(Tcp::connect(addrs.as_ref()).await?.raw()),
                    Addr::Unix(addrs) => Unix::connect(addrs.as_ref()).await?.encrypted().await,
                    Addr::InsecureUnix(addrs) => Ok(Unix::connect(addrs.as_ref()).await?.raw()),
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str()).await?.encrypted().await,
                    Addr::InsecureWss(addrs) => Ok(Wss::connect(addrs.as_str()).await?.raw()),
                }
            } else {
                match self {
                    Addr::Tcp(addrs) => Tcp::connect(addrs.as_ref()).await?.encrypted().await,
                    Addr::InsecureTcp(addrs) => Ok(Tcp::connect(addrs.as_ref()).await?.raw()),
                    Addr::Wss(addrs) => Wss::connect(addrs.as_str()).await?.encrypted().await,
                    Addr::InsecureWss(addrs) => Ok(Wss::connect(addrs.as_str()).await?.raw()),

                    Addr::Unix(addrs) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on non-unix platforms"
                    )),
                    Addr::InsecureUnix(addrs) => err!((
                        unsupported,
                        "connecting to unix providers is not supported on non-unix platforms"
                    )),,
                }
            }
        }
    }
}

impl FromStr for Addr {
    type Err = Error;

    #[inline]
    /// unix@address.sock
    /// tcp@127.0.0.1:8092
    /// tcp@127.0.0.1:8092
    /// unix@folder/address.sock
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
