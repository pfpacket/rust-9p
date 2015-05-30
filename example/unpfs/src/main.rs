
#![feature(metadata_ext)]

extern crate rs9p;

use std::io::{Error, ErrorKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::os::unix::fs as unix;
use std::os::unix::fs::MetadataExt;

use rs9p::fcall::*;
use rs9p::Request;
use rs9p::error;

macro_rules! strerror {
    ($err:ident) => { Err(error::$err.to_owned()) }
}

struct Unpfs {
    realroot: PathBuf,
    fids: HashMap<u32, PathBuf>
}

impl Unpfs {
    fn new(mountpoint: &str) -> Unpfs {
        Unpfs {
            realroot: PathBuf::from(mountpoint),
            fids: HashMap::new()
        }
    }
}

impl rs9p::srv::Filesystem for Unpfs {
    fn rflush(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rflush)
    }

    fn rattach(&mut self, req: &Request) -> rs9p::Result<MsgBody> {
        match req.ifcall {
            &MsgBody::Tattach { fid, afid: _, uname: _, aname: _ } => {
                self.fids.insert(fid, PathBuf::from(&self.realroot));
            }, _ => {}
        };

        let attr = try!(fs::metadata(&self.realroot).or(strerror!(ENOENT)));
        Ok(MsgBody::Rattach {
            qid: Qid {
                typ: qt::DIR, version: 0, path: attr.as_raw().ino()
            }
        })
    }

    fn rwalk(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
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

    fn rclunk(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rclunk)
    }

    fn rremove(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rremove)
    }

    fn rstat(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Err(error::ENOSYS.to_owned())
    }

    fn rwstat(&mut self, _: &Request) -> rs9p::Result<MsgBody> {
        Ok(MsgBody::Rwstat)
    }
}

fn unpfs_main(args: Vec<String>) -> Result<i32, Error> {
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
            Err(Error::new(ErrorKind::Other, "mount point must be a directory"))
        }
    }));

    let unpfs = Unpfs::new(mountpoint);
    let mut server = try!(rs9p::Server::announce(unpfs, &args[1]));

    println!("[*] Ready to accept the clients: {}", args[1]);
    try!(server.srv());

    return Ok(0);
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = match unpfs_main(args) {
        Ok(code) => code,
        Err(e) => { println!("Error: {}", e); -1 }
    };
    std::process::exit(exit_code);
}
