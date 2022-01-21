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
    map: DashMap<RouteKey, Storable>,
}

/// used for discovering services.
/// it stores services inside with a key and it can introduce channels to services.
pub enum Route {
    Static(&'static InnerRoute),
    Dynamic(Arc<InnerRoute>, RouteKey), // tree structure makes sure that arc cannot outlive inner, hence no possibilty of memory leaks
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

#[derive(From)]
/// context associated with a service
pub struct Ctx {
    top_route: Route, // routes are references by default
    id: RouteKey,
}

impl Ctx {
    /// create a new context from the route and id
    pub fn new(top_route: Route, id: RouteKey) -> Self {
        Self { top_route, id }
    }

    #[inline]
    /// Get a reference to the ctx's id.
    pub fn id(&self) -> &CompactStr {
        &self.id
    }
}

impl std::ops::Deref for Ctx {
    type Target = Route;

    #[inline]
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
    fn register<T: RouteLike>(top_route: &T, meta: Self::Meta) -> Result<()>;
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
    #[inline]
    fn context(self) -> Ctx {
        Ctx::new(Route::Static(self), "".into())
    }
}

impl Context for (Arc<InnerRoute>, RouteKey) {
    #[inline]
    fn context(self) -> Ctx {
        Ctx::new(Route::Dynamic(self.0, self.1.clone()), self.1)
    }
}

pub trait RouteLike: Sized {
    fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()>;
    fn remove_at(&self, at: &str) -> Result<()>;
    fn switch_raw(
        &self,
        at: &str,
        chan: BareChannel,
        discover: bool,
    ) -> std::result::Result<(), BareChannel>;
    fn insert_route_at(&self, at: &str, route: Route) -> Result<()>;
    fn register<T: Register>(&self, meta: T::Meta) -> Result<()>;
    #[inline]
    fn register_default<T: Register>(&self) -> Result<()>
    where
        T::Meta: Default,
    {
        self.register::<T>(Default::default())
    }
    #[inline]
    fn add_route_at(&self, at: &str) -> Result<Ctx> {
        let route = Route::new_dynamic(at);
        let ctx = route.context();
        self.insert_route_at(at, route)?;
        Ok(ctx)
    }

    #[inline]
    fn switch(&self, at: &str, chan: BareChannel) -> std::result::Result<(), BareChannel> {
        self.switch_raw(at, chan, false)
    }

    #[inline]
    fn remove_service<T: Service>(self) -> Result<()> {
        self.remove_at(T::ENDPOINT)
    }
    #[inline]
    fn add_service<T: Service>(&self, meta: T::Meta) -> Result<()> {
        self.add_service_at::<T>(T::ENDPOINT, meta)
    }
    #[inline]
    fn add_service_default<T: Service>(&self) -> Result<()>
    where
        T::Meta: Default,
    {
        self.add_service::<T>(Default::default())
    }
}

impl RouteLike for &'static InnerRoute {
    #[inline]
    fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()> {
        match self
            .map
            .insert(at.into(), Storable::Service(T::service(meta)))
        {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }

    #[inline]
    fn remove_at(&self, at: &str) -> Result<()> {
        match self.map.remove(at) {
            Some(_) => Ok(()),
            None => err!((not_found, format!("service `{}` doesn't exist", at))),
        }
    }

    #[inline]
    fn switch_raw(
        &self,
        at: &str,
        chan: BareChannel,
        discover: bool,
    ) -> ::std::result::Result<(), BareChannel> {
        let mut map = &self.map;
        let mut ctx = None;

        for segment in Utf8Path::new(at) {
            let storable = match map.get(segment) {
                Some(v) => v,
                None => return Err(chan),
            };

            match storable.value() {
                Storable::Route(route) => match route {
                    Route::Static(inner) => {
                        ctx = Some(inner.context());
                        map = &inner.map
                    }
                    Route::Dynamic(inner, _) => {
                        ctx = Some((inner.clone(), CompactStr::new(at)).context());
                        map = &inner.map
                    }
                },
                Storable::Service(svc) => {
                    let mut ctx = ctx.unwrap_or(self.context());
                    ctx.id = at.into();
                    svc(chan, ctx, discover);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn insert_route_at(&self, at: &str, route: Route) -> Result<()> {
        match self.map.insert(at.into(), Storable::Route(route)) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }

    #[inline]
    fn register<T: Register>(&self, meta: T::Meta) -> Result<()> {
        T::register(self, meta)
    }
}

impl RouteLike for Arc<InnerRoute> {
    #[inline]
    fn add_service_at<T: Service>(&self, at: &str, meta: T::Meta) -> Result<()> {
        match self
            .map
            .insert(at.into(), Storable::Service(T::service(meta)))
        {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }

    #[inline]
    fn remove_at(&self, at: &str) -> Result<()> {
        match self.map.remove(at) {
            Some(_) => Ok(()),
            None => err!((not_found, format!("service `{}` doesn't exist", at))),
        }
    }

    #[inline]
    fn switch_raw(
        &self,
        at: &str,
        chan: BareChannel,
        discover: bool,
    ) -> ::std::result::Result<(), BareChannel> {
        let mut map = &self.map;
        let mut ctx = None;

        for segment in Utf8Path::new(at) {
            let storable = match map.get(segment) {
                Some(v) => v,
                None => return Err(chan),
            };

            match storable.value() {
                Storable::Route(route) => match route {
                    Route::Static(inner) => {
                        ctx = Some(inner.context());
                        map = &inner.map
                    }
                    Route::Dynamic(inner, _) => {
                        ctx = Some((inner.clone(), CompactStr::new(at)).context());
                        map = &inner.map
                    }
                },
                Storable::Service(svc) => {
                    let mut ctx = ctx.unwrap_or((self.clone(), CompactStr::new(at)).context());
                    ctx.id = at.into();
                    svc(chan, ctx, discover);
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    #[inline]
    fn insert_route_at(&self, at: &str, route: Route) -> Result<()> {
        match self.map.insert(at.into(), Storable::Route(route)) {
            Some(_) => err!((in_use, format!("service `{}` already exists", at))),
            None => Ok(()),
        }
    }
    #[inline]
    fn register<T: Register>(&self, meta: T::Meta) -> Result<()> {
        T::register(self, meta)
    }
}

impl Route {
    #[inline]
    pub fn context(&self) -> Ctx {
        match self {
            Route::Static(ctx) => ctx.context(),
            Route::Dynamic(ctx, at) => (ctx.clone(), at.clone()).context(),
        }
    }

    pub fn register<T: Register>(&self, meta: T::Meta) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.register::<T>(meta),
            Route::Dynamic(ctx, _) => ctx.register::<T>(meta),
        }
    }

    pub fn register_default<T: Register>(&self) -> Result<()>
    where
        T::Meta: Default,
    {
        match self {
            Route::Static(ctx) => ctx.register_default::<T>(),
            Route::Dynamic(ctx, _) => ctx.register_default::<T>(),
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

    pub fn add_service_default<T: Service>(&self) -> Result<()>
    where
        T::Meta: Default,
    {
        match self {
            Route::Static(ctx) => ctx.add_service_default::<T>(),
            Route::Dynamic(ctx, _) => ctx.add_service_default::<T>(),
        }
    }

    #[inline]
    pub fn remove_at(&self, at: &str) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.remove_at(at),
            Route::Dynamic(ctx, _) => ctx.remove_at(at),
        }
    }

    #[inline]
    pub fn switch(&self, at: &str, chan: BareChannel) -> std::result::Result<(), BareChannel> {
        match self {
            Route::Static(ctx) => ctx.switch(at, chan),
            Route::Dynamic(ctx, _) => ctx.switch(at, chan),
        }
    }

    #[inline]
    pub fn switch_raw(
        &self,
        at: &str,
        chan: BareChannel,
        discover: bool,
    ) -> std::result::Result<(), BareChannel> {
        match self {
            Route::Static(ctx) => ctx.switch_raw(at, chan, discover),
            Route::Dynamic(ctx, _) => ctx.switch_raw(at, chan, discover),
        }
    }

    #[inline]
    pub fn insert_route_at(&self, at: &str, route: Route) -> Result<()> {
        match self {
            Route::Static(ctx) => ctx.insert_route_at(at, route),
            Route::Dynamic(ctx, _) => ctx.insert_route_at(at, route),
        }
    }

    #[inline]
    pub fn add_route_at(&self, at: &str) -> Result<Ctx> {
        match self {
            Route::Static(ctx) => ctx.add_route_at(at),
            Route::Dynamic(ctx, _) => ctx.add_route_at(at),
        }
    }

    #[inline]
    pub async fn introduce(&self, chan: BareChannel) -> Result<()> {
        let mut chan: Channel = chan.into();
        let at = chan.receive::<RouteKey>().await?;
        if let Err(chan) = self.switch_raw(&at, chan.bare(), true) {
            let mut chan: Channel = chan.into();
            chan.send(Status::NotFound).await?;
        };
        Ok(())
    }

    pub fn show(&self) {
        match self {
            Route::Static(route) => {
                for v in &route.map {
                    println!("{}", v.key());
                }
            }
            Route::Dynamic(route, _) => {
                for v in &route.map {
                    println!("{}", v.key());
                }
            }
        }
    }
}
