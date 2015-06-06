
extern crate nix;
extern crate rs9p;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rs9p::errno::*;
use rs9p::fcall::*;

#[macro_export]
macro_rules! errno {
    ($err:ident) => { Err(nix::Error::from_errno($err)) }
}

#[macro_export]
macro_rules! strerror {
    ($err:ident) => { Err(error::$err.to_owned()) }
}

#[macro_export]
macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
}

pub fn unpfs_get_qid_type(attr: &fs::Metadata) -> u8 {
    if attr.is_dir() { qt::DIR } else { qt::FILE }
}

pub fn to_stat(attr: &fs::Metadata) -> Stat {
    let raw_attr = attr.as_raw();
    Stat {
        mode: raw_attr.mode(),
        uid: raw_attr.uid(),
        gid: raw_attr.gid(),
        nlink: raw_attr.nlink(),
        rdev: raw_attr.rdev(),
        size: raw_attr.size() as u64,
        blksize: raw_attr.blksize() as u64,
        blocks: raw_attr.blocks() as u64,
        atime: Time { sec: raw_attr.atime() as u64, nsec: raw_attr.atime_nsec() as u64 },
        mtime: Time { sec: raw_attr.mtime() as u64, nsec: raw_attr.atime_nsec() as u64 },
        ctime: Time { sec: raw_attr.ctime() as u64, nsec: raw_attr.atime_nsec() as u64 },
    }
}

pub fn get_qid<T: AsRef<Path>>(path: &T) -> rs9p::Result<Qid> {
    let attr = try!(fs::metadata(path.as_ref()).or(errno!(ENOENT)));
    Ok(Qid {
        typ: unpfs_get_qid_type(&attr),
        version: 0,
        path: attr.as_raw().ino()
    })
}

