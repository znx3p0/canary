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

    (($($t: tt)*)) => {
        Err($crate::err!($($t)*))
    };
    (@$i: ident) => {
        {
            |e| $crate::err!($i, e)
        }
    };
    ($e: expr) => {
        $crate::err!(other, $e)
    };
}

use serde::{ser::SerializeTuple, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    fmt::{Debug, Display},
    io::ErrorKind as StdErrorKind,
};

#[repr(transparent)]
/// a result type equivalent to std::io::Error, but implements `Serialize` and `Deserialize`
pub struct Error(std::io::Error);
impl Error {
    #[inline]
    /// construct a new Error type from a std::io::Error
    pub fn new(e: std::io::Error) -> Self {
        Error(e)
    }
}

impl std::ops::Deref for Error {
    type Target = std::io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Error {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Error(error)
    }
}

impl From<Error> for std::io::Error {
    #[inline]
    fn from(error: Error) -> Self {
        error.0
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <std::io::Error as Display>::fmt(&self.0, f)
    }
}
impl Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <std::io::Error as Debug>::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }

    #[inline]
    fn description(&self) -> &str {
        #[allow(deprecated)]
        self.0.description()
    }

    #[inline]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        #[allow(deprecated)]
        self.0.cause()
    }
}

impl Serialize for Error {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.0.to_string();
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&string)?;
        let kind: ErrorKind = self.0.kind().into();
        tuple.serialize_element(&kind)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (error, kind) = <(String, ErrorKind)>::deserialize(deserializer)?;
        Ok(Error(std::io::Error::new(kind.into(), error)))
    }
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
/// Serializable version of `std::io::ErrorKind`
pub enum ErrorKind {
    /// An entity was not found, often a file.
    NotFound,
    /// The operation lacked the necessary privileges to complete.
    PermissionDenied,
    /// The connection was refused by the remote server.
    ConnectionRefused,
    /// The connection was reset by the remote server.
    ConnectionReset,
    /// The remote host is not reachable.
    HostUnreachable,
    /// The network containing the remote host is not reachable.
    NetworkUnreachable,
    /// The connection was aborted (terminated) by the remote server.
    ConnectionAborted,
    /// The network operation failed because it was not connected yet.
    NotConnected,
    /// A socket address could not be bound because the address is already in
    /// use elsewhere.
    AddrInUse,
    /// A nonexistent interface was requested or the requested address was not
    /// local.
    AddrNotAvailable,
    /// The system's networking is down.
    NetworkDown,
    /// The operation failed because a pipe was closed.
    BrokenPipe,
    /// An entity already exists, often a file.
    AlreadyExists,
    /// The operation needs to block to complete, but the blocking operation was
    /// requested to not occur.
    WouldBlock,
    /// A filesystem object is, unexpectedly, not a directory.
    ///
    /// For example, a filesystem path was specified where one of the intermediate directory
    /// components was, in fact, a plain file.
    NotADirectory,
    /// The filesystem object is, unexpectedly, a directory.
    ///
    /// A directory was specified when a non-directory was expected.
    IsADirectory,
    /// A non-empty directory was specified where an empty directory was expected.
    DirectoryNotEmpty,
    /// The filesystem or storage medium is read-only, but a write operation was attempted.
    ReadOnlyFilesystem,
    /// Loop in the filesystem or IO subsystem; often, too many levels of symbolic links.
    ///
    /// There was a loop (or excessively long chain) resolving a filesystem object
    /// or file IO object.
    ///
    /// On Unix this is usually the result of a symbolic link loop; or, of exceeding the
    /// system-specific limit on the depth of symlink traversal.
    FilesystemLoop,
    /// Stale network file handle.
    ///
    /// With some network filesystems, notably NFS, an open file (or directory) can be invalidated
    /// by problems with the network or server.
    StaleNetworkFileHandle,
    /// A parameter was incorrect.
    InvalidInput,
    /// Data not valid for the operation were encountered.
    ///
    /// Unlike [`InvalidInput`], this typically means that the operation
    /// parameters were valid, however the error was caused by malformed
    /// input data.
    ///
    /// For example, a function that reads a file into a string will error with
    /// `InvalidData` if the file's contents are not valid UTF-8.
    ///
    /// [`InvalidInput`]: ErrorKind::InvalidInput
    InvalidData,
    /// The I/O operation's timeout expired, causing it to be canceled.
    TimedOut,
    /// An error returned when an operation could not be completed because a
    /// call to [`write`] returned [`Ok(0)`].
    ///
    /// This typically means that an operation could only succeed if it wrote a
    /// particular number of bytes but only a smaller number of bytes could be
    /// written.
    ///
    /// [`write`]: crate::io::Write::write
    /// [`Ok(0)`]: Ok
    WriteZero,
    /// The underlying storage (typically, a filesystem) is full.
    ///
    /// This does not include out of quota errors.
    StorageFull,
    /// Seek on unseekable file.
    ///
    /// Seeking was attempted on an open file handle which is not suitable for seeking - for
    /// example, on Unix, a named pipe opened with `File::open`.
    NotSeekable,
    /// Filesystem quota was exceeded.
    FilesystemQuotaExceeded,
    /// File larger than allowed or supported.
    ///
    /// This might arise from a hard limit of the underlying filesystem or file access API, or from
    /// an administratively imposed resource limitation.  Simple disk full, and out of quota, have
    /// their own errors.
    FileTooLarge,
    /// Resource is busy.
    ResourceBusy,
    /// Executable file is busy.
    ///
    /// An attempt was made to write to a file which is also in use as a running program.  (Not all
    /// operating systems detect this situation.)
    ExecutableFileBusy,
    /// Deadlock (avoided).
    ///
    /// A file locking operation would result in deadlock.  This situation is typically detected, if
    /// at all, on a best-effort basis.
    Deadlock,
    /// Cross-device or cross-filesystem (hard) link or rename.
    CrossesDevices,
    /// Too many (hard) links to the same filesystem object.
    ///
    /// The filesystem does not support making so many hardlinks to the same file.
    TooManyLinks,
    /// Filename too long.
    ///
    /// The limit might be from the underlying filesystem or API, or an administratively imposed
    /// resource limit.
    FilenameTooLong,
    /// Program argument list too long.
    ///
    /// When trying to run an external program, a system or process limit on the size of the
    /// arguments would have been exceeded.
    ArgumentListTooLong,
    /// This operation was interrupted.
    ///
    /// Interrupted operations can typically be retried.
    Interrupted,
    /// This operation is unsupported on this platform.
    ///
    /// This means that the operation can never succeed.
    Unsupported,
    // ErrorKinds which are primarily categorisations for OS error
    // codes should be added above.
    //
    /// An error returned when an operation could not be completed because an
    /// "end of file" was reached prematurely.
    ///
    /// This typically means that an operation could only succeed if it read a
    /// particular number of bytes but only a smaller number of bytes could be
    /// read.
    UnexpectedEof,
    /// An operation could not be completed, because it failed
    /// to allocate enough memory.
    OutOfMemory,
    // "Unusual" error kinds which do not correspond simply to (sets
    // of) OS error codes, should be added just above this comment.
    // `Other` and `Uncategorised` should remain at the end:
    //
    /// A custom error that does not fall under any other I/O error kind.
    ///
    /// This can be used to construct your own [`Error`]s that do not match any
    /// [`ErrorKind`].
    ///
    /// This [`ErrorKind`] is not used by the standard library.
    ///
    /// Errors from the standard library that do not fall under any of the I/O
    /// error kinds cannot be `match`ed on, and will only match a wildcard (`_`) pattern.
    /// New [`ErrorKind`]s might be added in the future for some of those.
    Other,
    /// Any I/O error from the standard library that's not part of this list.
    ///
    /// Errors that are `Uncategorized` now may move to a different or a new
    /// [`ErrorKind`] variant in the future. It is not recommended to match
    /// an error against `Uncategorized`; use a wildcard match (`_`) instead.
    Uncategorized,
}

impl From<ErrorKind> for StdErrorKind {
    #[inline(always)]
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::NotFound => StdErrorKind::NotFound,
            ErrorKind::PermissionDenied => StdErrorKind::PermissionDenied,
            ErrorKind::ConnectionRefused => StdErrorKind::ConnectionRefused,
            ErrorKind::ConnectionReset => StdErrorKind::ConnectionReset,
            ErrorKind::ConnectionAborted => StdErrorKind::ConnectionAborted,
            ErrorKind::NotConnected => StdErrorKind::NotConnected,
            ErrorKind::AddrInUse => StdErrorKind::AddrInUse,
            ErrorKind::AddrNotAvailable => StdErrorKind::AddrNotAvailable,
            ErrorKind::BrokenPipe => StdErrorKind::BrokenPipe,
            ErrorKind::AlreadyExists => StdErrorKind::AlreadyExists,
            ErrorKind::WouldBlock => StdErrorKind::WouldBlock,
            ErrorKind::InvalidInput => StdErrorKind::InvalidInput,
            ErrorKind::InvalidData => StdErrorKind::InvalidData,
            ErrorKind::TimedOut => StdErrorKind::TimedOut,
            ErrorKind::WriteZero => StdErrorKind::WriteZero,
            ErrorKind::Interrupted => StdErrorKind::Interrupted,
            ErrorKind::Unsupported => StdErrorKind::Unsupported,
            ErrorKind::UnexpectedEof => StdErrorKind::UnexpectedEof,
            ErrorKind::OutOfMemory => StdErrorKind::OutOfMemory,
            ErrorKind::Other => StdErrorKind::Other,
            _ => StdErrorKind::Other,
        }
    }
}

impl From<StdErrorKind> for ErrorKind {
    #[inline(always)]
    fn from(kind: StdErrorKind) -> Self {
        match kind {
            StdErrorKind::NotFound => ErrorKind::NotFound,
            StdErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            StdErrorKind::ConnectionRefused => ErrorKind::ConnectionRefused,
            StdErrorKind::ConnectionReset => ErrorKind::ConnectionReset,
            StdErrorKind::ConnectionAborted => ErrorKind::ConnectionAborted,
            StdErrorKind::NotConnected => ErrorKind::NotConnected,
            StdErrorKind::AddrInUse => ErrorKind::AddrInUse,
            StdErrorKind::AddrNotAvailable => ErrorKind::AddrNotAvailable,
            StdErrorKind::BrokenPipe => ErrorKind::BrokenPipe,
            StdErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            StdErrorKind::WouldBlock => ErrorKind::WouldBlock,
            StdErrorKind::InvalidInput => ErrorKind::InvalidInput,
            StdErrorKind::InvalidData => ErrorKind::InvalidData,
            StdErrorKind::TimedOut => ErrorKind::TimedOut,
            StdErrorKind::WriteZero => ErrorKind::WriteZero,
            StdErrorKind::Interrupted => ErrorKind::Interrupted,
            StdErrorKind::Unsupported => ErrorKind::Unsupported,
            StdErrorKind::UnexpectedEof => ErrorKind::UnexpectedEof,
            StdErrorKind::OutOfMemory => ErrorKind::OutOfMemory,
            StdErrorKind::Other => ErrorKind::Other,
            _ => ErrorKind::Other,
        }
    }
}
