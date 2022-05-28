// #![cfg(not(target_arch = "wasm32"))]

// use crate::channel::Handshake;
// use crate::err;
// use crate::io::TcpListener;
// use crate::io::TcpStream;
// use crate::io::ToSocketAddrs;
// use crate::Channel;
// use crate::Result;

// use derive_more::{From, Into};

// #[derive(From, Into)]
// #[into(owned, ref, ref_mut)]
// /// Exposes routes over TCP
// pub struct Tcp(TcpListener);

// impl Tcp {
//     #[inline]
//     /// Bind the global route on the given address
//     pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
//         let listener = TcpListener::bind(addrs).await?;
//         Ok(Tcp(listener))
//     }
//     #[inline]
//     /// Get the next channel
//     /// ```norun
//     /// while let Ok(chan) = tcp.next().await {
//     ///     let mut chan = chan.encrypted().await?;
//     ///     chan.send("hello!").await?;
//     /// }
//     /// ```
//     pub async fn next(&self) -> Result<Handshake> {
//         let (chan, _) = self.0.accept().await?;
//         let chan: Channel = Channel::from(chan);
//         Ok(Handshake::from(chan))
//     }
//     #[inline]
//     /// Connect to the following address without discovery
//     pub async fn connect(
//         addrs: impl ToSocketAddrs + std::fmt::Debug,
//         retries: u32,
//         time_to_retry: u64,
//     ) -> Result<Handshake> {
//         let mut attempt = 0;
//         let stream = loop {
//             match TcpStream::connect(&addrs).await {
//                 Ok(s) => break s,
//                 Err(e) => {
//                     tracing::error!(
//                         "connecting to address `{:?}` failed, attempt {} starting",
//                         addrs,
//                         attempt
//                     );
//                     crate::io::sleep(std::time::Duration::from_millis(time_to_retry)).await;
//                     attempt += 1;
//                     if attempt == retries {
//                         err!((e))?
//                     }
//                     continue;
//                 }
//             }
//         };
//         let chan = Channel::from(stream);
//         Ok(Handshake::from(chan))
//     }
//     #[inline]
//     /// Connect to the following address with the following id. Defaults to 3 retries.
//     pub async fn connect(addrs: impl ToSocketAddrs + std::fmt::Debug) -> Result<Handshake> {
//         Self::connect_retry(addrs, 3, 10).await
//     }
//     #[inline]
//     /// Connect to the following address with the given id and retry in case of failure
//     pub async fn connect_retry(
//         addrs: impl ToSocketAddrs + std::fmt::Debug,
//         retries: u32,
//         time_to_retry: u64,
//     ) -> Result<Handshake> {
//         Self::connect(&addrs, retries, time_to_retry).await
//     }
// }

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
    /// Bind the global route on the given address
    pub async fn bind(addrs: impl ToSocketAddrs) -> Result<Self> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(Tcp(listener))
    }

    pub async fn next(&self) -> Result<Handshake> {
        let (stream, _) = self.0.accept().await?;
        Ok(Handshake::from(Channel::from_raw(
            stream,
            Default::default(),
            Default::default(),
        )))
    }
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
