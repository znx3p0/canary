#![allow(unused_imports)]
#![cfg(not(target_arch = "wasm32"))]

use std::future::Future;

cfg_if::cfg_if! {
    if #[cfg(feature = "async-std-net")] {
        pub use async_std::{
            future::timeout,
            task::sleep,
        };
    } else if #[cfg(feature = "tokio-net")] {
        pub use tokio::time::{sleep, timeout};

    } else {
        compile_error!("one of 'async-std-net' or 'tokio-net' features must be enabled");
    }
}
