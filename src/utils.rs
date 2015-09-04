
extern crate net2;
extern crate byteorder;

use std::net::TcpStream;
use self::byteorder::WriteBytesExt;
use self::net2::TcpStreamExt;

use fcall::*;
use error;
use serialize;

pub type Result<T> = ::std::result::Result<T, error::Error>;

#[macro_export]
macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(::std::io::Error::new(::std::io::ErrorKind::$kind, $msg))
    }
}

// return: (proto, addr:port)
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

pub fn respond<WExt: WriteBytesExt>(stream: &mut WExt, res: Fcall, tag: u16) -> Result<MsgType> {
    let msg_type = MsgType::from(&res);
    if msg_type.is_t() {
        return try!(io_error!(Other, "Invalid 9P message in this context"));
    };

    let msg = Msg { tag: tag, body: res };
    try!(serialize::write_msg(stream, &msg));

    Ok(msg_type)
}
