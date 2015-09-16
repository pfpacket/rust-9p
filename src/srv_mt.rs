
//! Server side 9P library, with multi-thread support
//!
//! # Protocol
//! 9P2000.L

extern crate nix;
extern crate byteorder;
extern crate comm;

use std::thread;
use std::net::{TcpStream, TcpListener};
use std::collections::HashMap;
use std::sync::{RwLock, Arc};

use fcall::*;
use serialize;
use error;
use error::errno::*;
use utils::{self, Result};

/// Represents a fid of clients holding associated `Filesystem::Fid`
#[derive(Debug)]
pub struct Fid<T> {
    fid: u32,
    pub aux: RwLock<Option<T>>,
}

impl<T> Fid<T> {
    /// Get the raw fid
    pub fn fid(&self) -> u32 { self.fid }
}

/// Filesystem server implementation
///
/// All methods are immutable for multi-threading.
/// Implementors are expected to use interior mutability such as Mutex or RwLock.
///
/// # NOTE
/// Defined as `Srv` in 9p.h of Plan 9.
///
/// # Protocol
/// 9P2000.L
pub trait Filesystem: Send + Sync {
    /// User defined fid type to be associated with a client's fid
    type Fid: Send + Sync + 'static;

    // 9P2000.L
    fn rstatfs(&self, _: Arc<Fid<Self::Fid>>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlopen(&self, _: Arc<Fid<Self::Fid>>, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlcreate(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _flags: u32, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rsymlink(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _sym: &str, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rmknod(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _mode: u32, _major: u32, _minor: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rrename(&self, _: Arc<Fid<Self::Fid>>, _: Arc<Fid<Self::Fid>>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rreadlink(&self, _: Arc<Fid<Self::Fid>>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rgetattr(&self, _: Arc<Fid<Self::Fid>>, _req_mask: GetattrMask)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rsetattr(&self, _: Arc<Fid<Self::Fid>>, _valid: SetattrMask, _stat: &SetAttr)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rxattrwalk(&self, _: Arc<Fid<Self::Fid>>, _: Arc<Fid<Self::Fid>>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rxattrcreate(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _attr_size: u64, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rreaddir(&self, _: Arc<Fid<Self::Fid>>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rfsync(&self, _: Arc<Fid<Self::Fid>>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlock(&self, _: Arc<Fid<Self::Fid>>, _lock: &Flock)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rgetlock(&self, _: Arc<Fid<Self::Fid>>, _lock: &Getlock)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rlink(&self, _: Arc<Fid<Self::Fid>>, _: Arc<Fid<Self::Fid>>, _name: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rmkdir(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rrenameat(&self, _: Arc<Fid<Self::Fid>>, _oldname: &str, _: Arc<Fid<Self::Fid>>, _newname: &str)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn runlinkat(&self, _: Arc<Fid<Self::Fid>>, _name: &str, _flags: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }

    // 9P2000.u subset
    fn rauth(&self, _: Arc<Fid<Self::Fid>>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rattach(&self, _: Arc<Fid<Self::Fid>>, _afid: Option<Arc<Fid<Self::Fid>>>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }

    // 9P2000 subset
    fn rflush(&self, _old: Option<&mut Fcall>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rwalk(&self, _: Arc<Fid<Self::Fid>>, _new: Arc<Fid<Self::Fid>>, _wnames: &[String])
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rread(&self, _: Arc<Fid<Self::Fid>>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rwrite(&self, _: Arc<Fid<Self::Fid>>, _offset: u64, _data: &Data)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rclunk(&self, _: Arc<Fid<Self::Fid>>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rremove(&self, _: Arc<Fid<Self::Fid>>)
        -> Result<Fcall> { Err(error::Error::No(ENOSYS)) }
    fn rversion(&self, ms: u32, ver: &str) -> Result<Fcall> {
        match ver {
            P92000L => Ok(Fcall::Rversion { msize: ms, version: ver.to_owned() }),
            _ => Err(error::Error::No(EPROTONOSUPPORT))
        }
    }
}

struct MtServerInstance<Fs: Filesystem> {
    fs: Arc<Fs>,
    stream: TcpStream,
    fids: Arc<RwLock<HashMap<u32, Arc<Fid<Fs::Fid>>>>>,
}

impl<Fs: Filesystem + 'static> MtServerInstance<Fs> {
    fn new(fs: Arc<Fs>, stream: TcpStream) -> Result<MtServerInstance<Fs>> {
        let server = MtServerInstance {
            fs: fs,
            stream: stream,
            fids: Arc::new(RwLock::new(HashMap::new())),
        };
        Ok(server)
    }

    fn dispatch(&self) -> Result<()> {
        let mut threads = Vec::new();
        let (tx, rx) = unsafe { comm::spmc::bounded_fast::new(4096) };

        // Message queueing
        let mut stream = try!(self.stream.try_clone());
        let thread = thread::spawn(move || { loop {
            let _ = try!( serialize::read_msg(&mut stream)
                .map(|msg| tx.send_sync(msg).map_err(|e| error!("queuer: {:?}", e)))
                .map_err(|e| { warn!("queuer: {:?}", e); e }) );
        }});
        threads.push(thread);

        // Message dispatching
        for _ in 0..5 {
            let (fsfids, fs) = (self.fids.clone(), self.fs.clone());
            let (rx, mut stream) = (rx.clone(), try!(self.stream.try_clone()));

            let thread = thread::spawn(move || -> Result<()> { loop {
                try!(rx.recv_sync().map_err(|e| {
                    warn!("dispatcher: {:?}", e);
                    error::Error::Io(io_err!(Other, format!("{:?}", e)))
                }).and_then(|msg| {
                    debug!("\t→ {:?}", msg);
                    let (fcall, tag) = try!(mt_dispatch_once(msg, &*fs, &fsfids));
                    utils::respond(&mut stream, tag, fcall)
                        .map_err(|e| { error!("dispatcher: {:?}", e); e })
                }));
            }});
            threads.push(thread);
        }

        threads.into_iter().all(|th| { let _ = th.join(); true }); Ok(())
    }
}

fn mt_dispatch_once<FsFid>(msg: Msg, fs: &Filesystem<Fid=FsFid>, fsfids: &RwLock<HashMap<u32, Arc<Fid<FsFid>>>>)
    -> Result<(Fcall, u16)> where FsFid: Send + Sync + 'static
{
    use Fcall::*;

    let fids: Vec<_> = msg.body.fids().iter()
        .map(|f| fsfids.read().unwrap().get(&f).unwrap().clone())
        .collect();
    let newfids: Vec<_> = msg.body.newfids().iter()
        .map(|f| Arc::new(Fid { fid: *f, aux: RwLock::new(None) }))
        .collect();

    let response = match msg.body {
        Tstatfs { fid: _ }                                                      => { fs.rstatfs(fids[0].clone()) },
        Tlopen { fid: _, ref flags }                                            => { fs.rlopen(fids[0].clone(), *flags) },
        Tlcreate { fid: _, ref name, ref flags, ref mode, ref gid }             => { fs.rlcreate(fids[0].clone(), name, *flags, *mode, *gid) },
        Tsymlink { fid: _, ref name, ref symtgt, ref gid }                      => { fs.rsymlink(fids[0].clone(), name, symtgt, *gid) },
        Tmknod { dfid: _, ref name, ref mode, ref major, ref minor, ref gid }   => { fs.rmknod(fids[0].clone(), name, *mode, *major, *minor, *gid) },
        Trename { fid: _, dfid: _, ref name }                                   => { fs.rrename(fids[0].clone(), fids[1].clone(), name) },
        Treadlink { fid: _ }                                                    => { fs.rreadlink(fids[0].clone()) },
        Tgetattr { fid: _, ref req_mask }                                       => { fs.rgetattr(fids[0].clone(), *req_mask) },
        Tsetattr { fid: _, ref valid, ref stat }                                => { fs.rsetattr(fids[0].clone(), *valid, stat) },
        Txattrwalk { fid: _, newfid: _, ref name }                              => { fs.rxattrwalk(fids[0].clone(), newfids[0].clone(), name) },
        Txattrcreate { fid: _, ref name, ref attr_size, ref flags }             => { fs.rxattrcreate(fids[0].clone(), name, *attr_size, *flags) },
        Treaddir { fid: _, ref offset, ref count }                              => { fs.rreaddir(fids[0].clone(), *offset, *count) },
        Tfsync { fid: _ }                                                       => { fs.rfsync(fids[0].clone()) },
        Tlock { fid: _, ref flock }                                             => { fs.rlock(fids[0].clone(), flock) },
        Tgetlock { fid: _, ref flock }                                          => { fs.rgetlock(fids[0].clone(), flock) },
        Tlink { dfid: _, fid: _, ref name }                                     => { fs.rlink(fids[0].clone(), fids[1].clone(), name) },
        Tmkdir { dfid: _, ref name, ref mode, ref gid }                         => { fs.rmkdir(fids[0].clone(), name, *mode, *gid) },
        Trenameat { olddirfid: _, ref oldname, newdirfid: _, ref newname }      => { fs.rrenameat(fids[0].clone(), oldname, fids[1].clone(), newname) },
        Tunlinkat { dirfd: _, ref name, ref flags }                             => { fs.runlinkat(fids[0].clone(), name, *flags) },
        Tauth { afid: _, ref uname, ref aname, ref n_uname }                    => { fs.rauth(newfids[0].clone(), uname, aname, *n_uname) },
        Tattach { fid: _, afid: _, ref uname, ref aname, ref n_uname }          => { fs.rattach(newfids[0].clone(), None, uname, aname, *n_uname) },
        Tversion { ref msize, ref version }                                     => { fs.rversion(*msize, version) },
        Tflush { oldtag: _ }                                                    => { fs.rflush(None) },
        Twalk { fid: _, newfid: _, ref wnames }                                 => { fs.rwalk(fids[0].clone(), newfids[0].clone(), wnames) },
        Tread { fid: _, ref offset, ref count }                                 => { fs.rread(fids[0].clone(), *offset, *count) },
        Twrite { fid: _, ref offset, ref data }                                 => { fs.rwrite(fids[0].clone(), *offset, data) },
        Tclunk { fid: _ }    /* Drop the fid which the request contains */      => {
            fs.rclunk(fids[0].clone()).map_err(|e| { fsfids.write().unwrap().remove(&fids[0].fid); e })
        },
        Tremove { fid: _ }                                                      => { fs.rremove(fids[0].clone()) },
        _                                                                       => return res!(io_err!(Other, "Invalid 9P message received")),
    }.unwrap_or_else(|e| Fcall::Rlerror { ecode: e.errno() as u32 });

    // Add newfids
    let mut fsfids_unlocked = fsfids.write().unwrap();
    for f in newfids { fsfids_unlocked.insert(f.fid, f); }

    Ok((response, msg.tag))
}

/// Start the 9P filesystem (multi threads)
///
/// This function spawns a new thread to handle its 9P messages
/// when a client connects to the server.
/// The each thread will spawn a new message queueing thread
/// and some message dispatching threads.
pub fn srv_mt<Fs: Filesystem + Send + 'static>(filesystem: Fs, addr: &str) -> Result<()> {
    let (proto, sockaddr) = try!(utils::parse_proto(addr).ok_or(
        io_err!(InvalidInput, "Invalid protocol or address")
    ));

    if proto != "tcp" {
        return res!(io_err!(InvalidInput, format!("Unsupported protocol: {}", proto)));
    }

    let arc_fs = Arc::new(filesystem);
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, remote) = try!(listener.accept());
        let (fs, thread_name) = (arc_fs.clone(), format!("{}", remote));

        let _ = thread::Builder::new().name(thread_name.clone()).spawn(move || {
            info!("ServerThread={:?} started", thread_name);
            let result = {|| {
                try!(utils::setup_tcp_stream(&stream));
                try!(MtServerInstance::new(fs, stream)).dispatch()
            }}();
            info!("ServerThread={:?} finished: {:?}", thread_name, result);
        });
    }
}
