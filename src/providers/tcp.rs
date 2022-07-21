#![cfg(not(target_arch = "wasm32"))]

use crate::channel::handshake::Handshake;
use crate::io::TcpListener;
use crate::io::TcpStream;
use crate::io::ToSocketAddrs;
use crate::Channel;
use crate::Result;

use backoff::ExponentialBackoff;
use derive_more::{From, Into};

#[derive(From, Into)]
#[into(owned, ref, ref_mut)]
#[repr(transparent)]
/// Exposes routes over TCP
pub struct Tcp(TcpListener);

impl Tcp {
    #[inline]
    /// Bind to this address
    /// ```no_run
    /// let tcp = Tcp::bind("127.0.0.1:8080").await?;
    /// while let Ok(chan) = tcp.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(Tcp(listener))
    }

    #[inline]
    /// get the next channel
    /// ```no_run
    /// while let Ok(chan) = tcp.next().await {
    ///     let mut chan = chan.encrypted().await?;
    ///     chan.send("hello!").await?;
    /// }
    /// ```
    pub async fn next(&self) -> Result<Handshake> {
        let (stream, _) = self.0.accept().await?;
        Ok(Handshake::from(Channel::from_raw(
            stream,
            Default::default(),
            Default::default(),
        )))
    }
    /// connect to address without any backoff strategy
    pub async fn connect_no_backoff(
        addrs: impl ToSocketAddrs + std::fmt::Debug,
    ) -> Result<Handshake> {
        let stream = TcpStream::connect(&addrs).await?;
        Ok(Handshake::from(Channel::from_raw(
            stream,
            Default::default(),
            Default::default(),
        )))
    }
    #[inline]
    /// Connect to the following address with the given id and retry in case of failure
    pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
        let hs = backoff::future::retry(ExponentialBackoff::default(), || async {
            let stream = TcpStream::connect(&addrs).await?;
            Ok(Handshake::from(Channel::from_raw(
                stream,
                Default::default(),
                Default::default(),
            )))
        })
        .await?;
        Ok(hs)
    }
}
