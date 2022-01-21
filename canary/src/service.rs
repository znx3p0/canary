#[cfg(not(target_arch = "wasm32"))]
use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::{spawn, JoinHandle};

#[cfg(not(target_arch = "wasm32"))]
use crate::routes::Ctx;

use crate::discovery::Status;
use crate::Result;
use igcp::BareChannel;
use igcp::Channel;

#[cfg(not(target_arch = "wasm32"))]
/// underlying service handle that is stored on a route
pub type Svc =
    Box<dyn Fn(BareChannel, Ctx, bool) -> JoinHandle<Result<()>> + Send + Sync + 'static>;

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

#[cfg(not(target_arch = "wasm32"))]
/// function used to create services from closures and functions
pub fn run_metadata<M, T, X, C>(meta: M, svc: X) -> Svc
where
    T: Future<Output = crate::Result<()>> + Send + 'static,
    C: From<BareChannel> + Send,
    X: Fn(M, C, Ctx) -> T,
    X: Send + Sync + 'static + Clone,
    M: Send + Clone + Sync + 'static,
{
    Box::new(move |chan, ctx, discover| {
        let meta = meta.clone();
        let svc = svc.clone();
        spawn(async move {
            let mut chan: Channel = chan.into();
            if discover {
                chan.send(Status::Found).await?;
                let chan = C::from(chan.bare());
                let svc = svc(meta, chan, ctx);
                svc.await?;
            } else {
                let chan = C::from(chan.bare());
                let svc = svc(meta, chan, ctx);
                svc.await?;
            }
            Ok(())
        })
    })
}
