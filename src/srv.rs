
//! Server side 9P library
//!
//! # Protocol
//! 9P2000.L

extern crate nix;
extern crate libc;
extern crate byteorder;

use std::{thread, process};
use std::ops::DerefMut;
use std::net::TcpListener;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use self::byteorder::{ReadBytesExt, WriteBytesExt};

use fcall::*;
use serialize;
use error;
use error::errno::*;
use utils::{self, Result};

/// Represents a fid of clients holding associated `Filesystem::Fid`
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fid<T> {
    /// Raw client side fid
    pub fid: u32,
    /// Qid of this fid
    pub qid: Option<Qid>,
    /// `Filesystem::Fid` associated with this fid.
    /// Changing this value affects the continuous callbacks.
    pub aux: Option<T>,
}

impl<T> Fid<T> {
    /// Unwrap and return a reference to the qid
    pub fn qid(&mut self) -> &mut Qid { self.qid.as_mut().unwrap() }
    /// Unwrap and return a reference to the aux
    ///
    /// # Panics
    /// Calling this method on an aux which is None will cause a panic
    pub fn aux(&mut self) -> &mut T { self.aux.as_mut().unwrap() }
}

/// Filesystem server implementation
///
/// Implementors can represent an error condition by
/// returning an error message string if an operation fails.
/// It is always recommended to choose the one of the error messages
/// in `error` module as the returned one.
///
/// The default implementation, returning ENOSYS error, is provided to the all methods
/// except Rversion.
/// The default implementation of Rversion returns a message accepting 9P2000.L.
///
/// # NOTE
/// Defined as `Srv` in 9p.h of Plan 9.
///
/// # Protocol
/// 9P2000.L
pub trait Filesystem {
    /// User defined fid type to be associated with a client's fid
    type Fid = ();

    // 9P2000.L
    fn rstatfs(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlopen(&mut self, _: &mut Fid<Self::Fid>, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlcreate(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rsymlink(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _sym: &str, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rmknod(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _mode: u32, _major: u32, _minor: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rrename(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rreadlink(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rgetattr(&mut self, _: &mut Fid<Self::Fid>, _req_mask: GetattrMask)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rsetattr(&mut self, _: &mut Fid<Self::Fid>, _valid: SetattrMask, _stat: &SetAttr)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rxattrwalk(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rxattrcreate(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _attr_size: u64, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rreaddir(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rfsync(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Flock)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rgetlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Getlock)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlink(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rmkdir(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rrenameat(&mut self, _: &mut Fid<Self::Fid>, _oldname: &str, _: &mut Fid<Self::Fid>, _newname: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn runlinkat(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }

    // 9P2000.u subset
    fn rauth(&mut self, _: &mut Fid<Self::Fid>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rattach(&mut self, _: &mut Fid<Self::Fid>, _afid: Option<&mut Fid<Self::Fid>>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }

    // 9P2000 subset
    fn rflush(&mut self, _old: Option<&mut Fcall>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rwalk(&mut self, _: &mut Fid<Self::Fid>, _new: &mut Fid<Self::Fid>, _wnames: &[String])
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rread(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rwrite(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _data: &Data)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rclunk(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rremove(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rversion(&mut self, ms: u32, ver: &str) -> Result<Fcall> {
        match ver {
            P92000L => Ok(Fcall::Rversion { msize: ms, version: ver.to_owned() }),
            _ => Err(error::Error::No(EPROTONOSUPPORT))
        }
    }
}

struct ServerInstance<Fs: Filesystem, RwExt> {
    fs: Fs,
    stream: RwExt,
    fids: HashMap<u32, Fid<Fs::Fid>>,
}

impl<Fs, RwExt> ServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Fs, stream: RwExt) -> Result<ServerInstance<Fs, RwExt>> {
        let server = ServerInstance {
            fs: fs,
            stream: stream,
            fids: HashMap::new(),
        };
        Ok(server)
    }

    fn dispatch(&mut self) -> Result<()> {
        loop {
            let msg = try!(serialize::read_msg(&mut self.stream));
            let (fcall, tag) = try!(dispatch_once(
                msg,
                &mut self.fs,
                &mut self.fids)
            );

            try!(utils::respond(&mut self.stream, fcall, tag));
        }
    }
}

struct SpawnServerInstance<Fs: Filesystem, RwExt> {
    fs: Arc<Mutex<Fs>>,
    stream: RwExt,
    fids: HashMap<u32, Fid<Fs::Fid>>,
}

impl<Fs, RwExt> SpawnServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt) -> Result<SpawnServerInstance<Fs, RwExt>> {
        let server = SpawnServerInstance {
            fs: fs,
            stream: stream,
            fids: HashMap::new(),
        };
        Ok(server)
    }

    fn dispatch(&mut self) -> Result<()> {
        loop {
            let msg = try!(serialize::read_msg(&mut self.stream));
            let (fcall, tag) = try!(dispatch_once(
                msg,
                self.fs.lock().unwrap().deref_mut(),
                &mut self.fids
            ));

            try!(utils::respond(&mut self.stream, fcall, tag));
        }
    }
}

fn dispatch_once<FsFid>(msg: Msg, fs: &mut Filesystem<Fid=FsFid>, fsfids: &mut HashMap<u32, Fid<FsFid>>) -> Result<(Fcall, u16)> {
    // Take all fids associated with the fids which the request contains
    let mut fids: Vec<_> = msg.body.fid().iter().map(|f| fsfids.remove(&f).unwrap()).collect();
    let mut newfids: Vec<_> = msg.body.newfid().iter().map(|f| Fid { fid: *f, qid: None, aux: None }).collect();

    let result = match msg.body {
        Fcall::Tstatfs { fid: _ }                                                       => { fs.rstatfs(&mut fids[0]) },
        Fcall::Tlopen { fid: _, ref flags }                                             => { fs.rlopen(&mut fids[0], *flags) },
        Fcall::Tlcreate { fid: _, ref name, ref flags, ref mode, ref gid }              => { fs.rlcreate(&mut fids[0], name, *flags, *mode, *gid) },
        Fcall::Tsymlink { fid: _, ref name, ref symtgt, ref gid }                       => { fs.rsymlink(&mut fids[0], name, symtgt, *gid) },
        Fcall::Tmknod { dfid: _, ref name, ref mode, ref major, ref minor, ref gid }    => { fs.rmknod(&mut fids[0], name, *mode, *major, *minor, *gid) },
        Fcall::Trename { fid: _, dfid: _, ref name }                                    => {
            let (mut fid, mut dfid) = (fids.remove(0), fids.remove(0));
            let r = fs.rrename(&mut fid, &mut dfid, name);
            fids.push(fid); fids.push(dfid);
            r
        },
        Fcall::Treadlink { fid: _ }                                                     => { fs.rreadlink(&mut fids[0]) },
        Fcall::Tgetattr { fid: _, ref req_mask }                                        => { fs.rgetattr(&mut fids[0], *req_mask) },
        Fcall::Tsetattr { fid: _, ref valid, ref stat }                                 => { fs.rsetattr(&mut fids[0], *valid, stat) },
        Fcall::Txattrwalk { fid: _, newfid: _, ref name }                               => { fs.rxattrwalk(&mut fids[0], &mut newfids[0], name) },
        Fcall::Txattrcreate { fid: _, ref name, ref attr_size, ref flags }              => { fs.rxattrcreate(&mut fids[0], name, *attr_size, *flags) },
        Fcall::Treaddir { fid: _, ref offset, ref count }                               => { fs.rreaddir(&mut fids[0], *offset, *count) },
        Fcall::Tfsync { fid: _ }                                                        => { fs.rfsync(&mut fids[0]) },
        Fcall::Tlock { fid: _, ref flock }                                              => { fs.rlock(&mut fids[0], flock) },
        Fcall::Tgetlock { fid: _, ref flock }                                           => { fs.rgetlock(&mut fids[0], flock) },
        Fcall::Tlink { dfid: _, fid: _, ref name }                                      => {
            let (mut dfid, mut fid) = (fids.remove(0), fids.remove(0));
            let r = fs.rlink(&mut dfid, &mut fid, name);
            fids.push(dfid); fids.push(fid);
            r
        },
        Fcall::Tmkdir { dfid: _, ref name, ref mode, ref gid }                          => { fs.rmkdir(&mut fids[0], name, *mode, *gid) },
        Fcall::Trenameat { olddirfid: _, ref oldname, newdirfid: _, ref newname }       => {
            let (mut old, mut new) = (fids.remove(0), fids.remove(0));
            let r = fs.rrenameat(&mut old, oldname, &mut new, newname);
            fids.push(old); fids.push(new);
            r
        },
        Fcall::Tunlinkat { dirfd: _, ref name, ref flags }                              => { fs.runlinkat(&mut fids[0], name, *flags) },

        // 9P2000.u
        Fcall::Tauth { afid: _, ref uname, ref aname, ref n_uname }                     => { fs.rauth(&mut newfids[0], uname, aname, *n_uname) },
        Fcall::Tattach { fid: _, afid: _, ref uname, ref aname, ref n_uname }           => { fs.rattach(&mut newfids[0], None, uname, aname, *n_uname) },

        // 9P2000
        Fcall::Tversion { ref msize, ref version }                                      => { fs.rversion(*msize, version) },
        Fcall::Tflush { oldtag: _ }                                                     => { fs.rflush(None) },
        Fcall::Twalk { fid: _, newfid: _, ref wnames }                                  => { fs.rwalk(&mut fids[0], &mut newfids[0], wnames) },
        Fcall::Tread { fid: _, ref offset, ref count }                                  => { fs.rread(&mut fids[0], *offset, *count) },
        Fcall::Twrite { fid: _, ref offset, ref data }                                  => { fs.rwrite(&mut fids[0], *offset, data) },
        Fcall::Tclunk { fid: _ }                                                        => {
            let r = fs.rclunk(&mut fids[0]);
            // Drop the fid which the request contains
            if r.is_ok() { fids.clear(); }
            r
        },
        Fcall::Tremove { fid: _ }                                                       => { fs.rremove(&mut fids[0]) },
        _ => return try!(io_error!(Other, "Invalid 9P message received")),
    };

    // Restore the fids taken
    for f in fids { fsfids.insert(f.fid, f); }
    for f in newfids { fsfids.insert(f.fid, f); }

    let response = match result {
        Ok(res)  => res,
        Err(err) => Fcall::Rlerror { ecode: err.errno() as u32 }
    };

    Ok((response, msg.tag))
}

/// Start the 9P filesystem (fork child processes)
///
/// This function forks a child process to handle its 9P messages
/// when a client connects to the server.
pub fn srv<Fs: Filesystem>(filesystem: Fs, addr: &str) -> Result<()> {
    let (proto, sockaddr) = try!(utils::parse_proto(addr).or(
        io_error!(InvalidInput, "Invalid protocol or address")
    ));

    if proto != "tcp" {
        return try!(io_error!(InvalidInput, format!("Unsupported protocol: {}", proto)));
    }

    // Do not wait for child processes
    unsafe { libc::funcs::posix01::signal::signal(nix::sys::signal::SIGCHLD, libc::SIG_IGN); }

    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, remote) = try!(listener.accept());
        match try!(nix::unistd::fork()) {
            nix::unistd::Fork::Parent(_) => {},
            nix::unistd::Fork::Child => {
                info!("ServerProcess={} starts", remote);

                try!(utils::setup_tcp_stream(&stream));
                let result = try!(ServerInstance::new(filesystem, stream)).dispatch();

                info!("ServerProcess={} finished: {:?}", remote, result);
                process::exit(1);
            }
        }
    }
}

/// Start the 9P filesystem (spawning threads)
///
/// This function spawns a new thread to handle its 9P messages
/// when a client connects to the server.
pub fn srv_spawn<Fs: Filesystem + Send + 'static>(filesystem: Fs, addr: &str) -> Result<()> {
    let (proto, sockaddr) = try!(utils::parse_proto(addr).or(
        io_error!(InvalidInput, "Invalid protocol or address")
    ));

    if proto != "tcp" {
        return try!(io_error!(InvalidInput, format!("Unsupported protocol: {}", proto)));
    }

    let arc_fs = Arc::new(Mutex::new(filesystem));
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, remote) = try!(listener.accept());
        let (fs, thread_name) = (arc_fs.clone(), format!("{}", remote));
        let _ = thread::Builder::new().name(thread_name.clone()).spawn(move || {
            info!("ServerThread={:?} started", thread_name);

            try!(utils::setup_tcp_stream(&stream));
            let result = try!(SpawnServerInstance::new(fs, stream)).dispatch();

            info!("ServerThread={:?} finished: {:?}", thread_name, result);
            result
        });
    }
}
