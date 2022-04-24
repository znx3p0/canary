use crate::err;

/// a result type equivalent to std::io::Result, but implements `Serialize` and `Deserialize`
pub type Result<T = (), E = super::Error> = std::result::Result<T, E>;

trait ResultExt<T> {
    fn map_other(self) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ResultExt<T> for core::result::Result<T, E> {
    fn map_other(self) -> Result<T> {
        self.map_err(err!(@other))
    }
}
