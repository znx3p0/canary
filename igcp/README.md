# IGCP

IGCP is the communication backend of Canary.
It provides a simple `Channel` type that represents a stream of objects,
and a serializable `std::io::Result`.

```rust
async fn send(mut channel: Channel) -> Result<()> {
    channel.send("hello world!").await?;
    channel.send(42).await?;
    Ok(())
}
```
