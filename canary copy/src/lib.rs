#![forbid(unsafe_code)]
// #![deny(missing_docs)]
#![cfg(any(feature = "tokio-net", feature = "async-std-net"))]

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

cfg_if::cfg_if! {
    if #[cfg(all(feature = "tokio-net", feature = "async-std-net"))] {
        compile_error!("only one of 'async-std-net' or 'tokio-net' features must be enabled; maybe you should try adding `default-features = false`?");
    } else if #[cfg(not(any(feature = "tokio-net", feature = "async-std-net")))] {
        compile_error!("one of 'async-std-net' or 'tokio-net' features must be enabled");
    } else {
        /// contains discovery structures
        pub mod discovery;
        /// offers the main types needed to use canary
        pub mod prelude;
        /// offers providers, which expose services through the network
        pub mod providers;
        /// offers the runtime used by Canary to run the services, `tokio` by default
        pub mod runtime;
        mod io;

        pub use canary_macro::*;
        pub use igcp;
        pub use igcp::{err, pipe, pipeline, Channel};
        pub use serde::{Deserialize, Serialize};

        pub use igcp::Result;
        pub use providers::Addr;
    }
}
