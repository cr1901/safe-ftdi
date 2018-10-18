#![feature(nll)]

extern crate argparse;
extern crate safe_ftdi as ftdi;
extern crate bitreader;

use std::fs::{File};
use ftdi::mpsse::MpsseMode;
use argparse::{ArgumentParser, Store};
use bitreader::BitReader;

// Rewrite of Mercury Programmer Command Line (mercpcl) utility in Rust.
// Meant to be demonstrative of functionality more than great coding practices
// (don't use unwrap())...

struct Mercury {
    context : ftdi::Context,
}

// CSN0-1, SCLK, MOSI outputs, MISO input
const FT245_DIR_IDLE : u8 = 0x17;
// FPGA_PROG output
const FT245_DIR_PROG : u8 = 0x37;
const CSN0 : u8 = 1 << 0;
const CSN1 : u8 = 1 << 1;
const SCLK : u8 = 1 << 2;
const MISO : u8 = 1 << 3;
const MOSI : u8 = 1 << 4;
const PROG : u8 = 1 << 5;

#[derive(Clone, Copy)]
enum DeviceSelect {
    Fpga = 0, // CSN0
    Flash = 1, // CSN1
    Idle = 2, // Neither
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

    fn program_mode(&mut self, enable : bool) -> ftdi::Result<()> {
        if enable {
            self.context.set_bitmode(FT245_DIR_PROG, MpsseMode::BITMODE_BITBANG)?;
        } else {
            self.context.set_bitmode(FT245_DIR_IDLE, MpsseMode::BITMODE_BITBANG)?;
        }
        Ok(())
    }

    // If deslect should happen after calling this function, it needs to be done manually!
    // Flash expects CS to stay asserted between data out and data in (PC's point-of-view).
    fn spi_out(&mut self, bytes : &[u8], sel : DeviceSelect) -> ftdi::Result<u32> {
        let mut spi_word : [u8; 17] = [0; 17];

        for b in bytes {
            for i in (0..16).step_by(2) {
                let mut bitread = BitReader::new(std::slice::from_ref(&b));

                let curr_bit = if bitread.read_bool().unwrap() {
                    MOSI
                } else {
                    0
                };

                spi_word[i] = curr_bit | (sel as u8);
                spi_word[i + 1] = curr_bit | SCLK | (sel as u8);
            }
        }

        spi_word[16] = (sel as u8);

        let rc = self.context.write_data(&spi_word)?;
        Ok((rc))
    }

    // Expects that SCLK is low when entering this function (type 0 only).
    fn spi_in(&mut self, bytes : &mut [u8], sel : DeviceSelect) -> ftdi::Result<u32> {
        let cnt : usize = 0;

        for b in bytes {
            let mut curr_byte = 0;
            for i in 0..8 {
                let clk_hi : u8 = SCLK | (sel as u8);
                let clk_lo : u8 = sel as u8;
                let mut pin_vals : u8;

                pin_vals = self.context.read_pins()?;

                //curr_byte |= (pin_vals & ) <<
            }
        }

        Ok(0)
    }


    fn flash_id(&mut self) -> ftdi::Result<()> {
        Ok(())
    }
}

fn main() {
    let mut parser = ArgumentParser::new();
    let mut bitstream_file = String::new();

    parser.refer(&mut bitstream_file)
          .add_argument("bitstream_file", Store, "Path to bitstream file")
          .required();
    parser.parse_args_or_exit();

    let bfile = match File::open(&bitstream_file) {
        Ok(x) => x,
        Err(_) => { println!("Error: File '{}' not found", bitstream_file); return; },
    };


    let mut merc = Mercury::new();
    merc.open().unwrap();
    merc.program_mode(false).unwrap();
}
