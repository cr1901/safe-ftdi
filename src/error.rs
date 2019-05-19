use std;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    LibFtdi(LibFtdiError),
    MallocFailure,
}

#[derive(Debug)]
pub struct LibFtdiError {
    // From looking at libftdi library, the error string is always a static
    // string literal, so this lifetime is safe.
    err_str : &'static str,
}

impl LibFtdiError {
    pub fn new(err_str : &'static str) -> LibFtdiError {
        LibFtdiError {
                err_str,
        }
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LibFtdi(_) => {
                write!(f, "libftdi-internal error")
            },
            Error::MallocFailure => {
                write!(f, "malloc() failure")
            }
        }
    }
}

impl fmt::Display for LibFtdiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_str)
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::LibFtdi(ref ftdi_err) => {
                Some(ftdi_err)
            },
            Error::MallocFailure => {
                None
            }
        }
    }
}

impl std::error::Error for LibFtdiError {}
