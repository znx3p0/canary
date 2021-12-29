pub mod providers;
pub mod routes;
pub mod runtime;
pub mod service;

pub use canary_macro::*;
pub use igcp;
pub use igcp::{err, pipe, pipeline, Channel};
pub use serde::{Deserialize, Serialize};

pub use igcp::Result;
pub use providers::Addr;
pub use providers::ServiceAddr;
