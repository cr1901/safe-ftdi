extern crate libftdi1_sys as ftdic;
use std::os::raw;
use std::fmt;
use std::error;
use std::result;


pub struct Context {
    context : *mut ftdic::ftdi_context,
}

type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    LibFtdiError(i32), /* libftdi-specific failure. */
    MallocFailure,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LibFtdiError(x) => {
                write!(f, "libftdi error: {}", x)
            },
            Error::MallocFailure => {
                write!(f, "malloc() failure")
            }
        }
    }
}

impl error::Error for Error {

}


impl Context {
    fn check_ftdi_error(rc : raw::c_int) -> Result<()> {
        if rc < 0 {
            Err(Error::LibFtdiError(rc as i32))
        } else {
            Ok(())
        }
    }

    pub fn new() -> Result<Context> {
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
    pub fn open(&mut self, vid : u16, pid : u16) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_open(self.context, vid as raw::c_int, pid as raw::c_int)
        };

        Context::check_ftdi_error(rc)
    }

    pub fn set_baudrate(&self, baudrate : u32) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_baudrate(self.context, baudrate as raw::c_int)
        };

        Context::check_ftdi_error(rc)
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
