
channels are unified types that can receive and send objects.
they offer the following methods:
- `.send(object).await?`
- `.receive().await?`
- `.split()`
- `.to_unformatted()`


the public api will have:

    all of these can be encrypted or raw.
    the user should not have to worry about whether a channel is encrypted or not.



------ unified
bidirectional

- `Channel` implemented with `RefChannel`
- `RefChannel` implemented with `RefUnformattedChannel`

- `UnformattedChannel` implemented with `RefUnformattedChannel`
- `RefUnformattedChannel` implemented from scratch

------  bipartite

bidirectional -
- `Channel` implemented with `RefChannel`
- `RefChannel` implemented with `RefUnformattedChannel`

- `UnformattedChannel` implemented with `RefUnformattedChannel`
- `RefUnformattedChannel` implemented from scratch

send

- `SendChannel` implemented with `RefSendChannel`
- `RefSendChannel`  implemented with `RefUnformattedSendChannel`

- `UnformattedSendChannel` implemented with `RefUnformattedSendChannel`
- `RefUnformattedSendChannel` implemented from scratch

receive

- `ReceiveChannel` implemented with `RefReceiveChannel`
- `RefReceiveChannel` implemented with `RefUnformattedReceiveChannel`

- `UnformattedReceiveChannel` implemented with `RefUnformattedReceiveChannel`
- `RefUnformattedReceiveChannel` implemented from scratch



Following changes:
- use `Bytes` instead of `Vec<u8>` since it has much better performance

