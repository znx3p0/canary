use std::future::Future;

use crate::runtime::{self, spawn};
use igcp::BareChannel;

/// underlying service handle that is stored on a route
pub type Svc = Box<dyn Fn(BareChannel) + Send + Sync + 'static>;

/// Services are backed by this trait.
/// Services fundamentally only have metadata,
/// but this trait also offers extra data such as
/// the pipeline associated with the service and the
/// endpoint of it.
///
/// A manual service should be written like this:
/// ```norun
/// struct MyService;
/// impl Service for MyService {
///     const ENDPOINT: &'static str = "MyService";
///     type Pipeline = ();
///     type Meta = ();
///     fn service(meta: ()) -> Svc {
///         async fn inner(_: (), mut channel: Channel) -> Result<()> {
///             channel.send("hello!").await?;
///             Ok(())
///         }
///         ::canary::service::run_metadata(meta, inner)
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

pub trait StaticService {
    type Meta: Send + Sync + 'static;
    type Chan: From<BareChannel>;
    fn introduce(meta: Self::Meta, c: Self::Chan) -> runtime::JoinHandle<crate::Result<()>>;
}

/// function used to create services from closures and functions
pub fn run_metadata<M, T, X, C>(meta: M, s: X) -> Svc
where
    T: Future<Output = crate::Result<()>> + Send + 'static,
    C: From<BareChannel>,
    X: Fn(M, C) -> T,
    X: Send + Sync + 'static,
    M: Send + Clone + Sync + 'static,
{
    Box::new(move |chan| {
        let s = s(meta.clone(), C::from(chan));
        spawn(async move {
            s.await.ok();
        });
    })
}
