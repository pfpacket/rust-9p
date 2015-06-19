
//! A library to deal with 9P protocol, a network filesystem

#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive;

pub use error::errno as errno;
pub use error::string as errstr;
pub use error::Error;
pub use fcall::*;
pub use server::{Fid, Filesystem, Result, srv, srv_mt};

pub mod error;
pub mod fcall;
pub mod serialize;
pub mod server;
