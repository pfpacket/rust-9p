use std::fmt;
use crate::error;
use std::result::Result as StdResult;
use std::str::FromStr;

pub type Result<T> = ::std::result::Result<T, error::Error>;

macro_rules! io_err {
    ($kind:ident, $msg:expr) => {
        ::std::io::Error::new(::std::io::ErrorKind::$kind, $msg)
    };
}

macro_rules! res {
    ($err:expr) => {
        Err(From::from($err))
    };
}

#[derive(Debug)]
pub struct AddrSpecParseError {
    message: String,
}

impl AddrSpecParseError {
    fn new(message :String) -> Self {
        return AddrSpecParseError{message}
    }
}

impl std::error::Error for AddrSpecParseError {}

impl fmt::Display for AddrSpecParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub enum AddrSpec {
    Tcp(String),
    Unix(String),
    Fd(i32, i32),
}

impl FromStr for AddrSpec {
    type Err = AddrSpecParseError;

    fn from_str(arg: &str) -> StdResult<Self, Self::Err> {
        let mut split = arg.split('!');
        let proto = match split.next() {
            Some(p) => p,
            None => return Err(AddrSpecParseError::new("No protocol specified".into())),
        };

        match proto {
            "tcp" => {
                let addr = match split.next() {
                    Some(p) => p,
                    None => return Err(AddrSpecParseError::new("No listen address specified".into())),
                };
                let port = match split.next() {
                    Some(p) => p,
                    None => return Err(AddrSpecParseError::new("No listen port specified".into())),
                };
                Ok(AddrSpec::Tcp(addr.to_owned() + ":" + port))
            },
            "unix" => {
                let addr = match split.next() {
                    Some(p) => p,
                    None => return Err(AddrSpecParseError::new("No listen socket path specified".into())),
                };
                let port = match split.next() {
                    Some(p) => p,
                    None => return Err(AddrSpecParseError::new("No listen socket port specified".into())),
                };
                Ok(AddrSpec::Unix(addr.to_owned() + ":" + port))
            },
            "fd" => {
                let readfd = match split.next() {
                    Some(p) => match p.parse::<i32>() {
                        Ok(p) => p,
                        Err(e) => return Err(AddrSpecParseError::new(format!("Invalid read file descriptor: {}", e))),
                    },
                    None => return Err(AddrSpecParseError::new("No read file descriptor specified".into())),
                };
                let writefd = match split.next() {
                    Some(p) => match p.parse::<i32>() {
                        Ok(p) => p,
                        Err(e) => return Err(AddrSpecParseError::new(format!("Invalid write file descriptor: {}", e))),
                    },
                    None => return Err(AddrSpecParseError::new("No file descriptor specified".into())),
                };
                Ok(AddrSpec::Fd(readfd, writefd))
            },
            _ => {
                Err(AddrSpecParseError::new(format!("Unsupported protocol {}", proto)))
            }
        }
    }
}
