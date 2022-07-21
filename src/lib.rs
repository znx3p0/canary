#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

//! # Canary
//! Canary is a library for making communication through the network easy.
//! It abstracts over network primitives such as streams and provides
//! constructs that are easier to use such as `Channel`.
//!
//! The main constructs offered by Canary are:
//! - Channels
//! - Providers
//!
//! Channels help communicate through the network,
//! and providers help create endpoints through which you can get Channels.
//!
//! The crate is well-documented, but if you need any examples
//! you should use [the book](https://znx3p0.github.io/canary-book/),
//! and additional questions should be asked in [the discord](https://discord.gg/QaWxMzAZs8)

/// Contains encrypted stream
pub mod async_snow;
/// Contains channels and constructs associated with them
pub mod channel;
mod io;
/// Contains common imports
pub mod prelude;
/// Contains providers and address
pub mod providers;

/// Contains the serialization methods for channels
/// and formats
pub mod serialization;

/// Contains types that allow compile-time checking of message order.
/// It can help debug complex systems.
pub mod type_iter;

pub use channel::channels::Channel;

pub use io_err::{err, Error, Result};
