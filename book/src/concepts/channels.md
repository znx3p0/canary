# Channels

Channels are a backend-agnostic way of communicating with peers.
You can think of it as a wrapper around a stream (tcp, udp or any other backend) that allows
you to send and receive objects or messages.

For example, let's assume you have connected Alice's machine and Bob's machine.

```rust , no_run
use sia::{Channel, Result};

// runs on Alice's machine
async fn alice(mut chan: Channel) -> Result<()> {
    chan.send("Hey Bob!").await?;
    Ok(())
}

// runs on Bob's machine
async fn bob(mut chan: Channel) -> Result<()> {
    let message: String = chan.receive().await?;
    println!("alice says: `{}`", message); // alice says: `Hey Bob!`
    Ok(())
}
```

It is important to note that you can send objects that
implement `Serialize` and receive objects that implement `Deserialize`.

Channels also have an address that represents the peer.
Let's say for example that Alice's machine is connected to
Bob's machine and Carl's machine, but Bob's machine and Carl's machine
are not connected. Alice's service mirrors messages between Bob and Carl.

```
# Bob sends a message to alice
Bob -> Alice -> Carl # Carl receives message


# Carl sends a message to alice
Carl -> Alice -> Bob # Bob receives message
```

This works nicely until Carl's service becomes viral, and Alice's machine
cannot keep up with Carl's messages. Carl's machine and Bob's machine should
be connected directly and Alice's machine should not be needed anymore.
A simple way of doing this in the architecture of channels is through addresses.

A simple example of addresses(only for explanation purposes, doesn't work):
```rust

use sia::{Channel, Result};

async fn alice(mut bob: Channel, carl: Channel) -> Result<()> {
    bob.send(carl.addr()?).await?;
    Ok(())
}

async fn bob(mut alice: Channel) -> Result<()> {
    // the connect method does not exist,
    // in reality, this method takes in the id of a service
    let mut carl = bob.receive().await?.connect();
    carl.send("Hey Carl!").await?;
    Ok(())
}

async fn carl(mut bob: Channel) -> Result<()> {
    let message: String = bob.receive().await?;
    println!("bob says: `{}`", message); // bob says: `Hey Carl!`
    Ok(())
}

```

Channels by default use Bincode for serialization, but they support various other
formats such as JSON, BSON and Postcard.


