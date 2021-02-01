# `safe-ftdi`

## Purpose

`safe-ftdi` is a set of (nominally!) safe API bindings to
[`libftdi`](https://www.intra2net.com/en/developer/libftdi/), implemented
as a thin wrapper around
[`libftdi1-sys`](https://github.com/tanriol/libftdi1-sys). Functions from
`libftdi` are implemented in `safe-ftdi` on an as-needed basis, and they
are mostly named the same as their `libftdi` counterparts with the `ftdi_`
prefix stripped.

Documentation on specific functions will come soon, but the example
directory contains a reimplementation of
[mercpcl](https://github.com/cr1901/mercpcl), my old command-line
application to program the flash on the
[Mercury](https://www.micro-nova.com/mercury/)
FPGA development board using the bitbang mode of the FT245.

## Prerequisites

[`libftdi1-sys`](https://github.com/tanriol/libftdi1-sys) requires the
[`pkg-config`](https://crates.io/crates/pkg-config) crate everywhere except on Windows/MSVC, where it requires [`vcpkg`](https://crates.io/crates/vcpkg), and so
transitively `safe-ftdi` requires one of those as well. I have tested the bindings
on the Windows MSVC environment, and the GNU ABI version of `rustc`.

The library in principle compiles on stable Rust 1.34 or greater,
which is what is required by `libftdi1-sys`.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your discretion.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
