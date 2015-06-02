
//! Server side 9P library

extern crate byteorder;

use error;
use serialize;
use fcall::*;
use std::io;
use std::result;
use std::net::TcpListener;
use self::byteorder::{ReadBytesExt, WriteBytesExt};
use std::thread;
use std::sync::{Mutex, Arc};

pub type Result<T> = result::Result<T, String>;

macro_rules! lock {
    ($mtx:expr) => { $mtx.lock().unwrap() }
}

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
pub struct Request<'a> {
    pub ifcall: &'a MsgBody
}

impl<'a> Request<'a> {
    pub fn from(msg: &'a Msg) -> Request<'a> {
        Request { ifcall: &msg.body }
    }
}

/// Filesystem server implementation
///
/// Return an error message if an operation failed.
///
/// NOTE: Defined as `Srv` in 9p.h of Plan 9.
pub trait Filesystem: Send {
    fn rflush(&mut self, _: &Request)   -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rattach(&mut self, _: &Request)  -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rwalk(&mut self, _: &Request)    -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn ropen(&mut self, _: &Request)    -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rcreate(&mut self, _: &Request)  -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rread(&mut self, _: &Request)    -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rwrite(&mut self, _: &Request)   -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rclunk(&mut self, _: &Request)   -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rremove(&mut self, _: &Request)  -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rstat(&mut self, _: &Request)    -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rwstat(&mut self, _: &Request)   -> Result<MsgBody> { Err(error::ENOSYS.to_owned()) }
    fn rauth(&mut self, _: &Request)    -> Result<MsgBody> { Err(error::ECONNREFUSED2.to_owned()) }
    fn rversion(&mut self, _res: &Request) -> Result<MsgBody> {
        Ok(MsgBody::Rversion {
            msize: 8192,
            version: "9P2000".to_owned()
        })
    }
}

/// Client's 9P message dispatcher
struct ClientDispatcher<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fs: Arc<Mutex<Fs>>,
    stream: RwExt
}

impl<Fs, RwExt>  ClientDispatcher<Fs, RwExt>
    where Fs: Filesystem, RwExt: ReadBytesExt + WriteBytesExt
{
    fn new(fs: Arc<Mutex<Fs>>, stream: RwExt) -> ClientDispatcher<Fs, RwExt> {
        ClientDispatcher {
            fs: fs,
            stream: stream
        }
    }

    fn dispatch(&mut self) -> io::Result<()> {
        loop {
            let msg = try!(serialize::read_msg(&mut self.stream));
            try!(self.handle_message(msg));
        }
    }

    fn handle_message(&mut self, msg: Msg) -> io::Result<()> {
        let result = match msg.typ {
            MsgType::Tversion   => lock!(self.fs).rversion(&Request::from(&msg)),
            MsgType::Tauth      => lock!(self.fs).rauth(&Request::from(&msg)),
            MsgType::Tflush     => lock!(self.fs).rflush(&Request::from(&msg)),
            MsgType::Tattach    => lock!(self.fs).rattach(&Request::from(&msg)),
            MsgType::Twalk      => lock!(self.fs).rwalk(&Request::from(&msg)),
            MsgType::Topen      => lock!(self.fs).ropen(&Request::from(&msg)),
            MsgType::Tcreate    => lock!(self.fs).rcreate(&Request::from(&msg)),
            MsgType::Tread      => lock!(self.fs).rread(&Request::from(&msg)),
            MsgType::Twrite     => lock!(self.fs).rwrite(&Request::from(&msg)),
            MsgType::Tclunk     => lock!(self.fs).rclunk(&Request::from(&msg)),
            MsgType::Tremove    => lock!(self.fs).rremove(&Request::from(&msg)),
            MsgType::Tstat      => lock!(self.fs).rstat(&Request::from(&msg)),
            MsgType::Twstat     => lock!(self.fs).rwstat(&Request::from(&msg)),
            _ => Err(error::EPROTO.to_owned()),
        };

        let res_body = match result {
            Ok(response) => response,
            Err(err) => MsgBody::Rerror { ename: err }
        };

        self.response(res_body, msg.tag)
    }

    fn response(&mut self, res: MsgBody, tag: u16) -> io::Result<()> {
        let typ = match &res {
            &MsgBody::Rversion { msize: _, version: _ } => MsgType::Rversion,
            &MsgBody::Rauth { aqid: _ }                 => MsgType::Rauth,
            &MsgBody::Rerror { ename: _ }               => MsgType::Rerror,
            &MsgBody::Rflush                            => MsgType::Rflush,
            &MsgBody::Rattach { qid: _ }                => MsgType::Rattach,
            &MsgBody::Rwalk { wqids: _ }                => MsgType::Rwalk,
            &MsgBody::Ropen { qid: _, iounit: _ }       => MsgType::Ropen,
            &MsgBody::Rcreate { qid: _, iounit: _ }     => MsgType::Rcreate,
            &MsgBody::Rread { data: _ }                 => MsgType::Rread,
            &MsgBody::Rwrite { count: _ }               => MsgType::Rwrite,
            &MsgBody::Rclunk                            => MsgType::Rclunk,
            &MsgBody::Rremove                           => MsgType::Rremove,
            &MsgBody::Rstat { stat: _ }                 => MsgType::Rstat,
            &MsgBody::Rwstat                            => MsgType::Rwstat,
            _ => return io_error!(Other, "Try to send invalid message in this context"),
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
        return io_error!(InvalidInput, "Unsupported proto");
    }

    let arc_fs = Arc::new(Mutex::new(filesystem));
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    loop {
        let (stream, _) = try!(listener.accept());
        let fs = arc_fs.clone();
        thread::spawn(move || {
            let mut dispatcher = ClientDispatcher::new(fs, stream);
            dispatcher.dispatch()
        });
    }
}
