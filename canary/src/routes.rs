use compact_str::CompactStr;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::sync::Arc;

use camino::Utf8Path;
use dashmap::DashMap;
use igcp::{err, BareChannel, Channel};
use once_cell::sync::Lazy;

use crate::runtime::spawn;
use crate::service::{Service, Svc};
use crate::Result;

type RouteKey = CompactStr;
#[derive(Default)]
pub struct Route(DashMap<RouteKey, Storable>);

enum Storable {
    Route(Arc<Route>),
    Service(Svc),
}

pub trait RegisterEndpoint {
    const ENDPOINT: &'static str;
}

pub trait Register: RegisterEndpoint {
    type Meta;
    fn register(top_route: &Route, meta: Self::Meta) -> Result<()>;
}

#[derive(Serialize_repr, Deserialize_repr)]
// used for discovery
#[repr(u8)]
pub enum Status {
    Found = 1,
    NotFound = 2,
}

pub static GLOBAL_ROUTE: Lazy<Route> = Lazy::new(Default::default);

impl Route {
    pub fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()> {
        match self
            .0
            .insert(at.into(), Storable::Service(T::service(meta)))
        {
            Some(_) => err!((in_use, format!("service `{at}` already exists"))),
            None => Ok(()),
        }
    }
    pub fn add_service<T: Service>(&self, meta: T::Meta) -> Result<()> {
        self.add_service_at::<T>(T::ENDPOINT, meta)
    }
    pub fn remove_service<T: Service>(&self) -> Result<()> {
        match self.0.remove(T::ENDPOINT) {
            Some(_) => Ok(()),
            None => err!((
                not_found,
                format!("service `{}` doesn't exist", T::ENDPOINT)
            )),
        }
    }
    pub fn remove_route<T: Register>(&self) -> Result<()> {
        match self.0.remove(T::ENDPOINT) {
            Some(_) => Ok(()),
            None => err!((not_found, format!("route `{}` doesn't exist", T::ENDPOINT))),
        }
    }
    pub fn remove_at(&self, at: &str) -> Result<()> {
        match self.0.remove(at) {
            Some(_) => Ok(()),
            None => err!((not_found, format!("route or service `{at}` doesn't exist"))),
        }
    }
    pub fn add_route_at(&self, at: &str, route: impl Into<Arc<Route>>) -> Result<()> {
        match self.0.insert(at.into(), Storable::Route(route.into())) {
            Some(_) => err!((in_use, format!("route `{at}` already exists"))),
            None => Ok(()),
        }
    }
    pub fn register_route_at<T: Register>(&self, at: &str, meta: T::Meta) -> Result<()> {
        let route = Route::default();
        T::register(&route, meta)?;
        self.add_route_at(at, route)?;
        Ok(())
    }
    pub fn register_route<T: Register>(&self, meta: T::Meta) -> Result<()> {
        self.register_route_at::<T>(T::ENDPOINT, meta)
    }
    pub fn register<T: Register>(&self, meta: T::Meta) -> Result<()> {
        T::register(self, meta)
    }
    pub(crate) fn introduce_static(&'static self, c: BareChannel) {
        let mut c: Channel = c.into();
        spawn(async move {
            let id = match c.rx::<RouteKey>().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("found error receiving id of service: {:?}", &e);
                    err!((other, e))?
                }
            };
            self.introduce_service(id.as_ref(), c.bare()).await?;
            Ok::<_, igcp::Error>(())
        });
    }

    pub(crate) async fn introduce_static_unspawn(&'static self, c: BareChannel) -> Result<()> {
        let mut c: Channel = c.into();
        let id = match c.rx::<RouteKey>().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("found error receiving id of service: {:?}", &e);
                err!((other, e))?
            }
        };
        self.introduce_service(id.as_ref(), c.bare()).await?;
        Ok(())
    }

    pub(crate) async fn introduce_service(
        &self,
        id: impl AsRef<Utf8Path>,
        bare: BareChannel,
    ) -> Result<()> {
        let id = id.as_ref();
        if let Err((e, c)) = self.__introduce_inner(id, bare).await {
            let mut chan: Channel = c.into();
            chan.tx(Status::NotFound).await?;
            err!((e))?
        }
        Ok(())
    }

    async fn __introduce_inner(
        &self,
        id: impl AsRef<Utf8Path>,
        chan: BareChannel,
    ) -> ::core::result::Result<(), (igcp::Error, BareChannel)> {
        let mut id = id.as_ref().into_iter();
        let first = match id.next() {
            Some(id) => id,
            None => return Err((err!(invalid_data, "service name is empty"), chan))?,
        };
        let value = match self.0.get(first) {
            Some(id) => id,
            None => {
                return Err((
                    err!(invalid_data, format!("service `{:?}` not found", id)),
                    chan,
                ))?
            }
        };
        match value.value() {
            Storable::Route(r) => {
                let mut map = r.clone();
                loop {
                    let next = match id.next() {
                        Some(id) => id,
                        None => {
                            return Err((
                                err!(not_found, format!("service `{:?}` not found", id)),
                                chan,
                            ))
                        }
                    };
                    let next_map = {
                        let val = match map.0.get(next) {
                            Some(val) => val,
                            None => {
                                return Err((
                                    err!(not_found, format!("service `{:?}` not found", id)),
                                    chan,
                                ))
                            }
                        };
                        match val.value() {
                            Storable::Route(r) => r.clone(),
                            Storable::Service(f) => {
                                let mut chan: Channel = chan.into();
                                chan.tx(Status::Found).await.ok();
                                f(chan.bare());
                                return Ok(());
                            }
                        }
                    };
                    map = next_map;
                }
            }
            Storable::Service(f) => {
                let mut chan: Channel = chan.into();
                chan.tx(Status::Found).await.ok();
                f(chan.bare());
                Ok(())
            }
        }
    }
    /// should only be used whenever debugging
    pub fn show(&self) {
        for i in &self.0 {
            match i.value() {
                &Storable::Route(_) => {
                    tracing::info!("Route({:?})", i.key());
                }
                &Storable::Service(_) => {
                    tracing::info!("Service({:?})", i.key());
                }
            }
        }
    }
}
