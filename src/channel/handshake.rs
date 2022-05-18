use derive_more::From;

use crate::{err, Channel, Result};

#[derive(From)]
#[repr(transparent)]
pub struct Handshake(Channel);

impl Handshake {
    pub async fn encrypted(self) -> Result<Channel> {
        let mut stream = self.0;
        let snow = crate::async_snow::new(&mut stream).await?;
        stream
            .encrypt(snow)
            .map_err(|_| err!("channel already encrypted"))?;
        Ok(stream)
    }

    pub fn raw(self) -> Channel {
        self.0
    }
}
