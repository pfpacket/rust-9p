
//! Define data types and constants used in 9P protocol
//!
//! Supported protocol: 9P2000.L

extern crate num;

/// Old 9P2000 protocol types
///
/// The types in this module are not used 9P2000.L
pub mod rs9p2000 {
    /// The type of I/O
    ///
    /// Open mode to be checked against the permissions for the file.
    pub mod om {
        /// Open for read
        pub const READ: u8      = 0;
        /// Write
        pub const WRITE: u8     = 1;
        /// Read and write
        pub const RDWR: u8      = 2;
        /// Execute, == read but check execute permission
        pub const EXEC: u8      = 3;
        /// Or'ed in (except for exec), truncate file first
        pub const TRUNC: u8     = 16;
        /// Or'ed in, close on exec
        pub const CEXEC: u8     = 32;
        /// Or'ed in, remove on close
        pub const RCLOSE: u8    = 64;
    }

    /// Bits in Stat.mode
    pub mod dm {
        /// Mode bit for directories
        pub const DIR: u32      = 0x80000000;
        /// Mode bit for append only files
        pub const APPEND: u32   = 0x40000000;
        /// Mode bit for exclusive use files
        pub const EXCL: u32     = 0x20000000;
        /// Mode bit for mounted channel
        pub const MOUNT: u32    = 0x10000000;
        /// Mode bit for authentication file
        pub const AUTH: u32     = 0x08000000;
        /// Mode bit for non-backed-up files
        pub const TMP: u32      = 0x04000000;
        /// Mode bit for read permission
        pub const READ: u32     = 0x4;
        /// Mode bit for write permission
        pub const WRITE: u32    = 0x2;
        /// Mode bit for execute permission
        pub const EXEC: u32     = 0x1;
    }

    /// Plan 9 Namespace metadata (somewhat like a unix fstat)
    ///
    /// NOTE: Defined as `Dir` in libc.h of Plan 9
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Stat {
        /// Server type
        pub typ: u16,
        /// Server subtype
        pub dev: u32,
        /// Unique id from server
        pub qid: super::Qid,
        /// Permissions
        pub mode: u32,
        /// Last read time
        pub atime: u32,
        /// Last write time
        pub mtime: u32,
        /// File length
        pub length: u64,
        /// Last element of path
        pub name: String,
        /// Owner name
        pub uid: String,
        /// Group name
        pub gid: String,
        /// Last modifier name
        pub muid: String
    }

    impl Stat {
        /// Get the current size of the stat
        pub fn size(&self) -> u16 {
            use std::mem::{size_of, size_of_val};
            (size_of_val(&self.typ) +
            size_of_val(&self.dev) +
            size_of_val(&self.qid) +
            size_of_val(&self.mode) +
            size_of_val(&self.atime) +
            size_of_val(&self.mtime) +
            size_of_val(&self.length) +
            (size_of::<u16>() * 4) +
            self.name.len() + self.uid.len() +
            self.gid.len() + self.muid.len()) as u16
        }
    }
}   // pub mod rs9p2000

/// File lock type, Flock.typ
pub mod ltype {
    pub const RDLOCK: u8    = 0;
    pub const WRLOCK: u8    = 1;
    pub const UNLOCK: u8    = 2;
}

/// File lock flags, Flock.flags
pub mod lflag {
    /// Blocking request
    pub const BLOCK: u32    = 1;
    /// Reserved for future use
    pub const RECLAIM: u32  = 2;
}

/// File lock status
pub mod lstatus {
    pub const SUCCESS: u8   = 0;
    pub const BLOCKED: u8   = 1;
    pub const ERROR: u8     = 2;
    pub const GRACE: u8     = 3;
}

/// Bits in Qid.typ
///
/// Protocol: 9P2000/9P2000.L
pub mod qt {
    /// Type bit for directories
    pub const DIR: u8       = 0x80;
    /// Type bit for append only files
    pub const APPEND: u8    = 0x40;
    /// Type bit for exclusive use files
    pub const EXCL: u8      = 0x20;
    /// Type bit for mounted channel
    pub const MOUNT: u8     = 0x10;
    /// Type bit for authentication file
    pub const AUTH: u8      = 0x08;
    /// Type bit for not-backed-up file
    pub const TMP: u8       = 0x04;
    /// Plain file
    pub const FILE: u8      = 0x00;
}

/// Bits in `mask` and `valid` of `Tgetattr` and `Rgetattr`.
///
/// Protocol: 9P2000.L
pub mod getattr {
    pub const MODE: u64         = 0x00000001;
    pub const NLINK: u64        = 0x00000002;
    pub const UID: u64          = 0x00000004;
    pub const GID: u64          = 0x00000008;
    pub const RDEV: u64         = 0x00000010;
    pub const ATIME: u64        = 0x00000020;
    pub const MTIME: u64        = 0x00000040;
    pub const CTIME: u64        = 0x00000080;
    pub const INO: u64          = 0x00000100;
    pub const SIZE: u64         = 0x00000200;
    pub const BLOCKS: u64       = 0x00000400;

    pub const BTIME: u64        = 0x00000800;
    pub const GEN: u64          = 0x00001000;
    pub const DATA_VERSION: u64 = 0x00002000;

    /// Mask for fields up to BLOCKS
    pub const BASIC: u64        = 0x000007ff;
    /// Mask for All fields above
    pub const ALL: u64          = 0x00003fff;
}

/// Bits in `mask` of `Tsetattr`.
///
/// If a time bit is set without the corresponding SET bit, the current
/// system time on the server is used instead of the value sent in the request.
///
/// Protocol: 9P2000.L
pub mod setattr {
    pub const MODE: u32         = 0x00000001;
    pub const UID: u32          = 0x00000002;
    pub const GID: u32          = 0x00000004;
    pub const SIZE: u32         = 0x00000008;
    pub const ATIME: u32        = 0x00000010;
    pub const MTIME: u32        = 0x00000020;
    pub const CTIME: u32        = 0x00000040;
    pub const ATIME_SET: u32    = 0x00000080;
    pub const MTIME_SET: u32    = 0x00000100;
}

/// Server side data type for path tracking
///
/// The server's unique identification for the file being accessed
///
/// Protocol: 9P2000/9P2000.L
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Qid {
    /// Specify whether the file is a directory, append-only file, etc.
    pub typ: u8,
    /// Version number for a file; typically, it is incremented every time the file is modified
    pub version: u32,
    /// An integer which is unique among all files in the hierarchy
    pub path: u64
}

/// Filesystem information corresponding to `struct statfs` of Linux.
/// 
/// Protocol: 9P2000.L
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Statfs {
    /// Type of file system (see below)
    pub typ: u32,
    /// Optimal transfer block size
    pub bsize: u32,
    /// Total data blocks in file system
    pub blocks: u64,
    /// Free blocks in fs
    pub bfree: u64,
    /// Free blocks avail to non-superuser
    pub bavail: u64,
    /// Total file nodes in file system
    pub files: u64,
    /// Free file nodes in fs
    pub ffree: u64,
    /// File system id
    pub fsid: u64,
    /// Maximum length of filenames
    pub namelen: u32,
}

/// Time struct
///
/// Protocol: 9P2000.L
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub sec: u64,
    pub nsec: u64,
}

/// File attributes corresponding to `struct stat` of Linux.
///
/// Protocol: 9P2000.L
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stat {
    /// Protection
    pub mode: u32,
    /// User ID of owner
    pub uid: u32,
    /// Group ID of owner
    pub gid: u32,
    /// Number of hard links
    pub nlink: u64,
    /// Device ID (if special file)
    pub rdev: u64,
    /// Total size, in bytes
    pub size: u64,
    /// Blocksize for file system I/O
    pub blksize: u64,
    /// Number of 512B blocks allocated
    pub blocks: u64,
    /// Time of last access
    pub atime: Time,
    /// Time of last modification
    pub mtime: Time,
    /// Time of last status change
    pub ctime: Time,
}

/// Directory entry used in `Rreaddir`
///
/// Protocol: 9P2000.L
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirEntry {
    pub qid: Qid,
    pub offset: u64,
    pub typ: u8,
    pub name: String
}

/// Directory entry array
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirEntryData(Vec<DirEntry>);

impl DirEntryData {
    pub fn new(v: Vec<DirEntry>) -> DirEntryData { DirEntryData(v) }
    pub fn data(&self) -> &[DirEntry] { &self.0 }
}

/// Data type used in Rread and Twrite
///
/// Protocol: 9P2000/9P2000.L
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Data(Vec<u8>);

impl Data {
    pub fn new(v: Vec<u8>) -> Data { Data(v) }
    pub fn data(&self) -> &[u8] { &self.0 }
}

/// Similar to Linux `struct flock`
///
/// Protocol: 9P2000.L
#[repr(C, packed)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Flock {
    pub typ: u8,
    pub flags: u32,
    pub start: u64,
    pub length: u64,
    pub proc_id: u32,
}

// Commented out the types not used in 9P2000.L
enum_from_primitive! {
    /// Message type, 9P operations
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum MsgType {
        // 9P2000.L
        Tlerror         = 6,
        Rlerror,
        Tstatfs         = 8,
        Rstatfs,
        Tlopen          = 12,
        Rlopen,
        Tlcreate        = 14,
        Rlcreate,
        Tsymlink        = 16,
        Rsymlink,
        Tmknod          = 18,
        Rmknod,
        Trename         = 20,
        Rrename,
        Treadlink       = 22,
        Rreadlink,
        Tgetattr        = 24,
        Rgetattr,
        Tsetattr        = 26,
        Rsetattr,
        Txattrwalk      = 30,
        Rxattrwalk,
        Txattrcreate    = 32,
        Rxattrcreate,
        Treaddir        = 40,
        Rreaddir,
        Tfsync          = 50,
        Rfsync,
        Tlock           = 52,
        Rlock,
        Tgetlock        = 54,
        Rgetlock,
        Tlink           = 70,
        Rlink,
        Tmkdir          = 72,
        Rmkdir,
        Trenameat       = 74,
        Rrenameat,
        Tunlinkat       = 76,
        Runlinkat,

        // 9P2000
        Tversion        = 100,
        Rversion,
        Tauth           = 102,
        Rauth,
        Tattach         = 104,
        Rattach,
        //Terror          = 106,  // Illegal, never used
        //Rerror,
        Tflush          = 108,
        Rflush,
        Twalk           = 110,
        Rwalk,
        //Topen           = 112,
        //Ropen,
        //Tcreate         = 114,
        //Rcreate,
        Tread           = 116,
        Rread,
        Twrite          = 118,
        Rwrite,
        Tclunk          = 120,
        Rclunk,
        Tremove         = 122,
        Rremove,
        //Tstat           = 124,
        //Rstat,
        //Twstat          = 126,
        //Rwstat,
    }
}

/// Envelope for 9P messages
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Msg {
    /// Message type, one of the constants in MsgType
    pub typ: MsgType,
    /// Chosen and used by the client to identify the message.
    /// The reply to the message will have the same tag
    pub tag: u16,
    /// Message body encapsulating the various 9P messages
    pub body: Fcall
}

/// A data type encapsulating the various 9P messages
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fcall {
    // 9P2000.L
    Rlerror { ecode: u32 },
    Tstatfs { fid: u32 },
    Rstatfs { statfs: Statfs },
    Tlopen { fid: u32, flags: u32 },
    Rlopen { qid: Qid, iounit: u32 },
    Tlcreate { fid: u32, name: String, flags: u32, mode: u32, gid: u32 },
    Rlcreate { qid: Qid, iounit: u32 },
    Tsymlink { fid: u32, name: String, symtgt: String, gid: u32 },
    Rsymlink { qid: Qid },
    Tmknod { dfid: u32, name: String, mode: u32, major: u32, minor: u32, gid: u32 },
    Rmknod { qid: Qid },
    Trename { fid: u32, dfid: u32, name: String },
    Rrename,
    Treadlink { fid: u32 },
    Rreadlink { target: String },
    Tgetattr { fid: u32, req_mask: u64 },
    Rgetattr { valid: u64, qid: Qid, stat: Stat /* reserved members are handled in En/Decodable traits */ },
    Tsetattr { fid: u32, valid: u32, stat: Stat },
    Rsetattr,
    Txattrwalk { fid: u32, newfid: u32, name: String },
    Rxattrwalk { size: u64 },
    Txattrcreate { fid: u32, name: String, attr_size: u64, flags: u32 },
    Rxattrcreate,
    Treaddir { fid: u32, offset: u64, count: u32 },
    Rreaddir { data: DirEntryData },
    Tfsync { fid: u32 },
    Rfsync,
    Tlock { fid: u32, flock: Flock, client_id: String },
    Rlock { status: u8 },
    Tgetlock { fid: u32, flock: Flock, client_id: String },
    Rgetlock { flock: Flock, client_id: String },
    Tlink { dfid: u32, fid: u32, name: String },
    Rlink,
    Tmkdir { dfid: u32, name: String, mode: u32, gid: u32 },
    Rmkdir { qid: Qid },
    Trenameat { olddirfid: u32, oldname: String, newdirfid: u32, newname: String },
    Rrenameat,
    Tunlinkat { dirfd: u32, name: String, flags: u32 },
    Runlinkat,

    // 9P2000.u
    Tauth { afid: u32, uname: String, aname: String, n_uname: u32 },
    Rauth { aqid: Qid },
    Tattach { fid: u32, afid: u32, uname: String, aname: String, n_uname: u32 },
    Rattach { qid: Qid },

    // 9P2000
    Tversion { msize: u32, version: String },
    Rversion { msize: u32, version: String },
    Tflush { oldtag: u16 },
    Rflush,
    Twalk { fid: u32, newfid: u32, wnames: Vec<String> },
    Rwalk { wqids: Vec<Qid> },
    Tread { fid: u32, offset: u64, count: u32 },
    Rread { data: Data },
    Twrite { fid: u32, offset: u64, data: Data },
    Rwrite { count: u32 },
    Tclunk { fid: u32 },
    Rclunk,
    Tremove { fid: u32 },
    Rremove,

    // 9P2000 operations not used for 9P2000.L
    //Tauth { afid: u32, uname: String, aname: String },
    //Rauth { aqid: Qid },
    //Rerror { ename: String },
    //Tattach { fid: u32, afid: u32, uname: String, aname: String },
    //Rattach { qid: Qid },
    //Topen { fid: u32, mode: u8 },
    //Ropen { qid: Qid, iounit: u32 },
    //Tcreate { fid: u32, name: String, perm: u32, mode: u8 },
    //Rcreate { qid: Qid, iounit: u32 },
    //Tstat { fid: u32 },
    //Rstat { stat: Stat },
    //Twstat { fid: u32, stat: Stat },
    //Rwstat,
}

impl Fcall {
    /// Get request's fid if available
    pub fn fid(&self) -> Vec<u32> {
        match self {
            &Fcall::Tstatfs { fid }                         => vec![fid],
            &Fcall::Tlopen { fid, .. }                      => vec![fid],
            &Fcall::Tlcreate { fid, .. }                    => vec![fid],
            &Fcall::Tsymlink { fid, .. }                    => vec![fid],
            &Fcall::Tmknod { dfid, .. }                     => vec![dfid],
            &Fcall::Trename { fid, dfid, .. }               => vec![fid, dfid],
            &Fcall::Treadlink { fid }                       => vec![fid],
            &Fcall::Tgetattr { fid, .. }                    => vec![fid],
            &Fcall::Tsetattr { fid, .. }                    => vec![fid],
            &Fcall::Txattrwalk { fid, .. }                  => vec![fid],
            &Fcall::Txattrcreate { fid, .. }                => vec![fid],
            &Fcall::Treaddir { fid, .. }                    => vec![fid],
            &Fcall::Tfsync { fid, .. }                      => vec![fid],
            &Fcall::Tlock { fid, .. }                       => vec![fid],
            &Fcall::Tgetlock { fid, .. }                    => vec![fid],
            &Fcall::Tlink { dfid, fid, .. }                 => vec![dfid, fid,],
            &Fcall::Tmkdir { dfid, .. }                     => vec![dfid],
            &Fcall::Trenameat { olddirfid, newdirfid, .. }  => vec![olddirfid, newdirfid],
            &Fcall::Tunlinkat { dirfd, .. }                 => vec![dirfd],
            //&Fcall::Tattach { afid, .. }                    => vec![afid],
            &Fcall::Twalk { fid, .. }                       => vec![fid],
            &Fcall::Tread { fid, .. }                       => vec![fid],
            &Fcall::Twrite { fid, .. }                      => vec![fid],
            &Fcall::Tclunk { fid, .. }                      => vec![fid],
            &Fcall::Tremove { fid }                         => vec![fid],
            _ => Vec::new()
        }
    }

    /// Get request's newfid if available
    pub fn newfid(&self) -> Vec<u32> {
        match self {
            &Fcall::Txattrwalk { newfid, .. }   => vec![newfid],
            &Fcall::Tauth { afid, .. }          => vec![afid],
            &Fcall::Tattach { fid, .. }         => vec![fid],
            &Fcall::Twalk { newfid, .. }        => vec![newfid],
            _ => Vec::new()
        }
    }

    pub fn qid(&self) -> Option<Qid> {
        match self {
            _ => None
        }
    }
}
