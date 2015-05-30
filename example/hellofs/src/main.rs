
extern crate rs9p;


use std::io::{self, Error, ErrorKind};
use std::net::{TcpListener};

// return: (proto, addr:port)
fn parse_proto(arg: &str) -> Result<(&str, String), ()> {
    let mut split = arg.split("!");
    let proto = try!(split.nth(0).ok_or(()));
    let addr  = try!(split.nth(0).ok_or(()));
    let port  = try!(split.nth(0).ok_or(()));
    Ok((proto, addr.to_owned() + ":" + port))
}

fn hellofs_main(args: Vec<String>) -> io::Result<i32> {
    if args.len() < 2 {
        println!("Usage: {} proto!address!port", args[0]);
        println!("  where: proto = tcp | unix");
        return Ok(-1)
    }

    let (proto, sockaddr) = try!(parse_proto(&args[1]).map_err(
        |_| Error::new(ErrorKind::InvalidInput, "Invalid proto or address")
    ));

    if proto != "tcp" {
        return Err(Error::new(ErrorKind::InvalidInput, "Unsupported proto"));
    }

    println!("[*] Waiting for a connection: proto={} addr={}", proto, sockaddr);
    let listener = try!(TcpListener::bind(&sockaddr[..]));

    return Ok(0);
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = match hellofs_main(args) {
        Ok(code) => code,
        Err(e) => {
            println!("Error: {}", e);
            -1
        }
    };
    std::process::exit(exit_code);
}
