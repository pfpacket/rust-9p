
//! A library to deal with 9P protocol, a network filesystem

#![feature(metadata_ext)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate enum_primitive;

pub use error::number as errno;
pub use error::string as errstr;
pub use fcall::*;
pub use server::{Fid, Filesystem, srv, Result};

pub mod error;
pub mod fcall;
pub mod serialize;
pub mod server;
