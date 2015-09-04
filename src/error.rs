
//! Define 9P error representations
//!
//! In 9P2000, errors are represented as strings.
//! All imported from include/net/9p/error.c of Linux kernel
//!
//! Since 9P2000.L, errors are represented as error numbers (errno).

extern crate nix;
extern crate byteorder;

use std::{io, fmt};
use std::io::ErrorKind::*;
use std::error as stderror;
use error::errno::*;

fn errno_from_ioerror(e: &io::Error) -> nix::errno::Errno {
    e.raw_os_error()
        .map(nix::errno::from_i32)
        .unwrap_or(match e.kind() {
            NotFound            => ENOENT,
            PermissionDenied    => EPERM,
            ConnectionRefused   => ECONNREFUSED,
            ConnectionReset     => ECONNRESET,
            ConnectionAborted   => ECONNABORTED,
            NotConnected        => ENOTCONN,
            AddrInUse           => EADDRINUSE,
            AddrNotAvailable    => EADDRNOTAVAIL,
            BrokenPipe          => EPIPE,
            AlreadyExists       => EALREADY,
            WouldBlock          => EAGAIN,
            InvalidInput        => EINVAL,
            InvalidData         => EINVAL,
            TimedOut            => ETIMEDOUT,
            WriteZero           => EAGAIN,
            Interrupted         => EINTR,
            Other | _           => EIO,
        }
    )
}

/// 9P error type which is convertible to an errno.
///
/// The value of `Error::errno()` will be used for Rlerror.
///
/// # Protocol
/// 9P2000.L
#[derive(Debug)]
pub enum Error {
    /// System error containing an errno
    No(nix::errno::Errno),
    /// I/O error
    Io(io::Error)
}

impl Error {
    /// Get an errno representations
    pub fn errno(&self) -> nix::errno::Errno {
        match *self {
            Error::No(ref e) => e.clone(),
            Error::Io(ref e) => errno_from_ioerror(e)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::No(ref e) => write!(f, "System error: {}", e.desc()),
            Error::Io(ref e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl stderror::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::No(ref e) => e.desc(),
            Error::Io(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&stderror::Error> {
        match *self {
            Error::No(_) => None,
            Error::Io(ref e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::Io(e) }
}

impl<'a> From<&'a io::Error> for Error {
    fn from(e: &'a io::Error) -> Self { Error::No(errno_from_ioerror(e)) }
}

impl From<nix::errno::Errno> for Error {
    fn from(e: nix::errno::Errno) -> Self { Error::No(e) }
}

impl From<nix::Error> for Error {
    fn from(e: nix::Error) -> Self { Error::No(e.errno()) }
}

impl From<byteorder::Error> for Error {
    fn from(e: byteorder::Error) -> Self {
        match e {
            byteorder::Error::UnexpectedEOF => Error::No(ECONNRESET),
            byteorder::Error::Io(e) => Error::Io(e)
        }
    }
}

/// Errno, error numbers
pub mod errno {
    extern crate nix;
    pub use self::nix::errno::Errno::*;
}

/// 9P error strings
///
/// # Protocol
/// 9P2000
pub mod string {
    pub const EPERM: &'static str               = "Operation not permitted";
    pub const EPERM_WSTAT: &'static str         = "wstat prohibited";
    pub const ENOENT: &'static str              = "No such file or directory";
    pub const ENOENT_DIR: &'static str          = "directory entry not found";
    pub const ENOENT_FILE: &'static str         = "file not found";
    pub const EINTR: &'static str               = "Interrupted system call";
    pub const EIO: &'static str                 = "Input/output error";
    pub const ENXIO: &'static str               = "No such device or address";
    pub const E2BIG: &'static str               = "Argument list too long";
    pub const EBADF: &'static str               = "Bad file descriptor";
    pub const EAGAIN: &'static str              = "Resource temporarily unavailable";
    pub const ENOMEM: &'static str              = "Cannot allocate memory";
    pub const EACCES: &'static str              = "Permission denied";
    pub const EFAULT: &'static str              = "Bad address";
    pub const ENOTBLK: &'static str             = "Block device required";
    pub const EBUSY: &'static str               = "Device or resource busy";
    pub const EEXIST: &'static str              = "File exists";
    pub const EXDEV: &'static str               = "Invalid cross-device link";
    pub const ENODEV: &'static str              = "No such device";
    pub const ENOTDIR: &'static str             = "Not a directory";
    pub const EISDIR: &'static str              = "Is a directory";
    pub const EINVAL: &'static str              = "Invalid argument";
    pub const ENFILE: &'static str              = "Too many open files in system";
    pub const EMFILE: &'static str              = "Too many open files";
    pub const ETXTBSY: &'static str             = "Text file busy";
    pub const EFBIG: &'static str               = "File too large";
    pub const ENOSPC: &'static str              = "No space left on device";
    pub const ESPIPE: &'static str              = "Illegal seek";
    pub const EROFS: &'static str               = "Read-only file system";
    pub const EMLINK: &'static str              = "Too many links";
    pub const EPIPE: &'static str               = "Broken pipe";
    pub const EDOM: &'static str                = "Numerical argument out of domain";
    pub const ERANGE: &'static str              = "Numerical result out of range";
    pub const EDEADLK: &'static str             = "Resource deadlock avoided";
    pub const ENAMETOOLONG: &'static str        = "File name too long";
    pub const ENOLCK: &'static str              = "No locks available";
    pub const ENOSYS: &'static str              = "Function not implemented";
    pub const ENOTEMPTY: &'static str           = "Directory not empty";
    pub const ELOOP: &'static str               = "Too many levels of symbolic links";
    pub const ENOMSG: &'static str              = "No message of desired type";
    pub const EIDRM: &'static str               = "Identifier removed";
    pub const ENODATA: &'static str             = "No data available";
    pub const ENONET: &'static str              = "Machine is not on the network";
    pub const ENOPKG: &'static str              = "Package not installed";
    pub const EREMOTE: &'static str             = "Object is remote";
    pub const ENOLINK: &'static str             = "Link has been severed";
    pub const ECOMM: &'static str               = "Communication error on send";
    pub const EPROTO: &'static str              = "Protocol error";
    pub const EBADMSG: &'static str             = "Bad message";
    pub const EBADFD: &'static str              = "File descriptor in bad state";
    pub const ESTRPIPE: &'static str            = "Streams pipe error";
    pub const EUSERS: &'static str              = "Too many users";
    pub const ENOTSOCK: &'static str            = "Socket operation on non-socket";
    pub const EMSGSIZE: &'static str            = "Message too long";
    pub const ENOPROTOOPT: &'static str         = "Protocol not available";
    pub const EPROTONOSUPPORT: &'static str     = "Protocol not supported";
    pub const ESOCKTNOSUPPORT: &'static str     = "Socket type not supported";
    pub const EOPNOTSUPP: &'static str          = "Operation not supported";
    pub const EPFNOSUPPORT: &'static str        = "Protocol family not supported";
    pub const ENETDOWN: &'static str            = "Network is down";
    pub const ENETUNREACH: &'static str         = "Network is unreachable";
    pub const ENETRESET: &'static str           = "Network dropped connection on reset";
    pub const ECONNABORTED: &'static str        = "Software caused connection abort";
    pub const ECONNRESET: &'static str          = "Connection reset by peer";
    pub const ENOBUFS: &'static str             = "No buffer space available";
    pub const EISCONN: &'static str             = "Transport endpoint is already connected";
    pub const ENOTCONN: &'static str            = "Transport endpoint is not connected";
    pub const ESHUTDOWN: &'static str           = "Cannot send after transport endpoint shutdown";
    pub const ETIMEDOUT: &'static str           = "Connection timed out";
    pub const ECONNREFUSED: &'static str        = "Connection refused";
    pub const EHOSTDOWN: &'static str           = "Host is down";
    pub const EHOSTUNREACH: &'static str        = "No route to host";
    pub const EALREADY: &'static str            = "Operation already in progress";
    pub const EINPROGRESS: &'static str         = "Operation now in progress";
    pub const EISNAM: &'static str              = "Is a named type file";
    pub const EREMOTEIO: &'static str           = "Remote I/O error";
    pub const EDQUOT: &'static str              = "Disk quota exceeded";
    pub const EBADF2: &'static str              = "fid unknown or out of range";
    pub const EACCES2: &'static str             = "permission denied";
    pub const ENOENT_FILE2: &'static str        = "file does not exist";
    pub const ECONNREFUSED2: &'static str       = "authentication failed";
    pub const ESPIPE2: &'static str             = "bad offset in directory read";
    pub const EBADF3: &'static str              = "bad use of fid";
    pub const EPERM_CONV: &'static str          = "wstat can't convert between files and directories";
    pub const ENOTEMPTY2: &'static str          = "directory is not empty";
    pub const EEXIST2: &'static str             = "file exists";
    pub const EEXIST3: &'static str             = "file already exists";
    pub const EEXIST4: &'static str             = "file or directory already exists";
    pub const EBADF4: &'static str              = "fid already in use";
    pub const ETXTBSY2: &'static str            = "file in use";
    pub const EIO2: &'static str                = "i/o error";
    pub const ETXTBSY3: &'static str            = "file already open for I/O";
    pub const EINVAL2: &'static str             = "illegal mode";
    pub const ENAMETOOLONG2: &'static str       = "illegal name";
    pub const ENOTDIR2: &'static str            = "not a directory";
    pub const EPERM_GRP: &'static str           = "not a member of proposed group";
    pub const EACCES3: &'static str             = "not owner";
    pub const EACCES4: &'static str             = "only owner can change group in wstat";
    pub const EROFS2: &'static str              = "read only file system";
    pub const EPERM_SPFILE: &'static str        = "no access to special file";
    pub const EIO3: &'static str                = "i/o count too large";
    pub const EINVAL3: &'static str             = "unknown group";
    pub const EINVAL4: &'static str             = "unknown user";
    pub const EPROTO2: &'static str             = "bogus wstat buffer";
    pub const EAGAIN2: &'static str             = "exclusive use file already open";
    pub const EIO4: &'static str                = "corrupted directory entry";
    pub const EIO5: &'static str                = "corrupted file entry";
    pub const EIO6: &'static str                = "corrupted block label";
    pub const EIO7: &'static str                = "corrupted meta data";
    pub const EINVAL5: &'static str             = "illegal offset";
    pub const ENOENT_PATH: &'static str         = "illegal path element";
    pub const EIO8: &'static str                = "root of file system is corrupted";
    pub const EIO9: &'static str                = "corrupted super block";
    pub const EPROTO3: &'static str             = "protocol botch";
    pub const ENOSPC2: &'static str             = "file system is full";
    pub const EAGAIN3: &'static str             = "file is in use";
    pub const ENOENT_ALLOC: &'static str        = "directory entry is not allocated";
    pub const EROFS3: &'static str              = "file is read only";
    pub const EIDRM2: &'static str              = "file has been removed";
    pub const EPERM_TRUNCATE: &'static str      = "only support truncation to zero length";
    pub const EPERM_RMROOT: &'static str        = "cannot remove root";
    pub const EFBIG2: &'static str              = "file too big";
    pub const EIO10: &'static str               = "venti i/o error";
}
