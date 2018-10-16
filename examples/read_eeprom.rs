extern crate safe_ftdi as ftdi;
use ftdi::error;

fn main() {
    let mut test = ftdi::Context::new().unwrap();
    test.open(0x0403, 0x6001).unwrap();
}
