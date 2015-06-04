
//! Server side 9P library

extern crate byteorder;

use error;
use serialize;
use fcall::*;
use std::{io, result, fmt, thread};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Mutex, Arc};
use self::byteorder::{ReadBytesExt, WriteBytesExt};

pub type Result<T> = result::Result<T, String>;

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

/// The client's request
#[derive(Clone, Debug)]
pub struct Request<'a, 'b, T> {
    /// The request message which a client sent
    pub ifcall: &'a Fcall,
    /// The socket address of the remote peer
    pub remote: &'b SocketAddr,
    /// Fid associated with the request's fid
    pub fid: Option<Fid<T>>,
    /// New fid associated with the Twalk's newfid
    pub newfid: Option<Fid<T>>,
}

impl<'a, 'b, T> Request<'a, 'b, T> {
    /// Unwrap the fid
    pub fn fid(&mut self) -> &mut Fid<T> { self.fid.as_mut().unwrap() }
    /// Unwrap the newfid
    pub fn newfid(&mut self) -> &mut Fid<T> { self.newfid.as_mut().unwrap() }
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
/// The default implementation of Rversion returns a message accepting 9P2000.
///
/// NOTE: Defined as `Srv` in 9p.h of Plan 9.
pub trait Filesystem: Send {
    /// User defined fid type to be associated with a client's fid
    type Fid: fmt::Debug = ();
    fn rauth(&mut self, _: &mut Request<Self::Fid>)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rflush(&mut self, _: &mut Request<Self::Fid>)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rattach(&mut self, _: &mut Request<Self::Fid>)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwalk(&mut self, _: &mut Request<Self::Fid>)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn ropen(&mut self, _: &mut Request<Self::Fid>)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rcreate(&mut self, _: &mut Request<Self::Fid>)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rread(&mut self, _: &mut Request<Self::Fid>)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwrite(&mut self, _: &mut Request<Self::Fid>)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rclunk(&mut self, _: &mut Request<Self::Fid>)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rremove(&mut self, _: &mut Request<Self::Fid>)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rstat(&mut self, _: &mut Request<Self::Fid>)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwstat(&mut self, _: &mut Request<Self::Fid>)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rversion(&mut self, _: &mut Request<Self::Fid>) -> Result<Fcall> {
        Ok(Fcall::Rversion {
            msize: 8192,
            version: "9P2000".to_owned()
        })
    }
}

struct ServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fs: Arc<Mutex<Fs>>,
    stream: RwExt,
    sockaddr: SocketAddr,
    fids: HashMap<u32, Fid<Fs::Fid>>,
    msize: Option<u32>,
    uname: Option<String>,
    aname: Option<String>,
}

macro_rules! lock { ($mtx:expr) => { $mtx.lock().unwrap() } }
impl<Fs, RwExt> ServerInstance<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt, addr: SocketAddr)
        -> io::Result<ServerInstance<Fs, RwExt>>
    {
        let mut server = ServerInstance {
            fs: fs,
            stream: stream,
            sockaddr: addr,
            fids: HashMap::new(),
            msize: None,
            uname: None,
            aname: None,
        };

        try!(server.dispatch_once());
        if server.msize.is_none() {
            return io_error!(Other, "Unexpected packet before Tversion")
        }

        try!(server.dispatch_once());
        if server.uname.is_none() {
            return io_error!(Other, "Unexpected packet before Tattach")
        }

        Ok(server)
    }

    fn dispatch_once(&mut self) -> io::Result<()> {
        let msg = try!(serialize::read_msg(&mut self.stream));
        match self.handle_message(msg) {
            Ok(v) => Ok(v),
            Err(byteorder::Error::UnexpectedEOF) => {
                return io_error!(ConnectionRefused, "Unexpected EOF")
            },
            Err(byteorder::Error::Io(e))=> { return Err(e) },
        }
    }

    fn dispatch(&mut self) -> io::Result<()> {
        loop {
            try!(self.dispatch_once());
        }
    }

    fn rversion(&mut self, req: &mut Request<Fs::Fid>, msize: u32) -> Result<Fcall> {
        self.msize = Some(msize);
        lock!(self.fs).rversion(req)
    }

    fn rattach(&mut self, req: &mut Request<Fs::Fid>, fid: u32, uname: &String, aname: &String) -> Result<Fcall> {
        self.uname = Some(uname.clone());
        self.aname = Some(aname.clone());
        req.fid = Some(Fid { fid: fid, qid: None, aux: None });
        lock!(self.fs).rattach(req)
    }

    fn rwalk(&mut self, req: &mut Request<Fs::Fid>, newfid: u32) -> Result<Fcall> {
        req.newfid = Some(Fid { fid: newfid, qid: None, aux: None });
        lock!(self.fs).rwalk(req)
    }

    fn rclunk(&mut self, req: &mut Request<Fs::Fid>, fid: u32) -> Result<Fcall> {
        self.fids.remove(&fid);
        lock!(self.fs).rclunk(req)
    }

    fn register_fids(&mut self, mut req: Request<Fs::Fid>) {
        if req.fid.is_some() {
            let fid = req.fid.as_mut().unwrap().fid;
            self.fids.insert(fid, req.fid.unwrap());
        }

        if req.newfid.is_some() {
            let newfid = req.newfid.as_mut().unwrap().fid;
            self.fids.insert(newfid, req.newfid.unwrap());
        }
    }

    fn handle_message(&mut self, msg: Msg) -> byteorder::Result<()> {
        let fid = msg.body.fid().and_then(|f| self.fids.remove(&f));
        let mut req = Request {
            ifcall: &msg.body,
            remote: &self.sockaddr.clone(),
            fid: fid, newfid: None
        };

        let result = match msg.body {
            Fcall::Tversion { msize, .. }                       => self.rversion(&mut req, msize),
            Fcall::Tauth { .. }                                 => lock!(self.fs).rauth(&mut req),
            Fcall::Tflush { .. }                                => lock!(self.fs).rflush(&mut req),
            Fcall::Tattach { fid, ref uname, ref aname, .. }    => self.rattach(&mut req, fid, uname, aname),
            Fcall::Twalk { newfid, .. }                         => self.rwalk(&mut req, newfid),
            Fcall::Topen { .. }                                 => lock!(self.fs).ropen(&mut req),
            Fcall::Tcreate { .. }                               => lock!(self.fs).rcreate(&mut req),
            Fcall::Tread { .. }                                 => lock!(self.fs).rread(&mut req),
            Fcall::Twrite { .. }                                => lock!(self.fs).rwrite(&mut req),
            Fcall::Tremove { .. }                               => lock!(self.fs).rremove(&mut req),
            Fcall::Tclunk { fid }                               => self.rclunk(&mut req, fid),
            Fcall::Tstat { .. }                                 => lock!(self.fs).rstat(&mut req),
            Fcall::Twstat { .. }                                => lock!(self.fs).rwstat(&mut req),
            _ => Err(error::EPROTO.to_owned())
        };

        let res_body = match result {
            Ok(response) => response,
            Err(err) => Fcall::Rerror { ename: err }
        };

        self.register_fids(req);
        self.response(res_body, msg.tag)
    }

    fn response(&mut self, res: Fcall, tag: u16) -> byteorder::Result<()> {
        let typ = match res {
            Fcall::Rversion { .. }  => MsgType::Rversion,
            Fcall::Rauth { .. }     => MsgType::Rauth,
            Fcall::Rerror { .. }    => MsgType::Rerror,
            Fcall::Rflush           => MsgType::Rflush,
            Fcall::Rattach { .. }   => MsgType::Rattach,
            Fcall::Rwalk { .. }     => MsgType::Rwalk,
            Fcall::Ropen { .. }     => MsgType::Ropen,
            Fcall::Rcreate { .. }   => MsgType::Rcreate,
            Fcall::Rread { .. }     => MsgType::Rread,
            Fcall::Rwrite { .. }    => MsgType::Rwrite,
            Fcall::Rclunk           => MsgType::Rclunk,
            Fcall::Rremove          => MsgType::Rremove,
            Fcall::Rstat { .. }     => MsgType::Rstat,
            Fcall::Rwstat           => MsgType::Rwstat,
            _ => return Err(byteorder::Error::Io(io::Error::new(
                    io::ErrorKind::Other, "Try to send invalid message in this context"))),
        };

        let response_msg = Msg { typ: typ, tag: tag, body: res };
        serialize::write_msg(&mut self.stream, &response_msg).and(Ok(()))
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
        let (stream, addr) = try!(listener.accept());
        let fs = arc_fs.clone();
        let _ = thread::Builder::new().name(format!("{}", addr)).spawn(move || {
            let result = try!(ServerInstance::new(fs, stream, addr)).dispatch();
            println!("[!] ServerThread={:?} finished: {:?}",
                thread::current().name().unwrap_or("NoInfo"), result);
            result
        });
    }
}
