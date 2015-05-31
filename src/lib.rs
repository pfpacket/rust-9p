
//! Deal with 9P protocol, a network filesystem

#![feature(metadata_ext)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate enum_primitive;

pub mod fcall;
pub mod serialize;
pub mod srv;
pub mod error;

pub use fcall::*;
pub use srv::Result;
pub use srv::Request;
pub use srv::Filesystem;
pub use srv::Server;
