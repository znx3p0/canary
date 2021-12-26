# Services

Services are functions that are stored on a route.

They take two parameters: metadata and a channel.
They also return a result, but failures are silently logged.

A simple service looks like this:
```rust , no_run
use sia::{service, Channel, Result};

#[service]
async fn my_service(channel: Channel) -> Result<()> {
    Ok(())
}
```

Although barebones services might be enough for some use cases,
most use cases need to have context with the service (e.g. atomic counter, etc.)

For example, let's say Alice needs to build a counter service.
The service must count every call to the service and send back the current number.
A service like that could be implemented like this:
```rust , no_run
use sia::{service, Channel, Result};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[service]
async fn counter_service(counter: Arc<AtomicU64>, mut peer: Channel) -> Result<()> {
    let current_val = counter.fetch_add(1, Ordering::Relaxed);
    peer.send(current_val).await?;
    Ok(())
}
```

These services can be registered on a route and exposed through a provider,
and they are designed to be embarrasingly parallel.

