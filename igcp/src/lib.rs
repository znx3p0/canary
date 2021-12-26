///! IGCP | intergalactic communications protocol
///!
///! IGCP is designed to abstract over streams or low-level communication protocols.
///! IGCP should be as high-level as possible while keeping configurability and zero-cost to the wire.
///! The main abstraction IGCP offers are channels, which represent a stream of objects
///! that can be sent or received.
///! At the moment, IGCP channels are not completely zero-cost.
mod addr;
pub mod async_snow;
mod channel;
pub mod err;
mod io;
pub mod serialization;
pub mod sia;
pub mod type_iter;

pub use addr::Addr;
pub use channel::*;
pub use err::Error;
pub use err::Result;
