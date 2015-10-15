
extern crate nix;
extern crate rs9p;
extern crate env_logger;

use std::fs;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::io::{Seek, SeekFrom, Read, Write};
use std::os::unix::prelude::*;
use rs9p::*;
use rs9p::srv::{Fid, Filesystem};

#[macro_use]
mod utils;
use utils::*;

struct UnpfsFid {
    realpath: PathBuf,
    file: Option<fs::File>
}

impl UnpfsFid {
    fn new<P: AsRef<OsStr> + ?Sized>(path: &P) -> UnpfsFid {
        UnpfsFid { realpath: PathBuf::from(path), file: None, }
    }
}

struct Unpfs {
    realroot: PathBuf,
}

impl Unpfs {
    fn new(mountpoint: &str) -> Unpfs {
        Unpfs { realroot: PathBuf::from(mountpoint), }
    }
}

impl Filesystem for Unpfs {
    type Fid = UnpfsFid;

    fn rattach(&mut self, fid: &mut Fid<Self::Fid>, _afid: Option<&mut Fid<Self::Fid>>, _uname: &str, _aname: &str, _n_uname: u32) -> Result<Fcall> {
        fid.aux = Some(UnpfsFid::new(&self.realroot));
        Ok(Fcall::Rattach { qid: try!(get_qid(&self.realroot)) })
    }

    fn rwalk(&mut self, fid: &mut Fid<Self::Fid>, newfid: &mut Fid<Self::Fid>, wnames: &[String]) -> Result<Fcall> {
        let mut wqids = Vec::new();
        let mut path = fid.aux().realpath.clone();

        for (i, name) in wnames.iter().enumerate() {
            path.push(name);
            let qid = match get_qid(&path) {
                Ok(qid) => qid,
                Err(e) => if i == 0 { return Err(e) } else { break },
            };
            wqids.push(qid);
        }
        newfid.aux = Some(UnpfsFid::new(&path));

        Ok(Fcall::Rwalk { wqids: wqids })
    }

    fn rgetattr(&mut self, fid: &mut Fid<Self::Fid>, req_mask: GetattrMask) -> Result<Fcall> {
        let attr = try!(fs::symlink_metadata(&fid.aux().realpath));
        Ok(Fcall::Rgetattr {
            valid: req_mask,
            qid: qid_from_attr(&attr),
            stat: From::from(attr)
        })
    }

    fn rsetattr(&mut self, fid: &mut Fid<Self::Fid>, valid: SetattrMask, stat: &SetAttr) -> Result<Fcall> {
        if valid.contains(setattr::MODE) {
            let perm = PermissionsExt::from_mode(stat.mode);
            try!(fs::set_permissions(&fid.aux().realpath, perm));
        }
        if valid.contains(setattr::UID) {
            try!(chown(&fid.aux().realpath, Some(stat.uid), None));
        }
        if valid.contains(setattr::GID) {
            try!(chown(&fid.aux().realpath, None, Some(stat.gid)));
        }
        if valid.contains(setattr::SIZE) {
            let _ = try!(fs::File::open(&fid.aux().realpath)).set_len(stat.size);
        }
        if valid.contains(setattr::ATIME) {}
        if valid.contains(setattr::MTIME) {}
        if valid.contains(setattr::CTIME) {}
        Ok(Fcall::Rsetattr)
    }

    fn rreadlink(&mut self, fid: &mut Fid<Self::Fid>) -> Result<Fcall> {
        let link = try!(fs::read_link(&fid.aux().realpath));
        Ok(Fcall::Rreadlink { target: link.to_string_lossy().into_owned() })
    }

    fn rreaddir(&mut self, fid: &mut Fid<Self::Fid>, off: u64, count: u32) -> Result<Fcall> {
        let mut dirents = DirEntryData::new();

        let offset = if off == 0 {
            dirents.push(try!(get_dirent_from(&".", 0)));
            dirents.push(try!(get_dirent_from(&"..", 1)));
            off
        } else { off - 1 } as usize;

        let entries = try!(fs::read_dir(&fid.aux().realpath));
        for (i, entry) in entries.enumerate().skip(offset) {
            let dirent = try!(get_dirent(&try!(entry), 2 + i as u64));
            if dirents.size() + dirent.size() > count {
                break;
            }
            dirents.push(dirent);
        }

        Ok(Fcall::Rreaddir { data: dirents })
    }

    fn rlopen(&mut self, fid: &mut Fid<Self::Fid>, flags: u32) -> Result<Fcall> {
        let qid = try!(get_qid(&fid.aux().realpath));

        if !qid.typ.contains(qt::DIR) {
            let oflags = nix::fcntl::OFlag::from_bits_truncate(flags as i32);
            let omode = nix::sys::stat::Mode::from_bits_truncate(0);
            let fd = try!(nix::fcntl::open(&fid.aux().realpath, oflags, omode));
            fid.aux_mut().file = Some(unsafe { fs::File::from_raw_fd(fd) });
        }

        Ok(Fcall::Rlopen { qid: qid, iounit: 0 })
    }

    fn rlcreate(&mut self, fid: &mut Fid<Self::Fid>, name: &str, flags: u32, mode: u32, _gid: u32) -> Result<Fcall> {
        let path = fid.aux().realpath.join(name);
        let oflags = nix::fcntl::OFlag::from_bits_truncate(flags as i32);
        let omode = nix::sys::stat::Mode::from_bits_truncate(mode);
        let fd = try!(nix::fcntl::open(&path, oflags, omode));

        fid.aux = Some(UnpfsFid::new(&path));
        fid.aux_mut().file = Some(unsafe { fs::File::from_raw_fd(fd) });

        Ok(Fcall::Rlcreate { qid: try!(get_qid(&path)), iounit: 0 })
    }

    fn rread(&mut self, fid: &mut Fid<Self::Fid>, offset: u64, count: u32) -> Result<Fcall> {
        let file = fid.aux_mut().file.as_mut().unwrap();
        try!(file.seek(SeekFrom::Start(offset)));

        let mut buf = create_buffer(count as usize);
        let bytes = try!(file.read(&mut buf[..]));
        buf.truncate(bytes);

        Ok(Fcall::Rread { data: Data(buf) })
    }

    fn rwrite(&mut self, fid: &mut Fid<Self::Fid>, offset: u64, data: &Data) -> Result<Fcall> {
        let file = fid.aux_mut().file.as_mut().unwrap();
        try!(file.seek(SeekFrom::Start(offset)));
        Ok(Fcall::Rwrite { count: try!(file.write(&data.0)) as u32 })
    }

    fn rmkdir(&mut self, dfid: &mut Fid<Self::Fid>, name: &str, _mode: u32, _gid: u32) -> Result<Fcall> {
        let path = dfid.aux().realpath.join(name);
        try!(fs::create_dir(&path));
        Ok(Fcall::Rmkdir { qid: try!(get_qid(&path)) })
    }

    fn rrenameat(&mut self, olddir: &mut Fid<Self::Fid>, oldname: &str, newdir: &mut Fid<Self::Fid>, newname: &str) -> Result<Fcall> {
        let oldpath = olddir.aux().realpath.join(oldname);
        let newpath = newdir.aux().realpath.join(newname);
        try!(fs::rename(&oldpath, &newpath));
        Ok(Fcall::Rrenameat)
    }

    fn runlinkat(&mut self, dirfid: &mut Fid<Self::Fid>, name: &str, _flags: u32) -> Result<Fcall> {
        let path = dirfid.aux().realpath.join(name);
        match try!(fs::symlink_metadata(&path)) {
            ref attr if attr.is_dir() => try!(fs::remove_dir(&path)),
            _ => try!(fs::remove_file(&path)),
        };
        Ok(Fcall::Runlinkat)
    }

    fn rfsync(&mut self, fid: &mut Fid<Self::Fid>) -> Result<Fcall> {
        try!(fid.aux_mut().file.as_mut().unwrap().sync_all());
        Ok(Fcall::Rfsync)
    }

    fn rclunk(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Ok(Fcall::Rclunk)
    }
}

fn unpfs_main(args: Vec<String>) -> rs9p::Result<i32> {
    if args.len() < 3 {
        println!("Usage: {} proto!address!port mountpoint", args[0]);
        println!("  where: proto = tcp | unix");
        return Ok(-1);
    }

    let mountpoint = &args[2];
    if !try!(fs::metadata(mountpoint)).is_dir() {
        return res!(io_err!(Other, "mount point must be a directory"));
    }

    println!("[*] Ready to accept clients: {}", args[1]);
    try!(rs9p::srv_spawn(Unpfs::new(mountpoint), &args[1]));

    return Ok(0);
}

fn main() {
    env_logger::init().unwrap();
    let args = std::env::args().collect();
    let exit_code = match unpfs_main(args) {
        Ok(code) => code,
        Err(e) => { println!("Error: {}", e); -1 }
    };
    std::process::exit(exit_code);
}
