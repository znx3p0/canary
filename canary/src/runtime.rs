#![allow(unused_imports)]
#![cfg(not(target_arch = "wasm32"))]

#[cfg(not(any(feature = "rt-tokio", feature = "rt-async-std")))]
compile_error!("one of 'rt-async-std' or 'rt-tokio' features must be enabled");

#[cfg(all(feature = "rt-tokio", feature = "rt-async-std"))]
compile_error!("only one of 'rt-async-std' or 'rt-tokio' features must be enabled");

#[cfg(feature = "rt-async-std")]
pub use async_std::{
    future::timeout,
    task::{block_on, sleep, spawn, spawn_local, JoinHandle},
    task_local,
};

use std::future::Future;
#[cfg(feature = "rt-tokio")]
pub use tokio::{
    task::{spawn, JoinHandle},
    task_local,
    time::{sleep, timeout},
};

#[cfg(feature = "rt-tokio")]
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}
