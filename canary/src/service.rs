#![cfg(not(target_arch = "wasm32"))]

use crate::discovery::Status;
use crate::routes::Ctx;
use crate::runtime::JoinHandle;
use crate::Result;
use async_channel::{Sender, Receiver};
use derive_more::From;
use igcp::{BareChannel, Channel, err};
use std::future::Future;

type ServiceMessage = (BareChannel, bool);

/// underlying service handle that is stored on a route
pub type Svc = Sender<ServiceMessage>;

#[derive(From)]
pub struct ServiceHandle {
    chan: Receiver<ServiceMessage>,
}

impl ServiceHandle {
    pub async fn next(&mut self) -> Result<Channel> {
        let (channel, discover) = self.chan.recv().await
            .map_err(|e| err!(e))?;
        if discover {
            let mut channel: Channel = channel.into();
            channel.send(Status::Found).await?;
            return Ok(channel)
        }
        Ok(channel.into())
    }
}

/// Services are backed by this trait.
/// Services fundamentally only have metadata,
/// but this trait also offers extra data such as
/// the pipeline associated with the service and the
/// endpoint of it.
///
/// A manual service can be written as follows:
/// ```norun
/// struct Ping;
/// impl canary::service::Service for Ping {
///     const ENDPOINT: &'static str = "ping";
///     type Pipeline = ();
///     type Meta = ();
///     fn service(meta: Self::Meta) -> canary::service::Svc {
///         canary::service::run_metadata(
///             meta, |_, mut chan: Channel, _| async move {
///                 chan.send(123).await?;
///                 Ok(())
///             }
///         )
///     }
/// }
/// ```
pub trait Service {
    /// endpoint of the service
    const ENDPOINT: &'static str;
    /// pipeline associated with the service
    type Pipeline;
    /// metadata of the service
    type Meta;
    /// create a new service from the metadata
    fn service(meta: Self::Meta) -> Svc;
}
