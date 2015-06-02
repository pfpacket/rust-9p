
//! A library to deal with 9P protocol, a network filesystem

#![feature(metadata_ext)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate enum_primitive;

pub mod fcall;
pub mod serialize;
pub mod server;
pub mod error;

pub use fcall::*;
pub use server::Result;
pub use server::Request;
pub use server::Filesystem;
pub use server::srv;
