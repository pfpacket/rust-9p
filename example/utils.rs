
extern crate nix;
extern crate rs9p;

use std::fs;
use std::path::Path;
use std::os::unix::prelude::*;
use rs9p::fcall::*;

macro_rules! res { ($err:expr) => { Err(From::from($err)) } }
macro_rules! io_err { ($kind:ident, $msg:expr) => {
    ::std::io::Error::new(::std::io::ErrorKind::$kind, $msg)
}}
macro_rules! INVALID_FID { () => (io_err!(InvalidInput, "Invalid fid")) }

pub fn create_buffer(size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    unsafe { buffer.set_len(size); }
    buffer
}

pub fn get_qid<T: AsRef<Path> + ?Sized>(path: &T) -> rs9p::Result<Qid> {
    Ok(qid_from_attr( &try!(fs::symlink_metadata(path.as_ref())) ))
}

pub fn qid_from_attr(attr: &fs::Metadata) -> Qid {
    Qid {
        typ: From::from(attr.file_type()),
        version: 0,
        path: attr.ino()
    }
}

pub fn get_dirent_from<P: AsRef<Path> + ?Sized>(p: &P, offset: u64) -> rs9p::Result<DirEntry> {
    Ok(DirEntry {
        qid: try!(get_qid(p)),
        offset: offset,
        typ: 0,
        name: p.as_ref().to_string_lossy().into_owned()
    })
}

pub fn get_dirent(entry: &fs::DirEntry, offset: u64) -> rs9p::Result<DirEntry> {
    Ok(DirEntry {
        qid: qid_from_attr(&try!(entry.metadata())),
        offset: offset,
        typ: 0,
        name: entry.file_name().to_string_lossy().into_owned()
    })
}
