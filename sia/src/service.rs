use std::future::Future;

use crate::runtime::spawn;
use igcp::BareChannel;

pub type Svc = Box<dyn Fn(BareChannel) + Send + Sync + 'static>;

pub trait Service {
    const ENDPOINT: &'static str;
    type Pipeline;
    type Meta;
    fn service(meta: Self::Meta) -> Svc;
}

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
