
extern crate rs9p;

use std::io::Error;

struct Hellofs;

impl rs9p::srv::Filesystem for Hellofs {
}

fn hellofs_main(args: Vec<String>) -> Result<i32, Error> {
    if args.len() < 2 {
        println!("Usage: {} proto!address!port", args[0]);
        println!("  where: proto = tcp | unix");
        return Ok(-1);
    }

    let mut srv = try!(rs9p::srv::Server::announce(Hellofs, &args[1]));

    println!("Waiting for a 9P client on: {}", args[1]);
    try!(srv.srv());

    return Ok(0);
}

fn main() {
    let args = std::env::args().collect();
    let exit_code = match hellofs_main(args) {
        Ok(code) => code,
        Err(e) => { println!("Error: {}", e); -1 }
    };
    std::process::exit(exit_code);
}
