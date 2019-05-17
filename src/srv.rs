//! Server side 9P library.
//!
//! # Protocol
//! 9P2000.L

use byteorder::{ReadBytesExt, WriteBytesExt};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::collections::HashMap;
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};

use crate::error;
use crate::error::errno::*;
use crate::fcall::*;
use crate::serialize;
use crate::utils::{self, Result};

/// Represents a fid of clients holding associated `Filesystem::Fid`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fid<T> {
    /// Raw client side fid.
    fid: u32,

    /// `Filesystem::Fid` associated with this fid.
    /// Changing this value affects the continuous callbacks.
    pub aux: Option<T>,
}

impl<T> Fid<T> {
    /// Get the raw fid.
    pub fn fid(&self) -> u32 {
        self.fid
    }

    /// Unwrap and return a reference to the aux.
    ///
    /// # Panics
    /// Calling this method on an aux which is None will cause a panic.
    pub fn aux(&self) -> &T {
        self.aux.as_ref().unwrap()
    }

    /// Unwrap and return a mutable reference to the aux.
    ///
    /// # Panics
    /// Calling this method on an aux which is None will cause a panic.
    pub fn aux_mut(&mut self) -> &mut T {
        self.aux.as_mut().unwrap()
    }
}

/// Filesystem server trait.
///
/// Implementors can represent an error condition by returning an `Err`.
/// Otherwise, they must return `Fcall` with the required fields filled.
///
/// The default implementation, returning EOPNOTSUPP error, is provided to the all methods
/// except Rversion.
/// The default implementation of Rversion returns a message accepting 9P2000.L.
///
/// # NOTE
/// Defined as `Srv` in 9p.h of Plan 9.
///
/// # Protocol
/// 9P2000.L
pub trait Filesystem {
    /// User defined fid type to be associated with a client's fid.
    type Fid;

    // 9P2000.L
    fn rstatfs(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rlopen(&mut self, _: &mut Fid<Self::Fid>, _flags: u32) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rlcreate(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _name: &str,
        _flags: u32,
        _mode: u32,
        _gid: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rsymlink(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _name: &str,
        _sym: &str,
        _gid: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rmknod(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _name: &str,
        _mode: u32,
        _major: u32,
        _minor: u32,
        _gid: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rrename(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _: &mut Fid<Self::Fid>,
        _name: &str,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rreadlink(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rgetattr(&mut self, _: &mut Fid<Self::Fid>, _req_mask: GetattrMask) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rsetattr(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _valid: SetattrMask,
        _stat: &SetAttr,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rxattrwalk(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _: &mut Fid<Self::Fid>,
        _name: &str,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rxattrcreate(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _name: &str,
        _attr_size: u64,
        _flags: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rreaddir(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rfsync(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Flock) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rgetlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Getlock) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rlink(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _: &mut Fid<Self::Fid>,
        _name: &str,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rmkdir(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _name: &str,
        _mode: u32,
        _gid: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rrenameat(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _oldname: &str,
        _: &mut Fid<Self::Fid>,
        _newname: &str,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn runlinkat(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    /*
     * 9P2000.u subset
     */
    fn rauth(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _uname: &str,
        _aname: &str,
        _n_uname: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rattach(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _afid: Option<&mut Fid<Self::Fid>>,
        _uname: &str,
        _aname: &str,
        _n_uname: u32,
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    /*
     * 9P2000 subset
     */
    fn rflush(&mut self, _old: Option<&mut Fcall>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rwalk(
        &mut self,
        _: &mut Fid<Self::Fid>,
        _new: &mut Fid<Self::Fid>,
        _wnames: &[String],
    ) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rread(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rwrite(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _data: &Data) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rclunk(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rremove(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Err(error::Error::No(EOPNOTSUPP))
    }

    fn rversion(&mut self, ms: u32, ver: &str) -> Result<Fcall> {
        match ver {
            P92000L => Ok(Fcall::Rversion {
                msize: ms,
                version: ver.to_owned(),
            }),
            _ => Err(error::Error::No(EPROTONOSUPPORT)),
        }
    }
}

struct ServerInstance<Fs: Filesystem, RwExt> {
    fs: Fs,
    stream: RwExt,
    fids: HashMap<u32, Fid<Fs::Fid>>,
}

impl<Fs, RwExt> ServerInstance<Fs, RwExt>
where
    Fs: Filesystem,
    RwExt: ReadBytesExt + WriteBytesExt,
{
    fn new(fs: Fs, stream: RwExt) -> Result<ServerInstance<Fs, RwExt>> {
        let server = ServerInstance {
            fs,
            stream,
            fids: HashMap::new(),
        };
        Ok(server)
    }

    fn dispatch(&mut self) -> Result<()> {
        loop {
            let msg = serialize::read_msg(&mut self.stream)?;

            debug!("\t→ {:?}", msg);
            let (fcall, tag) = dispatch_once(msg, &mut self.fs, &mut self.fids)?;

            utils::respond(&mut self.stream, tag, fcall)?;
        }
    }
}

struct SpawnServerInstance<Fs: Filesystem, RwExt> {
    fs: Arc<Mutex<Fs>>,
    stream: RwExt,
    fids: HashMap<u32, Fid<Fs::Fid>>,
}

impl<Fs, RwExt> SpawnServerInstance<Fs, RwExt>
where
    Fs: Filesystem,
    RwExt: ReadBytesExt + WriteBytesExt,
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt) -> Result<SpawnServerInstance<Fs, RwExt>> {
        let server = SpawnServerInstance {
            fs,
            stream,
            fids: HashMap::new(),
        };
        Ok(server)
    }

    fn dispatch(&mut self) -> Result<()> {
        loop {
            //let msg = serialize::read_msg(&mut self.stream)?;
            let msg = serialize::read_msg(&mut self.stream)?;

            debug!("\t→ {:?}", msg);
            let (fcall, tag) = dispatch_once(msg, &mut *self.fs.lock().unwrap(), &mut self.fids)?;

            utils::respond(&mut self.stream, tag, fcall)?;
        }
    }
}

fn dispatch_once<FsFid>(
    msg: Msg,
    fs: &mut Filesystem<Fid = FsFid>,
    fsfids: &mut HashMap<u32, Fid<FsFid>>,
) -> Result<(Fcall, u16)> {
    use crate::Fcall::*;

    let mut fids = Vec::new();
    for fid in msg.body.fids().iter().map(|f| fsfids.remove(&f).ok_or(f)) {
        match fid {
            Ok(fid) => fids.push(fid),
            Err(rawfid) => {
                error!("No Fid associated with: {}", rawfid);
                return res!(io_err!(NotFound, "No associated Fid"));
            }
        }
    }

    let mut newfids: Vec<_> = msg
        .body
        .newfids()
        .iter()
        .map(|f| Fid { fid: *f, aux: None })
        .collect();

    let response = match msg.body {
        Tstatfs { fid: _ }                                                  => { fs.rstatfs(&mut fids[0]) },
        Tlopen { fid: _, ref flags }                                        => { fs.rlopen(&mut fids[0], *flags) },
        Tlcreate { fid: _, ref name, ref flags, ref mode, ref gid }         => { fs.rlcreate(&mut fids[0], name, *flags, *mode, *gid) },
        Tsymlink { fid: _, ref name, ref symtgt, ref gid }                  => { fs.rsymlink(&mut fids[0], name, symtgt, *gid) },
        Tmknod { ref name, ref mode, ref major, ref minor, ref gid, .. }    => { fs.rmknod(&mut fids[0], name, *mode, *major, *minor, *gid) },
        Trename { fid: _, dfid: _, ref name }                               => {
            let (fid, dfid) = fids.split_at_mut(1);
            fs.rrename(&mut fid[0], &mut dfid[0], name)
        },
        Treadlink { fid: _ }                                                => { fs.rreadlink(&mut fids[0]) },
        Tgetattr { fid: _, ref req_mask }                                   => { fs.rgetattr(&mut fids[0], *req_mask) },
        Tsetattr { fid: _, ref valid, ref stat }                            => { fs.rsetattr(&mut fids[0], *valid, stat) },
        Txattrwalk { fid: _, newfid: _, ref name }                          => { fs.rxattrwalk(&mut fids[0], &mut newfids[0], name) },
        Txattrcreate { fid: _, ref name, ref attr_size, ref flags }         => { fs.rxattrcreate(&mut fids[0], name, *attr_size, *flags) },
        Treaddir { fid: _, ref offset, ref count }                          => { fs.rreaddir(&mut fids[0], *offset, *count) },
        Tfsync { fid: _ }                                                   => { fs.rfsync(&mut fids[0]) },
        Tlock { fid: _, ref flock }                                         => { fs.rlock(&mut fids[0], flock) },
        Tgetlock { fid: _, ref flock }                                      => { fs.rgetlock(&mut fids[0], flock) },
        Tlink { dfid: _, fid: _, ref name }                                 => {
            let (dfid, fid) = fids.split_at_mut(1);
            fs.rlink(&mut dfid[0], &mut fid[0], name)
        },
        Tmkdir { dfid: _, ref name, ref mode, ref gid }                     => { fs.rmkdir(&mut fids[0], name, *mode, *gid) },
        Trenameat { olddirfid: _, ref oldname, newdirfid: _, ref newname }  => {
            let (old, new) = fids.split_at_mut(1);
            fs.rrenameat(&mut old[0], oldname, &mut new[0], newname)
        },
        Tunlinkat { dirfd: _, ref name, ref flags }                         => { fs.runlinkat(&mut fids[0], name, *flags) },
        Tauth { afid: _, ref uname, ref aname, ref n_uname }                => { fs.rauth(&mut newfids[0], uname, aname, *n_uname) },
        Tattach { fid: _, afid: _, ref uname, ref aname, ref n_uname }      => { fs.rattach(&mut newfids[0], None, uname, aname, *n_uname) },
        Tversion { ref msize, ref version }                                 => { fs.rversion(*msize, version) },
        Tflush { oldtag: _ }                                                => { fs.rflush(None) },
        Twalk { fid: _, newfid: _, ref wnames }                             => { fs.rwalk(&mut fids[0], &mut newfids[0], wnames) },
        Tread { fid: _, ref offset, ref count }                             => { fs.rread(&mut fids[0], *offset, *count) },
        Twrite { fid: _, ref offset, ref data }                             => { fs.rwrite(&mut fids[0], *offset, data) },
        Tclunk { fid: _ }   /* Drop the fid which the request contains */   => { fs.rclunk(&mut fids[0]).map(|e| { fids.clear(); e }) },
        Tremove { fid: _ }                                                  => { fs.rremove(&mut fids[0]) },
        _                                                                   => return res!(io_err!(Other, "Invalid 9P message received")),
    }.unwrap_or_else(|e| Fcall::Rlerror {
        ecode: e.errno() as u32,
    });

    for f in fids {
        fsfids.insert(f.fid, f);
    }

    for f in newfids {
        fsfids.insert(f.fid, f);
    }

    Ok((response, msg.tag))
}

// Just for ReadBytesExt and WriteBytesExt
trait ReadWriteBytesExt: std::io::Read + std::io::Write {}

impl<T> ReadWriteBytesExt for T where T: std::io::Read + std::io::Write {}

trait SocketListener {
    fn accept_client(&self) -> Result<(Box<dyn ReadWriteBytesExt + Send>, String)>;
}

impl SocketListener for TcpListener {
    fn accept_client(&self) -> Result<(Box<dyn ReadWriteBytesExt + Send>, String)> {
        let (stream, remote) = self.accept()?;
        utils::setup_tcp_stream(&stream)?;

        Ok((Box::new(stream), remote.to_string()))
    }
}

impl SocketListener for UnixListener {
    fn accept_client(&self) -> Result<(Box<dyn ReadWriteBytesExt + Send>, String)> {
        let (stream, remote) = self.accept()?;
        let remote = remote
            .as_pathname()
            .and_then(std::path::Path::to_str)
            .unwrap_or(":unnamed:")
            .to_owned();

        Ok((Box::new(stream), remote))
    }
}

/// Start a 9P filesystem (forking child processes).
///
/// This function forks a child process to handle its 9P messages
/// when a client connects to the server.
pub fn srv<Fs: Filesystem>(filesystem: Fs, addr: &str) -> Result<()> {
    let (proto, sockaddr) =
        utils::parse_proto(addr).ok_or(io_err!(InvalidInput, "Invalid protocol or address"))?;

    let listener: Box<dyn SocketListener> = match proto {
        "tcp" => Box::new(TcpListener::bind(&sockaddr[..])?),
        "unix" => Box::new(UnixListener::bind(&sockaddr[..])?),
        _ => {
            return res!(io_err!(
                InvalidInput,
                format!("Unsupported protocol: {}", proto)
            ));
        }
    };

    // Do not wait for child processes
    unsafe {
        sigaction(
            Signal::SIGCHLD,
            &SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty()),
        )?;
    }

    loop {
        let (stream, remote) = listener.accept_client()?;

        match nix::unistd::fork()? {
            nix::unistd::ForkResult::Parent { .. } => {}
            nix::unistd::ForkResult::Child => {
                info!("ServerProcess={} starts", remote);

                let result = ServerInstance::new(filesystem, stream)?.dispatch();

                info!("ServerProcess={} finished: {:?}", remote, result);
                std::process::exit(1);
            }
        }
    }
}

/// Start a 9P filesystem (spawning threads).
///
/// This function spawns a new thread to handle its 9P messages
/// when a client connects to the server.
pub fn srv_spawn<Fs: Filesystem + Send + 'static>(filesystem: Fs, addr: &str) -> Result<()> {
    let (proto, sockaddr) =
        utils::parse_proto(addr).ok_or(io_err!(InvalidInput, "Invalid protocol or address"))?;

    let listener: Box<dyn SocketListener> = match proto {
        "tcp" => Box::new(TcpListener::bind(&sockaddr[..])?),
        "unix" => Box::new(UnixListener::bind(&sockaddr[..])?),
        _ => {
            return res!(io_err!(
                InvalidInput,
                format!("Unsupported protocol: {}", proto)
            ))
        }
    };

    let arc_fs = Arc::new(Mutex::new(filesystem));

    loop {
        let (stream, remote) = listener.accept_client()?;
        let fs = arc_fs.clone();

        let _ = std::thread::Builder::new().spawn(move || {
            info!("ServerThread={:?} started", remote);

            let result = SpawnServerInstance::new(fs, stream).and_then(|mut s| s.dispatch());

            info!("ServerThread={} finished: {:?}", remote, result);
        });
    }
}
