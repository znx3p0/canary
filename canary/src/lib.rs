#![forbid(unsafe_code)]
// #![forbid(missing_docs)]

//! IGCP | intergalactic communications protocol
//!
//! IGCP is designed to abstract over streams or low-level communication protocols.
//! IGCP should be as high-level as possible while keeping configurability and zero-cost to the wire.
//! The main abstraction IGCP offers are channels, which represent a stream of objects
//! that can be sent or received.
//! At the moment, IGCP channels are not completely zero-cost.

/// contains encrypted stream
mod async_snow;
pub mod channel;
/// contains custom error types and result
pub mod err;
mod io;
pub mod providers;
/// contains the serialization methods for channels
/// and formats
pub mod serialization;
pub mod type_iter;

pub use channel::Channel;
pub use err::Error;
pub use err::Result;
