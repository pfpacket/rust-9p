
extern crate nix;
extern crate rs9p;

use std::fs;
use std::path::Path;
use std::os::unix::fs::MetadataExt;

use rs9p::fcall::*;

#[macro_export]
macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
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
        mtime: Time { sec: raw_attr.mtime() as u64, nsec: raw_attr.mtime_nsec() as u64 },
        ctime: Time { sec: raw_attr.ctime() as u64, nsec: raw_attr.ctime_nsec() as u64 },
    }
}

pub fn get_qid<T: AsRef<Path>>(path: &T) -> rs9p::Result<Qid> {
    let attr = try!(fs::metadata(path.as_ref()));
    let mut typ = 0;
    if attr.is_dir() { typ |= qt::DIR }
    if attr.file_type().is_symlink() { typ |= qt::SYMLINK }
    Ok(Qid {
        typ: typ,
        version: 0,
        path: attr.as_raw().ino()
    })
}

pub fn get_dirent<T: AsRef<Path>>(path: &T, offset: u64) -> rs9p::Result<DirEntry> {
    let p = path.as_ref();
    let name = p.file_name().or(Some(p.as_os_str())).unwrap();
    Ok(DirEntry {
        qid: try!(get_qid(&path)),
        offset: offset,
        typ: 0,
        name: name.to_str().unwrap().to_owned()
    })
}
