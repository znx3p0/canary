use bincode::Options;
use serde::{de::DeserializeOwned, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::err;

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy)]
#[repr(u8)]
/// formats allowed for channels
pub enum Format {
    /// the Bincode serialization format
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
    #[cfg(feature = "messagepack_ser")]
    /// the MessagePack serialization format
    MessagePack = 5,
}

impl Default for Format {
    fn default() -> Self {
        Format::Bincode
    }
}

impl SendFormat for Format {
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        match self {
            Format::Bincode => Bincode.serialize(obj),
            #[cfg(feature = "json_ser")]
            Format::Json => Json.serialize(obj),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard.serialize(obj),
            #[cfg(feature = "messagepack_ser")]
            Format::MessagePack => MessagePack.serialize(obj),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson.serialize(obj),
        }
    }
}

impl ReadFormat for Format {
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        match self {
            Format::Bincode => Bincode.deserialize(bytes),
            #[cfg(feature = "json_ser")]
            Format::Json => Json.deserialize(bytes),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard.deserialize(bytes),
            #[cfg(feature = "messagepack_ser")]
            Format::MessagePack => MessagePack.deserialize(bytes),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson.deserialize(bytes),
        }
    }
}

impl SendFormat for &mut Format {
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        match self {
            Format::Bincode => Bincode.serialize(obj),
            #[cfg(feature = "json_ser")]
            Format::Json => Json.serialize(obj),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard.serialize(obj),
            #[cfg(feature = "messagepack_ser")]
            Format::MessagePack => MessagePack.serialize(obj),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson.serialize(obj),
        }
    }
}

impl ReadFormat for &mut Format {
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        match self {
            Format::Bincode => Bincode.deserialize(bytes),
            #[cfg(feature = "json_ser")]
            Format::Json => Json.deserialize(bytes),
            #[cfg(feature = "postcard_ser")]
            Format::Postcard => Postcard.deserialize(bytes),
            #[cfg(feature = "messagepack_ser")]
            Format::MessagePack => MessagePack.deserialize(bytes),
            #[cfg(feature = "bson_ser")]
            Format::Bson => Bson.deserialize(bytes),
        }
    }
}

/// bincode serialization format
pub struct Bincode;

#[cfg(feature = "json_ser")]
/// JSON serialization format
pub struct Json;
#[cfg(feature = "bson_ser")]
/// Postcard serialization format
pub struct Bson;

#[cfg(feature = "postcard_ser")]
/// Postcard serialization format
pub struct Postcard;

#[cfg(feature = "messagepack_ser")]
/// Postcard serialization format
pub struct MessagePack;

/// trait that represents the serialize side of a format
pub trait SendFormat {
    /// serialize object in this format
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>>;
}

/// trait that represents the deserialize side of a format
pub trait ReadFormat {
    /// deserialize object in this format
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned;
}

/// trait that represents a format that can serialize and deserialize
pub trait CompleteFormat: SendFormat + ReadFormat {}

impl SendFormat for Bincode {
    #[inline]
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        let obj = bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .serialize(obj)
            .map_err(err!(@invalid_data))?;
        Ok(obj.into())
    }
}
impl ReadFormat for Bincode {
    #[inline]
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .deserialize(bytes)
            .map_err(err!(@invalid_data))
    }
}

#[cfg(feature = "json_ser")]
impl SendFormat for Json {
    #[inline]
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(obj).map_err(err!(@invalid_data))
    }
}

#[cfg(feature = "json_ser")]
impl ReadFormat for Json {
    #[inline]
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(bytes).map_err(err!(@invalid_data))
    }
}

#[cfg(feature = "bson_ser")]
impl SendFormat for Bson {
    #[inline]
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        bson::to_vec(obj).map_err(err!(@invalid_data))
    }
}

#[cfg(feature = "bson_ser")]
impl ReadFormat for Bson {
    #[inline]
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        bson::from_slice(bytes).map_err(err!(@invalid_data))
    }
}
#[cfg(feature = "postcard_ser")]
impl SendFormat for Postcard {
    #[inline]
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        postcard::to_allocvec(obj).map_err(err!(@invalid_data))
    }
}
#[cfg(feature = "postcard_ser")]
impl ReadFormat for Postcard {
    #[inline]
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        postcard::from_bytes(bytes).map_err(err!(@invalid_data))
    }
}

#[cfg(feature = "messagepack_ser")]
impl SendFormat for MessagePack {
    #[inline]
    fn serialize<O: Serialize>(&mut self, obj: &O) -> crate::Result<Vec<u8>> {
        rmp_serde::to_vec(obj).map_err(err!(@invalid_data))
    }
}
#[cfg(feature = "messagepack_ser")]
impl ReadFormat for MessagePack {
    #[inline]
    fn deserialize<T>(&mut self, bytes: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        rmp_serde::from_slice(bytes).map_err(err!(@invalid_data))
    }
}
