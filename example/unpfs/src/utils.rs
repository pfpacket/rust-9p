
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

pub fn create_buffer(size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    unsafe { buffer.set_len(size); }
    buffer
}

pub fn rm_head_path<P1: ?Sized, P2: ?Sized>(path: &P1, head: &P2) -> PathBuf
    where P1: AsRef<Path>, P2: AsRef<Path>
{
    let p = path.as_ref();
    if p.is_absolute() && p.starts_with(head.as_ref()) {
        p.components().skip(2)
            .fold(PathBuf::from("/"), |acc, c| acc.join(c.as_ref()))
    } else {
        p.to_path_buf()
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
    Ok(qid_from_attr( &try!(fs::metadata(path.as_ref())) ))
}

pub fn qid_from_attr(attr: &fs::Metadata) -> Qid {
    Qid {
        typ: From::from(attr.file_type()),
        version: 0,
        path: attr.ino()
    }
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
