use derive_more::From;

use crate::{Channel, Result};

#[derive(From)]
#[repr(transparent)]
pub struct Handshake(Channel);

impl Handshake {
    pub async fn encrypted(self) -> Result<Channel> {
        let mut stream = self.0;
        let snow = crate::async_snow::new(&mut stream).await?;
        let chan = stream.encrypt(snow);
        Ok(chan)
    }

    pub fn raw(self) -> Channel {
        self.0
    }
}
