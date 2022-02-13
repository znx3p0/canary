#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![cfg_attr(feature = "nightly", feature(generic_associated_types))] // used for async_trait support
#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]

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
//! and providers help expose services through the network.
//!
//! The crate is well-documented, but if you need any examples
//! you should use [the book](https://znx3p0.github.io/canary-book/),
//! and any questions should be asked in [the discord](https://discord.gg/QaWxMzAZs8)

/// contains encrypted stream
pub mod async_snow;
/// contains channels and constructs associated with them
pub mod channel;
/// contains custom error types and result
pub mod err;
mod io;
/// contains common imports
pub mod prelude;
/// contains providers and address
pub mod providers;
/// contains the serialization methods for channels
/// and formats
pub mod serialization;
pub mod type_iter;

pub use channel::Channel;
pub use err::Error;
pub use err::Result;
