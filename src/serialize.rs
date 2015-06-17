
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

// Create an unintialized buffer
// Safe to use only for writing data to it
fn create_buffer(size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    unsafe { buffer.set_len(size); }
    buffer
}

fn read_exact<R: Read>(r: &mut R, size: usize) -> Result<Vec<u8>> {
    let mut pos = 0;
    let mut buf = create_buffer(size);
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
        let mut bytes = try!{ (self.len() as u16).encode(w) };
        bytes += try!(w.write_all(self.as_bytes()).and(Ok(self.len())));
        Ok(bytes)
    }
}

impl Encodable for Qid {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.typ.encode(w));
        bytes += try!(self.version.encode(w));
        bytes += try!(self.path.encode(w));
        Ok(bytes)
    }
}

impl Encodable for Statfs {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.typ.encode(w));
        bytes += try!(self.bsize.encode(w));
        bytes += try!(self.blocks.encode(w));
        bytes += try!(self.bfree.encode(w));
        bytes += try!(self.bavail.encode(w));
        bytes += try!(self.files.encode(w));
        bytes += try!(self.ffree.encode(w));
        bytes += try!(self.fsid.encode(w));
        bytes += try!(self.namelen.encode(w));
        Ok(bytes)
    }
}

impl Encodable for Time {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.sec.encode(w));
        bytes += try!(self.nsec.encode(w));
        Ok(bytes)
    }
}

impl Encodable for Stat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.mode.encode(w));
        bytes += try!(self.uid.encode(w));
        bytes += try!(self.gid.encode(w));
        bytes += try!(self.nlink.encode(w));
        bytes += try!(self.rdev.encode(w));
        bytes += try!(self.size.encode(w));
        bytes += try!(self.blksize.encode(w));
        bytes += try!(self.blocks.encode(w));
        bytes += try!(self.atime.encode(w));
        bytes += try!(self.mtime.encode(w));
        bytes += try!(self.ctime.encode(w));
        Ok(bytes)
    }
}

impl Encodable for SetAttr {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.mode.encode(w));
        bytes += try!(self.uid.encode(w));
        bytes += try!(self.gid.encode(w));
        bytes += try!(self.size.encode(w));
        bytes += try!(self.atime.encode(w));
        bytes += try!(self.mtime.encode(w));
        Ok(bytes)
    }
}

impl Encodable for DirEntry {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.qid.encode(w));
        bytes += try!(self.offset.encode(w));
        bytes += try!(self.typ.encode(w));
        bytes += try!(self.name.encode(w));
        Ok(bytes)
    }
}

impl Encodable for DirEntryData {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.size().encode(w));
        for entry in self.data() {
            bytes += try!(entry.encode(w));
        }
        Ok(bytes)
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

impl Encodable for Flock {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.typ.encode(w));
        bytes += try!(self.flags.encode(w));
        bytes += try!(self.start.encode(w));
        bytes += try!(self.length.encode(w));
        bytes += try!(self.proc_id.encode(w));
        bytes += try!(self.client_id.encode(w));
        Ok(bytes)
    }
}

impl Encodable for Getlock {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes = 0;
        bytes += try!(self.typ.encode(w));
        bytes += try!(self.start.encode(w));
        bytes += try!(self.length.encode(w));
        bytes += try!(self.proc_id.encode(w));
        bytes += try!(self.client_id.encode(w));
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
        macro_rules! encode {
            ( $encoder:expr, $( $x:expr ),* ) => {
                $( let _ = try!($x.encode(&mut $encoder)); )*
            }
        }

        let mut buf: Vec<u8> = Vec::new();
        encode!(buf, &(self.typ as u8));
        encode!(buf, &self.tag);
        match self.body {
            // 9P2000.L
            Fcall::Rlerror { ref ecode }                                                    => { encode!(buf, ecode); },
            Fcall::Tstatfs { ref fid }                                                      => { encode!(buf, fid); },
            Fcall::Rstatfs { ref statfs }                                                   => { encode!(buf, statfs); },
            Fcall::Tlopen { ref fid, ref flags }                                            => { encode!(buf, fid, flags); },
            Fcall::Rlopen { ref qid, ref iounit }                                           => { encode!(buf, qid, iounit); },
            Fcall::Tlcreate { ref fid, ref name, ref flags, ref mode, ref gid }             => { encode!(buf, fid, name, flags, mode, gid); },
            Fcall::Rlcreate { ref qid, ref iounit }                                         => { encode!(buf, qid, iounit); },
            Fcall::Tsymlink { ref fid, ref name, ref symtgt, ref gid }                      => { encode!(buf, fid, name, symtgt, gid); },
            Fcall::Rsymlink { ref qid }                                                     => { encode!(buf, qid); },
            Fcall::Tmknod { ref dfid, ref name, ref mode, ref major, ref minor, ref gid }   => { encode!(buf, dfid, name, mode, major, minor, gid); },
            Fcall::Rmknod { ref qid }                                                       => { encode!(buf, qid); },
            Fcall::Trename { ref fid, ref dfid, ref name }                                  => { encode!(buf, fid, dfid, name); },
            Fcall::Rrename                                                                  => {},
            Fcall::Treadlink { ref fid }                                                    => { encode!(buf, fid); },
            Fcall::Rreadlink { ref target }                                                 => { encode!(buf, target); },
            Fcall::Tgetattr { ref fid, ref req_mask }                                       => { encode!(buf, fid, req_mask); },
            Fcall::Rgetattr { ref valid, ref qid, ref stat }                                => { encode!(buf, valid, qid, stat, 0u64, 0u64, 0u64, 0u64); },
            Fcall::Tsetattr { ref fid, ref valid, ref stat }                                => { encode!(buf, fid, valid, stat); },
            Fcall::Rsetattr                                                                 => {},
            Fcall::Txattrwalk { ref fid, ref newfid, ref name }                             => { encode!(buf, fid, newfid, name); },
            Fcall::Rxattrwalk { ref size }                                                  => { encode!(buf, size); },
            Fcall::Txattrcreate { ref fid, ref name, ref attr_size, ref flags }             => { encode!(buf, fid, name, attr_size, flags); },
            Fcall::Rxattrcreate                                                             => {},
            Fcall::Treaddir { ref fid, ref offset, ref count }                              => { encode!(buf, fid, offset, count); },
            Fcall::Rreaddir { ref data }                                                    => { encode!(buf, data); },
            Fcall::Tfsync { ref fid }                                                       => { encode!(buf, fid); },
            Fcall::Rfsync                                                                   => {},
            Fcall::Tlock { ref fid, ref flock }                                             => { encode!(buf, fid, flock ); },
            Fcall::Rlock { ref status }                                                     => { encode!(buf, status); },
            Fcall::Tgetlock { ref fid, ref flock }                                          => { encode!(buf, fid, flock); },
            Fcall::Rgetlock { ref flock }                                                   => { encode!(buf, flock); },
            Fcall::Tlink { ref dfid, ref fid, ref name }                                    => { encode!(buf, dfid, fid, name); },
            Fcall::Rlink                                                                    => {},
            Fcall::Tmkdir { ref dfid, ref name, ref mode, ref gid }                         => { encode!(buf, dfid, name, mode, gid); },
            Fcall::Rmkdir { ref qid }                                                       => { encode!(buf, qid); },
            Fcall::Trenameat { ref olddirfid, ref oldname, ref newdirfid, ref newname }     => { encode!(buf, olddirfid, oldname, newdirfid, newname); },
            Fcall::Rrenameat                                                                => {},
            Fcall::Tunlinkat { ref dirfd, ref name, ref flags }                             => { encode!(buf, dirfd, name, flags); },
            Fcall::Runlinkat                                                                => {},

            // 9P2000.u
            Fcall::Tauth { ref afid, ref uname, ref aname, ref n_uname }                    => { encode!(buf, afid, uname, aname, n_uname); },
            Fcall::Rauth { ref aqid }                                                       => { encode!(buf, aqid); },
            Fcall::Tattach { ref fid, ref afid, ref uname, ref aname, ref n_uname }         => { encode!(buf, fid, afid, uname, aname, n_uname); },
            Fcall::Rattach { ref qid }                                                      => { encode!(buf, qid); },

            // 9P2000
            Fcall::Tversion { ref msize, ref version }                                      => { encode!(buf, msize, version); },
            Fcall::Rversion { ref msize, ref version }                                      => { encode!(buf, msize, version); },
            Fcall::Tflush { ref oldtag }                                                    => { encode!(buf, oldtag); },
            Fcall::Rflush                                                                   => {},
            Fcall::Twalk { ref fid, ref newfid, ref wnames }                                => { encode!(buf, fid, newfid, wnames); },
            Fcall::Rwalk { ref wqids }                                                      => { encode!(buf, wqids); },
            Fcall::Tread { ref fid, ref offset, ref count }                                 => { encode!(buf, fid, offset, count); },
            Fcall::Rread { ref data }                                                       => { encode!(buf, data); },
            Fcall::Twrite { ref fid, ref offset, ref data }                                 => { encode!(buf, fid, offset, data); },
            Fcall::Rwrite { ref count }                                                     => { encode!(buf, count); },
            Fcall::Tclunk { ref fid }                                                       => { encode!(buf, fid); },
            Fcall::Rclunk                                                                   => {},
            Fcall::Tremove { ref fid }                                                      => { encode!(buf, fid); },
            Fcall::Rremove                                                                  => {},
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
        Ok(Data::new(buf))
    }
}

impl Decodable for Flock {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Flock {
            typ: try!(Decodable::decode(r)),
            flags: try!(Decodable::decode(r)),
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
            typ: try!(Decodable::decode(r)),
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

        macro_rules! decode {
            ($decoder:expr) => {
                try!(Decodable::decode(&mut $decoder))
            }
        }

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
            Some(MsgType::Tgetattr)     => Fcall::Tgetattr { fid: decode!(buf), req_mask: decode!(buf) },
            Some(MsgType::Rgetattr)     => {
                let r = Fcall::Rgetattr { valid: decode!(buf), qid: decode!(buf), stat: decode!(buf) };
                let _btime: Time = decode!(buf);
                let _gen: u64 = decode!(buf);
                let _data_version: u64 = decode!(buf);
                r
            },
            Some(MsgType::Tsetattr)     => Fcall::Tsetattr { fid: decode!(buf), valid: decode!(buf), stat: decode!(buf) },
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
            Some(MsgType::Rlock)        => Fcall::Rlock { status: decode!(buf) },
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
            version: P92000L.to_owned()
        }
    };
    let mut buf = Vec::new();
    let _ = expected.encode(&mut buf);

    let mut readbuf = Cursor::new(buf);
    let actual = Decodable::decode(&mut readbuf);

    assert_eq!(expected, actual.unwrap());
}
