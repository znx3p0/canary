use crate::serialization::formats::Format;

use super::encrypted::{bidirectional, receive_channel, send_channel};

pub type Channel<R = Format, W = Format> = bidirectional::Channel<R, W>;
pub type RefChannel<'a, F = Format> = bidirectional::RefChannel<'a, F>;

pub type SendChannel<F = Format> = send_channel::SendChannel<F>;
pub type RefSendChannel<'a, F = Format> = send_channel::RefSendChannel<'a, F>;

pub type ReceiveChannel<F = Format> = receive_channel::ReceiveChannel<F>;
pub type RefReceiveChannel<'a, F = Format> = receive_channel::RefReceiveChannel<'a, F>;

// this will allow channels to abstract over any type that can receive or send bytes.

// #[async_trait]
// trait Sender {
//     async fn send(&mut self, obj: Bytes) -> Result<()>;
// }

// #[async_trait]
// trait Receiver {
//     async fn receive(&mut self) -> Result<Bytes>;
// }
