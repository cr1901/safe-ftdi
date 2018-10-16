extern crate libftdi1_sys as ftdic;
use std::os::raw;
use std::fmt;
use std::ffi::CStr;
use std::error;
use std::result;


pub struct Context {
    context : *mut ftdic::ftdi_context,
}

type Result<'a, T> = result::Result<T, Error<'a>>;

#[derive(Debug)]
pub enum Error<'a> {
    LibFtdi(LibFtdiError<'a>),
    MallocFailure,
}

#[derive(Debug)]
pub struct LibFtdiError<'a> {
    err_str : &'a str,
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

impl<'a> error::Error for Error<'a> {
    fn cause(&self) -> Option<&error::Error> {
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

impl<'a> error::Error for LibFtdiError<'a> {}



impl Context {
    fn check_ftdi_error<'a>(&'a self, rc : raw::c_int) -> Result<'a, ()> {
        if rc < 0 {
            // From looking at libftdi library, the error string is valid as
            // long as the ftdi context is alive. Each error string is a null-terminated
            // string literal.
            let slice = unsafe {
                let err_raw = ftdic::ftdi_get_error_string(self.context);
                CStr::from_ptr(err_raw)
            };

            // If UTF8 validation fails, no point in continuing.
            Err(Error::LibFtdi(LibFtdiError {
                    err_str : slice.to_str().unwrap(),
            }))
        } else {
            Ok(())
        }
    }

    pub fn new<'a>() -> Result<'a, Context> {
        let ctx = unsafe { ftdic::ftdi_new() };

        if ctx.is_null() {
            Err(Error::MallocFailure)
        } else {
            Ok(Context {
                context : ctx
            })
        }
    }

    // Combine with new()?
    pub fn open<'a>(&'a mut self, vid : u16, pid : u16) -> Result<'a, ()> {
        let rc = unsafe {
            ftdic::ftdi_usb_open(self.context, vid as raw::c_int, pid as raw::c_int)
        };

        self.check_ftdi_error(rc)
    }

    pub fn set_baudrate<'a>(&'a self, baudrate : u32) -> Result<'a, ()> {
        let rc = unsafe {
            ftdic::ftdi_set_baudrate(self.context, baudrate as raw::c_int)
        };

        self.check_ftdi_error(rc)
    }

    //pub fn read_pins(&self)
}

/* impl Write for Context {

} */



impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ftdic::ftdi_free(self.context) }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
