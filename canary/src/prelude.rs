pub use crate::err;
pub use crate::service;
pub use crate::Addr;
pub use crate::Channel;
pub use crate::Result as Res; // imported as Res to avoid collisions with Result
pub use crate::ServiceAddr;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::routes::GLOBAL_ROUTE;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::Ctx;
