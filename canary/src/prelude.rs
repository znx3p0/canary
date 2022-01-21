
pub use crate::Addr;
pub use crate::ServiceAddr;
pub use crate::Channel;
pub use crate::err;
pub use crate::Result as Res; // imported as Res to avoid collisions with Result
pub use crate::service;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::Ctx;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::routes::GLOBAL_ROUTE;

