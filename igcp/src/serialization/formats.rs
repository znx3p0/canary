use std::marker::PhantomData;

use serde::Serialize;

use crate::err;

pub struct Bincode;
pub struct Json;
pub struct Bson;
pub struct Postcard;

pub trait SendFormat {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>>;
}

pub trait ReadFormat {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>;
}

pub trait Format: SendFormat + ReadFormat {}

impl SendFormat for Bincode {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        // use bincode::Options;
        // let obj = bincode::options()
        //     .serialize(obj)
        //     .or_else(|e| err!((invalid_data, e)))?;
        let obj = bincode::serialize(obj).or_else(|e| err!((invalid_data, e)))?;
        Ok(obj)
    }
}
impl ReadFormat for Bincode {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        // use bincode::Options;
        // bincode::options()
        //     .deserialize(bytes)
        //     .or_else(|e| err!((invalid_data, e)))
        bincode::deserialize(bytes).or_else(|e| err!((invalid_data, e)))
    }
}

impl SendFormat for Json {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(obj).or_else(|e| err!((invalid_data, e)))
    }
}
impl ReadFormat for Json {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        serde_json::from_slice(bytes).or_else(|e| err!((invalid_data, e)))
    }
}
impl SendFormat for Bson {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        bson::ser::to_vec(obj).or_else(|e| err!((invalid_data, e)))
    }
}
impl ReadFormat for Bson {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        bson::de::from_slice(bytes).or_else(|e| err!((invalid_data, e)))
    }
}
impl SendFormat for Postcard {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        postcard::to_allocvec(obj).or_else(|e| err!((invalid_data, e)))
    }
}
impl ReadFormat for Postcard {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        postcard::from_bytes(bytes).or_else(|e| err!((invalid_data, e)))
    }
}

pub struct Any<T, X>(PhantomData<(T, X)>);
impl<T: SendFormat, X: SendFormat> SendFormat for Any<T, X> {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        match T::serialize(obj) {
            Ok(obj) => Ok(obj),
            Err(_) => X::serialize(obj).or_else(|e| err!((invalid_data, e))),
        }
    }
}
impl<F: ReadFormat, X: ReadFormat> ReadFormat for Any<F, X> {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        match F::deserialize(bytes) {
            Ok(s) => Ok(s),
            Err(_) => X::deserialize(bytes).or_else(|e| err!((invalid_data, e))),
        }
    }
}
