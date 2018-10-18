#![feature(nll)]

extern crate argparse;
extern crate safe_ftdi as ftdi;
extern crate bitreader;
extern crate byteorder;
#[macro_use]
extern crate bitflags;

use std::fs::{File};
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
        let mut spi_word : [u8; 17] = [0; 17];
        let mut cnt : u32 = 0;

        for b in bytes {
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

            cnt += 1;
        }

        spi_word[16] = sel.bits;
        println!("{:X?}", spi_word);

        self.context.write_data(&spi_word)?;
        Ok(cnt + 1)
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

                println!("{0:2X}", pin_vals);
                if (Pins::MISO.bits & pin_vals) != 0 {
                    curr_byte |= 1 << i;
                    println!("{}", curr_byte);
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

        self.spi_sel(DeviceSelect::IDLE)?;
        Ok(LittleEndian::read_u32(&id_arr))
    }
}

fn main() {
    let mut parser = ArgumentParser::new();
    let mut bitstream_file = String::new();

    parser.refer(&mut bitstream_file)
          .add_argument("bitstream_file", Store, "Path to bitstream file");
    parser.parse_args_or_exit();

    // let bfile = match File::open(&bitstream_file) {
    //     Ok(x) => x,
    //     Err(_) => { println!("Error: File '{}' not found", bitstream_file); return; },
    // };


    let mut merc = Mercury::new();
    merc.open().unwrap();
    merc.program_mode(true).unwrap();
    println!("{0:08X}", merc.flash_id().unwrap());
    merc.program_mode(false).unwrap();
}
