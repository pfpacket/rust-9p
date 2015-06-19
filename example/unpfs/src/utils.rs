
extern crate nix;
extern crate libc;
extern crate rs9p;

use std::fs;
use std::path::{Path, PathBuf};
use std::os::unix::io::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

use rs9p::fcall::*;

#[macro_export]
macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
}

#[macro_export]
macro_rules! errno {
    () => { nix::errno::from_i32(nix::errno::errno()) }
}

pub fn pathconv<P1: AsRef<Path> + ?Sized, P2: AsRef<Path> + ?Sized>(path: &P1, root: &P2) -> PathBuf {
    let p = path.as_ref().to_str().unwrap();
    let r = root.as_ref().to_str().unwrap();
    if path.as_ref().is_absolute() && p.len() >= r.len() {
        Path::new(if p.len() == r.len() { "/" } else { &p[r.len()..] })
    } else {
        path.as_ref()
    }.to_path_buf()
}

#[test]
fn test_pathconv1() {
    assert_eq!(Path::new("/"), pathconv("/tmp", "/tmp").as_ref());
    assert_eq!(Path::new("/"), pathconv("/tmp/", "/tmp").as_ref());
    assert_eq!(Path::new("/tmp"), pathconv("/tmp", "/tmp/").as_ref());
    assert_eq!(Path::new("/test"), pathconv("/tmp/test", "/tmp").as_ref());
}

pub fn chmod<T: AsRef<Path>>(path: &T, mode: u32) -> rs9p::Result<()> {
    unsafe {
        let ptr = path.as_ref().as_os_str().as_bytes().as_ptr();
        match libc::chmod(ptr as *const i8, mode) {
            0 => Ok(()), _ => Err(rs9p::Error::No(errno!()))
        }
    }
}

pub fn chown<T: AsRef<Path>>(path: &T, uid: Option<u32>, gid: Option<u32>) -> rs9p::Result<()> {
    unsafe {
        let ptr = path.as_ref().as_os_str().as_bytes().as_ptr();
        match libc::chown(ptr as *const i8, uid.unwrap_or(u32::max_value()), gid.unwrap_or(u32::max_value())) {
            0 => Ok(()), _ => Err(rs9p::Error::No(errno!()))
        }
    }
}

pub fn fsync<T: AsRawFd>(fd: &T) -> rs9p::Result<()> {
    unsafe {
        match libc::fsync(fd.as_raw_fd()) {
            0 => Ok(()), _ => Err(rs9p::Error::No(errno!()))
        }
    }
}

pub fn get_qid<T: AsRef<Path>>(path: &T) -> rs9p::Result<Qid> {
    let attr = try!(fs::metadata(path.as_ref()));
    Ok(Qid {
        typ: From::from(attr.file_type()),
        version: 0,
        path: attr.ino()
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
