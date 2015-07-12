
//! Serialize/deserialize 9P messages into/from binary

extern crate num;
extern crate byteorder;

use fcall::*;
use std::mem;
use std::ops::{Shl, Shr};
use std::io::{self, Read, Write, Cursor};
use self::num::FromPrimitive;
use self::byteorder::{Error, Result, LittleEndian, ReadBytesExt, WriteBytesExt};

macro_rules! bo_io_error {
    ($kind:ident, $msg:expr) => {
        Err(byteorder::Error::Io(io::Error::new(io::ErrorKind::$kind, $msg)))
    }
}

macro_rules! decode {
    ($decoder:expr) => {
        try!(Decodable::decode(&mut $decoder))
    }
}

macro_rules! decode_trunc {
    ($typ:ident, $buf:expr) => { $typ::from_bits_truncate(decode!($buf)) }
}

// Create an unintialized buffer
// Safe to use only for writing data to it
fn create_buffer(size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    unsafe { buffer.set_len(size); }
    buffer
}

fn read_exact<R: Read + ?Sized>(r: &mut R, size: usize) -> Result<Vec<u8>> {
    let mut buf = create_buffer(size);
    read_full(r, &mut buf[..]).and(Ok(buf))
}

fn read_full<R: Read + ?Sized>(r: &mut R, buf: &mut [u8]) -> Result<()> {
    let mut nread = 0usize;
    while nread < buf.len() {
        match r.read(&mut buf[nread..]) {
            Ok(0) => return Err(Error::UnexpectedEOF),
            Ok(n) => nread += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
            Err(e) => return Err(From::from(e))
        }
    }
    Ok(())
}

/// A serializing specific result to overload operators on Result
pub struct SResult<T>(::std::result::Result<T, Error>);

/// A macro to try! `SResult`
#[macro_export]
macro_rules! stry { ($sres:expr) => { try!($sres.0) } }

macro_rules! etry {
    ($res:expr) => {
        match $res {
            Ok(v) => v,
            Err(e) => return ::serialize::SResult(Err(From::from(e))),
        }
    }
}

/// A wrapper class of WriteBytesExt to provide operator overloads
/// for serializing
///
/// Operator '<<' serializes the right hand side argument into
/// the left hand side encoder
#[derive(Clone, Debug)]
pub struct Encoder<W> {
    writer: W,
    bytes: usize
}

impl<W: WriteBytesExt> Encoder<W> {
    pub fn new(writer: W) -> Encoder<W> { Encoder { writer: writer, bytes: 0 } }
    /// Return total bytes written
    pub fn bytes_written(&self) -> usize { self.bytes }
    /// Encode data, equivalent to: decoder << data
    pub fn encode<T: Encodable,>(&mut self, data: &T) -> Result<usize> {
        let bytes = try!(data.encode(&mut self.writer));
        self.bytes += bytes;
        Ok(bytes)
    }
    /// Get inner writer
    pub fn into_inner(self) -> W { self.writer }
}

impl<'a, T: Encodable, W: WriteBytesExt> Shl<&'a T> for Encoder<W> {
    type Output = SResult<Encoder<W>>;
    fn shl(mut self, rhs: &'a T) -> Self::Output {
        etry!(self.encode(rhs));
        SResult(Ok(self))
    }
}

impl<'a, T: Encodable, W: WriteBytesExt> Shl<&'a T> for SResult<Encoder<W>> {
    type Output = Self;
    fn shl(self, rhs: &'a T) -> Self::Output {
        let mut encoder = etry!(self.0);
        etry!(encoder.encode(rhs));
        SResult(Ok(encoder))
    }
}

/// A wrapper class of ReadBytesExt to provide operator overloads
/// for deserializing
#[derive(Clone, Debug)]
pub struct Decoder<R> {
    reader: R,
}

impl<R: ReadBytesExt> Decoder<R> {
    pub fn new(reader: R) -> Decoder<R> { Decoder { reader: reader } }
    pub fn decode<T: Decodable,>(&mut self) -> Result<T> {
        Decodable::decode(&mut self.reader)
    }
    /// Get inner reader
    pub fn into_inner(self) -> R { self.reader }
}

impl<'a, T: Decodable, R: ReadBytesExt> Shr<&'a mut T> for Decoder<R> {
    type Output = SResult<Decoder<R>>;
    fn shr(mut self, rhs: &'a mut T) -> Self::Output {
        *rhs = etry!(self.decode());
        SResult(Ok(self))
    }
}

impl<'a, T: Decodable, R: ReadBytesExt> Shr<&'a mut T> for SResult<Decoder<R>> {
    type Output = Self;
    fn shr(self, rhs: &'a mut T) -> Self::Output {
        let mut decoder = etry!(self.0);
        *rhs = etry!(decoder.decode());
        SResult(Ok(decoder))
    }
}

/// Trait representing a type which can be serialized into binary
pub trait Encodable {
    /// Encode self to w and returns the number of bytes encoded
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize>;
}

impl Encodable for u8 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u8(*self).and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u16 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u16::<LittleEndian>(*self).and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u32 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u32::<LittleEndian>(*self).and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for u64 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u64::<LittleEndian>(*self).and(Ok(mem::size_of::<Self>()))
    }
}

impl Encodable for String {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = try!((self.len() as u16).encode(w));
        bytes += try!(w.write_all(self.as_bytes()).and(Ok(self.len())));
        Ok(bytes)
    }
}

impl Encodable for Qid {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(
            Encoder::new(w) << &self.typ.bits() << &self.version << &self.path
        ).bytes_written())
    }
}

impl Encodable for Statfs {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.typ << &self.bsize << &self.blocks
            << &self.bfree << &self.bavail << &self.files
            << &self.ffree << &self.fsid << &self.namelen
        ).bytes_written())
    }
}

impl Encodable for Time {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(
            Encoder::new(w) << &self.sec << &self.nsec
        ).bytes_written())
    }
}

impl Encodable for Stat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.mode << &self.uid << &self.gid
            << &self.nlink << &self.rdev << &self.size
            << &self.blksize << &self.blocks << &self.atime
            << &self.mtime << &self.ctime
        ).bytes_written())
    }
}

impl Encodable for SetAttr {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.mode << &self.uid << &self.gid
            << &self.size << &self.atime << &self.mtime
        ).bytes_written())
    }
}

impl Encodable for DirEntry {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.qid << &self.offset << &self.typ << &self.name
        ).bytes_written())
    }
}

impl Encodable for DirEntryData {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(self.data().iter()
            .fold(Encoder::new(w) << &self.size(), |acc, e| acc << e)
        ).bytes_written())
    }
}

impl Encodable for Data {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let size = self.0.len();
        let bytes = try!((size as u32).encode(w)) + size;
        try!(w.write_all(&self.0));
        Ok(bytes)
    }
}

impl Encodable for Flock {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.typ.bits() << &self.flags.bits() << &self.start
            << &self.length << &self.proc_id << &self.client_id
        ).bytes_written())
    }
}

impl Encodable for Getlock {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(Encoder::new(w)
            << &self.typ.bits() << &self.start << &self.length
            << &self.proc_id << &self.client_id
        ).bytes_written())
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        Ok(stry!(self.iter()
            .fold(Encoder::new(w) << &(self.len() as u16), |acc, s| acc << s)
        ).bytes_written())
    }
}

impl Encodable for Msg {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let buf = Encoder::new(Vec::with_capacity(8196)) << &(self.typ as u8) << &self.tag;
        let buf = stry!(match self.body {
            // 9P2000.L
            Fcall::Rlerror { ref ecode }                                                    => { buf << ecode },
            Fcall::Tstatfs { ref fid }                                                      => { buf << fid },
            Fcall::Rstatfs { ref statfs }                                                   => { buf << statfs },
            Fcall::Tlopen { ref fid, ref flags }                                            => { buf << fid << flags },
            Fcall::Rlopen { ref qid, ref iounit }                                           => { buf << qid << iounit },
            Fcall::Tlcreate { ref fid, ref name, ref flags, ref mode, ref gid }             => { buf << fid << name << flags << mode << gid },
            Fcall::Rlcreate { ref qid, ref iounit }                                         => { buf << qid << iounit },
            Fcall::Tsymlink { ref fid, ref name, ref symtgt, ref gid }                      => { buf << fid << name << symtgt << gid },
            Fcall::Rsymlink { ref qid }                                                     => { buf << qid },
            Fcall::Tmknod { ref dfid, ref name, ref mode, ref major, ref minor, ref gid }   => { buf << dfid << name << mode << major << minor << gid },
            Fcall::Rmknod { ref qid }                                                       => { buf << qid },
            Fcall::Trename { ref fid, ref dfid, ref name }                                  => { buf << fid << dfid << name },
            Fcall::Rrename                                                                  => { buf },
            Fcall::Treadlink { ref fid }                                                    => { buf << fid },
            Fcall::Rreadlink { ref target }                                                 => { buf << target },
            Fcall::Tgetattr { ref fid, ref req_mask }                                       => { buf << fid << &req_mask.bits() },
            Fcall::Rgetattr { ref valid, ref qid, ref stat }                                => { buf << &valid.bits() << qid << stat << &0u64 << &0u64 << &0u64 << &0u64 },
            Fcall::Tsetattr { ref fid, ref valid, ref stat }                                => { buf << fid << &valid.bits() << stat },
            Fcall::Rsetattr                                                                 => { buf },
            Fcall::Txattrwalk { ref fid, ref newfid, ref name }                             => { buf << fid << newfid << name },
            Fcall::Rxattrwalk { ref size }                                                  => { buf << size },
            Fcall::Txattrcreate { ref fid, ref name, ref attr_size, ref flags }             => { buf << fid << name << attr_size << flags },
            Fcall::Rxattrcreate                                                             => { buf },
            Fcall::Treaddir { ref fid, ref offset, ref count }                              => { buf << fid << offset << count },
            Fcall::Rreaddir { ref data }                                                    => { buf << data },
            Fcall::Tfsync { ref fid }                                                       => { buf << fid },
            Fcall::Rfsync                                                                   => { buf },
            Fcall::Tlock { ref fid, ref flock }                                             => { buf << fid << flock  },
            Fcall::Rlock { ref status }                                                     => { buf << &status.bits() },
            Fcall::Tgetlock { ref fid, ref flock }                                          => { buf << fid << flock },
            Fcall::Rgetlock { ref flock }                                                   => { buf << flock },
            Fcall::Tlink { ref dfid, ref fid, ref name }                                    => { buf << dfid << fid << name },
            Fcall::Rlink                                                                    => { buf },
            Fcall::Tmkdir { ref dfid, ref name, ref mode, ref gid }                         => { buf << dfid << name << mode << gid },
            Fcall::Rmkdir { ref qid }                                                       => { buf << qid },
            Fcall::Trenameat { ref olddirfid, ref oldname, ref newdirfid, ref newname }     => { buf << olddirfid << oldname << newdirfid << newname },
            Fcall::Rrenameat                                                                => { buf },
            Fcall::Tunlinkat { ref dirfd, ref name, ref flags }                             => { buf << dirfd << name << flags },
            Fcall::Runlinkat                                                                => { buf },

            // 9P2000.u
            Fcall::Tauth { ref afid, ref uname, ref aname, ref n_uname }                    => { buf << afid << uname << aname << n_uname },
            Fcall::Rauth { ref aqid }                                                       => { buf << aqid },
            Fcall::Tattach { ref fid, ref afid, ref uname, ref aname, ref n_uname }         => { buf << fid << afid << uname << aname << n_uname },
            Fcall::Rattach { ref qid }                                                      => { buf << qid },

            // 9P2000
            Fcall::Tversion { ref msize, ref version }                                      => { buf << msize << version },
            Fcall::Rversion { ref msize, ref version }                                      => { buf << msize << version },
            Fcall::Tflush { ref oldtag }                                                    => { buf << oldtag },
            Fcall::Rflush                                                                   => { buf },
            Fcall::Twalk { ref fid, ref newfid, ref wnames }                                => { buf << fid << newfid << wnames },
            Fcall::Rwalk { ref wqids }                                                      => { buf << wqids },
            Fcall::Tread { ref fid, ref offset, ref count }                                 => { buf << fid << offset << count },
            Fcall::Rread { ref data }                                                       => { buf << data },
            Fcall::Twrite { ref fid, ref offset, ref data }                                 => { buf << fid << offset << data },
            Fcall::Rwrite { ref count }                                                     => { buf << count },
            Fcall::Tclunk { ref fid }                                                       => { buf << fid },
            Fcall::Rclunk                                                                   => { buf },
            Fcall::Tremove { ref fid }                                                      => { buf << fid },
            Fcall::Rremove                                                                  => { buf },
        });

        let mut raw_buf = buf.into_inner();
        let size = mem::size_of::<u32>() + raw_buf.len();

        let size_buf = stry!(Encoder::new(Vec::new()) << &(size as u32)).into_inner();
        for v in size_buf.iter().rev() { raw_buf.insert(0, *v); }

        try!(w.write_all(&raw_buf));
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
        String::from_utf8(buf).or(bo_io_error!(Other, "Invalid UTF-8 sequence"))
    }
}

impl Decodable for Qid {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Qid {
            typ: decode_trunc!(QidType, *r),
            version: try!(Decodable::decode(r)),
            path:    try!(Decodable::decode(r))
        })
    }
}

impl Decodable for Statfs {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Statfs {
            typ: try!(Decodable::decode(r)),
            bsize: try!(Decodable::decode(r)),
            blocks: try!(Decodable::decode(r)),
            bfree: try!(Decodable::decode(r)),
            bavail: try!(Decodable::decode(r)),
            files: try!(Decodable::decode(r)),
            ffree: try!(Decodable::decode(r)),
            fsid: try!(Decodable::decode(r)),
            namelen: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for Time {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Time {
            sec: try!(Decodable::decode(r)),
            nsec: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for Stat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Stat {
            mode: try!(Decodable::decode(r)),
            uid: try!(Decodable::decode(r)),
            gid: try!(Decodable::decode(r)),
            nlink: try!(Decodable::decode(r)),
            rdev: try!(Decodable::decode(r)),
            size: try!(Decodable::decode(r)),
            blksize: try!(Decodable::decode(r)),
            blocks: try!(Decodable::decode(r)),
            atime: try!(Decodable::decode(r)),
            mtime: try!(Decodable::decode(r)),
            ctime: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for SetAttr {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(SetAttr {
            mode: try!(Decodable::decode(r)),
            uid: try!(Decodable::decode(r)),
            gid: try!(Decodable::decode(r)),
            size: try!(Decodable::decode(r)),
            atime: try!(Decodable::decode(r)),
            mtime: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for DirEntry {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(DirEntry {
            qid: try!(Decodable::decode(r)),
            offset: try!(Decodable::decode(r)),
            typ: try!(Decodable::decode(r)),
            name: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for DirEntryData {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let count: u32 = try!(Decodable::decode(r));
        let mut data: Vec<DirEntry> = Vec::with_capacity(count as usize);
        for _ in 0..count {
            data.push(try!(Decodable::decode(r)));
        }
        Ok(DirEntryData::with(data))
    }
}

impl Decodable for Data {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len: u32 = try!(Decodable::decode(r));
        let buf = try!(read_exact(r, len as usize));
        Ok(Data(buf))
    }
}

impl Decodable for Flock {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Flock {
            typ: decode_trunc!(LockType, *r),
            flags: decode_trunc!(LockFlag, *r),
            start: try!(Decodable::decode(r)),
            length: try!(Decodable::decode(r)),
            proc_id: try!(Decodable::decode(r)),
            client_id: try!(Decodable::decode(r)),
        })
    }
}

impl Decodable for Getlock {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Getlock {
            typ: decode_trunc!(LockType, *r),
            start: try!(Decodable::decode(r)),
            length: try!(Decodable::decode(r)),
            proc_id: try!(Decodable::decode(r)),
            client_id: try!(Decodable::decode(r)),
        })
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

        let msg_type = MsgType::from_u8(decode!(buf));
        let tag = decode!(buf);
        let body = match msg_type {
            // 9P2000.L
            Some(MsgType::Rlerror)      => Fcall::Rlerror { ecode: decode!(buf) },
            Some(MsgType::Tstatfs)      => Fcall::Tstatfs { fid: decode!(buf) },
            Some(MsgType::Rstatfs)      => Fcall::Rstatfs { statfs: decode!(buf) },
            Some(MsgType::Tlopen)       => Fcall::Tlopen { fid: decode!(buf), flags: decode!(buf) },
            Some(MsgType::Rlopen)       => Fcall::Rlopen { qid: decode!(buf), iounit: decode!(buf) },
            Some(MsgType::Tlcreate)     => Fcall::Tlcreate { fid: decode!(buf), name: decode!(buf), flags: decode!(buf), mode: decode!(buf), gid: decode!(buf) },
            Some(MsgType::Rlcreate)     => Fcall::Rlcreate { qid: decode!(buf), iounit: decode!(buf) },
            Some(MsgType::Tsymlink)     => Fcall::Tsymlink { fid: decode!(buf), name: decode!(buf), symtgt: decode!(buf), gid: decode!(buf) },
            Some(MsgType::Rsymlink)     => Fcall::Rsymlink { qid: decode!(buf) },
            Some(MsgType::Tmknod)       => Fcall::Tmknod { dfid: decode!(buf), name: decode!(buf), mode: decode!(buf), major: decode!(buf), minor: decode!(buf), gid: decode!(buf) },
            Some(MsgType::Rmknod)       => Fcall::Rmknod { qid: decode!(buf) },
            Some(MsgType::Trename)      => Fcall::Trename { fid: decode!(buf), dfid: decode!(buf), name: decode!(buf) },
            Some(MsgType::Rrename)      => Fcall::Rrename,
            Some(MsgType::Treadlink)    => Fcall::Treadlink { fid: decode!(buf) },
            Some(MsgType::Rreadlink)    => Fcall::Rreadlink { target: decode!(buf) },
            Some(MsgType::Tgetattr)     => Fcall::Tgetattr { fid: decode!(buf), req_mask: decode_trunc!(GetattrMask, buf) },
            Some(MsgType::Rgetattr)     => {
                let r = Fcall::Rgetattr { valid: decode_trunc!(GetattrMask, buf), qid: decode!(buf), stat: decode!(buf) };
                let _btime: Time = decode!(buf);
                let _gen: u64 = decode!(buf);
                let _data_version: u64 = decode!(buf);
                r
            },
            Some(MsgType::Tsetattr)     => Fcall::Tsetattr { fid: decode!(buf), valid: decode_trunc!(SetattrMask, buf), stat: decode!(buf) },
            Some(MsgType::Rsetattr)     => Fcall::Rsetattr,
            Some(MsgType::Txattrwalk)   => Fcall::Txattrwalk { fid: decode!(buf), newfid: decode!(buf), name: decode!(buf) },
            Some(MsgType::Rxattrwalk)   => Fcall::Rxattrwalk { size: decode!(buf) },
            Some(MsgType::Txattrcreate) => Fcall::Txattrcreate { fid: decode!(buf), name: decode!(buf), attr_size: decode!(buf), flags: decode!(buf) },
            Some(MsgType::Rxattrcreate) => Fcall::Rxattrcreate,
            Some(MsgType::Treaddir)     => Fcall::Treaddir { fid: decode!(buf), offset: decode!(buf), count: decode!(buf) },
            Some(MsgType::Rreaddir)     => Fcall::Rreaddir { data: decode!(buf) },
            Some(MsgType::Tfsync)       => Fcall::Tfsync { fid: decode!(buf) },
            Some(MsgType::Rfsync)       => Fcall::Rfsync,
            Some(MsgType::Tlock)        => Fcall::Tlock { fid: decode!(buf), flock: decode!(buf) },
            Some(MsgType::Rlock)        => Fcall::Rlock { status: decode_trunc!(LockStatus, buf) },
            Some(MsgType::Tgetlock)     => Fcall::Tgetlock { fid: decode!(buf), flock: decode!(buf) },
            Some(MsgType::Rgetlock)     => Fcall::Rgetlock { flock: decode!(buf) },
            Some(MsgType::Tlink)        => Fcall::Tlink { dfid: decode!(buf), fid: decode!(buf), name: decode!(buf) },
            Some(MsgType::Rlink)        => Fcall::Rlink,
            Some(MsgType::Tmkdir)       => Fcall::Tmkdir { dfid: decode!(buf), name: decode!(buf), mode: decode!(buf), gid: decode!(buf) },
            Some(MsgType::Rmkdir)       => Fcall::Rmkdir { qid: decode!(buf) },
            Some(MsgType::Trenameat)    => Fcall::Trenameat { olddirfid: decode!(buf), oldname: decode!(buf), newdirfid: decode!(buf), newname: decode!(buf) },
            Some(MsgType::Rrenameat)    => Fcall::Rrenameat,
            Some(MsgType::Tunlinkat)    => Fcall::Tunlinkat { dirfd: decode!(buf), name: decode!(buf), flags: decode!(buf) },
            Some(MsgType::Runlinkat)    => Fcall::Runlinkat,

            // 9P2000.u
            Some(MsgType::Tauth)        => Fcall::Tauth { afid: decode!(buf), uname: decode!(buf), aname: decode!(buf), n_uname: decode!(buf) },
            Some(MsgType::Rauth)        => Fcall::Rauth { aqid: decode!(buf) },
            Some(MsgType::Tattach)      => Fcall::Tattach { fid: decode!(buf), afid: decode!(buf), uname: decode!(buf), aname: decode!(buf), n_uname: decode!(buf) },
            Some(MsgType::Rattach)      => Fcall::Rattach { qid: decode!(buf) },

            // 9P2000
            Some(MsgType::Tversion)     => Fcall::Tversion { msize: decode!(buf), version: decode!(buf) },
            Some(MsgType::Rversion)     => Fcall::Rversion { msize: decode!(buf), version: decode!(buf) },
            Some(MsgType::Tflush)       => Fcall::Tflush { oldtag: decode!(buf) },
            Some(MsgType::Rflush)       => Fcall::Rflush,
            Some(MsgType::Twalk)        => Fcall::Twalk { fid: decode!(buf), newfid: decode!(buf), wnames: decode!(buf) },
            Some(MsgType::Rwalk)        => Fcall::Rwalk { wqids: decode!(buf) },
            Some(MsgType::Tread)        => Fcall::Tread { fid: decode!(buf), offset: decode!(buf), count: decode!(buf) },
            Some(MsgType::Rread)        => Fcall::Rread { data: decode!(buf) },
            Some(MsgType::Twrite)       => Fcall::Twrite { fid: decode!(buf), offset: decode!(buf), data: decode!(buf) },
            Some(MsgType::Rwrite)       => Fcall::Rwrite { count: decode!(buf) },
            Some(MsgType::Tclunk)       => Fcall::Tclunk { fid: decode!(buf) },
            Some(MsgType::Rclunk)       => Fcall::Rclunk,
            Some(MsgType::Tremove)      => Fcall::Tremove { fid: decode!(buf) },
            Some(MsgType::Rremove)      => Fcall::Rremove,
            Some(MsgType::Tlerror) | None =>
                return bo_io_error!(Other, "Invalid message type")
        };

        Ok(Msg { typ: msg_type.unwrap(), tag: tag, body: body })
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
    let mut encoder = Vec::new();
    for i in 0..10 {
        (&(i as u8)).encode(&mut encoder).unwrap();
    }
    assert_eq!(expected, encoder);
}

#[test]
fn decoder_test1() {
    let expected: Vec<u8> = (0..10).collect();
    let mut decoder = Cursor::new(expected.clone());
    let mut actual: Vec<u8> = Vec::new();
    loop {
        match Decodable::decode(&mut decoder) {
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
            version: P92000L.to_owned()
        }
    };
    let mut buf = Vec::new();
    let _ = expected.encode(&mut buf);

    let mut readbuf = Cursor::new(buf);
    let actual = Decodable::decode(&mut readbuf);

    assert_eq!(expected, actual.unwrap());
}
