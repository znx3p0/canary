#![cfg(not(target_arch = "wasm32"))]

use crate::discovery::Status;
use crate::Result;
use flume::{Sender, Receiver};
use derive_more::From;
use igcp::{BareChannel, Channel, err};

pub type ServiceMessage = (BareChannel, bool);

/// underlying service handle that is stored on a route
pub type Svc = Sender<ServiceMessage>;

#[derive(From)]
pub struct ServiceHandle {
    chan: Receiver<ServiceMessage>,
}

impl ServiceHandle {
    pub async fn next(&mut self) -> Result<Channel> {
        let (channel, discover) = self.chan.recv_async().await
            .map_err(|e| err!(e))?;
        if discover {
            let mut channel: Channel = channel.into();
            channel.send(Status::Found).await?;
            return Ok(channel)
        }
        Ok(channel.into())
    }
}
