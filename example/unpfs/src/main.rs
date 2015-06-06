
#![feature(metadata_ext)]

extern crate nix;
extern crate rs9p;

use std::{io, fs};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rs9p::*;
use rs9p::errno::*;

#[macro_use]
mod utils;
use utils::*;

#[derive(Clone, Debug)]
struct UnpfsFid {
    path: PathBuf,
    realpath: PathBuf,
}

impl UnpfsFid {
    fn new<P1: ?Sized, P2: ?Sized>(path: &P1, real: &P2) -> UnpfsFid
        where P1: AsRef<OsStr>, P2: AsRef<OsStr>
    {
        UnpfsFid {
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

    fn fid_from_realpath(&self, realpath: &Path) -> UnpfsFid {
        let root_len = self.realroot.to_str().unwrap().len();
        let path = if realpath == self.realroot.as_ref() {
            "/"
        } else {
            &realpath.to_str().unwrap()[root_len..]
        };
        UnpfsFid::new(path, realpath)
    }
}

impl rs9p::Filesystem for Unpfs {
    type Fid = UnpfsFid;

    fn rattach(&mut self, fid: &mut Fid<Self::Fid>, _afid: Option<&mut Fid<Self::Fid>>, _uname: &str, _aname: &str, _n_uname: u32) -> Result<Fcall> {
        fid.aux = Some(self.fid_from_realpath(&self.realroot));
        Ok(Fcall::Rattach {
            qid: try!(get_qid(&self.realroot))
        })
    }

    fn rwalk(&mut self, fid: &mut Fid<Self::Fid>, newfid: &mut Fid<Self::Fid>, wnames: &[String]) -> Result<Fcall> {
        let mut wqids = Vec::new();
        let mut path = fid.aux().realpath.clone();

        for (i, name) in wnames.iter().enumerate() {
            path.push(name);
            let qid = match get_qid(&path) {
                Ok(attr) => attr,
                Err(e) => if i == 0 { return Err(e) } else { break },
            };
            wqids.push(qid);
        }
        newfid.aux = Some(self.fid_from_realpath(&path));

        Ok(Fcall::Rwalk { wqids: wqids })
    }

    fn rgetattr(&mut self, fid: &mut Fid<Self::Fid>, req_mask: u64) -> Result<Fcall> {
        let attr = try!(fs::metadata(&fid.aux().realpath).or(errno!(ENOENT)));
        Ok(Fcall::Rgetattr {
            valid: req_mask,
            qid: try!(get_qid(&fid.aux().realpath)),
            stat: to_stat(&attr)
        })
    }

    fn rreaddir(&mut self, fid: &mut Fid<Self::Fid>, offset: u64, _count: u32) -> Result<Fcall> {
        let mut dirents = Vec::new();

        if offset != 0 {
            return Ok(Fcall::Rreaddir { data: DirEntryData::new(Vec::new()) })
        }

        for entry in try!(fs::read_dir(&fid.aux().realpath).or(errno!(ENOENT))) {
            let path = try!(entry.or(errno!(ENOENT))).path();
            let qid = try!(get_qid(&path));
            let name = path.file_name().unwrap().to_str().unwrap();
            dirents.push(DirEntry {
                qid: qid,
                offset: 1 + name.len() as u64,
                typ: qid.typ,
                name: name.to_owned()
            })
        }

        Ok(Fcall::Rreaddir { data: DirEntryData::new(dirents) })
    }

    fn rlopen(&mut self, fid: &mut Fid<Self::Fid>, _flags: u32) -> Result<Fcall> {
        Ok(Fcall::Rlopen {
            qid: try!(get_qid(&fid.aux().realpath)),
            iounit: 8192 - 24
        })
    }

    fn rsetattr(&mut self, _: &mut Fid<Self::Fid>, _valid: u32, _stat: &Stat)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rlcreate(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rread(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rwrite(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _data: &Data)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rmkdir(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rrenameat(&mut self, _: &mut Fid<Self::Fid>, _oldname: &str, _: &mut Fid<Self::Fid>, _newname: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn runlinkat(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }

    fn rclunk(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Ok(Fcall::Rclunk)
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
