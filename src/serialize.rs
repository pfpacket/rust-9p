
//! Serialize/deserialize 9P messages into/from binary

extern crate num;
extern crate byteorder;

use fcall::*;
use std::mem;
use self::num::FromPrimitive;
use std::io::{self, Read, Write, Cursor, BufWriter};
use self::byteorder::{Result, LittleEndian, ReadBytesExt, WriteBytesExt};

macro_rules! io_error {
    ($kind:ident, $msg:expr) => {
        Err(byteorder::Error::Io(io::Error::new(io::ErrorKind::$kind, $msg)))
    }
}

fn read_exact<R: Read>(r: &mut R, size: usize) -> Result<Vec<u8>> {
    let mut pos = 0;
    let mut buf = vec![0; size];
    loop {
        let bytes_read = try!(r.read(&mut buf[pos..]));
        pos += bytes_read;
        if pos >= size { break; }
        if bytes_read == 0 {
            return io_error!(Other, "Cannot read specified amount of data");
        }
    }
    assert_eq!(pos, size);
    Ok(buf)
}


/// Trait representing a type which can be serialized into binary
///
/// Returns the number of bytes encoded
pub trait Encodable {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize>;
}

impl Encodable for u8 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u8(*self).and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u16 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u16::<LittleEndian>(*self)
            .and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u32 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u32::<LittleEndian>(*self)
            .and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u64 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u64::<LittleEndian>(*self)
            .and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for String {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = try!{ (self.len() as u16).encode(w) };
        bytes += try!(w.write_all(self.as_bytes()).and(Ok(self.len())));
        Ok(bytes)
    }
}

impl Encodable for Qid {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = try!(self.typ.encode(w));
        bytes += try!(self.version.encode(w));
        bytes += try!(self.path.encode(w));
        Ok(bytes)
    }
}

impl Encodable for Stat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let stat_size: u16 = self.size();
        try!((stat_size + 2).encode(w));
        try!(stat_size.encode(w));
        try!(self.typ.encode(w));
        try!(self.dev.encode(w));
        try!(self.qid.encode(w));
        try!(self.mode.encode(w));
        try!(self.atime.encode(w));
        try!(self.mtime.encode(w));
        try!(self.length.encode(w));
        try!(self.name.encode(w));
        try!(self.uid.encode(w));
        try!(self.gid.encode(w));
        try!(self.muid.encode(w));
        Ok(stat_size as usize + 4)
    }
}

impl Encodable for Data {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let size = self.data().len();
        let bytes = try!((size as u32).encode(w)) + size;
        try!(w.write_all(self.data()));
        Ok(bytes)
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = try!((self.len() as u16).encode(w));
        for ref s in self {
            bytes += try!(s.encode(w))
        }
        Ok(bytes)
    }
}

impl Encodable for Msg {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut buf: Vec<u8> = Vec::new();

        macro_rules! encode {
            ( $encoder:expr, $( $x:expr ),* ) => {
                $( let _ = try!($x.encode(&mut $encoder)); )*
            }
        }

        encode!(buf, &(self.typ as u8));
        encode!(buf, &self.tag);
        match self.body {
            Fcall::Tversion { ref msize, ref version }                  => { encode!(buf, msize, version); },
            Fcall::Rversion { ref msize, ref version }                  => { encode!(buf, msize, version); },
            Fcall::Tauth { ref afid, ref uname, ref aname }             => { encode!(buf, afid, uname, aname); },
            Fcall::Rauth { ref aqid }                                   => { encode!(buf, aqid); },
            Fcall::Rerror { ref ename }                                 => { encode!(buf, ename); },
            Fcall::Tflush { ref oldtag }                                => { encode!(buf, oldtag); },
            Fcall::Rflush                                               => {},
            Fcall::Tattach { ref fid, ref afid, ref uname, ref aname }  => { encode!(buf, fid, afid, uname, aname); }
            Fcall::Rattach { ref qid }                                  => { encode!(buf, qid); },
            Fcall::Twalk { ref fid, ref newfid, ref wnames }            => { encode!(buf, fid, newfid, wnames); },
            Fcall::Rwalk { ref wqids }                                  => { encode!(buf, wqids); },
            Fcall::Topen { ref fid, ref mode }                          => { encode!(buf, fid, mode); },
            Fcall::Ropen { ref qid, ref iounit }                        => { encode!(buf, qid, iounit); },
            Fcall::Tcreate { ref fid, ref name, ref perm, ref mode }    => { encode!(buf, fid, name, perm, mode); },
            Fcall::Rcreate { ref qid, ref iounit }                      => { encode!(buf, qid, iounit); },
            Fcall::Tread { ref fid, ref offset, ref count }             => { encode!(buf, fid, offset, count); },
            Fcall::Rread { ref data }                                   => { encode!(buf, data); },
            Fcall::Twrite { ref fid, ref offset, ref data }             => { encode!(buf, fid, offset, data); },
            Fcall::Rwrite { ref count }                                 => { encode!(buf, count); },
            Fcall::Tclunk { ref fid }                                   => { encode!(buf, fid); },
            Fcall::Rclunk                                               => {},
            Fcall::Tremove { ref fid }                                  => { encode!(buf, fid); },
            Fcall::Rremove                                              => {},
            Fcall::Tstat { ref fid }                                    => { encode!(buf, fid); },
            Fcall::Rstat { ref stat }                                   => { encode!(buf, stat); },
            Fcall::Twstat { ref fid, ref stat }                         => { encode!(buf, fid, stat); },
            Fcall::Rwstat                                               => {},
        };

        let size = mem::size_of::<u32>() + buf.len();
        let mut stream = BufWriter::new(w);
        try!(stream.write_u32::<LittleEndian>(size as u32));
        try!(stream.write_all(&buf));
        Ok(size)
    }
}


/// Trait representing a type which can be deserialized from binary
pub trait Decodable {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self>;
}

impl Decodable for u8 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        r.read_u8()
    }
}

impl Decodable for u16 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        r.read_u16::<LittleEndian>()
    }
}

impl Decodable for u32 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        r.read_u32::<LittleEndian>()
    }
}

impl Decodable for u64 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        r.read_u64::<LittleEndian>()
    }
}

impl Decodable for String {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len: u16 = try!(Decodable::decode(r));
        let buf = try!(read_exact(r, len as usize));
        String::from_utf8(buf).or(io_error!(Other, "Invalid UTF-8 sequence"))
    }
}

impl Decodable for Qid {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Qid {
            typ:     try!(Decodable::decode(r)),
            version: try!(Decodable::decode(r)),
            path:    try!(Decodable::decode(r))
        })
    }
}

impl Decodable for Stat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let _: u16 = try!(Decodable::decode(r));
        let _: u16 = try!(Decodable::decode(r));
        Ok(Stat {
            typ: try!(Decodable::decode(r)),    dev: try!(Decodable::decode(r)),
            qid: try!(Decodable::decode(r)),    mode: try!(Decodable::decode(r)),
            atime: try!(Decodable::decode(r)),  mtime: try!(Decodable::decode(r)),
            length: try!(Decodable::decode(r)),
            name: try!(Decodable::decode(r)),   uid: try!(Decodable::decode(r)),
            gid: try!(Decodable::decode(r)),    muid: try!(Decodable::decode(r))
        })
    }
}

impl Decodable for Data {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len: u32 = try!(Decodable::decode(r));
        let buf = try!(read_exact(r, len as usize));
        Ok(Data::new(buf))
    }
}

impl<T: Decodable> Decodable for Vec<T> {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len: u16 = try!(Decodable::decode(r));
        let mut buf = Vec::new();
        for _ in 0..len {
            buf.push(try!(Decodable::decode(r)));
        }
        Ok(buf)
    }
}

impl Decodable for Msg {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let size = try!(r.read_u32::<LittleEndian>()) - 4;
        let mut buf = Cursor::new(try!(read_exact(r, size as usize)));

        macro_rules! decode {
            ($decoder:expr) => {
                try!(Decodable::decode(&mut $decoder))
            }
        }

        let msg_type = MsgType::from_u8(decode!(buf));
        let tag = decode!(buf);
        let body = match msg_type {
            Some(MsgType::Tversion) => Fcall::Tversion { msize: decode!(buf), version: decode!(buf) },
            Some(MsgType::Rversion) => Fcall::Rversion { msize: decode!(buf), version: decode!(buf) },
            Some(MsgType::Tauth)    => Fcall::Tauth { afid: decode!(buf), uname: decode!(buf), aname: decode!(buf) },
            Some(MsgType::Rauth)    => Fcall::Rauth { aqid: decode!(buf) },
            Some(MsgType::Rerror)   => Fcall::Rerror { ename: decode!(buf) },
            Some(MsgType::Tflush)   => Fcall::Tflush { oldtag: decode!(buf) },
            Some(MsgType::Rflush)   => Fcall::Rflush,
            Some(MsgType::Tattach)  => Fcall::Tattach { fid: decode!(buf), afid: decode!(buf), uname: decode!(buf), aname: decode!(buf) },
            Some(MsgType::Rattach)  => Fcall::Rattach { qid: decode!(buf) },
            Some(MsgType::Twalk)    => Fcall::Twalk { fid: decode!(buf), newfid: decode!(buf), wnames: decode!(buf) },
            Some(MsgType::Rwalk)    => Fcall::Rwalk { wqids: decode!(buf) },
            Some(MsgType::Topen)    => Fcall::Topen { mode: decode!(buf), fid: decode!(buf) },
            Some(MsgType::Ropen)    => Fcall::Ropen { qid: decode!(buf), iounit: decode!(buf) },
            Some(MsgType::Tcreate)  => Fcall::Tcreate { fid: decode!(buf), name: decode!(buf), perm: decode!(buf), mode: decode!(buf) },
            Some(MsgType::Rcreate)  => Fcall::Rcreate { iounit: decode!(buf), qid: decode!(buf) },
            Some(MsgType::Tread)    => Fcall::Tread { fid: decode!(buf), offset: decode!(buf), count: decode!(buf) },
            Some(MsgType::Rread)    => Fcall::Rread { data: decode!(buf) },
            Some(MsgType::Twrite)   => Fcall::Twrite { fid: decode!(buf), offset: decode!(buf), data: decode!(buf) },
            Some(MsgType::Rwrite)   => Fcall::Rwrite { count: decode!(buf) },
            Some(MsgType::Tclunk)   => Fcall::Tclunk { fid: decode!(buf) },
            Some(MsgType::Rclunk)   => Fcall::Rclunk,
            Some(MsgType::Tremove)  => Fcall::Tremove { fid: decode!(buf) },
            Some(MsgType::Rremove)  => Fcall::Rremove,
            Some(MsgType::Tstat)    => Fcall::Tstat { fid: decode!(buf) },
            Some(MsgType::Rstat)    => Fcall::Rstat { stat: decode!(buf) },
            Some(MsgType::Twstat)   => Fcall::Twstat { fid: decode!(buf), stat: decode!(buf) },
            Some(MsgType::Rwstat)   => Fcall::Rwstat,
            Some(MsgType::Terror) | None =>
                return io_error!(Other, "Invalid message type")
        };

        Ok(Msg { typ: msg_type.unwrap(), tag: tag, body: body })
    }
}

/// 9P message encoder
///
/// Helper class to serialize various data types in 9P messages into binary
#[derive(Clone, Debug)]
pub struct MsgEncoder {
    data: Vec<u8>
}

impl MsgEncoder {
    pub fn new() -> MsgEncoder {
        MsgEncoder { data: Vec::new() }
    }

    pub fn get_ref(&self) -> &[u8] {
        &self.data[..]
    }

    pub fn encode<T: Encodable,>(&mut self, data: &T) -> Result<usize> {
        data.encode(&mut self.data)
    }
}

/// 9P message decoder
///
/// Helper class to deserialize various data types in 9P messages from binary
#[derive(Clone, Debug)]
pub struct MsgDecoder {
    data: Cursor<Vec<u8>>
}

impl MsgDecoder {
    pub fn new(data: Vec<u8>) -> MsgDecoder {
        MsgDecoder { data: Cursor::new(data) }
    }

    pub fn decode<T: Decodable>(&mut self) -> Result<T> {
        Decodable::decode(&mut self.data)
    }
}

/// Helper function to read a 9P message from a byte-oriented stream
pub fn read_msg<R: ReadBytesExt>(r: &mut R) -> Result<Msg> {
    Decodable::decode(r)
}

/// Helper function to write a 9P message into a byte-oriented stream
pub fn write_msg<W: WriteBytesExt>(w: &mut W, msg: &Msg) -> Result<usize> {
    msg.encode(w)
}


#[test]
fn encoder_test1() {
    let expected: Vec<u8> = (0..10).collect();
    let mut encoder = MsgEncoder::new();
    for i in 0..10 {
        encoder.encode(&(i as u8)).unwrap();
    }
    assert_eq!(expected, encoder.get_ref());
}

#[test]
fn decoder_test1() {
    let expected: Vec<u8> = (0..10).collect();
    let mut decoder = MsgDecoder::new(expected.clone());
    let mut actual: Vec<u8> = Vec::new();
    loop {
        match decoder.decode() {
            Ok(i) => actual.push(i),
            Err(_) => break
        }
    }
    assert_eq!(expected, actual);
}

#[test]
fn msg_encode_decode1() {
    let expected = Msg {
        typ: MsgType::Rversion,
        tag: 0xdead,
        body: Fcall::Rversion {
            msize: 40,
            version: "9P2000".to_owned()
        }
    };
    let mut buf = Vec::new();
    let _ = expected.encode(&mut buf);

    let mut readbuf = Cursor::new(buf);
    let actual = Decodable::decode(&mut readbuf);

    assert_eq!(expected, actual.unwrap());
}

#[test]
fn serialize_rstat() {
    use std::fs;
    use std::path;
    use std::env;
    use std::os::unix::fs::MetadataExt;

    let path = path::Path::new("/tmp");
    let attr = fs::metadata(path).unwrap();
    let raw_attr = attr.as_raw();
    let mut mode = raw_attr.mode() & 0o777;
    if attr.is_dir() { mode |= dm::DIR }
    let qid_type = if attr.is_dir() {
        qt::DIR
    } else {
        qt::FILE
    };

    let stat = Stat {
        typ: 0,
        dev: raw_attr.dev() as u32,
        qid: Qid {
            typ: qid_type,
            version: 0,
            path: raw_attr.ino(),
        },
        mode: mode,
        atime: raw_attr.atime() as u32,
        mtime: raw_attr.mtime() as u32,
        length: raw_attr.size() as u64,
        name: path.file_name().unwrap().to_str().unwrap().to_owned(),
        uid: env::var("USER").unwrap(),
        gid: env::var("USER").unwrap(),
        muid: env::var("USER").unwrap(),
    };

    let expected = Msg {
        typ: MsgType::Rstat,
        tag: 1,
        body: Fcall::Rstat { stat: stat }
    };

    let mut buf = Vec::new();
    let _ = expected.encode(&mut buf);

    let mut readbuf = Cursor::new(buf);
    let actual = Decodable::decode(&mut readbuf);

    assert_eq!(expected, actual.unwrap());
}

//#[test]
//fn recv() {
//    use std::net::*;
//
//    println!("Waiting for a connection...");
//
//    let listener = TcpListener::bind("127.0.0.1:55555").unwrap();
//    let (mut stream, _) = listener.accept().unwrap();
//
//    let tversion = read_msg(&mut stream);
//    let tversion_msg = tversion.unwrap();
//    println!("Client Tversion: {:?}", tversion_msg);
//
//    let rversion = Msg {
//        typ: MsgType::Rversion,
//        tag: tversion_msg.tag,
//        body: Fcall::Rversion {
//            msize: 8192,
//            version: "9P2000".to_owned()
//        }
//    };
//
//    println!("Send   Rversion: {:?}", rversion);
//    let _ = write_msg(&mut stream, &rversion);
//
//    let tattach = read_msg(&mut stream);
//    let tattach_msg = tattach.unwrap();
//    println!("Client Tattach: {:?}", tattach_msg);
//
//    let rattach = Msg {
//        typ: MsgType::Rattach,
//        tag: tattach_msg.tag,
//        body: Fcall::Rattach {
//            qid: Qid {
//                typ: qt::DIR,
//                version: 1,
//                path: 1
//            }
//        }
//    };
//
//    println!("Send   Rattach: {:?}", rattach);
//    let _ = write_msg(&mut stream, &rattach);
//
//    let tstat = read_msg(&mut stream);
//    println!("Client Tstat: {:?}", tstat);
//}
