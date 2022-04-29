use std::sync::Arc;

use crate::async_snow::Snow;
use crate::channel::bipartite::UnformattedRawReceiveChannel;
use crate::channel::bipartite::UnformattedRawSendChannel;
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
pub enum UnformattedSendChannel {
    Raw(UnformattedRawSendChannel),
    Encrypted(UnformattedRawSendChannel, Snow),
}

impl UnformattedSendChannel {
    pub async fn send<T: Serialize, F: SendFormat>(&mut self, obj: T, format: &F) -> Result<usize> {
        match self {
            Self::Raw(chan) => chan.send(obj, format).await,
            Self::Encrypted(chan, snow) => {
                let with = SnowWith { snow, format };
                chan.send(obj, &with).await
            }
        }
    }
}
