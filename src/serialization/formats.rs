use bincode::Options;
use serde::Serialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::err;

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
/// formats allowed for channels
pub enum Format {
    /// the bincode serialization format
    Bincode = 1,
    #[cfg(feature = "json_ser")]
    /// the JSON serialization format
    Json = 2,
    #[cfg(feature = "bson_ser")]
    /// the BSON serialization format
    Bson = 3,
    #[cfg(feature = "postcard_ser")]
    /// the Postcard serialization format
    Postcard = 4,
}

impl SendFormat for Format {
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>> {
        match self {
            Format::Bincode => Bincode::serialize(&Bincode, obj),
            #[cfg(feature = "json_ser")]
            Format::Json => Json::serialize(&Json, obj),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson::serialize(&Bson, obj),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard::serialize(&Postcard, obj),
        }
    }
}

impl ReadFormat for Format {
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        match self {
            Format::Bincode => Bincode::deserialize(&Bincode, bytes),
            #[cfg(feature = "json_ser")]
            Format::Json => Json::deserialize(&Json, bytes),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson::deserialize(&Bson, bytes),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard::deserialize(&Postcard, bytes),
        }
    }
}

/// bincode serialization format
pub struct Bincode;

#[cfg(feature = "json_ser")]
/// JSON serialization format
pub struct Json;
/// BSON serialization format
#[cfg(feature = "bson_ser")]
pub struct Bson;

#[cfg(feature = "postcard_ser")]
/// Postcard serialization format
pub struct Postcard;

/// trait that represents the serialize side of a format
pub trait SendFormat {
    /// serialize object in this format
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>>;
}

/// trait that represents the deserialize side of a format
pub trait ReadFormat {
    /// deserialize object in this format
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>;
}

/// trait that represents a format that can serialize and deserialize
pub trait CompleteFormat: SendFormat + ReadFormat {}

impl SendFormat for Bincode {
    #[inline]
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>> {
        let obj = bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .serialize(obj)
            .map_err(|e| err!(invalid_data, e))?;
        Ok(obj)
    }
}
impl ReadFormat for Bincode {
    #[inline]
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .deserialize(bytes)
            .map_err(|e| err!(invalid_data, e))
    }
}

#[cfg(feature = "json_ser")]
impl SendFormat for Json {
    #[inline]
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(obj).map_err(|e| err!((invalid_data, e)))
    }
}
#[cfg(feature = "json_ser")]
impl ReadFormat for Json {
    #[inline]
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        serde_json::from_slice(bytes).map_err(|e| err!((invalid_data, e)))
    }
}
#[cfg(feature = "bson_ser")]
impl SendFormat for Bson {
    #[inline]
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>> {
        bson::ser::to_vec(obj).map_err(|e| err!((invalid_data, e)))
    }
}
#[cfg(feature = "bson_ser")]
impl ReadFormat for Bson {
    #[inline]
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        bson::de::from_slice(bytes).map_err(|e| err!((invalid_data, e)))
    }
}
#[cfg(feature = "postcard_ser")]
impl SendFormat for Postcard {
    #[inline]
    fn serialize<O: Serialize>(&self, obj: &O) -> crate::Result<Vec<u8>> {
        postcard::to_allocvec(obj).map_err(|e| err!((invalid_data, e)))
    }
}
#[cfg(feature = "postcard_ser")]
impl ReadFormat for Postcard {
    #[inline]
    fn deserialize<'a, T>(&self, bytes: &'a [u8]) -> crate::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        postcard::from_bytes(bytes).map_err(|e| err!((invalid_data, e)))
    }
}
