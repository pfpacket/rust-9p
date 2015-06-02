
//! Server side 9P library

extern crate byteorder;

use error;
use serialize;
use fcall::*;
use std::io;
use std::result;
use std::net::{SocketAddr, TcpListener};
use self::byteorder::{ReadBytesExt, WriteBytesExt};
use std::thread;
use std::sync::{Mutex, Arc};

pub type Result<T> = result::Result<T, String>;

macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
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

/// The client's request
pub struct Request<'a, 'b> {
    /// The request message which a client sent
    pub ifcall: &'a Fcall,
    /// The socket address of the remote peer
    pub remote: &'b SocketAddr,
}

impl<'a, 'b> Request<'a, 'b> {
    fn from(msg: &'a Msg, addr: &'b SocketAddr) -> Request<'a, 'b> {
        Request { ifcall: &msg.body, remote: addr }
    }
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
    fn rflush(&mut self, _: &Request)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rattach(&mut self, _: &Request)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwalk(&mut self, _: &Request)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn ropen(&mut self, _: &Request)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rcreate(&mut self, _: &Request)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rread(&mut self, _: &Request)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwrite(&mut self, _: &Request)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rclunk(&mut self, _: &Request)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rremove(&mut self, _: &Request)  -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rstat(&mut self, _: &Request)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rwstat(&mut self, _: &Request)   -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rauth(&mut self, _: &Request)    -> Result<Fcall> { Err(error::ENOSYS.to_owned()) }
    fn rversion(&mut self, _: &Request) -> Result<Fcall> {
        Ok(Fcall::Rversion {
            msize: 8192,
            version: "9P2000".to_owned()
        })
    }
}

/// Incoming 9P message dispatcher
struct ClientDispatcher<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fs: Arc<Mutex<Fs>>,
    stream: RwExt,
    sockaddr: SocketAddr,
}

impl<Fs, RwExt>  ClientDispatcher<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt, addr: SocketAddr) -> ClientDispatcher<Fs, RwExt> {
        ClientDispatcher {
            fs: fs,
            stream: stream,
            sockaddr: addr
        }
    }

    fn dispatch(&mut self) -> io::Result<()> {
        loop {
            let msg = try!(serialize::read_msg(&mut self.stream));
            match self.handle_message(msg) {
                Err(byteorder::Error::UnexpectedEOF) => {
                    return io_error!(ConnectionRefused, "Unexpected EOF")
                },
                Err(byteorder::Error::Io(e))=> { return Err(e) },
                Ok(_) => {}
            };
        }
    }

    fn handle_message(&mut self, msg: Msg) -> byteorder::Result<()> {
        macro_rules! lock { ($mtx:expr) => { $mtx.lock().unwrap() } }
        let result = {
            let request = &Request::from(&msg, &self.sockaddr);
            match msg.typ {
                MsgType::Tversion   => lock!(self.fs).rversion(&request),
                MsgType::Tauth      => lock!(self.fs).rauth(&request),
                MsgType::Tflush     => lock!(self.fs).rflush(&request),
                MsgType::Tattach    => lock!(self.fs).rattach(&request),
                MsgType::Twalk      => lock!(self.fs).rwalk(&request),
                MsgType::Topen      => lock!(self.fs).ropen(&request),
                MsgType::Tcreate    => lock!(self.fs).rcreate(&request),
                MsgType::Tread      => lock!(self.fs).rread(&request),
                MsgType::Twrite     => lock!(self.fs).rwrite(&request),
                MsgType::Tclunk     => lock!(self.fs).rclunk(&request),
                MsgType::Tremove    => lock!(self.fs).rremove(&request),
                MsgType::Tstat      => lock!(self.fs).rstat(&request),
                MsgType::Twstat     => lock!(self.fs).rwstat(&request),
                _ => Err(error::EPROTO.to_owned()),
            }
        };

        let res_body = match result {
            Ok(response) => response,
            Err(err) => Fcall::Rerror { ename: err }
        };

        self.response(res_body, msg.tag)
    }

    fn response(&mut self, res: Fcall, tag: u16) -> byteorder::Result<()> {
        let typ = match &res {
            &Fcall::Rversion { msize: _, version: _ }   => MsgType::Rversion,
            &Fcall::Rauth { aqid: _ }                   => MsgType::Rauth,
            &Fcall::Rerror { ename: _ }                 => MsgType::Rerror,
            &Fcall::Rflush                              => MsgType::Rflush,
            &Fcall::Rattach { qid: _ }                  => MsgType::Rattach,
            &Fcall::Rwalk { wqids: _ }                  => MsgType::Rwalk,
            &Fcall::Ropen { qid: _, iounit: _ }         => MsgType::Ropen,
            &Fcall::Rcreate { qid: _, iounit: _ }       => MsgType::Rcreate,
            &Fcall::Rread { data: _ }                   => MsgType::Rread,
            &Fcall::Rwrite { count: _ }                 => MsgType::Rwrite,
            &Fcall::Rclunk                              => MsgType::Rclunk,
            &Fcall::Rremove                             => MsgType::Rremove,
            &Fcall::Rstat { stat: _ }                   => MsgType::Rstat,
            &Fcall::Rwstat                              => MsgType::Rwstat,
            _ => return Err(byteorder::Error::Io(io::Error::new(
                    io::ErrorKind::Other, "Try to send invalid message in this context"))),
        };

        let response_msg = Msg { typ: typ, tag: tag, body: res };

        try!(serialize::write_msg(&mut self.stream, &response_msg));
        Ok(())
    }
}

/// Start the 9P filesystem
///
/// This function invokes a new thread to handle its 9P messages
/// when a client connects to the server.
pub fn srv<Fs: Filesystem + 'static>(filesystem: Fs, addr: &str) -> io::Result<()> {
    let (proto, sockaddr) = try!(parse_proto(addr).or(
        io_error!(InvalidInput, "Invalid proto or address")
    ));

    if proto != "tcp" {
        return io_error!(InvalidInput, "Unsupported protocol");
    }

    let arc_fs = Arc::new(Mutex::new(filesystem));
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, addr) = try!(listener.accept());
        let fs = arc_fs.clone();
        thread::spawn(move || {
            ClientDispatcher::new(fs, stream, addr).dispatch()
        });
    }
}
