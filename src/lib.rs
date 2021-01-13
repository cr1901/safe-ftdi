extern crate libftdi1_sys as ftdic;
use std::os::raw;
use std::ffi::CStr;

pub mod mpsse;
use mpsse::{MpsseMode};

pub mod error;
use error::{Error, LibFtdiError};

/// Low-level wrapper around a ftdi_context instance
pub struct Context (*mut ftdic::ftdi_context);

pub type Result<T> = std::result::Result<T, Error>;

impl Context {
    pub fn new() -> Result<Context> {
        let ctx = unsafe { ftdic::ftdi_new() };

        if ctx.is_null() {
            Err(Error::MallocFailure)
        } else {
            Ok(Context(ctx))
        }
    }

    pub fn check_ftdi_error(&self, rc : raw::c_int) -> Result<()> {
        if rc < 0 {
            // From looking at libftdi library, the error string is always a static
            // string literal.
            let slice = unsafe {
                let err_raw = ftdic::ftdi_get_error_string(self.0);
                CStr::from_ptr(err_raw)
            };

            // If UTF8 validation fails, no point in continuing.
            Err(Error::LibFtdi(LibFtdiError::new(slice.to_str().unwrap())))
        } else {
            Ok(())
        }
    }

    pub fn get_ftdi_context(&self) -> *mut ftdic::ftdi_context {
        self.0
    }
}


impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ftdic::ftdi_free(self.0) }
    }
}

pub struct Device {
    context: Context
}

impl Device {
    pub fn open(vid : u16, pid : u16) -> Result<Device> {
        let context = Context::new()?;
        let rc = unsafe {
            ftdic::ftdi_usb_open(context.0, raw::c_int::from(vid), raw::c_int::from(pid))
        };

        context.check_ftdi_error(rc)?;
        Ok(Device {context})
    }

    pub fn set_baudrate(&self, baudrate : u32) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_baudrate(self.context.0, baudrate as raw::c_int)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn set_bitmode(&self, bitmask : u8, mode : MpsseMode) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_bitmode(self.context.0, bitmask as raw::c_uchar, mode.0 as raw::c_uchar)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_buffers(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_buffers(self.context.0)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_rx_buffer(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_rx_buffer(self.context.0)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_tx_buffer(&self) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_usb_purge_tx_buffer(self.context.0)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn read_pins(&self) -> Result<u8> {
        let mut pins : u8 = 0;
        let pins_ptr = std::slice::from_mut(&mut pins).as_mut_ptr();

        let rc = unsafe {
            ftdic::ftdi_read_pins(self.context.0, pins_ptr)
        };

        self.context.check_ftdi_error(rc)?;
        Ok(pins)
    }

    pub fn read_data(&self, data : &mut [u8]) -> Result<u32> {
        let raw_ptr = data.as_mut_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_read_data(self.context.0, raw_ptr, raw_len)
        };

        self.context.check_ftdi_error(rc)?;
        Ok(rc as u32)
    }

    pub fn write_data(&self, data : &[u8]) -> Result<u32> {
        let raw_ptr = data.as_ptr();
        let raw_len = data.len() as i32;

        let rc = unsafe {
            ftdic::ftdi_write_data(self.context.0, raw_ptr, raw_len)
        };

        self.context.check_ftdi_error(rc)?;
        Ok(rc as u32)
    }
}
