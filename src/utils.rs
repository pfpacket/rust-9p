
extern crate byteorder;

use std::net::TcpStream;
use self::byteorder::WriteBytesExt;

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
    try!(stream.set_nodelay(true));
    stream.set_keepalive(Some(120))
}

pub fn respond<WExt: WriteBytesExt>(stream: &mut WExt, res: Fcall, tag: u16) -> Result<MsgType> {
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
        _ => return try!(io_error!(Other, "Invalid 9P message in this context")),
    };

    let msg = Msg { typ: msg_type, tag: tag, body: res };
    try!(serialize::write_msg(stream, &msg));

    Ok(msg_type)
}
