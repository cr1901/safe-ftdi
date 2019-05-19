extern crate libftdi1_sys as ftdic;
use std::os::raw;
use std::ffi::CStr;
use std::result;

pub mod mpsse;
use mpsse::{MpsseMode};

pub mod error;
use error::*;


pub struct Context {
    context : *mut ftdic::ftdi_context,
}

pub type Result<T> = result::Result<T, error::Error>;

impl Context {
    fn check_ftdi_error<T>(&self, rc : raw::c_int, ok_val : T) -> Result<T> {
        if rc < 0 {
            // From looking at libftdi library, the error string is always a static
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

        self.check_ftdi_error(rc, ())
    }

    pub fn set_baudrate(&self, baudrate : u32) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_baudrate(self.context, baudrate as raw::c_int)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn set_bitmode(&self, bitmask : u8, mode : MpsseMode) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_bitmode(self.context, bitmask as raw::c_uchar, mode as raw::c_uchar)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn purge_usb_buffers(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_buffers(self.context)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn purge_usb_rx_buffer(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_rx_buffer(self.context)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn purge_usb_tx_buffer(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_tx_buffer(self.context)
        };

        self.check_ftdi_error(rc, ())
    }

    pub fn read_pins(&self) -> Result<u8> {
        let mut pins : u8 = 0;
        let pins_ptr = std::slice::from_mut(&mut pins).as_mut_ptr();

        let rc = unsafe {
            ftdic::ftdi_read_pins(self.context, pins_ptr)
        };

        self.check_ftdi_error(rc, pins)
    }

    pub fn read_data(&self, data : &mut [u8]) -> Result<u32> {
        let raw_ptr = data.as_mut_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_read_data(self.context, raw_ptr, raw_len)
        };

        self.check_ftdi_error(rc, rc as u32)
    }

    pub fn write_data(&self, data : &[u8]) -> Result<u32> {
        let raw_ptr = data.as_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_write_data(self.context, raw_ptr, raw_len)
        };

        self.check_ftdi_error(rc, rc as u32)
    }
}


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
