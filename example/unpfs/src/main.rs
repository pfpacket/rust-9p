extern crate env_logger;
extern crate filetime;
extern crate nix;
extern crate rs9p;

use self::filetime::FileTime;
use nix::libc::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};
use rs9p::srv::{Fid, Filesystem};
use rs9p::*;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::prelude::*;
use std::path::PathBuf;

// Some clients will incorrectly set bits in 9p flags that don't make sense.
// For exmaple, the linux 9p kernel client propagates O_DIRECT to TCREATE and TOPEN
// and from there to the server.
// Processes on client machines set O_DIRECT to bypass the cache, but if
// the server uses O_DIRECT in the open or create, then subsequent server
// write and read system calls will fail, as O_DIRECT requires at minimum 512
// byte aligned data, and the data is almost always not aligned.
// While the linux kernel client is arguably broken, we won't be able
// to fix every kernel out there, and this is surely not the only buggy client
// we will see.
// The fix is to enumerate the set of flags we support and then and that with
// the flags received in a TCREATE or TOPEN. This nicely fixes a real problem
// we are seeing with a file system benchmark.
const UNIX_FLAGS: u32 = (O_WRONLY | O_RDONLY | O_RDWR | O_CREAT | O_TRUNC) as u32;

#[macro_use]
mod utils;
use utils::*;

struct UnpfsFid {
    realpath: PathBuf,
    file: Option<fs::File>,
}

impl UnpfsFid {
    fn new<P: AsRef<std::ffi::OsStr> + ?Sized>(path: &P) -> UnpfsFid {
        UnpfsFid {
            realpath: PathBuf::from(path),
            file: None,
        }
    }
}

struct Unpfs {
    realroot: PathBuf,
}

impl Filesystem for Unpfs {
    type Fid = UnpfsFid;

    fn rattach(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        _afid: Option<&mut Fid<Self::Fid>>,
        _uname: &str,
        _aname: &str,
        _n_uname: u32,
    ) -> Result<Fcall> {
        fid.aux = Some(UnpfsFid::new(&self.realroot));

        Ok(Fcall::Rattach {
            qid: get_qid(&self.realroot)?,
        })
    }

    fn rwalk(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        newfid: &mut Fid<Self::Fid>,
        wnames: &[String],
    ) -> Result<Fcall> {
        let mut wqids = Vec::new();
        let mut path = fid.aux().realpath.clone();

        for (i, name) in wnames.iter().enumerate() {
            path.push(name);
            let qid = match get_qid(&path) {
                Ok(qid) => qid,
                Err(e) => {
                    if i == 0 {
                        return Err(e);
                    } else {
                        break;
                    }
                }
            };
            wqids.push(qid);
        }
        newfid.aux = Some(UnpfsFid::new(&path));

        Ok(Fcall::Rwalk { wqids: wqids })
    }

    fn rgetattr(&mut self, fid: &mut Fid<Self::Fid>, req_mask: GetattrMask) -> Result<Fcall> {
        let attr = fs::symlink_metadata(&fid.aux().realpath)?;

        Ok(Fcall::Rgetattr {
            valid: req_mask,
            qid: qid_from_attr(&attr),
            stat: From::from(attr),
        })
    }

    fn rsetattr(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        valid: SetattrMask,
        stat: &SetAttr,
    ) -> Result<Fcall> {
        let filepath = &fid.aux().realpath;

        if valid.contains(SetattrMask::MODE) {
            fs::set_permissions(filepath, PermissionsExt::from_mode(stat.mode))?;
        }

        if valid.intersects(SetattrMask::UID | SetattrMask::GID) {
            let uid = if valid.contains(SetattrMask::UID) {
                Some(nix::unistd::Uid::from_raw(stat.uid))
            } else {
                None
            };
            let gid = if valid.contains(SetattrMask::GID) {
                Some(nix::unistd::Gid::from_raw(stat.gid))
            } else {
                None
            };
            nix::unistd::chown(filepath, uid, gid)?;
        }

        if valid.contains(SetattrMask::SIZE) {
            let _ = fs::File::open(filepath)?.set_len(stat.size);
        }

        if valid.intersects(SetattrMask::ATIME_SET | SetattrMask::MTIME_SET) {
            let atime = if valid.contains(SetattrMask::ATIME_SET) {
                FileTime::from_unix_time(stat.atime.sec as i64, stat.atime.nsec as u32)
            } else {
                FileTime::from_last_access_time(&fs::metadata(filepath)?)
            };
            let mtime = if valid.contains(SetattrMask::MTIME_SET) {
                FileTime::from_unix_time(stat.mtime.sec as i64, stat.mtime.nsec as u32)
            } else {
                FileTime::from_last_modification_time(&fs::metadata(filepath)?)
            };
            filetime::set_file_times(filepath, atime, mtime)?
        }

        Ok(Fcall::Rsetattr)
    }

    fn rreadlink(&mut self, fid: &mut Fid<Self::Fid>) -> Result<Fcall> {
        let link = fs::read_link(&fid.aux().realpath)?;

        Ok(Fcall::Rreadlink {
            target: link.to_string_lossy().into_owned(),
        })
    }

    fn rreaddir(&mut self, fid: &mut Fid<Self::Fid>, off: u64, count: u32) -> Result<Fcall> {
        let mut dirents = DirEntryData::new();

        let offset = if off == 0 {
            dirents.push(get_dirent_from(".", 0)?);
            dirents.push(get_dirent_from("..", 1)?);
            off
        } else {
            off - 1
        } as usize;

        let entries = fs::read_dir(&fid.aux().realpath)?;
        for (i, entry) in entries.enumerate().skip(offset) {
            let dirent = get_dirent(&entry?, 2 + i as u64)?;
            if dirents.size() + dirent.size() > count {
                break;
            }
            dirents.push(dirent);
        }

        Ok(Fcall::Rreaddir { data: dirents })
    }

    fn rlopen(&mut self, fid: &mut Fid<Self::Fid>, flags: u32) -> Result<Fcall> {
        let qid = get_qid(&fid.aux().realpath)?;

        if !qid.typ.contains(QidType::DIR) {
            let oflags = nix::fcntl::OFlag::from_bits_truncate((flags & UNIX_FLAGS) as i32);
            let omode = nix::sys::stat::Mode::from_bits_truncate(0);
            let fd = nix::fcntl::open(&fid.aux().realpath, oflags, omode)?;
            fid.aux_mut().file = Some(unsafe { fs::File::from_raw_fd(fd) });
        }

        Ok(Fcall::Rlopen {
            qid: qid,
            iounit: 0,
        })
    }

    fn rlcreate(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        name: &str,
        flags: u32,
        mode: u32,
        _gid: u32,
    ) -> Result<Fcall> {
        let path = fid.aux().realpath.join(name);
        let oflags = nix::fcntl::OFlag::from_bits_truncate((flags & UNIX_FLAGS) as i32);
        let omode = nix::sys::stat::Mode::from_bits_truncate(mode);
        let fd = nix::fcntl::open(&path, oflags, omode)?;

        fid.aux = Some(UnpfsFid::new(&path));
        fid.aux_mut().file = Some(unsafe { fs::File::from_raw_fd(fd) });

        Ok(Fcall::Rlcreate {
            qid: get_qid(&path)?,
            iounit: 0,
        })
    }

    fn rread(&mut self, fid: &mut Fid<Self::Fid>, offset: u64, count: u32) -> Result<Fcall> {
        let file = fid.aux_mut().file.as_mut().ok_or(INVALID_FID!())?;
        file.seek(SeekFrom::Start(offset))?;

        let mut buf = create_buffer(count as usize);
        let bytes = file.read(&mut buf[..])?;
        buf.truncate(bytes);

        Ok(Fcall::Rread { data: Data(buf) })
    }

    fn rwrite(&mut self, fid: &mut Fid<Self::Fid>, offset: u64, data: &Data) -> Result<Fcall> {
        let file = fid.aux_mut().file.as_mut().ok_or(INVALID_FID!())?;
        file.seek(SeekFrom::Start(offset))?;

        Ok(Fcall::Rwrite {
            count: file.write(&data.0)? as u32,
        })
    }

    fn rmkdir(
        &mut self,
        dfid: &mut Fid<Self::Fid>,
        name: &str,
        _mode: u32,
        _gid: u32,
    ) -> Result<Fcall> {
        let path = dfid.aux().realpath.join(name);
        fs::create_dir(&path)?;

        Ok(Fcall::Rmkdir {
            qid: get_qid(&path)?,
        })
    }

    fn rrenameat(
        &mut self,
        olddir: &mut Fid<Self::Fid>,
        oldname: &str,
        newdir: &mut Fid<Self::Fid>,
        newname: &str,
    ) -> Result<Fcall> {
        let oldpath = olddir.aux().realpath.join(oldname);
        let newpath = newdir.aux().realpath.join(newname);
        fs::rename(&oldpath, &newpath)?;

        Ok(Fcall::Rrenameat)
    }

    fn runlinkat(&mut self, dirfid: &mut Fid<Self::Fid>, name: &str, _flags: u32) -> Result<Fcall> {
        let path = dirfid.aux().realpath.join(name);

        match fs::symlink_metadata(&path)? {
            ref attr if attr.is_dir() => fs::remove_dir(&path)?,
            _ => fs::remove_file(&path)?,
        };

        Ok(Fcall::Runlinkat)
    }

    fn rfsync(&mut self, fid: &mut Fid<Self::Fid>) -> Result<Fcall> {
        fid.aux_mut()
            .file
            .as_mut()
            .ok_or(INVALID_FID!())?
            .sync_all()?;

        Ok(Fcall::Rfsync)
    }

    fn rclunk(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Ok(Fcall::Rclunk)
    }

    fn rstatfs(&mut self, fid: &mut Fid<Self::Fid>) -> Result<Fcall> {
        let fs = nix::sys::statvfs::statvfs(&fid.aux().realpath)?;

        Ok(Fcall::Rstatfs {
            statfs: From::from(fs),
        })
    }
}

fn unpfs_main(args: Vec<String>) -> rs9p::Result<i32> {
    if args.len() < 3 {
        println!("Usage: {} proto!address!port mountpoint", args[0]);
        println!("  where: proto = tcp");
        return Ok(-1);
    }

    let (addr, mountpoint) = (&args[1], PathBuf::from(&args[2]));
    if !fs::metadata(&mountpoint)?.is_dir() {
        return res!(io_err!(Other, "mount point must be a directory"));
    }

    println!("[*] Ready to accept clients: {}", addr);
    rs9p::srv_spawn(
        Unpfs {
            realroot: mountpoint,
        },
        addr,
    )
    .and(Ok(0))
}

fn main() {
    env_logger::init();

    let args = std::env::args().collect();
    let exit_code = unpfs_main(args).unwrap_or_else(|e| {
        println!("Error: {}", e);
        -1
    });

    std::process::exit(exit_code);
}
