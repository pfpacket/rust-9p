
#![feature(metadata_ext)]
#![feature(dir_entry_ext)]

extern crate rs9p;

use std::{io, fs};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rs9p::error;
use rs9p::Request;
use rs9p::fcall::*;
use rs9p::serialize::Encodable;

#[macro_use]
mod utils;
use utils::*;

#[derive(Clone, Debug)]
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

    fn fid_from_realpath(&self, realpath: &Path) -> Fid {
        let root_len = self.realroot.to_str().unwrap().len();
        let path = if realpath == self.realroot.as_ref() {
            "/"
        } else {
            &realpath.to_str().unwrap()[root_len..]
        };
        Fid::new(path, realpath)
    }
}

impl rs9p::Filesystem for Unpfs {
    type Fid = Fid;

    fn rflush(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Rflush)
    }

    fn rattach(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        let qid = try!(get_qid(&self.realroot));
        req.fid().qid = Some(qid);
        req.fid().aux = Some(Fid::new("/", &self.realroot));
        Ok(Fcall::Rattach { qid: qid })
    }

    fn rwalk(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        let (wnames, newfid) = match req.ifcall {
            &Fcall::Twalk { ref wnames, newfid, .. } => (wnames.clone(), newfid),
            _ => unreachable!()
        };

        let mut wqids = Vec::new();
        let mut result_path = req.fid().aux().realpath.clone();

        if wnames.len() == 0 {
            req.newfid = req.fid.clone();
            req.newfid().fid = newfid;
        } else {
            for ref name in wnames {
                result_path.push(name);
                println!("rwalk: result_path={:?}", result_path);
                wqids.push( try!(get_qid(&result_path)) );
            }
            let newfid = self.fid_from_realpath(&result_path);
            req.newfid().aux = Some(newfid);
        }

        Ok(Fcall::Rwalk { wqids: wqids })
    }

    fn ropen(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Ok(Fcall::Ropen { qid: *req.fid().qid(), iounit: 8192 })
    }

    fn rcreate(&mut self, _: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        Err(error::ENOSYS.to_owned())
    }

    fn rread(&mut self, req: &mut Request<Self::Fid>) -> rs9p::Result<Fcall> {
        let offset = match req.ifcall {
            &Fcall::Tread { ref offset, .. } => *offset,
            _ => unreachable!()
        };

        let mut buf = Vec::new();
        if offset == 0 && (req.fid().qid().typ & qt::DIR) >= 0 {
            let mut stats = Vec::new();
            for entry in try!(fs::read_dir(&req.fid().aux().realpath).or(strerror!(ENOENT))) {
                let entry = try!(entry.or(strerror!(ENOENT)));
                let attr = try!(entry.metadata().or(strerror!(ENOENT)));
                let name = entry.file_name();
                let stat = unpfs_stat_from_unix(&attr, Path::new(&name));
                stats.push(stat);
            }
            stats.encode(&mut buf);
        };

        Ok(Fcall::Rread { data: Data::new(buf) })
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
            stat: unpfs_stat_from_unix(&attr, &fid.path)
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
