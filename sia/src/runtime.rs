#![allow(unused_imports)]

#[cfg(not(any(feature = "runtime-tokio", feature = "runtime-async-std")))]
compile_error!("one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

#[cfg(all(feature = "runtime-tokio", feature = "runtime-async-std"))]
compile_error!("only one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

#[cfg(feature = "runtime-async-std")]
pub use async_std::{
    future::timeout,
    task::{block_on, sleep, spawn, spawn_local, JoinHandle},
    task_local,
};

use std::future::Future;
#[cfg(feature = "runtime-tokio")]
pub use tokio::{
    task::{spawn, JoinHandle},
    task_local,
    time::{sleep, timeout},
};

#[cfg(feature = "runtime-tokio")]
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}
