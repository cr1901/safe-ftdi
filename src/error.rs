use std;
use std::fmt;

#[derive(Debug)]
pub enum Error<'a> {
    LibFtdi(LibFtdiError<'a>),
    MallocFailure,
}

#[derive(Debug)]
pub struct LibFtdiError<'a> {
    err_str : &'a str,
}

impl<'a> LibFtdiError<'a> {
    pub fn new(err_str : &'a str) -> LibFtdiError<'a> {
        LibFtdiError {
                err_str : err_str,
        }
    }
}


impl<'a> fmt::Display for Error<'a> {
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

impl<'a> fmt::Display for LibFtdiError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.err_str)
    }
}

impl<'a> std::error::Error for Error<'a> {
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

impl<'a> std::error::Error for LibFtdiError<'a> {}
