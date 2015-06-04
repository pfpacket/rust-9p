
extern crate rs9p;

use std::{fs, env};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rs9p::error;
use rs9p::fcall::*;

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

pub fn unpfs_stat_from_unix(attr: &fs::Metadata, path: &Path) -> Stat {
    let raw_attr = attr.as_raw();
    let mut mode = raw_attr.mode() & 0o777;
    if attr.is_dir() { mode |= dm::DIR }

    let name = if path == Path::new("/") {
        "/".to_owned()
    } else {
        path.file_name().unwrap().to_str().unwrap().to_owned()
    };

    Stat {
        typ: 0,
        dev: raw_attr.dev() as u32,
        qid: Qid {
            typ: unpfs_get_qid_type(attr),
            version: 0,
            path: raw_attr.ino(),
        },
        mode: mode,
        atime: raw_attr.atime() as u32,
        mtime: raw_attr.mtime() as u32,
        length: raw_attr.size() as u64,
        name: name,
        uid: env::var("USER").unwrap(),
        gid: env::var("USER").unwrap(),
        muid: env::var("USER").unwrap(),
    }
}

pub fn get_qid(path: &Path) -> rs9p::Result<Qid> {
    let attr = try!(fs::metadata(path).or(strerror!(ENOENT)));
    Ok(Qid {
        typ: unpfs_get_qid_type(&attr),
        version: 0,
        path: attr.as_raw().ino()
    })
}

