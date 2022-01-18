use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
/// used for discovery
pub enum Status {
    /// indicates a service has been found
    Found = 1,
    /// indicates a service has not been found
    NotFound = 2,
}