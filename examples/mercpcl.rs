#![feature(nll)]

extern crate argparse;
extern crate safe_ftdi as ftdi;
extern crate bitreader;
extern crate byteorder;
#[macro_use]
extern crate bitflags;

use std::fs::{File};
use std::result;
use std::fmt;
use std::io::{Read, Error};
use ftdi::mpsse::MpsseMode;
use argparse::{ArgumentParser, Store};
use bitreader::BitReader;
use byteorder::{ByteOrder, LittleEndian};

// Rewrite of Mercury Programmer Command Line (mercpcl) utility in Rust.
// Meant to be demonstrative of functionality more than great coding practices
// (don't use unwrap())...

struct Mercury {
    context : ftdi::Context,
}

#[derive(Debug)]
enum MercuryError<'a> {
    SafeFtdi(ftdi::error::Error<'a>),
    Timeout
}

type MercuryResult<'a, T> = result::Result<T, MercuryError<'a>>;

impl<'a> fmt::Display for MercuryError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
             MercuryError::SafeFtdi(_) => {
                write!(f, "safe-ftdi error")
            },
            MercuryError::Timeout => {
                write!(f, "timeout waiting for flash")
            }
        }
    }
}

impl<'a> std::error::Error for MercuryError<'a> {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            MercuryError::SafeFtdi(ref ftdi_err) => {
                Some(ftdi_err)
            },
            MercuryError::Timeout => {
                None
            }
        }
    }
}

impl<'a> From<ftdi::error::Error<'a>> for MercuryError<'a> {
    fn from(error: ftdi::error::Error<'a>) -> Self {
        MercuryError::SafeFtdi(error)
    }
}

// A "1" means "treat pin as output".
bitflags! {
    struct Pins: u8 {
        const CSN0 = 0b00000001;
        const CSN1 = 0b00000010;
        const SCLK = 0b00000100;
        const MISO = 0b00001000;
        const MOSI = 0b00010000;
        const PROG = 0b00100000;
        const FT245_DIR_IDLE = Self::CSN0.bits | Self::CSN1.bits | Self::SCLK.bits | Self::MOSI.bits;
        const FT245_DIR_PROG = Self::FT245_DIR_IDLE.bits | Self::PROG.bits;
    }
}

bitflags!{
    struct DeviceSelect: u8 {
        // CSN0 low selects FLASH
        // CSN1 low selects FPGA
        const FPGA = Pins::CSN0.bits;
        const FLASH = Pins::CSN1.bits;
        const IDLE = Pins::CSN0.bits | Pins::CSN0.bits; // Neither
    }
}

impl Mercury {
    fn new() -> Mercury {
        Mercury {
            context : ftdi::Context::new().unwrap()
        }
    }

    fn open(&mut self) -> ftdi::Result<()> {
        self.context.open(0x0403, 0x6001)?;
        self.context.set_baudrate(3000000)?;
        Ok(())
    }

//0x251F
//0010010100011111

    fn program_mode(&mut self, enable : bool) -> ftdi::Result<()> {
        if enable {
            self.context.set_bitmode(Pins::FT245_DIR_PROG.bits, MpsseMode::BITMODE_BITBANG)?;
        } else {
            self.context.set_bitmode(Pins::FT245_DIR_IDLE.bits, MpsseMode::BITMODE_BITBANG)?;
        }
        Ok(())
    }

    fn spi_sel(&mut self, sel : DeviceSelect) -> ftdi::Result<()> {
        self.context.write_data(std::slice::from_ref(&sel.bits))?;
        Ok(())
    }

    // If deslect should happen after calling this function, it needs to be done manually!
    // Flash expects CS to stay asserted between data out and data in (PC's point-of-view).
    fn spi_out(&mut self, bytes : &[u8], sel : DeviceSelect) -> ftdi::Result<u32> {
        let mut cnt : u32 = 0;

        for b in bytes {
            let mut spi_word : [u8; 17] = [0; 17];
            let mut bitread = BitReader::new(std::slice::from_ref(&b));
            for i in (0..16).step_by(2) {
                let curr_bit = if bitread.read_bool().unwrap() {
                    Pins::MOSI.bits
                } else {
                    0
                };

                spi_word[i] = curr_bit | sel.bits;
                spi_word[i + 1] = curr_bit | Pins::SCLK.bits | sel.bits;
            }

            spi_word[16] = sel.bits;

            // FIXME: Should "not all data was written" be considered an error?
            self.context.write_data(&spi_word)?;
            cnt += 1;
        }

        Ok(cnt)
    }

    // Expects that SCLK is low when entering this function (type 0 only).
    fn spi_in(&mut self, bytes : &mut [u8], sel : DeviceSelect) -> ftdi::Result<u32> {
        let mut cnt : u32 = 0;

        //println!("Hello!");
        for b in bytes.iter_mut() {
            let mut curr_byte = 0;
            for i in (0..8).rev() {
                let clk_hi : u8 = Pins::SCLK.bits | sel.bits;
                let clk_lo : u8 = sel.bits;
                let pin_vals : u8;

                pin_vals = self.context.read_pins()?;

                //println!("{0:2X}", pin_vals);
                if (Pins::MISO.bits & pin_vals) != 0 {
                    curr_byte |= 1 << i;
                    //println!("{}", curr_byte);
                }

                self.context.write_data(std::slice::from_ref(&clk_hi))?;
                self.context.write_data(std::slice::from_ref(&clk_lo))?;
            }

            *b = curr_byte;
            cnt += 1;
        }

        let idle = DeviceSelect::IDLE.bits;
        match self.context.write_data(std::slice::from_ref(&idle)) {
            Ok(v) => v,
            Err(e) => {return Err(e)}
        };

        Ok(cnt)
    }


    fn flash_id(&mut self) -> ftdi::Result<u32> {
        let id_cmd = 0x9F;
        let mut id_arr : [u8; 4] = [0; 4];

        match self.spi_out(std::slice::from_ref(&id_cmd), DeviceSelect::FLASH) {
            Ok(v) => v,
            Err(e) => {
                self.spi_sel(DeviceSelect::IDLE)?;
                return Err(e);
            }
        };

        match self.spi_in(&mut id_arr, DeviceSelect::FLASH) {
            Ok(v) => v,
            Err(e) => {
                self.spi_sel(DeviceSelect::IDLE)?;
                return Err(e);
            }
        };

        //self.spi_out
        self.spi_sel(DeviceSelect::IDLE)?;
        Ok(LittleEndian::read_u32(&id_arr))
    }

    fn flash_write(&mut self, buf : &[u8; 264], page_addr : u32) -> MercuryResult<()> {
        let mut write_flash_buffer_cmd = [0x84, 0x00, 0x00, 0x00];
        let mut buf_to_flash_cmd = [0x88, 0x00, 0x00, 0x00];

        let paddr_hi = (((page_addr & 0x3FF) >> 7) & 0x07) as u8;
        let paddr_lo = (((page_addr & 0x3FF) << 1) & 0xFE) as u8;

        buf_to_flash_cmd[1] = paddr_hi;
        buf_to_flash_cmd[2] = paddr_lo;

        self.spi_out(&write_flash_buffer_cmd, DeviceSelect::FLASH)?;
        self.spi_out(buf, DeviceSelect::FLASH)?;
        self.spi_sel(DeviceSelect::IDLE)?; // Stop write command.
        self.spi_out(&buf_to_flash_cmd, DeviceSelect::FLASH)?;
        self.spi_sel(DeviceSelect::IDLE)?; // Command doesn't start until CS=>high.

        self.flash_poll(300000)?;

        println!("Write page {}", page_addr);
        Ok(())
    }


    fn flash_erase(&mut self) -> MercuryResult<()> {
        let erase_sector_0a_cmd = [0x7C, 0x00, 0x00, 0x00];
        let erase_sector_0b_cmd = [0x7C, 0x00, 0x10, 0x00];
        let mut erase_sector_other_cmd = [0x7C, 0x00, 0x00, 0x00];

        self.do_erase_cmd(&erase_sector_0a_cmd, 300000)?;
        self.do_erase_cmd(&erase_sector_0b_cmd, 300000)?;

        for i in 0..16 {
            erase_sector_other_cmd[1] = i << 1; // Sectors begin at PA8, or Bit 17.
            self.do_erase_cmd(&erase_sector_other_cmd, 300000)?;
        }

        Ok(())
    }

    fn do_erase_cmd(&mut self, cmd : &[u8; 4], timeout : u32) -> MercuryResult<()> {
        self.spi_out(cmd, DeviceSelect::FLASH)?;
        self.spi_sel(DeviceSelect::IDLE)?;
        self.flash_poll(timeout)?;
        Ok(())
    }

    fn flash_poll(&mut self, timeout : u32) -> MercuryResult<()> {
        let status_read : u8 = 0xD7;
        let mut status_code : u8 = 0;

        for _a in 0..timeout {
            self.spi_out(std::slice::from_ref(&status_read), DeviceSelect::FLASH)?;
            self.spi_in(std::slice::from_mut(&mut status_code), DeviceSelect::FLASH)?;
            self.spi_sel(DeviceSelect::IDLE)?;

            if (status_code & 0x80) != 0 {
                return Ok(());
            }
        }

        Err(MercuryError::Timeout)
    }
}


fn main() {
    let mut parser = ArgumentParser::new();
    let mut bitstream_file = String::new();

    parser.refer(&mut bitstream_file)
          .add_argument("bitstream_file", Store, "Path to bitstream file")
          .required();
    parser.parse_args_or_exit();

    let mut bfile = match File::open(&bitstream_file) {
        Ok(x) => x,
        Err(_) => { println!("Error: File '{}' not found", bitstream_file); return; },
    };


    let mut merc = Mercury::new();
    merc.open().unwrap();

    merc.program_mode(true).unwrap();
    println!("Flash ID is: {0:08X}", merc.flash_id().unwrap());
    merc.program_mode(false).unwrap();

    merc.program_mode(true).unwrap();
    let mut page_buf : [u8; 264] = [0; 264];

    merc.program_mode(true).unwrap();
    merc.flash_erase().unwrap();
    merc.program_mode(false).unwrap();

    let mut last_page_written : u16 = 0;
    let mut last_page_size : u16 = 0;
    for page_num in 0..8192u16 {
        match bfile.read(&mut page_buf) {
            Ok(n) => {
                if n < page_buf.len() {
                    // Check for EOF condition.
                    // last_page_size = n as u16;
                    break;
                } else {
                    merc.flash_write(&page_buf, page_num as u32).unwrap();
                    last_page_written = page_num;
                }
            },
            Err(_) => { println!("Unexpected I/O Error."); return; }
        }
    }

    match bfile.read(&mut page_buf) {
        Ok(n) => {
            // If this was actually end of file, everything's fine.
            if n == 0 {
                // TODO: Zero-pad page_buf by 264 - last_page_size
                merc.flash_write(&page_buf, (last_page_written + 1) as u32).unwrap();
            } else {
                println!("Expected End of File- file is too large to write.");
                return;
            }
        },
        Err(_) => { println!("Unexpected I/O Error."); return; }
    }


}
