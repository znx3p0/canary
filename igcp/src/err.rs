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
        std::io::Error::new(std::io::ErrorKind::NotFound, $e)
    };
    (permission_denied, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, $e)
    };
    (conn_refused, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ConnectionRefused, $e)
    };
    (conn_reset, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ConnectionReset, $e)
    };
    (host_unreachable, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::HostUnreachable, $e)
    };
    (net_unreachable, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, $e)
    };
    (conn_aborted, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ConnectionAborted, $e)
    };
    (not_connected, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::NotConnected, $e)
    };
    (in_use, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::AddrInUse, $e)
    };
    (addr_not_available, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, $e)
    };
    (net_down, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::NetworkDown, $e)
    };
    (broken_pipe, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, $e)
    };
    (already_exists, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::AlreadyExists, $e)
    };
    (would_block, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::WouldBlock, $e)
    };
    (not_a_dir, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::NotADirectory, $e)
    };
    (is_a_dir, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::IsADirectory, $e)
    };
    (dir_not_empty, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::DirectoryNotEmpty, $e)
    };
    (read_only_fs, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ReadOnlyFilesystem, $e)
    };
    (fs_loop, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::FilesystemLoop, $e)
    };
    (stale_net_filehandle, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::StaleNetworkFileHandle, $e)
    };
    (invalid_input, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, $e)
    };
    (invalid_data, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::InvalidData, $e)
    };
    (timeout, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::TimedOut, $e)
    };
    (write_zero, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::WriteZero, $e)
    };
    (storage_full, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::StorageFull, $e)
    };
    (not_seekable, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::NotSeekable, $e)
    };
    (fs_quota_exceeded, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::FilesystemQuotaExceeded, $e)
    };
    (file_too_large, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::FileTooLarge, $e)
    };
    (resource_busy, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ResourceBusy, $e)
    };
    (executable_busy, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ExecutableFileBusy, $e)
    };
    (deadlock, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::Deadlock, $e)
    };
    (crosses_devices, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::CrossesDevices, $e)
    };
    (too_many_links, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::TooManyLinks, $e)
    };
    (filename_too_long, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::FilenameTooLong, $e)
    };
    (argument_list_too_long, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::ArgumentListTooLong, $e)
    };
    (interrupted, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::Interrupted, $e)
    };
    (unsupported, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::Unsupported, $e)
    };
    (unexpected_eof, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, $e)
    };
    (out_of_memory, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::OutOfMemory, $e)
    };
    (other, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::Other, $e)
    };
    (uncategorized, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::Uncategorized, $e)
    };
    ($p: ident, $e: expr) => {
        std::io::Error::new(std::io::ErrorKind::$p, $e)
    };
    (($e: expr)) => {
        Err($crate::err!(other, $e))
    };
    ($e: expr) => {
        $crate::err!(other, $e)
    };
}
