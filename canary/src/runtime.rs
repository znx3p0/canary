#![allow(unused_imports)]
#![cfg(not(target_arch = "wasm32"))]

use std::future::Future;

cfg_if::cfg_if! {
    if #[cfg(feature = "async-std-rt")] {
        pub use async_std::{
            future::timeout,
            task::{block_on, spawn_local, sleep, spawn, JoinHandle},
            task_local,
        };
    } else if #[cfg(feature = "tokio-rt")] {
        pub use tokio::{
            task::{spawn, spawn_local, JoinHandle},
            task_local,
            time::{sleep, timeout},
        };
        /// creates a new runtime and blocks on the future
        pub fn block_on<F, T>(future: F) -> T
        where
            F: Future<Output = T>,
        {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(future)
        }
    } else {
        compile_error!("one of 'async-std-rt' or 'tokio-rt' features must be enabled");
    }
}
