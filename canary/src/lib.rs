#![forbid(unsafe_code)]
// #![deny(missing_docs)]

//! # Canary
//! Canary is a library for making communication through the network easy.
//! It abstracts over network primitives such as streams and provides
//! constructs that are easier to use such as `Channel`.
//!
//! The main constructs offered by Canary are:
//! - Channels
//! - Services
//! - Providers
//!
//! Channels help communicate through the network,
//! services help offer multiple endpoints of communication
//! and providers help expose services through the network.
//!
//! The crate is well-documented, but if you need any examples
//! you should use [the book](https://znx3p0.github.io/canary-book/),
//! and any questions should be asked in [the discord](https://discord.gg/QaWxMzAZs8)

pub mod discovery;
/// offers providers, which expose services through the network
pub mod providers;
/// offers the routing system used by services
pub mod routes;
/// offers the runtime used by Canary to run the services, `async-std` by default
pub mod runtime;
/// offers services and helper traits
pub mod service;

pub use canary_macro::*;
pub use igcp;
pub use igcp::{err, pipe, pipeline, Channel};
pub use serde::{Deserialize, Serialize};

pub use igcp::Result;
pub use providers::Addr;
pub use providers::ServiceAddr;

#[cfg(not(target_arch = "wasm32"))]
pub use routes::Ctx;
