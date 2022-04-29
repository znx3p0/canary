use std::sync::Arc;

use crate::async_snow::Snow;
use crate::channel::bipartite::UnformattedRawReceiveChannel;
use crate::channel::encrypted::SnowWith;
use crate::serialization::formats::Format;
use crate::serialization::formats::ReadFormat;
use crate::serialization::formats::SendFormat;
use crate::Result;
use derive_more::From;
use serde::de::DeserializeOwned;
use serde::Serialize;

// encrypted
#[derive(From)]
pub enum UnformattedReceiveChannel {
    Raw(UnformattedRawReceiveChannel),
    Encrypted(UnformattedRawReceiveChannel, Snow),
}

impl UnformattedReceiveChannel {
    pub async fn receive<T: DeserializeOwned, F: ReadFormat>(&mut self, format: &F) -> Result<T> {
        match self {
            Self::Raw(chan) => chan.receive(format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.receive(&with).await
            }
        }
    }
}
