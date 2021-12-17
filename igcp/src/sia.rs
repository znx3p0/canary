use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
// used for discovery
pub(crate) enum Status {
    Found,
    NotFound,
}
