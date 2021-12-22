use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Serialize_repr, Deserialize_repr)]
// used for discovery
#[repr(u8)]
pub enum Status {
    Found = 1,
    NotFound = 2,
}
