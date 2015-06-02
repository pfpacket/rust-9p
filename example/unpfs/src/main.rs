
#![feature(metadata_ext)]

extern crate rs9p;

use std::io;
use std::fs;
use std::ffi::OsStr;
use std::error::Error;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rs9p::error;
use rs9p::Request;
use rs9p::fcall::*;

macro_rules! strerror {
    ($err:ident) => { Err(error::$err.to_owned()) }
}

macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
}

fn rs9p_get_qid_type(attr: &fs::Metadata) -> u8 {
    if attr.is_dir() {
        qt::DIR
    } else {
        qt::FILE
    }
}

fn rs9p_stat_from_unix(attr: &fs::Metadata, path: &Path) -> Stat {
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
            typ: rs9p_get_qid_type(attr),
            version: 0,
            path: raw_attr.ino(),
        },
        mode: mode,
        atime: raw_attr.atime() as u32,
        mtime: raw_attr.mtime() as u32,
        length: raw_attr.size() as u64,
        name: name,
        uid: std::env::var("USER").unwrap(),
        gid: std::env::var("USER").unwrap(),
        muid: std::env::var("USER").unwrap(),
    }
}

struct Fid {
    path: PathBuf,
    realpath: PathBuf,
}

impl Fid {
    fn new<P1: ?Sized, P2: ?Sized>(path: &P1, real: &P2) -> Fid
        where P1: AsRef<OsStr>, P2: AsRef<OsStr> {
        Fid {
            path: Path::new(path).to_path_buf(),
            realpath: Path::new(real).to_path_buf()
        }
    }
}

struct Unpfs {
    realroot: PathBuf,
    fids: HashMap<u32, Fid>
}

impl Unpfs {
    fn new(mountpoint: &str) -> Unpfs {
        Unpfs {
            realroot: PathBuf::from(mountpoint),
            fids: HashMap::new()
        }
    }

    fn fid_from_realpath(&self, realpath: &str) -> Fid {
        let root_len = self.realroot.to_str().unwrap().len();
        Fid::new(&realpath[root_len..], realpath)
    }
}

macro_rules! get_fid {
    ($fs:expr, $fid:expr) => {
        try!($fs.fids.get($fid).ok_or(error::EBADF2.to_owned()))
    }
}

impl rs9p::srv::Filesystem for Unpfs {
    fn rflush(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rflush)
    }

    fn rattach(&mut self, req: &Request) -> rs9p::Result<MsgBody> {
        match req.ifcall {
            &MsgBody::Tattach { fid, afid: _, uname: _, aname: _ } => {
                self.fids.insert(fid, Fid::new("/", &self.realroot));
            }, _ => unreachable!()
        };

        let attr = try!(fs::metadata(&self.realroot).or(strerror!(ENOENT)));
        Ok(MsgBody::Rattach {
            qid: Qid {
                typ: qt::DIR, version: 0, path: attr.as_raw().ino()
            }
        })
    }

    fn rwalk(&mut self, req: &Request) -> rs9p::Result<MsgBody> {
        let (newfid, result_path, wqids) = match req.ifcall {
            &MsgBody::Twalk { fid, newfid, ref wnames } => {
                let parent_fid = get_fid!(self, &fid);
                let mut result_path = parent_fid.realpath.clone();

                let mut wqids = Vec::new();
                for ref name in wnames {
                    result_path.push(name);
                    let attr = try!(fs::metadata(&result_path).or(strerror!(ENOENT)));
                    wqids.push(Qid {
                        typ: rs9p_get_qid_type(&attr),
                        version: 0,
                        path: attr.as_raw().ino()
                    });
                }

                (newfid, result_path, wqids)
            }, _ => unreachable!()
        };

        let fid = self.fid_from_realpath(result_path.to_str().unwrap());
        self.fids.insert(newfid, fid);

        Ok(MsgBody::Rwalk { wqids: wqids })
    }

    fn ropen(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
    }

    fn rcreate(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
    }

    fn rread(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
    }

    fn rwrite(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
    }

    fn rclunk(&mut self, req: &Request) -> rs9p::Result<MsgBody> {
        match req.ifcall {
            &MsgBody::Tclunk { fid } => { self.fids.remove(&fid) },
            _ => unreachable!(),
        };
        Ok(MsgBody::Rclunk)
    }

    fn rremove(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rremove)
    }

    fn rstat(&mut self, req: &Request) -> rs9p::Result<MsgBody> {
        let fid = match req.ifcall {
            &MsgBody::Tstat { fid } => { get_fid!(self, &fid) },
            _ => unreachable!()
        };

        let attr = try!(fs::metadata(&fid.realpath).or(strerror!(ENOENT)));
        Ok(MsgBody::Rstat {
            stat: rs9p_stat_from_unix(&attr, (&fid.path))
        })
    }

    fn rwstat(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rwstat)
    }
}

fn unpfs_main(args: Vec<String>) -> io::Result<i32> {
    if args.len() < 3 {
        println!("Usage: {} proto!address!port mountpoint", args[0]);
        println!("  where: proto = tcp | unix");
        return Ok(-1);
    }

    let mountpoint = &args[2];
    try!(fs::metadata(mountpoint).and_then(|m| {
        if m.is_dir() {
            Ok(())
        } else {
            io_error!(Other, "mount point must be a directory")
        }
    }));

    println!("[*] Ready to accept clients: {}", args[1]);
    try!(rs9p::srv::srv(Unpfs::new(mountpoint), &args[1]));

    return Ok(0);
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = match unpfs_main(args) {
        Ok(code) => code,
        Err(e) => {
            if e.description() == "unexpected EOF" {
                0
            } else {
                println!("Error: {:?}", e); -1
            }
        }
    };
    std::process::exit(exit_code);
}
