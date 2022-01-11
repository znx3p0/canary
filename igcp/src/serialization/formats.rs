use std::marker::PhantomData;

use serde::Serialize;
use bincode::Options;

use crate::err;

/// bincode serialization format
pub struct Bincode;
/// JSON serialization format
pub struct Json;
/// BSON serialization format
pub struct Bson;
/// Postcard serialization format
pub struct Postcard;

/// trait that represents the serialize side of a format
pub trait SendFormat {
    /// serialize object in this format
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>>;
}

/// trait that represents the deserialize side of a format
pub trait ReadFormat {
    /// deserialize object in this format
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>;
}

/// trait that represents a format that can serialize and deserialize
pub trait Format: SendFormat + ReadFormat {}

impl SendFormat for Bincode {
    fn serialize<O: Serialize>(obj: &O) -> crate::Result<Vec<u8>> {
        let obj = bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .serialize(obj)
            .or_else(|e| err!((invalid_data, e)))?;
        Ok(obj)
    }
}
impl ReadFormat for Bincode {
    fn deserialize<'a, T>(bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .deserialize(bytes)
            .or_else(|e| err!((invalid_data, e)))
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

/// combinator that allows input from any two formats:
/// ```norun
/// type BincodeOrJson = Any<Bincode, Json>
/// ```
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
