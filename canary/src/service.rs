use std::future::Future;

use crate::runtime::{self, spawn};
use igcp::BareChannel;

pub type Svc = Box<dyn Fn(BareChannel) + Send + Sync + 'static>;

pub trait Service {
    const ENDPOINT: &'static str;
    type Pipeline;
    type Meta;
    fn service(meta: Self::Meta) -> Svc;
}

pub trait StaticService {
    type Meta: Send + Sync + 'static;
    type Chan: From<BareChannel>;
    fn introduce(meta: Self::Meta, c: Self::Chan) -> runtime::JoinHandle<crate::Result<()>>;
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
