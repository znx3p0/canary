/// construct an io error rapidly
/// ```
/// let error: std::io::Error = err!(other, "another error");
/// let other: std::io::Error = err!("another unspecified error, will be categorized as `Other`");
/// let result: Result<(), std::io::Error> = err!(("this error will be encapsulated under an Err()"));
/// fn chosen_one<T>(ty: T) -> Result<T> {
///     if random() {
///         // unspecified errors will always be classified as `Other`
///         err!(("you died in the process"))? // this will return early
///     } else if !chosen() {
///         err!((permission_denied, "you are not the chosen one"))?
///     }
///     Ok(ty)
/// }
/// ```
#[macro_export]
macro_rules! err {
    (($t: ident, $s: expr)) => {
        Err($crate::err!($t, $s))
    };
    (not_found, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, $e))
    };
    (permission_denied, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, $e))
    };
    (conn_refused, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, $e))
    };
    (conn_reset, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ConnectionReset, $e))
    };
    (host_unreachable, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::HostUnreachable, $e))
    };
    (net_unreachable, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, $e))
    };
    (conn_aborted, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, $e))
    };
    (not_connected, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NotConnected, $e))
    };
    (in_use, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::AddrInUse, $e))
    };
    (addr_not_available, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, $e))
    };
    (net_down, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NetworkDown, $e))
    };
    (broken_pipe, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::BrokenPipe, $e))
    };
    (already_exists, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::AlreadyExists, $e))
    };
    (would_block, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::WouldBlock, $e))
    };
    (not_a_dir, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NotADirectory, $e))
    };
    (is_a_dir, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::IsADirectory, $e))
    };
    (dir_not_empty, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::DirectoryNotEmpty, $e))
    };
    (read_only_fs, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ReadOnlyFilesystem, $e))
    };
    (fs_loop, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::FilesystemLoop, $e))
    };
    (stale_net_filehandle, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::StaleNetworkFileHandle, $e))
    };
    (invalid_input, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, $e))
    };
    (invalid_data, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::InvalidData, $e))
    };
    (timeout, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::TimedOut, $e))
    };
    (write_zero, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::WriteZero, $e))
    };
    (storage_full, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::StorageFull, $e))
    };
    (not_seekable, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::NotSeekable, $e))
    };
    (fs_quota_exceeded, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::FilesystemQuotaExceeded, $e))
    };
    (file_too_large, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::FileTooLarge, $e))
    };
    (resource_busy, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ResourceBusy, $e))
    };
    (executable_busy, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ExecutableFileBusy, $e))
    };
    (deadlock, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Deadlock, $e))
    };
    (crosses_devices, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::CrossesDevices, $e))
    };
    (too_many_links, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::TooManyLinks, $e))
    };
    (filename_too_long, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::FilenameTooLong, $e))
    };
    (argument_list_too_long, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::ArgumentListTooLong, $e))
    };
    (interrupted, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Interrupted, $e))
    };
    (unsupported, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Unsupported, $e))
    };
    (unexpected_eof, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, $e))
    };
    (out_of_memory, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::OutOfMemory, $e))
    };
    (other, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Other, $e))
    };
    (uncategorized, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::Uncategorized, $e))
    };
    ($p: ident, $e: expr) => {
        $crate::err::Error::new(std::io::Error::new(std::io::ErrorKind::$p, $e))
    };
    (($e: expr)) => {
        Err($crate::err!(other, $e))
    };
    ($e: expr) => {
        $crate::err!(other, $e)
    };
}


use std::{io::ErrorKind, fmt::{Display, Debug}};
use serde::{Serialize, Deserialize, ser::SerializeTuple};
use serde_repr::{Serialize_repr, Deserialize_repr};

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct Error(std::io::Error);
impl Error {
    pub fn new(e: std::io::Error) -> Self {
        Error(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <std::io::Error as Display>::fmt(&self.0, f)
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <std::io::Error as Debug>::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }

    fn description(&self) -> &str {
        #[allow(deprecated)]
        self.0.description()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        #[allow(deprecated)]
        self.0.cause()
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let string = self.0.to_string();
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&string)?;
        let kind: ErrorKindSer = self.0.kind().into();
        tuple.serialize_element(&kind)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let (error, kind) = <(String, ErrorKindSer)>::deserialize(deserializer)?;
        Ok(Error(std::io::Error::new(kind.into(), error)))
    }
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ErrorKindSer {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    FilesystemQuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    FilenameTooLong,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
    Uncategorized,
}

impl From<ErrorKindSer> for ErrorKind {
    fn from(kind: ErrorKindSer) -> Self {
        match kind {
            ErrorKindSer::NotFound => ErrorKind::NotFound,
            ErrorKindSer::PermissionDenied => ErrorKind::PermissionDenied,
            ErrorKindSer::ConnectionRefused => ErrorKind::ConnectionRefused,
            ErrorKindSer::ConnectionReset => ErrorKind::ConnectionReset,
            ErrorKindSer::ConnectionAborted => ErrorKind::ConnectionAborted,
            ErrorKindSer::NotConnected => ErrorKind::NotConnected,
            ErrorKindSer::AddrInUse => ErrorKind::AddrInUse,
            ErrorKindSer::AddrNotAvailable => ErrorKind::AddrNotAvailable,
            ErrorKindSer::BrokenPipe => ErrorKind::BrokenPipe,
            ErrorKindSer::AlreadyExists => ErrorKind::AlreadyExists,
            ErrorKindSer::WouldBlock => ErrorKind::WouldBlock,
            ErrorKindSer::InvalidInput => ErrorKind::InvalidInput,
            ErrorKindSer::InvalidData => ErrorKind::InvalidData,
            ErrorKindSer::TimedOut => ErrorKind::TimedOut,
            ErrorKindSer::WriteZero => ErrorKind::WriteZero,
            ErrorKindSer::Interrupted => ErrorKind::Interrupted,
            ErrorKindSer::Unsupported => ErrorKind::Unsupported,
            ErrorKindSer::UnexpectedEof => ErrorKind::UnexpectedEof,
            ErrorKindSer::OutOfMemory => ErrorKind::OutOfMemory,
            ErrorKindSer::Other => ErrorKind::Other,
            _ => ErrorKind::Other,
        }
    }
}

impl From<ErrorKind> for ErrorKindSer {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::NotFound => ErrorKindSer::NotFound,
            ErrorKind::PermissionDenied => ErrorKindSer::PermissionDenied,
            ErrorKind::ConnectionRefused => ErrorKindSer::ConnectionRefused,
            ErrorKind::ConnectionReset => ErrorKindSer::ConnectionReset,
            ErrorKind::ConnectionAborted => ErrorKindSer::ConnectionAborted,
            ErrorKind::NotConnected => ErrorKindSer::NotConnected,
            ErrorKind::AddrInUse => ErrorKindSer::AddrInUse,
            ErrorKind::AddrNotAvailable => ErrorKindSer::AddrNotAvailable,
            ErrorKind::BrokenPipe => ErrorKindSer::BrokenPipe,
            ErrorKind::AlreadyExists => ErrorKindSer::AlreadyExists,
            ErrorKind::WouldBlock => ErrorKindSer::WouldBlock,
            ErrorKind::InvalidInput => ErrorKindSer::InvalidInput,
            ErrorKind::InvalidData => ErrorKindSer::InvalidData,
            ErrorKind::TimedOut => ErrorKindSer::TimedOut,
            ErrorKind::WriteZero => ErrorKindSer::WriteZero,
            ErrorKind::Interrupted => ErrorKindSer::Interrupted,
            ErrorKind::Unsupported => ErrorKindSer::Unsupported,
            ErrorKind::UnexpectedEof => ErrorKindSer::UnexpectedEof,
            ErrorKind::OutOfMemory => ErrorKindSer::OutOfMemory,
            ErrorKind::Other => ErrorKindSer::Other,
            _ => ErrorKindSer::Other,
        }
    }
}



