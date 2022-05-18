use crate::{
    async_snow::{Decrypt, Encrypt},
    Result,
};
use derive_more::From;
use serde::{de::DeserializeOwned, Serialize};

use crate::serialization::formats::{ReadFormat, SendFormat};

#[derive(From)]
pub struct WithCipher<'a, C, F> {
    pub snow: &'a mut C,
    pub format: &'a mut F,
}

impl<C: Encrypt, F: SendFormat> SendFormat for WithCipher<'_, C, F> {
    fn serialize<O: Serialize>(&mut self, obj: &O) -> Result<Vec<u8>> {
        let obj = self.format.serialize(obj)?;
        self.snow.encrypt_packets(obj)
    }
}

impl<C: Decrypt, F: ReadFormat> ReadFormat for WithCipher<'_, C, F> {
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        let bytes = self.snow.decrypt(bytes)?;
        self.format.deserialize(&bytes)
    }
}
