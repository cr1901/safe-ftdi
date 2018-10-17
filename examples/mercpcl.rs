#![feature(nll)]

extern crate argparse;
extern crate safe_ftdi as ftdi;

// Rewrite of Mercury Programmer Command Line (mercpcl) utility in Rust.
// Meant to be demonstrative of functi

struct Mercury {
    context : ftdi::Context,
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
}

fn main() {
    let mut ctx = ftdi::Context::new().unwrap();
    let result = ctx.open(0, 0).unwrap_err();
    println!("{}", result);
}
