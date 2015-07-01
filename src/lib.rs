
//! A library to deal with 9P, especially 9P2000.L, a Plan 9 file protocol

#![feature(tcp)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive;

#[macro_use]
mod utils;
pub mod error;
pub mod fcall;
pub mod serialize;
pub mod server;
pub mod thread;

pub use utils::Result;
pub use error::errno as errno;
pub use error::string as errstr;
pub use error::Error;
pub use fcall::*;
pub use server::{srv, srv_spawn};
pub use thread::{srv_mt};
