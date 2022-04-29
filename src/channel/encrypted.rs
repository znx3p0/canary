use crate::Result;
use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    async_snow::Snow,
    serialization::formats::{ReadFormat, SendFormat},
};

#[derive(From)]
pub struct SnowWith<'a, F> {
    pub snow: &'a mut Snow,
    pub format: &'a F,
}

impl<F: SendFormat> SendFormat for SnowWith<'_, F> {
    fn serialize<O: Serialize>(&self, obj: &O) -> Result<Vec<u8>> {
        let obj = self.format.serialize(obj)?;
        self.snow.encrypt_packets(obj)
    }
}

impl<F: ReadFormat> ReadFormat for SnowWith<'_, F> {
    fn deserialize<T>(&self, bytes: &[u8]) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        let bytes = self.snow.decrypt(bytes)?;
        self.format.deserialize(&bytes)
    }
}
