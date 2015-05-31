
//! Server side 9P library

extern crate byteorder;

use error;
use serialize;
use fcall::*;
use std::io;
use std::result;
use std::net::{TcpListener, TcpStream};
use self::byteorder::{ReadBytesExt, WriteBytesExt};

pub type Result<T> = result::Result<T, String>;

macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(io::Error::new(io::ErrorKind::$kind, $msg))
    }
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
pub trait Filesystem {
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
}

// return: (proto, addr:port)
fn parse_proto(arg: &str) -> result::Result<(&str, String), ()> {
    let mut split = arg.split("!");
    let proto = try!(split.nth(0).ok_or(()));
    let addr  = try!(split.nth(0).ok_or(()));
    let port  = try!(split.nth(0).ok_or(()));
    Ok((proto, addr.to_owned() + ":" + port))
}

/// 9P network server implementation
pub struct Server<Fs: Filesystem> {
    fs: Fs,
    listener: TcpListener
}

impl<Fs: Filesystem> Server<Fs> {
    /// Create a server instance
    ///
    /// Announce the network server
    pub fn announce(fs: Fs, addr: &str) -> io::Result<Server<Fs>> {
        let (proto, sockaddr) = try!(parse_proto(addr).or(
            io_error!(InvalidInput, "Invalid proto or address")
        ));

        if proto != "tcp" {
            return io_error!(InvalidInput, "Unsupported proto");
        }

        Ok(Server {
            fs: fs,
            listener: try!(TcpListener::bind(&sockaddr[..]))
        })
    }

    /// Start the 9P filesystem server
    pub fn srv(&mut self) -> io::Result<()> {
        let (stream, _) = try!(self.listener.accept());
        self.handle_client(stream)
    }

    fn handle_client(&mut self, mut stream: TcpStream) -> io::Result<()> {
        loop {
            let msg = try!(serialize::read_msg(&mut stream));
            try!(self.handle_message(msg, &mut stream));
        }
    }

    fn handle_message<Rw>(&mut self, msg: Msg, stream: &mut Rw) -> io::Result<()>
        where Rw: WriteBytesExt + ReadBytesExt
    {
        println!("[*] Message received: {:?}", msg);

        let result = match msg.typ {
            MsgType::Tversion   => self.rversion(&Request::from(&msg)),
            MsgType::Tauth      => Err(error::ECONNREFUSED2.to_owned()),
            MsgType::Tflush     => self.fs.rflush(&Request::from(&msg)),
            MsgType::Tattach    => self.fs.rattach(&Request::from(&msg)),
            MsgType::Twalk      => self.fs.rwalk(&Request::from(&msg)),
            MsgType::Topen      => self.fs.ropen(&Request::from(&msg)),
            MsgType::Tcreate    => self.fs.rcreate(&Request::from(&msg)),
            MsgType::Tread      => self.fs.rread(&Request::from(&msg)),
            MsgType::Twrite     => self.fs.rwrite(&Request::from(&msg)),
            MsgType::Tclunk     => self.fs.rclunk(&Request::from(&msg)),
            MsgType::Tremove    => self.fs.rremove(&Request::from(&msg)),
            MsgType::Tstat      => self.fs.rstat(&Request::from(&msg)),
            MsgType::Twstat     => self.fs.rwstat(&Request::from(&msg)),
            _ => Err(error::EPROTO.to_owned()),
        };

        let res_body = match result {
            Ok(response) => response,
            Err(err) => MsgBody::Rerror { ename: err }
        };

        self.response(stream, res_body, msg.tag)
    }

    fn rversion(&self, _res: &Request) -> Result<MsgBody> {
        Ok(MsgBody::Rversion {
            msize: 8192,
            version: "9P2000".to_owned()
        })
    }

    fn response<W: WriteBytesExt>(&self, stream: &mut W, res: MsgBody, tag: u16) -> io::Result<()> {
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

        println!("[*] Sending message: {:?}", response_msg);

        try!(serialize::write_msg(stream, &response_msg));
        Ok(())
    }
}
