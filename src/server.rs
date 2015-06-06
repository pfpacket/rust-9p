
//! Server side 9P library

extern crate nix;
extern crate byteorder;

use serialize;
use fcall::*;
use std::{io, result, fmt, thread};
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Mutex, Arc};
use self::byteorder::{ReadBytesExt, WriteBytesExt};
use error::number::*;

pub type Result<T> = result::Result<T, nix::Error>;

macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
}

/// Represents a fid of clients holding associated `Filesystem::Fid`
#[derive(Clone, Debug)]
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
    /// Unwrap the aux and returns a reference to it
    pub fn aux(&mut self) -> &mut T { self.aux.as_mut().unwrap() }
    pub fn qid(&mut self) -> &mut Qid { self.qid.as_mut().unwrap() }
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
///
/// The default implementation of Rversion returns a message accepting 9P2000.L.
///
/// Protocol: 9P2000.L
///
/// NOTE: Defined as `Srv` in 9p.h of Plan 9.
pub trait Filesystem: Send {
    /// User defined fid type to be associated with a client's fid
    type Fid: fmt::Debug = ();

    // 9P2000.L
    fn rstatfs(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rlopen(&mut self, _: &mut Fid<Self::Fid>, _flags: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rlcreate(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rsymlink(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _sym: &str, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rmknod(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _mode: u32, _major: u32, _minor: u32, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rrename(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rreadlink(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rgetattr(&mut self, _: &mut Fid<Self::Fid>, _req_mask: u64)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rsetattr(&mut self, _: &mut Fid<Self::Fid>, _valid: u32, _stat: &Stat)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rxattrwalk(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rxattrcreate(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _attr_size: u64, _flags: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rreaddir(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rfsync(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Flock, _client_id: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rgetlock(&mut self, _: &mut Fid<Self::Fid>, _lock: &Flock, _client_id: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rlink(&mut self, _: &mut Fid<Self::Fid>, _: &mut Fid<Self::Fid>, _name: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rmkdir(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _mode: u32, _gid: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rrenameat(&mut self, _: &mut Fid<Self::Fid>, _oldname: &str, _: &mut Fid<Self::Fid>, _newname: &str)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn runlinkat(&mut self, _: &mut Fid<Self::Fid>, _name: &str, _flags: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }

    // 9P2000.u subset
    fn rauth(&mut self, _: &mut Fid<Self::Fid>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rattach(&mut self, _: &mut Fid<Self::Fid>, _afid: Option<&mut Fid<Self::Fid>>, _uname: &str, _aname: &str, _n_uname: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }

    // 9P2000 subset
    fn rflush(&mut self, _old: Option<&mut Fcall>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rwalk(&mut self, _: &mut Fid<Self::Fid>, _new: &mut Fid<Self::Fid>, _wnames: &[String])
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rread(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _count: u32)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rwrite(&mut self, _: &mut Fid<Self::Fid>, _offset: u64, _data: &Data)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rclunk(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rremove(&mut self, _: &mut Fid<Self::Fid>)
        -> Result<Fcall> { Err(nix::Error::from_errno(ENOSYS)) }
    fn rversion(&mut self, _msize: u32, _version: &str)      -> Result<Fcall> {
        Ok(Fcall::Rversion {
            msize: 8192,
            version: "9P2000.L".to_owned()
        })
    }
}

struct ServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fs: Arc<Mutex<Fs>>,
    stream: RwExt,
    fids: HashMap<u32, Fid<Fs::Fid>>,
}

macro_rules! lock { ($mtx:expr) => { $mtx.lock().unwrap() } }
impl<Fs, RwExt> ServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt)
        -> io::Result<ServerInstance<Fs, RwExt>>
    {
        let server = ServerInstance {
            fs: fs,
            stream: stream,
            fids: HashMap::new(),
        };
        Ok(server)
    }

    fn dispatch(&mut self) -> io::Result<()> {
        loop {
            try!(self.dispatch_once());
        }
    }

    fn fid(&mut self, fid: &u32) -> Fid<Fs::Fid> {
        self.fids.remove(&fid).unwrap()
    }

    fn newfid(fid: &u32) -> Fid<Fs::Fid> {
        Fid { fid: *fid, qid: None, aux: None }
    }

    fn dispatch_once(&mut self) -> io::Result<MsgType> {
        let msg = try!(serialize::read_msg(&mut self.stream));

        // Take all fids associated with the fids which the request contains
        let mut fids: Vec<_> = msg.body.fid().iter().map(|f| self.fid(f)).collect();
        let mut newfids: Vec<_> = msg.body.newfid().iter().map(|f| Self::newfid(f)).collect();

        let result = match msg.body {
            Fcall::Tstatfs { fid: _ }                                                       => { lock!(self.fs).rstatfs(&mut fids[0]) },
            Fcall::Tlopen { fid: _, ref flags }                                             => { lock!(self.fs).rlopen(&mut fids[0], *flags) },
            Fcall::Tlcreate { fid: _, ref name, ref flags, ref mode, ref gid }              => { lock!(self.fs).rlcreate(&mut fids[0], name, *flags, *mode, *gid) },
            Fcall::Tsymlink { fid: _, ref name, ref symtgt, ref gid }                       => { lock!(self.fs).rsymlink(&mut fids[0], name, symtgt, *gid) },
            Fcall::Tmknod { dfid: _, ref name, ref mode, ref major, ref minor, ref gid }    => { lock!(self.fs).rmknod(&mut fids[0], name, *mode, *major, *minor, *gid) },
            Fcall::Trename { fid: _, dfid: _, ref name }                                    => {
                let (mut fid, mut dfid) = (fids.remove(0), fids.remove(0));
                let r = lock!(self.fs).rrename(&mut fid, &mut dfid, name);
                fids.push(fid); fids.push(dfid);
                r
            },
            Fcall::Treadlink { fid: _ }                                                     => { lock!(self.fs).rreadlink(&mut fids[0]) },
            Fcall::Tgetattr { fid: _, ref req_mask }                                        => { lock!(self.fs).rgetattr(&mut fids[0], *req_mask) },
            Fcall::Tsetattr { fid: _, ref valid, ref stat }                                 => { lock!(self.fs).rsetattr(&mut fids[0], *valid, stat) },
            Fcall::Txattrwalk { fid: _, newfid: _, ref name }                               => { lock!(self.fs).rxattrwalk(&mut fids[0], &mut newfids[0], name) },
            Fcall::Txattrcreate { fid: _, ref name, ref attr_size, ref flags }              => { lock!(self.fs).rxattrcreate(&mut fids[0], name, *attr_size, *flags) },
            Fcall::Treaddir { fid: _, ref offset, ref count }                               => { lock!(self.fs).rreaddir(&mut fids[0], *offset, *count) },
            Fcall::Tfsync { fid: _ }                                                        => { lock!(self.fs).rfsync(&mut fids[0]) },
            Fcall::Tlock { fid: _, ref flock, ref client_id }                               => { lock!(self.fs).rlock(&mut fids[0], flock, client_id) },
            Fcall::Tgetlock { fid: _, ref flock, ref client_id }                            => { lock!(self.fs).rgetlock(&mut fids[0], flock, client_id) },
            Fcall::Tlink { dfid: _, fid: _, ref name }                                      => {
                let (mut dfid, mut fid) = (fids.remove(0), fids.remove(0));
                let r = lock!(self.fs).rlink(&mut dfid, &mut fid, name);
                fids.push(dfid); fids.push(fid);
                r
            },
            Fcall::Tmkdir { dfid: _, ref name, ref mode, ref gid }                          => { lock!(self.fs).rmkdir(&mut fids[0], name, *mode, *gid) },
            Fcall::Trenameat { olddirfid: _, ref oldname, newdirfid: _, ref newname }       => {
                let (mut old, mut new) = (fids.remove(0), fids.remove(0));
                let r = lock!(self.fs).rrenameat(&mut old, oldname, &mut new, newname);
                fids.push(old); fids.push(new);
                r
            },
            Fcall::Tunlinkat { dirfd: _, ref name, ref flags }                              => { lock!(self.fs).runlinkat(&mut fids[0], name, *flags) },

            // 9P2000.u
            Fcall::Tauth { afid: _, ref uname, ref aname, ref n_uname }                     => { lock!(self.fs).rauth(&mut newfids[0], uname, aname, *n_uname) },
            Fcall::Tattach { fid: _, afid: _, ref uname, ref aname, ref n_uname }           => { lock!(self.fs).rattach(&mut newfids[0], None, uname, aname, *n_uname) },

            // 9P2000
            Fcall::Tversion { ref msize, ref version }                                      => { lock!(self.fs).rversion(*msize, version) },
            Fcall::Tflush { oldtag: _ }                                                     => { lock!(self.fs).rflush(None) },
            Fcall::Twalk { fid: _, newfid: _, ref wnames }                                  => { lock!(self.fs).rwalk(&mut fids[0], &mut newfids[0], wnames) },
            Fcall::Tread { fid: _, ref offset, ref count }                                  => { lock!(self.fs).rread(&mut fids[0], *offset, *count) },
            Fcall::Twrite { fid: _, ref offset, ref data }                                  => { lock!(self.fs).rwrite(&mut fids[0], *offset, data) },
            Fcall::Tclunk { fid: _ }                                                        => {
                let r = lock!(self.fs).rclunk(&mut fids[0]);
                // Drop the fid which the request contains
                if r.is_ok() { fids.clear(); }
                r
            },
            Fcall::Tremove { fid: _ }                                                       => { lock!(self.fs).rremove(&mut fids[0]) },
            _ => return io_error!(Other, "Invalid 9P message received"),
        };

        // Restore the fids taken
        for f in fids { self.fids.insert(f.fid, f); }
        for f in newfids { self.fids.insert(f.fid, f); }

        let response = match result {
            Ok(res)  => res,
            Err(err) => Fcall::Rlerror { ecode: err.errno() as u32 }
        };

        try!(self.respond(response, msg.tag));
        Ok(msg.typ)
    }

    fn respond(&mut self, res: Fcall, tag: u16) -> io::Result<MsgType> {
        let msg_type = match res {
            // 9P2000.L
            Fcall::Rlerror { .. }       => MsgType::Rlerror,
            Fcall::Rstatfs { .. }       => MsgType::Rstatfs,
            Fcall::Rlopen { .. }        => MsgType::Rlopen,
            Fcall::Rlcreate { .. }      => MsgType::Rlcreate,
            Fcall::Rsymlink { .. }      => MsgType::Rsymlink,
            Fcall::Rmknod { .. }        => MsgType::Rmknod,
            Fcall::Rrename              => MsgType::Rrename,
            Fcall::Rreadlink { .. }     => MsgType::Rreadlink,
            Fcall::Rgetattr { .. }      => MsgType::Rgetattr,
            Fcall::Rsetattr             => MsgType::Rsetattr,
            Fcall::Rxattrwalk { .. }    => MsgType::Rxattrwalk,
            Fcall::Rxattrcreate         => MsgType::Rxattrcreate,
            Fcall::Rreaddir { .. }      => MsgType::Rreaddir,
            Fcall::Rfsync               => MsgType::Rfsync,
            Fcall::Rlock { .. }         => MsgType::Rlock,
            Fcall::Rgetlock { .. }      => MsgType::Rgetlock,
            Fcall::Rlink                => MsgType::Rlink,
            Fcall::Rmkdir { .. }        => MsgType::Rmkdir,
            Fcall::Rrenameat            => MsgType::Rrenameat,
            Fcall::Runlinkat            => MsgType::Runlinkat,

            // 9P2000.u
            Fcall::Rauth { .. }         => MsgType::Rauth,
            Fcall::Rattach { .. }       => MsgType::Rattach,

            // 9P2000
            Fcall::Rversion { .. }      => MsgType::Rversion,
            Fcall::Rflush               => MsgType::Rflush,
            Fcall::Rwalk { .. }         => MsgType::Rwalk,
            Fcall::Rread { .. }         => MsgType::Rread,
            Fcall::Rwrite { .. }        => MsgType::Rwrite,
            Fcall::Rclunk               => MsgType::Rclunk,
            Fcall::Rremove              => MsgType::Rremove,
            _ => return io_error!(Other, "Invalid 9P message in this context"),
        };

        let msg = Msg { typ: msg_type, tag: tag, body: res };
        try!(serialize::write_msg(&mut self.stream, &msg));

        Ok(msg_type)
    }
}

// return: (proto, addr:port)
fn parse_proto(arg: &str) -> result::Result<(&str, String), ()> {
    let mut split = arg.split("!");
    let proto = try!(split.nth(0).ok_or(()));
    let addr  = try!(split.nth(0).ok_or(()));
    let port  = try!(split.nth(0).ok_or(()));
    Ok((proto, addr.to_owned() + ":" + port))
}

/// Start the 9P filesystem
///
/// This function invokes a new thread to handle its 9P messages
/// when a client connects to the server.
pub fn srv<Fs: Filesystem + 'static>(filesystem: Fs, addr: &str) -> io::Result<()> {
    let (proto, sockaddr) = try!(parse_proto(addr).or(
        io_error!(InvalidInput, "Invalid protocol or address")
    ));

    if proto != "tcp" {
        return io_error!(InvalidInput, "Unsupported protocol");
    }

    let arc_fs = Arc::new(Mutex::new(filesystem));
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, _) = try!(listener.accept());
        let fs = arc_fs.clone();
        let _ = thread::Builder::new().name(format!("{}", addr)).spawn(move || {
            let result = try!(ServerInstance::new(fs, stream)).dispatch();
            println!("[!] ServerThread={:?} finished: {:?}",
                thread::current().name().unwrap_or("NoInfo"), result);
            result
        });
    }
}
