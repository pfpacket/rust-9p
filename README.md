rust-9p
=====
Filesystems library using 9P2000.L protocol, an extended variant of 9P from Plan 9.

[![Build Status](https://travis-ci.org/pfpacket/rust-9p.svg?branch=master)](https://travis-ci.org/pfpacket/rust-9p)

[Documentation](https://pfpacket.github.io/rust-9p/rs9p/index.html)


## Build
Use Rust nightly.


## Usage
Add this to your crate:

```rust
extern crate rs9p;
```


## unpfs
unpfs is an example file server which just exports your filesystem.
You can build unpfs with the following commands below:

```bash
cd example/unpfs/
cargo build --verbose --release
```
and run unpfs with the following command to export `/exportdir`:

```bash
cargo run --release "tcp\!0.0.0.0\!564" /exportdir
# or
# ./target/release/unpfs "tcp\!0.0.0.0\!564" /exportdir
```
You are now ready to import/mount the remote filesystem.
Let's mount it at `/mountdir`:

```bash
sudo mount -t 9p -o version=9p2000.L,trans=tcp,port=564,uname=$USER 127.0.0.1 /mountdir
```

| Option Name | Value |
|---|---|
| version | version must be 9p2000.L |
| trans | trans must be tcp |
| port | port number |
| uname | user name for accessing file server |


## License
rust-9p is distributed under the BSD 3-Clause License.  
See LICENSE for details.
