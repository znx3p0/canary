#![cfg(not(target_arch = "wasm32"))]

use compact_str::CompactStr;
use derive_more::From;
use std::sync::Arc;

use camino::Utf8Path;
use dashmap::DashMap;
use igcp::{err, BareChannel, Channel};
use once_cell::sync::Lazy;

use crate::discovery::Status;
use crate::service::{Service, Svc};
use crate::Result;

type RouteKey = CompactStr;

#[derive(From)]
pub struct InnerRoute {
    map: DashMap<RouteKey, Storable>
}

/// used for discovering services.
/// it stores services inside with a key and it can introduce channels to services.
pub enum Route {
    Static(&'static InnerRoute),
    Dynamic(Arc<InnerRoute>, RouteKey) // tree structure makes sure that arc cannot outlive inner, hence no possibilty of memory leaks
}

impl Route {
    pub fn new_dynamic(name: impl Into<RouteKey>) -> Self {
        Route::Dynamic(Arc::new(InnerRoute::from(DashMap::new())), name.into())
    }
    pub fn new_static(route: &'static InnerRoute) -> Self {
        Route::Static(route)
    }
}

enum Storable {
    Route(Route),
    Service(Svc),
}

#[derive(From, Debug)]
/// context associated with a service
pub struct Ctx {
    top_route: Route, // routes are references by default
    id: RouteKey
}

impl Ctx {
    pub fn new(top_route: Route, id: RouteKey) -> Self { Self { top_route, id } }

    /// Get a reference to the ctx's id.
    pub fn id(&self) -> &CompactStr {
        &self.id
    }
}

impl std::ops::Deref for Ctx {
    type Target = Route;

    fn deref(&self) -> &Self::Target {
        &self.top_route
    }
}

/// Register is how a specific type should be registered on a route,
/// and the metadata needed for it.
pub trait Register {
    /// inner endpoint
    const ENDPOINT: &'static str;
    /// metadata of type
    type Meta;
    /// register implementation of type
    fn register(top_route: &Route, meta: Self::Meta) -> Result<()>;
}

/// global route on which initial services are laid on
pub static GLOBAL_ROUTE: Lazy<Route> = Lazy::new(|| {
    let route = Box::new(InnerRoute::from(DashMap::new()));
    let route: &'static InnerRoute = Box::leak(route);
    Route::new_static(route)
});

trait Context {
    fn context(self) -> Ctx;
}

impl Context for &'static InnerRoute {
    fn context(self) -> Ctx {
        Ctx::new(Route::Static(self), "".into())
    }
}

impl Context for (Arc<InnerRoute>, RouteKey) {
    fn context(self) -> Ctx {
        Ctx::new(Route::Dynamic(self.0, self.1.clone()), self.1)
    }
}

trait RouteLike: Sized {
    fn add_service_at<T: Service>(self, at: &str, meta: T::Meta) -> Result<()>;
    fn remove_at(self, at: &str) -> Result<()>;
    fn switch_raw(self, at: &str, chan: BareChannel, discover: bool) -> std::result::Result<(), BareChannel>;
    fn insert_route_at(self, at: &str, route: Route) -> Result<()>;
    fn add_route_at(self, at: &str) -> Result<Ctx> {
        let route = Route::new_dynamic(at);
        let ctx = route.context();
        self.insert_route_at(at, route)?;
        Ok(ctx)
    }

    fn switch(self, at: &str, chan: BareChannel) -> std::result::Result<(), BareChannel> {
        self.switch_raw(at, chan, false)
    }

    fn remove_service<T: Service>(self) -> Result<()> {
        self.remove_at(T::ENDPOINT)
    }
    fn add_service<T: Service>(self, meta: T::Meta) -> Result<()> {
        self.add_service_at::<T>(T::ENDPOINT, meta)
    }
}

impl RouteLike for &'static InnerRoute {
    fn add_service_at<T: Service>(self, at: &str, meta: T::Meta) -> Result<()> {
        match self.map.insert(at.into(), Storable::Service(T::service(meta))) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }

    fn remove_at(self, at: &str) -> Result<()> {
        match self.map.remove(at) {
            Some(_) => Ok(()),
            None => err!((
                not_found,
                format!("service `{}` doesn't exist", at)
            )),
        }
    }

    fn switch_raw(self, at: &str, chan: BareChannel, discover: bool) -> std::result::Result<(), BareChannel> {
        let mut map = &self.map;

        for segment in Utf8Path::new(at) {
            let storable = match map.get(segment) {
                Some(v) => v,
                None => return Err(chan),
            };

            match storable.value() {
                Storable::Route(route) => match route {
                    Route::Static(inner) => map = &inner.map,
                    Route::Dynamic(inner, _) => map = &inner.map,
                },
                Storable::Service(svc) => {
                    let mut ctx = self.context();
                    ctx.id = at.into();
                    svc(chan, ctx, discover);
                    return Ok(())
                },
            }
        }
        Ok(())
    }

    fn insert_route_at(self, at: &str, route: Route) -> Result<()> {
        match self.map.insert(at.into(), Storable::Route(route)) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }
}

impl RouteLike for &Arc<InnerRoute> {
    fn add_service_at<T: Service>(self, at: &str, meta: T::Meta) -> Result<()> {
        match self.map.insert(at.into(), Storable::Service(T::service(meta))) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }

    fn remove_at(self, at: &str) -> Result<()> {
        match self.map.remove(at) {
            Some(_) => Ok(()),
            None => err!((
                not_found,
                format!("service `{}` doesn't exist", at)
            )),
        }
    }

    fn switch_raw(self, at: &str, chan: BareChannel, discover: bool) -> ::std::result::Result<(), BareChannel> {
        let mut map = &self.map;

        for segment in Utf8Path::new(at) {
            let storable = match map.get(segment) {
                Some(v) => v,
                None => return Err(chan),
            };

            match storable.value() {
                Storable::Route(route) => match route {
                    Route::Static(inner) => map = &inner.map,
                    Route::Dynamic(inner, _) => map = &inner.map,
                },
                Storable::Service(svc) => {
                    let mut ctx = (self.clone(), CompactStr::new(at)).context();
                    ctx.id = at.into();
                    svc(chan, ctx, discover);
                    return Ok(())
                },
            }
        }
        Ok(())
    }
    fn insert_route_at(self, at: &str, route: Route) -> Result<()> {
        match self.map.insert(at.into(), Storable::Route(route)) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }
}

impl Route {
    pub fn context(&self) -> Ctx {
        match self {
            Route::Static(ctx) => ctx.context(),
            Route::Dynamic(ctx, at) => (ctx.clone(), at.clone()).context(),
        }
    }

    pub fn add_service<T: Service>(&self, meta: T::Meta) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.add_service::<T>(meta),
            Route::Dynamic(ctx, _) => ctx.add_service::<T>(meta),
        }
    }

    pub fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.add_service_at::<T>(at, meta),
            Route::Dynamic(ctx, _) => ctx.add_service_at::<T>(at, meta),
        }
    }

    pub fn remove_at(&self, at: &str) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.remove_at(at),
            Route::Dynamic(ctx, _) => ctx.remove_at(at),
        }
    }

    pub fn switch(&self, at: &str, chan: BareChannel) -> std::result::Result<(), BareChannel> {
        match self {
            Route::Static(ctx) => ctx.switch(at, chan),
            Route::Dynamic(ctx, _) => ctx.switch(at, chan),
        }
    }

    pub fn switch_raw(&self, at: &str, chan: BareChannel, discover: bool) -> std::result::Result<(), BareChannel> {
        match self {
            Route::Static(ctx) => ctx.switch_raw(at, chan, discover),
            Route::Dynamic(ctx, _) => ctx.switch_raw(at, chan, discover),
        }
    }

    pub fn insert_route_at(&self, at: &str, route: Route) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.insert_route_at(at, route),
            Route::Dynamic(ctx, _) => ctx.insert_route_at(at, route),
        }
    }

    pub fn add_route_at(&self, at: &str) -> Result<Ctx> {
        match self {
            Route::Static(ctx) => ctx.add_route_at(at),
            Route::Dynamic(ctx, _) => ctx.add_route_at(at),
        }
    }

    pub async fn introduce(&self, chan: BareChannel) -> Result<()> {
        let mut chan: Channel = chan.into();
        let at = chan.receive::<RouteKey>().await?;
        if let Err(chan) = self.switch_raw(&at, chan.bare(), true) {
            let mut chan: Channel = chan.into();
            chan.send(Status::NotFound).await?;
        };
        Ok(())
    }
}

impl std::fmt::Debug for InnerRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for val in &self.map {
            match val.value() {
                Storable::Route(route) => write!(f, "{} => {{ {:?} }}", val.key(), route)?,
                Storable::Service(_) => write!(f, "{},", val.key())?,
            };
        }
        Ok(())
    }
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(route) => {
                route.fmt(f)
            },
            Self::Dynamic(route, _) => {
                route.fmt(f)
            },
        }
    }
}

// impl Route {
//     /// adds a service at a specific id to the route
//     /// ```norun
//     /// #[service]
//     /// async fn ping_service(mut channel: Channel) -> Result<()> {
//     ///     let ping: String = channel.receive().await?;
//     ///     println!("received {}", ping);
//     ///     channel.send("Pong!").await?;
//     ///     Ok(())
//     /// }
//     ///
//     /// GLOBAL_ROUTE.add_service_at::<ping_service>("ping", ())?;
//     /// ```
//     pub fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()> {
//         match self
//             .0
//             .insert(at.into(), Storable::Service(T::service(meta)))
//         {
//             Some(_) => err!((in_use, format!("service `{}` already exists", at))),
//             None => Ok(()),
//         }
//     }
//     /// adds a service to the route
//     /// ```norun
//     /// #[service]
//     /// async fn ping_service(mut channel: Channel) -> Result<()> {
//     ///     let ping: String = channel.receive().await?;
//     ///     println!("received {}", ping);
//     ///     channel.send("Pong!").await?;
//     ///     Ok(())
//     /// }
//     ///
//     /// GLOBAL_ROUTE.add_service::<ping_service>(())?;
//     /// ```
//     pub fn add_service<T: Service>(&self, meta: T::Meta) -> Result<()> {
//         self.add_service_at::<T>(T::ENDPOINT, meta)
//     }
//     /// removes a service from the route
//     /// ```norun
//     /// GLOBAL_ROUTE.remove_service::<my_service>()?;
//     /// ```
//     pub fn remove_service<T: Service>(&self) -> Result<()> {
//         match self.0.remove(T::ENDPOINT) {
//             Some(_) => Ok(()),
//             None => err!((
//                 not_found,
//                 format!("service `{}` doesn't exist", T::ENDPOINT)
//             )),
//         }
//     }
//     /// remove the register endpoint from the route
//     /// ```norun
//     /// GLOBAL_ROUTE.remove_register::<my_custom_register>()?
//     /// ```
//     pub fn remove_register<T: Register>(&self) -> Result<()> {
//         match self.0.remove(T::ENDPOINT) {
//             Some(_) => Ok(()),
//             None => err!((not_found, format!("route `{}` doesn't exist", T::ENDPOINT))),
//         }
//     }
//     /// remove the specified id from the route
//     /// ```norun
//     /// GLOBAL_ROUTE.remove_at("my_service")?
//     /// ```
//     pub fn remove_at(&self, at: &str) -> Result<()> {
//         match self.0.remove(at) {
//             Some(_) => Ok(()),
//             None => err!((
//                 not_found,
//                 format!("route or service `{}` doesn't exist", at)
//             )),
//         }
//     }
//     /// add a route into the route at the specified id.
//     /// ```norun
//     /// GLOBAL_ROUTE.add_route_at("MyRoute", Route::default())?;
//     /// ```
//     pub fn add_route_at(&self, at: &str, route: impl Into<Arc<Route>>) -> Result<()> {
//         match self.0.insert(at.into(), Storable::Route(route.into())) {
//             Some(_) => err!((in_use, format!("route `{}` already exists", at))),
//             None => Ok(()),
//         }
//     }
//     /// register into a new route and add the new route at the specified id
//     /// ```norun
//     /// GLOBAL_ROUTE.register_route_at::<MyType>("MyRoute", ())?;
//     /// ```
//     /// the global route now looks like this:
//     ///
//     /// GLOBAL_ROUTE:
//     /// - MyRoute:
//     ///   - ... whatever the register implementation added
//     pub fn register_route_at<T: Register>(&self, at: &str, meta: T::Meta) -> Result<()> {
//         let route = Route::default();
//         T::register(&route, meta)?;
//         self.add_route_at(at, route)?;
//         Ok(())
//     }
//     /// register into a new route and add it
//     /// ```norun
//     /// GLOBAL_ROUTE.register_route::<MyRoute>(())?;
//     /// ```
//     /// the global route now looks like this:
//     ///
//     /// GLOBAL_ROUTE:
//     /// - MyRoute:
//     ///   - ... whatever the register implementation added
//     pub fn register_route<T: Register>(&self, meta: T::Meta) -> Result<()> {
//         self.register_route_at::<T>(T::ENDPOINT, meta)
//     }
//     /// registers the type on the route
//     /// ```norun
//     /// GLOBAL_ROUTE.register::<MyRoute>(())?;
//     /// ```
//     pub fn register<T: Register>(&self, meta: T::Meta) -> Result<()> {
//         T::register(self, meta)
//     }
//     fn static_switch(
//         &'static self,
//         id: impl AsRef<Utf8Path>,
//         chan: impl Into<BareChannel>,
//     ) -> ::core::result::Result<(), (igcp::Error, BareChannel)> {
//         let mut id = id.as_ref().into_iter();
//         let chan = chan.into();
//         let first = match id.next() {
//             Some(id) => id,
//             None => return Err((err!(invalid_data, "service name is empty"), chan))?,
//         };
//         let value = match self.0.get(first) {
//             Some(id) => id,
//             None => {
//                 return Err((
//                     err!(invalid_data, format!("service `{:?}` not found", id)),
//                     chan,
//                 ))?
//             }
//         };
//         let ctx = self.context_static();
//         match value.value() {
//             Storable::Route(r) => {
//                 let mut map = r.clone();
//                 loop {
//                     let next = match id.next() {
//                         Some(id) => id,
//                         None => {
//                             return Err((
//                                 err!(not_found, format!("service `{:?}` not found", id)),
//                                 chan,
//                             ))
//                         }
//                     };
//                     let next_map = {
//                         let val = match map.0.get(next) {
//                             Some(val) => val,
//                             None => {
//                                 return Err((
//                                     err!(not_found, format!("service `{:?}` not found", id)),
//                                     chan,
//                                 ))
//                             }
//                         };
//                         match val.value() {
//                             Storable::Route(r) => r.clone(),
//                             Storable::Service(f) => {
//                                 f(chan, ctx);
//                                 return Ok(());
//                             }
//                         }
//                     };
//                     map = next_map;
//                 }
//             }
//             Storable::Service(f) => {
//                 f(chan, ctx);
//                 Ok(())
//             }
//         }
//     }
//     // all next are used for the routing system
//     pub(crate) fn introduce_static(&'static self, c: BareChannel) {
//         let mut c: Channel = c.into();
//         spawn(async move {
//             let id = match c.receive::<RouteKey>().await {
//                 Ok(s) => s,
//                 Err(e) => {
//                     tracing::error!("found error receiving id of service: {:?}", &e);
//                     err!((other, e))?
//                 }
//             };
//             self.introduce_service_static(id.as_ref(), c.bare()).await?;
//             Ok::<_, igcp::Error>(())
//         });
//     }
//     pub(crate) async fn introduce_static_unspawn(&'static self, c: BareChannel) -> Result<()> {
//         let mut c: Channel = c.into();
//         let id = match c.receive::<RouteKey>().await {
//             Ok(s) => s,
//             Err(e) => {
//                 tracing::error!("found error receiving id of service: {:?}", &e);
//                 err!((other, e))?
//             }
//         };
//         self.introduce_service_static(id.as_ref(), c.bare()).await?;
//         Ok(())
//     }
//     pub(crate) async fn introduce_service_static(
//         &'static self,
//         id: impl AsRef<Utf8Path>,
//         bare: BareChannel,
//     ) -> Result<()> {
//         let id = id.as_ref();
//         if let Err((e, c)) = self.__introduce_inner_static(id, bare).await {
//             let mut chan: Channel = c.into();
//             chan.send(Status::NotFound).await?;
//             err!((e))?
//         }
//         Ok(())
//     }
//     async fn __introduce_inner_static(
//         &'static self,
//         id: impl AsRef<Utf8Path>,
//         chan: BareChannel,
//     ) -> ::core::result::Result<(), (igcp::Error, BareChannel)> {
//         let mut id = id.as_ref().into_iter();
//         let first = match id.next() {
//             Some(id) => id,
//             None => return Err((err!(invalid_data, "service name is empty"), chan))?,
//         };
//         let value = match self.0.get(first) {
//             Some(id) => id,
//             None => {
//                 return Err((
//                     err!(invalid_data, format!("service `{:?}` not found", id)),
//                     chan,
//                 ))?
//             }
//         };
//         let ctx = self.context_static();
//         match value.value() {
//             Storable::Route(r) => {
//                 let mut map = r.clone();
//                 loop {
//                     let next = match id.next() {
//                         Some(id) => id,
//                         None => {
//                             return Err((
//                                 err!(not_found, format!("service `{:?}` not found", id)),
//                                 chan,
//                             ))
//                         }
//                     };
//                     let next_map = {
//                         let val = match map.0.get(next) {
//                             Some(val) => val,
//                             None => {
//                                 return Err((
//                                     err!(not_found, format!("service `{:?}` not found", id)),
//                                     chan,
//                                 ))
//                             }
//                         };
//                         match val.value() {
//                             Storable::Route(r) => r.clone(),
//                             Storable::Service(f) => {
//                                 let mut chan: Channel = chan.into();
//                                 chan.tx(Status::Found).await.ok();
//                                 f(chan.bare(), ctx);
//                                 return Ok(());
//                             }
//                         }
//                     };
//                     map = next_map;
//                 }
//             }
//             Storable::Service(f) => {
//                 let mut chan: Channel = chan.into();
//                 chan.tx(Status::Found).await.ok();
//                 f(chan.bare(), ctx);
//                 Ok(())
//             }
//         }
//     }
//     // pub(crate) async fn introduce_service(
//     //     &self,
//     //     id: impl AsRef<Utf8Path>,
//     //     bare: BareChannel,
//     // ) -> Result<()> {
//     //     let id = id.as_ref();
//     //     if let Err((e, c)) = self.__introduce_inner(id, bare).await {
//     //         let mut chan: Channel = c.into();
//     //         chan.send(Status::NotFound).await?;
//     //         err!((e))?
//     //     }
//     //     Ok(())
//     // }
//     // pub(crate) async fn introduce_service(
//     //     &self,
//     //     id: impl AsRef<Utf8Path>,
//     //     bare: BareChannel,
//     // ) -> Result<()> {
//     //     let id = id.as_ref();
//     //     if let Err((e, c)) = self.__introduce_inner(id, bare).await {
//     //         let mut chan: Channel = c.into();
//     //         chan.send(Status::NotFound).await?;
//     //         err!((e))?
//     //     }
//     //     Ok(())
//     // }
//     // async fn __introduce_inner(
//     //     &self,
//     //     id: impl AsRef<Utf8Path>,
//     //     chan: BareChannel,
//     // ) -> ::core::result::Result<(), (igcp::Error, BareChannel)> {
//     //     let mut id = id.as_ref().into_iter();
//     //     let first = match id.next() {
//     //         Some(id) => id,
//     //         None => return Err((err!(invalid_data, "service name is empty"), chan))?,
//     //     };
//     //     let value = match self.0.get(first) {
//     //         Some(id) => id,
//     //         None => {
//     //             return Err((
//     //                 err!(invalid_data, format!("service `{:?}` not found", id)),
//     //                 chan,
//     //             ))?
//     //         }
//     //     };
//     //     match value.value() {
//     //         Storable::Route(r) => {
//     //             let mut map = r.clone();
//     //             loop {
//     //                 let next = match id.next() {
//     //                     Some(id) => id,
//     //                     None => {
//     //                         return Err((
//     //                             err!(not_found, format!("service `{:?}` not found", id)),
//     //                             chan,
//     //                         ))
//     //                     }
//     //                 };
//     //                 let next_map = {
//     //                     let val = match map.0.get(next) {
//     //                         Some(val) => val,
//     //                         None => {
//     //                             return Err((
//     //                                 err!(not_found, format!("service `{:?}` not found", id)),
//     //                                 chan,
//     //                             ))
//     //                         }
//     //                     };
//     //                     match val.value() {
//     //                         Storable::Route(r) => r.clone(),
//     //                         Storable::Service(f) => {
//     //                             let mut chan: Channel = chan.into();
//     //                             chan.tx(Status::Found).await.ok();
//     //                             f(chan.bare());
//     //                             return Ok(());
//     //                         }
//     //                     }
//     //                 };
//     //                 map = next_map;
//     //             }
//     //         }
//     //         Storable::Service(f) => {
//     //             let mut chan: Channel = chan.into();
//     //             chan.tx(Status::Found).await.ok();
//     //             f(chan.bare());
//     //             Ok(())
//     //         }
//     //     }
//     // }
// }
