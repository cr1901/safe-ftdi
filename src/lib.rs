extern crate libftdi1_sys as ftdic;
use std::os::raw;
use std::ffi::CStr;
use std::result;

pub mod error;
use error::*;


pub struct Context {
    context : *mut ftdic::ftdi_context,
}

type Result<'a, T> = result::Result<T, error::Error<'a>>;

impl Context {
    fn check_ftdi_error<'a, T>(&'a self, rc : raw::c_int, ok_val : T) -> Result<'a, T> {
        if rc < 0 {
            // From looking at libftdi library, the error string is valid as
            // long as the ftdi context is alive. Each error string is a null-terminated
            // string literal.
            let slice = unsafe {
                let err_raw = ftdic::ftdi_get_error_string(self.context);
                CStr::from_ptr(err_raw)
            };

            // If UTF8 validation fails, no point in continuing.
            Err(Error::LibFtdi(error::LibFtdiError::new(slice.to_str().unwrap())))
        } else {
            Ok(ok_val)
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

        self.check_ftdi_error(rc, ())
    }

    pub fn set_baudrate<'a>(&'a self, baudrate : u32) -> Result<'a, ()> {
        let rc = unsafe {
            ftdic::ftdi_set_baudrate(self.context, baudrate as raw::c_int)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn read_pins<'a>(&'a self) -> Result<'a, u8> {
        let mut pins : u8 = 0;
        let pins_ptr = std::slice::from_mut(&mut pins).as_mut_ptr();

        let rc = unsafe {
            ftdic::ftdi_read_pins(self.context, pins_ptr)
        };

        self.check_ftdi_error(rc, pins)
    }

    pub fn read_data<'a>(&'a self, data : &mut [u8]) -> Result<'a, u32> {
        let raw_ptr = data.as_mut_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_read_data(self.context, raw_ptr, raw_len)
        };

        self.check_ftdi_error(rc, rc as u32)
    }

    pub fn write_data<'a>(&'a self, data : &[u8]) -> Result<'a, u32> {
        let raw_ptr = data.as_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_write_data(self.context, raw_ptr, raw_len)
        };

        self.check_ftdi_error(rc, rc as u32)
    }
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
