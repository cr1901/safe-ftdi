extern crate libftdi1_sys as ftdic;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw;
use std::pin::Pin;
use std::time::Duration;

pub mod error;
use error::{Error, LibFtdiError};

/// Low-level wrapper around a ftdi_context instance
pub struct Context(*mut ftdic::ftdi_context);

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

    pub fn set_interface(&self, interface: Interface) -> Result<()> {
        let interface = match interface {
            Interface::Any => ftdic::ftdi_interface::INTERFACE_ANY,
            Interface::A => ftdic::ftdi_interface::INTERFACE_A,
            Interface::B => ftdic::ftdi_interface::INTERFACE_B,
            Interface::C => ftdic::ftdi_interface::INTERFACE_C,
            Interface::D => ftdic::ftdi_interface::INTERFACE_D,
        };

        let rc = unsafe { ftdic::ftdi_set_interface(self.get_ftdi_context(), interface) };
        self.check_ftdi_error(rc)
    }

    pub fn check_ftdi_error(&self, rc: raw::c_int) -> Result<()> {
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
        unsafe { ftdic::ftdi_free(self.get_ftdi_context()) }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Interface {
    Any,
    A,
    B,
    C,
    D,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FlowControl {
    Disabled,
    RtsCts,
    DtrDsr,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BitMode {
    Reset,
    Bitbang,
    Mpsse,
    SyncBB,
    Mcu,
    Opto,
    Cbus,
    SyncFF,
    FT1284,
}

pub struct Device {
    context: Context,
    eeprom_read: bool,
}

pub struct AsyncRead<'b> {
    phantom: PhantomData<&'b mut [u8]>,
    transfer_control: *mut ftdic::ftdi_transfer_control,
}

impl<'b> AsyncRead<'b> {
    /// Wait for completion of the transfer.
    pub fn wait(self) -> Result<usize> {
        let rc = unsafe { ftdic::ftdi_transfer_data_done(self.transfer_control) };
        if rc < 0 {
            Err(Error::LibFtdi(LibFtdiError::new(
                "Error completing transfer",
            )))
        } else {
            Ok(rc as usize)
        }
    }

    /// Cancel transfer and wait for completion.
    pub fn cancel(self, timeout: Duration) {
        let mut time = ftdic::timeval {
            tv_sec: (timeout.as_secs() as i32).into(),
            tv_usec: (timeout.subsec_micros() as i32).into(),
        };

        unsafe { ftdic::ftdi_transfer_data_cancel(self.transfer_control, &mut time) };
    }
}

impl Device {
    /// Opens the first device with a given vendor and product ids
    pub fn from_vid_pid(interface: Interface, vid: u16, pid: u16) -> Result<Device> {
        Device::from_description_serial(interface, vid, pid, None, None)
    }

    /// Opens the first device with a given, vendor id, product id, description, and serial
    pub fn from_description_serial(
        interface: Interface,
        vid: u16,
        pid: u16,
        description: Option<String>,
        serial: Option<String>,
    ) -> Result<Device> {
        Device::from_description_serial_index(interface, vid, pid, description, serial, 0)
    }

    /// Opens the index-th device with a given, vendor id, product id, description, and serial
    pub fn from_description_serial_index(
        interface: Interface,
        vid: u16,
        pid: u16,
        description: Option<String>,
        serial: Option<String>,
        index: u32,
    ) -> Result<Device> {
        let context = Context::new()?;
        context.set_interface(interface)?;

        let (desc, desc_supplied) = match description {
            Some(d) => (CString::new(d).unwrap().into_raw(), true),
            None => (std::ptr::null_mut(), false),
        };
        let (ser, ser_supplied) = match serial {
            Some(s) => (CString::new(s).unwrap().into_raw(), true),
            None => (std::ptr::null_mut(), false),
        };

        let rc = unsafe {
            ftdic::ftdi_usb_open_desc_index(
                context.0,
                vid as raw::c_int,
                pid as raw::c_int,
                desc,
                ser,
                index as raw::c_uint,
            )
        };

        if desc_supplied {
            drop(unsafe { CString::from_raw(desc) }); // String must be manually free'd
        }
        if ser_supplied {
            drop(unsafe { CString::from_raw(ser) }); // String must be manually free'd
        }
        context.check_ftdi_error(rc)?;
        Ok(Device {
            context,
            eeprom_read: false,
        })
    }

    /// Opens the device at a given USB bus and device address
    pub fn from_bus_addr(interface: Interface, bus: u8, addr: u8) -> Result<Device> {
        let context = Context::new()?;
        context.set_interface(interface)?;

        let rc = unsafe { ftdic::ftdi_usb_open_bus_addr(context.get_ftdi_context(), bus, addr) };
        context.check_ftdi_error(rc)?;
        Ok(Device {
            context,
            eeprom_read: false,
        })
    }

    /// Opens the ftdi-device described by a description-string
    ///
    /// Intended to be used for parsing a device-description given as commandline argument
    ///
    /// - d:<devicenode> path of bus and device-node (e.g. "003/001") within usb device tree (usually at /proc/bus/usb/)
    /// - i:<vendor>:<product> first device with given vendor and product id, ids can be decimal, octal (preceded by "0") or hex (preceded by "0x")
    /// - i:<vendor>:<product>:<index> as above with index being the number of the device (starting with 0) if there are more than one
    /// - s:<vendor>:<product>:<serial> first device with given vendor id, product id and serial string
    pub fn from_description_string(interface: Interface, description: String) -> Result<Device> {
        let context = Context::new()?;
        context.set_interface(interface)?;

        let desc = CString::new(description).unwrap().into_raw();
        let rc = unsafe { ftdic::ftdi_usb_open_string(context.get_ftdi_context(), desc) };
        drop(unsafe { CString::from_raw(desc) }); // String must be manually free'd

        context.check_ftdi_error(rc)?;
        Ok(Device {
            context,
            eeprom_read: false,
        })
    }

    /// Set the special event character
    pub fn set_event_char(&self, event_char: u8, enable: bool) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_event_char(
                self.context.get_ftdi_context(),
                event_char as raw::c_uchar,
                enable as raw::c_uchar,
            )
        };
        self.context.check_ftdi_error(rc)
    }

    /// Set error character
    pub fn set_error_char(&self, error_char: u8, enable: bool) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_error_char(
                self.context.get_ftdi_context(),
                error_char as raw::c_uchar,
                enable as raw::c_uchar,
            )
        };
        self.context.check_ftdi_error(rc)
    }

    pub fn set_baudrate(&self, baudrate: u32) -> Result<()> {
        let rc = unsafe { ftdic::ftdi_set_baudrate(self.context.0, baudrate as raw::c_int) };

        self.context.check_ftdi_error(rc)
    }

    pub fn set_bitmode(&self, bitmask: u8, mode: BitMode) -> Result<()> {
        let mode = match mode {
            BitMode::Reset => ftdic::ftdi_mpsse_mode::BITMODE_RESET.0,
            BitMode::Bitbang => ftdic::ftdi_mpsse_mode::BITMODE_BITBANG.0,
            BitMode::Mpsse => ftdic::ftdi_mpsse_mode::BITMODE_MPSSE.0,
            BitMode::SyncBB => ftdic::ftdi_mpsse_mode::BITMODE_SYNCBB.0,
            BitMode::Mcu => ftdic::ftdi_mpsse_mode::BITMODE_MCU.0,
            BitMode::Opto => ftdic::ftdi_mpsse_mode::BITMODE_OPTO.0,
            BitMode::Cbus => ftdic::ftdi_mpsse_mode::BITMODE_CBUS.0,
            BitMode::SyncFF => ftdic::ftdi_mpsse_mode::BITMODE_SYNCFF.0,
            BitMode::FT1284 => ftdic::ftdi_mpsse_mode::BITMODE_FT1284.0,
        };

        let rc = unsafe {
            ftdic::ftdi_set_bitmode(
                self.context.get_ftdi_context(),
                bitmask as raw::c_uchar,
                mode as raw::c_uchar,
            )
        };

        self.context.check_ftdi_error(rc)
    }

    /// Set latency timer
    /// The FTDI chip keeps data in the internal buffer for a specific amount of time if the buffer is not full yet to decrease load on the usb bus.
    pub fn set_latency_timer(&self, latency: u8) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_set_latency_timer(self.context.get_ftdi_context(), latency as raw::c_uchar)
        };

        self.context.check_ftdi_error(rc)
    }

    pub fn set_timeouts(&self, read_timeout: i32, write_timeout: i32) {
        let ctx = self.context.get_ftdi_context();
        unsafe { (*ctx).usb_read_timeout = read_timeout as raw::c_int };
        unsafe { (*ctx).usb_write_timeout = write_timeout as raw::c_int };
    }

    /// Set flowcontrol for ftdi chip
    /// Note: Do not use this function to enable XON/XOFF mode, use [`set_flow_control_xonxoff`][Device::set_flow_control_xonxoff] instead.
    pub fn set_flow_control(&self, flow_control: FlowControl) -> Result<()> {
        let flow_control = match flow_control {
            FlowControl::Disabled => ftdic::SIO_DISABLE_FLOW_CTRL,
            FlowControl::RtsCts => ftdic::SIO_RTS_CTS_HS,
            FlowControl::DtrDsr => ftdic::SIO_DTR_DSR_HS,
        };

        let rc = unsafe {
            ftdic::ftdi_setflowctrl(self.context.get_ftdi_context(), flow_control as i32)
        };

        self.context.check_ftdi_error(rc)
    }

    /// Set XON/XOFF flowcontrol for ftdi chip
    pub fn set_flow_control_xonxoff(&self, xon: u8, xoff: u8) -> Result<()> {
        let rc =
            unsafe { ftdic::ftdi_setflowctrl_xonxoff(self.context.get_ftdi_context(), xon, xoff) };

        self.context.check_ftdi_error(rc)
    }

    /// Configure read buffer chunk size. Default is 4096.
    /// This is capped to 16,384 on Linux by libftdi1
    pub fn set_read_chunk_size(&self, size: u32) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_read_data_set_chunksize(
                self.context.get_ftdi_context(),
                size as raw::c_uint,
            )
        };
        self.context.check_ftdi_error(rc)
    }

    /// Configure read buffer chunk size. Default is 4096.
    pub fn set_write_chunk_size(&self, size: u32) -> Result<()> {
        let rc = unsafe {
            ftdic::ftdi_write_data_set_chunksize(
                self.context.get_ftdi_context(),
                size as raw::c_uint,
            )
        };
        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_buffers(&self) -> Result<()> {
        let rc = unsafe { ftdic::ftdi_usb_purge_buffers(self.context.get_ftdi_context()) };

        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_rx_buffer(&self) -> Result<()> {
        let rc = unsafe { ftdic::ftdi_usb_purge_rx_buffer(self.context.get_ftdi_context()) };

        self.context.check_ftdi_error(rc)
    }

    pub fn purge_usb_tx_buffer(&self) -> Result<()> {
        let rc = unsafe { ftdic::ftdi_usb_purge_tx_buffer(self.context.get_ftdi_context()) };

        self.context.check_ftdi_error(rc)
    }

    pub fn read_pins(&self) -> Result<u8> {
        let mut pins: u8 = 0;
        let pins_ptr = std::slice::from_mut(&mut pins).as_mut_ptr();

        let rc = unsafe { ftdic::ftdi_read_pins(self.context.get_ftdi_context(), pins_ptr) };

        self.context.check_ftdi_error(rc)?;
        Ok(pins)
    }

    pub fn read_data(&self, data: &mut [u8]) -> Result<u32> {
        let raw_ptr = data.as_mut_ptr();
        let raw_len = data.len() as i32;

        let rc =
            unsafe { ftdic::ftdi_read_data(self.context.get_ftdi_context(), raw_ptr, raw_len) };

        self.context.check_ftdi_error(rc)?;
        Ok(rc as u32)
    }

    /// Reads data from the chip. Does not wait for completion of the transfer nor does it make sure that the transfer was successful.
    pub fn read_data_async<'b>(&self, mut buf: Pin<&'b mut [u8]>) -> Result<AsyncRead<'b>> {
        let res = unsafe {
            ftdic::ftdi_read_data_submit(
                self.context.get_ftdi_context(),
                buf.as_mut_ptr(),
                buf.len() as i32,
            )
        };
        if res.is_null() {
            Err(Error::LibFtdi(LibFtdiError::new(
                "Error starting async read",
            )))
        } else {
            Ok(AsyncRead {
                phantom: PhantomData::default(),
                transfer_control: res,
            })
        }
    }

    pub fn write_data(&self, data: &[u8]) -> Result<u32> {
        let raw_ptr = data.as_ptr();
        let raw_len = data.len() as i32;

        let rc =
            unsafe { ftdic::ftdi_write_data(self.context.get_ftdi_context(), raw_ptr, raw_len) };

        self.context.check_ftdi_error(rc)?;
        Ok(rc as u32)
    }

    /// Load and decode the data from the chip EEPROM
    pub fn load_eeprom_data(&mut self) -> Result<()> {
        let mut rc = unsafe { ftdic::ftdi_read_eeprom(self.context.get_ftdi_context()) };
        self.context.check_ftdi_error(rc)?;

        rc = unsafe {
            ftdic::ftdi_eeprom_decode(self.context.get_ftdi_context(), false as raw::c_int)
        };
        self.context.check_ftdi_error(rc)?;

        self.eeprom_read = true;
        Ok(())
    }

    /// Return device ID strings from the eeprom. Device needs to be connected.
    pub fn eeprom_get_strings(&mut self) -> Result<DeviceInfo> {
        if !self.eeprom_read {
            self.load_eeprom_data()?;
        }

        let mut manufacturer_buf = [0i8; 100];
        let mut description_buf = [0i8; 100];
        let mut serial_buf = [0i8; 100];

        let rc = unsafe { ftdic::ftdi_read_eeprom(self.context.get_ftdi_context()) };
        self.context.check_ftdi_error(rc)?;

        let rc = unsafe {
            ftdic::ftdi_eeprom_get_strings(
                self.context.get_ftdi_context(),
                manufacturer_buf.as_mut_ptr(),
                manufacturer_buf.len() as i32,
                description_buf.as_mut_ptr(),
                description_buf.len() as i32,
                serial_buf.as_mut_ptr(),
                serial_buf.len() as i32,
            )
        };

        self.context.check_ftdi_error(rc)?;

        let manufacturer = unsafe { CStr::from_ptr(manufacturer_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();
        let description = unsafe { CStr::from_ptr(description_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();
        let serial = unsafe { CStr::from_ptr(serial_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();

        Ok(DeviceInfo {
            manufacturer,
            description,
            serial,
        })
    }

    /// Close device
    pub fn close(self) -> Result<()> {
        let rc = unsafe { ftdic::ftdi_usb_close(self.context.get_ftdi_context()) };
        self.context.check_ftdi_error(rc)?;
        drop(self);
        Ok(())
    }
}

/// List available devices.
///
/// This uses [`to_string_lossy`][std::ffi::CStr::to_string_lossy] when copying strings from libftdi1,
/// meaning it will replace any invalid UTF-8 sequences with
/// [`U+FFFD REPLACEMENT CHARACTER`][std::char::REPLACEMENT_CHARACTER]
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let context = Context::new()?;
    let mut device_list: *mut ftdic::ftdi_device_list = std::ptr::null_mut();

    let rc =
        unsafe { ftdic::ftdi_usb_find_all(context.get_ftdi_context(), &mut device_list, 0, 0) };
    context.check_ftdi_error(rc)?;

    let mut devices = Vec::with_capacity(rc as usize);
    let mut manufacturer_buf = [0i8; 100];
    let mut description_buf = [0i8; 100];
    let mut serial_buf = [0i8; 100];

    let mut curdev = device_list;
    while !curdev.is_null() {
        let rc = unsafe {
            ftdic::ftdi_usb_get_strings(
                context.get_ftdi_context(),
                (*curdev).dev,
                manufacturer_buf.as_mut_ptr(),
                manufacturer_buf.len() as i32,
                description_buf.as_mut_ptr(),
                description_buf.len() as i32,
                serial_buf.as_mut_ptr(),
                serial_buf.len() as i32,
            )
        };
        if let Err(e) = context.check_ftdi_error(rc) {
            unsafe { ftdic::ftdi_list_free(&mut device_list) };
            return Err(e);
        }

        let manufacturer = unsafe { CStr::from_ptr(manufacturer_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();
        let description = unsafe { CStr::from_ptr(description_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();
        let serial = unsafe { CStr::from_ptr(serial_buf.as_mut_ptr()) }
            .to_string_lossy()
            .into_owned();

        devices.push(DeviceInfo {
            manufacturer,
            description,
            serial,
        });

        curdev = unsafe { (*curdev).next };
    }

    unsafe { ftdic::ftdi_list_free(&mut device_list) };
    Ok(devices)
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub description: String,
    pub serial: String,
}
