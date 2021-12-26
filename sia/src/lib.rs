pub mod providers;
pub mod routes;
pub mod runtime;
pub mod service;

pub use igcp;
pub use igcp::{err, pipe, pipeline, Channel};
pub use serde::{Deserialize, Serialize};
pub use sia_macro::*;

pub use igcp::Result;
pub use providers::Addr;
pub use providers::ServiceAddr;
