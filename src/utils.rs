
extern crate net2;
extern crate byteorder;

use std::net::TcpStream;
use self::byteorder::WriteBytesExt;
use self::net2::TcpStreamExt;

use fcall::*;
use error;
use serialize;

pub type Result<T> = ::std::result::Result<T, error::Error>;

macro_rules! io_err {
    ($kind:ident, $msg:expr) => { ::std::io::Error::new(::std::io::ErrorKind::$kind, $msg) }
}

macro_rules! bo_err {
    ($kind:ident, $msg:expr) => { byteorder::Error::Io(io_err!($kind, $msg)) }
}

macro_rules! res {
    ($err:expr) => { Err(From::from($err)) }
}

pub fn parse_proto(arg: &str) -> ::std::result::Result<(&str, String), ()> {
    let mut split = arg.split("!");
    let proto = try!(split.nth(0).ok_or(()));
    let addr  = try!(split.nth(0).ok_or(()));
    let port  = try!(split.nth(0).ok_or(()));
    Ok((proto, addr.to_owned() + ":" + port))
}

// See also: diod/libdiod/diod_sock.c
pub fn setup_tcp_stream(stream: &TcpStream) -> ::std::io::Result<()> {
    //try!(TcpStreamExt::set_nodelay(stream, true));
    //TcpStreamExt::set_keepalive(stream, Some(Duration::from_secs(120)))
    TcpStreamExt::set_nodelay(stream, true)
}

pub fn respond<WExt: WriteBytesExt>(stream: &mut WExt, body: Fcall, tag: u16) -> Result<MsgType> {
    let msg_type = MsgType::from(&body);
    if msg_type.is_t() {
        return res!(io_err!(Other, "Invalid 9P message in this context"));
    };

    let msg = Msg { tag: tag, body: body };
    try!(serialize::write_msg(stream, &msg));

    debug!("\t← {:?}", msg);

    Ok(msg_type)
}
