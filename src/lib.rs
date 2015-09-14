
//! A library to deal with 9P, especially 9P2000.L, a Plan 9 file protocol

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
pub mod srv;
pub mod srv_mt;

pub use utils::Result;
pub use error::Error;
pub use error::errno as errno;
pub use error::string as errstr;
pub use fcall::*;
pub use srv::{srv, srv_spawn};
pub use srv_mt::{srv_mt};
