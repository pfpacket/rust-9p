
#![feature(metadata_ext)]

extern crate rs9p;

use std::{io, fs};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rs9p::error;
use rs9p::Request;
use rs9p::fcall::*;

#[macro_use]
mod utils;
use utils::*;

#[derive(Debug)]
struct Fid {
    path: PathBuf,
    realpath: PathBuf,
}

impl Fid {
    fn new<P1: ?Sized, P2: ?Sized>(path: &P1, real: &P2) -> Fid
        where P1: AsRef<OsStr>, P2: AsRef<OsStr>
    {
        Fid {
            path: Path::new(path).to_path_buf(),
            realpath: Path::new(real).to_path_buf()
        }
    }
}

struct Unpfs {
    realroot: PathBuf,
}

impl Unpfs {
    fn new(mountpoint: &str) -> Unpfs {
        Unpfs {
            realroot: PathBuf::from(mountpoint),
        }
    }

    fn fid_from_realpath(&self, realpath: &str) -> Fid {
        let root_len = self.realroot.to_str().unwrap().len();
        Fid::new(&realpath[root_len..], realpath)
    }
}

impl rs9p::Filesystem for Unpfs {
    type Fid = Fid;

    fn rflush(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Rflush)
    }

    fn rattach(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        req.fid().aux = Some(Fid::new("/", &self.realroot));
        Ok(Fcall::Rattach {
            qid: try!(get_qid(&self.realroot))
        })
    }

    fn rwalk(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        let wnames = match req.ifcall {
            &Fcall::Twalk { fid: _, newfid: _, ref wnames } => wnames.clone(),
            _ => unreachable!()
        };

        let mut wqids = Vec::new();
        let mut result_path = req.fid().aux().realpath.clone();

        for ref name in wnames {
            result_path.push(name);
            println!("rwalk: result_path={:?}", result_path);
            wqids.push( try!(get_qid(&result_path)) );
        }

        let unpfs_newfid = self.fid_from_realpath(result_path.to_str().unwrap());
        req.newfid().aux = Some(unpfs_newfid);

        Ok(Fcall::Rwalk { wqids: wqids })
    }

    fn ropen(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Err(error::ENOSYS.to_owned())
    }

    fn rcreate(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Err(error::ENOSYS.to_owned())
    }

    fn rread(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Err(error::ENOSYS.to_owned())
    }

    fn rwrite(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Err(error::ENOSYS.to_owned())
    }

    fn rclunk(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Rclunk)
    }

    fn rremove(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Rremove)
    }

    fn rstat(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        let fid = req.fid().aux();
        let attr = try!(fs::metadata(&fid.realpath).or(strerror!(ENOENT)));
        Ok(Fcall::Rstat {
            stat: unpfs_stat_from_unix(&attr, (&fid.path))
        })
    }

    fn rwstat(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Rwstat)
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
    try!(rs9p::srv(Unpfs::new(mountpoint), &args[1]));

    return Ok(0);
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = match unpfs_main(args) {
        Ok(code) => code,
        Err(e) => {
            if e.kind() == io::ErrorKind::ConnectionRefused {
                0
            } else {
                println!("Error: {:?}", e); -1
            }
        }
    };
    std::process::exit(exit_code);
}
