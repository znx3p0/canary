# Routes

Routes are concurrent hashmaps that can store services and routes.
By default, Sia uses a global route, but a local ones can be created and worked with too.

Services are registered in routes and routes can also be registered in routes.

A route roughly looks like this:
```rust , no_run

struct Route {
               // name | service or route
    map: HashMap<String, RouteValue>
}

enum RouteValue {
    Route(Route),
    Service(Service),
}

```

Registering a service on a route looks like this (we're using the example from the services chapter):
```rust , no_run

use sia::{service, Channel, Result};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[main]
async fn main() -> Result<()> {
                          // name of the service   |  metadata of service
    GLOBAL_ROUTE.add_service::<counter_service>(Arc::new(AtomicU64::new(0)))?;
    Ok(())
}

#[service]
async fn counter_service(counter: Arc<AtomicU64>, mut peer: Channel) -> Result<()> {
    let current_val = counter.fetch_add(1, Ordering::Relaxed);
    peer.send(current_val).await?;
    Ok(())
}

```

Routes can also be an abstraction over an actor, with services being their methods
and metadata being their state. Sia provides an ergonomic way of creating routes
as actors.

```rust , no_run

// this is the equivalent to the counter service,
// but implemented with routes.
// using this macro automatically implements `Register` to the structure.
// the register trait allows it to be registered under another route,
// which may be a nested route or the global route.


use sia::{Channel, Result};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use sia::route;

// this is an actor.
// doesn't need to be wrapped under an Arc as it is done automatically.
// do note that as services, actors only have read-access to their state.
#[route]
pub struct MyCounter(AtomicU64);

#[route]
impl MyCounter {
    async fn service(&self, mut peer: Channel) -> Result<()> {
        let current_val = self.0.fetch_add(1, Ordering::Relaxed);
        peer.send(current_val).await?;
        Ok(())
    }
}

```

Registering the route should be as easy as this:
```rust , no_run
let my_counter = Arc::new(MyCounter(AtomicU64::new(0)));
GLOBAL_ROUTE.register_route::<MyCounter>(my_counter)?;
```

Accessing the service looks like this:
```rust , no_run

use sia::Result;
use sia::providers::Tcp;

#[main]
async fn main() -> Result<()> {
    // routes use PascalCase for their naming nomenclature
    // and services use snake_case
    let mut counter_service_chan = Tcp::connect("127.0.0.1:8080", "MyCounter/service").await?;
    let current = counter_service_chan.receive().await?;
    println!("current value: {}", current);
}

```

It is important to note that while it is possible to keep routes analogous to actors, they're very
different, and actors can be easily made with pure services without the need for routes. Still,
they're an important tool to keep up your belt while working with Sia. You'll get a sense
of when and where to use routes with experience.

A good rule to avoid name collisions on the global route is that libraries should not use the global route,
but instead provide register implementations for their services and routes.

Binaries should be able to use the global route directly.

Don't forget to also create drop implementations on the services and routes to avoid memory leaks.



