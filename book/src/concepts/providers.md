# Providers

Providers are a way of exposing a route (usually the global route)
for external access.

A simple example of a provider is the insecure TCP provider,
which provides non-encrypted TCP channels to a route.

Providers offer a simple interface for exposing routes through them.
An example of a TCP provider in use:

```rust , no_run

use sia::{service, Channel, Result};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use sia::providers::Tcp;

#[main]
async fn main() -> Result<()> {
    // bind the global route to this tcp socket
    Tcp::bind("127.0.0.1:8080").await?;
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

Accessing the counter service should be as easy as:
```rust , no_run

use sia::Result;
use sia::providers::Tcp;

#[main]
async fn main() -> Result<()> {
    let mut counter_service_chan = Tcp::connect("127.0.0.1:8080", "counter_service").await?;
    let current = counter_service_chan.receive().await?;
    println!("current value: {}", current);
}

```



